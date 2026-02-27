// Port of go-trafilatura/core.go

//! Web content extraction library.
//!
//! `trafilatura` extracts the main text, comments, and metadata from web pages,
//! stripping boilerplate (navigation, ads, footers) while preserving the
//! article body. It is a faithful Rust port of
//! [go-trafilatura](https://github.com/markusmobius/go-trafilatura).
//!
//! # Quick start
//!
//! ```rust
//! use trafilatura::{extract, Options};
//!
//! let html = r#"<html><body>
//!   <nav>Menu items</nav>
//!   <article><p>This is the main article content.</p></article>
//!   <footer>Copyright 2024</footer>
//! </body></html>"#;
//!
//! let result = extract(html, Options::default()).unwrap();
//! assert!(result.content_text.contains("main article content"));
//! ```
//!
//! # Features
//!
//! - **Content extraction** — identifies and extracts the main body text using
//!   CSS selector rules, paragraph scoring, and heuristic filters.
//! - **Comment extraction** — separately extracts user comments (optional).
//! - **Metadata** — extracts title, author, date, description, categories, tags,
//!   license, and more from meta tags, OpenGraph, and JSON-LD.
//! - **Fallback strategies** — when primary extraction yields too little content,
//!   falls back to readability-based or baseline extraction.
//! - **Language filtering** — optionally reject documents that don't match a
//!   target language (detected via `whatlang`).
//! - **Deduplication** — LRU-based detection of duplicate content across multiple
//!   extractions.
//!
//! # Builder-style options
//!
//! ```rust
//! use trafilatura::{extract, Options, ExtractionFocus};
//!
//! let html = "<html><body><article><p>Hello world</p></article></body></html>";
//! let opts = Options::default()
//!     .with_fallback(true)
//!     .with_links(true)
//!     .with_focus(ExtractionFocus::FavorRecall);
//! let result = extract(html, opts).unwrap();
//! assert_eq!(result.content_text, "Hello world");
//! ```

pub mod dom;
pub mod error;
pub(crate) mod extraction;
pub mod metadata;
pub mod options;
pub mod result;
pub(crate) mod selector;
pub(crate) mod settings;
pub mod utils;

// Convenience re-exports for library users and the CLI binary.
pub use error::TrafilaturaError;
pub use options::{Config, ExtractionFocus, FallbackCandidates, HtmlDateMode, Options};
pub use result::{ExtractResult, Metadata};

use crate::dom::Document;
use crate::extraction::{
    baseline::baseline,
    external::compare_external_extraction,
    html_processing::{convert_tags, doc_cleaning, post_cleaning, prune_unwanted_nodes},
    {extract_comments, extract_content},
};
use crate::settings::FORMAT_TAG_CATALOG;
use crate::utils::{
    language::{check_html_language, language_classifier},
    lru::LruCache,
    text::duplicate_test,
};

/// Parse an HTML string and extract its main readable content.
///
/// This is the primary entry point. It parses the HTML, extracts metadata,
/// identifies the main content and (optionally) comments, and returns
/// everything as both plain text and cleaned HTML.
///
/// # Errors
///
/// Returns [`TrafilaturaError`] if:
/// - The target language doesn't match (`LanguageMismatch`)
/// - Required metadata is missing when `has_essential_metadata` is set
/// - Extracted content is too short (`InsufficientContent`)
/// - The document is a duplicate (`DuplicateContent`)
///
/// # Example
///
/// ```rust
/// use trafilatura::{extract, Options};
///
/// let html = "<html><body><article><p>Hello world</p></article></body></html>";
/// let result = extract(html, Options::default()).unwrap();
/// assert_eq!(result.content_text, "Hello world");
/// ```
pub fn extract(html: &str, opts: Options) -> Result<ExtractResult, TrafilaturaError> {
    let doc = Document::parse(html);
    extract_document(doc, opts)
}

/// Extract readable content from an already-parsed [`Document`](dom::Document).
///
/// Use this when you have a pre-parsed DOM (e.g., from [`dom::Document::parse`])
/// and want to avoid re-parsing. The extraction pipeline is the same as [`extract`].
///
/// Port of `ExtractDocument`.
pub fn extract_document(doc: Document, opts: Options) -> Result<ExtractResult, TrafilaturaError> {
    let mut opts = opts;

    // Prepare LRU cache for duplicate detection.
    let mut cache = LruCache::new(opts.config.cache_size);

    // HTML language check (fast early-exit before expensive extraction).
    if opts.target_language.is_some() && !check_html_language(&doc, &opts, false) {
        return Err(TrafilaturaError::LanguageMismatch {
            expected: opts.target_language.clone().unwrap_or_default(),
            got: String::new(),
        });
    }

    // Extract metadata (happens before content extraction in the Go pipeline).
    let mut meta = metadata::extract_metadata(&doc, &opts);

    // Check essential metadata requirements.
    if opts.has_essential_metadata {
        if meta.title.is_empty() {
            return Err(TrafilaturaError::MissingMetadata("title".into()));
        }
        if meta.url.is_empty() {
            return Err(TrafilaturaError::MissingMetadata("url".into()));
        }
        if meta.date.is_none() {
            return Err(TrafilaturaError::MissingMetadata("date".into()));
        }
    }

    // If OriginalURL was not provided, try to use the one found in metadata.
    if opts.original_url.is_none() && !meta.url.is_empty() {
        if let Ok(parsed) = url::Url::parse(&meta.url) {
            // Mirror Go's nurl.ParseRequestURI: only accept absolute URLs with a scheme.
            if matches!(parsed.scheme(), "http" | "https") {
                opts.original_url = Some(parsed);
            }
        }
    }

    // Apply user-specified prune selector (no backup — this is under full user control).
    let mut doc = doc;
    if let Some(sel) = &opts.prune_selector {
        let root = doc.root();
        let to_remove = doc.query_selector_all(root, sel);
        for id in to_remove.into_iter().rev() {
            doc.remove(id, false);
        }
    }

    // Clone the document before destructive cleaning for fallback strategies.
    let doc_backup1 = doc.clone_document();
    let mut doc_backup2 = doc.clone_document();

    // Clean and normalise tags on the main work document.
    doc_cleaning(&mut doc, &opts);
    convert_tags(&mut doc, &opts);

    // Extract comments first (comments sections are removed from `doc` as a side-effect).
    let (comments_doc, tmp_comments) = if !opts.exclude_comments {
        extract_comments(&mut doc, &mut cache, &opts)
    } else {
        if opts.focus == ExtractionFocus::FavorPrecision {
            doc = prune_unwanted_nodes(&doc, selector::discard::REMOVED_COMMENTS, false);
        }
        (None, String::new())
    };
    let len_comments = tmp_comments.chars().count();

    // Main content extraction.
    let (mut content_doc, mut tmp_body_text) = extract_content(&doc, &mut cache, &opts);

    // External fallback comparison (readability / domdistiller).
    if opts.enable_fallback {
        (content_doc, tmp_body_text) =
            compare_external_extraction(&doc_backup1, content_doc, &opts);
    }

    // Rescue with baseline if text is still too short and we are not in precision mode.
    let len_text = tmp_body_text.chars().count();
    if len_text < opts.config.min_extracted_size && opts.focus != ExtractionFocus::FavorPrecision {
        (content_doc, tmp_body_text) = baseline(&mut doc_backup2);
    }

    // Tree size sanity check.
    if let Some(max_tree) = opts.max_tree_size {
        let content_body = content_doc.body().unwrap_or_else(|| content_doc.root());
        if content_doc.children(content_body).len() > max_tree {
            // Strip formatting tags to reduce tree size.
            let fmt_tags: Vec<&str> = FORMAT_TAG_CATALOG.iter().copied().collect();
            content_doc.strip_tags(content_body, &fmt_tags);

            let n_children = content_doc.children(content_body).len();
            if n_children > max_tree {
                return Err(TrafilaturaError::TreeTooLarge(n_children));
            }
        }
    }

    // Size checks.
    let len_text = tmp_body_text.chars().count();
    if len_text < opts.config.min_output_size && len_comments < opts.config.min_output_comment_size
    {
        return Err(TrafilaturaError::InsufficientContent {
            text_len: len_text,
            comment_len: len_comments,
        });
    }

    // Duplicate check at body level.
    if opts.deduplicate {
        let content_body = content_doc.body().unwrap_or_else(|| content_doc.root());
        if duplicate_test(&content_doc, content_body, &mut cache, &opts) {
            return Err(TrafilaturaError::DuplicateContent);
        }
    }

    // Language classification and validation.
    let lang = language_classifier(&tmp_body_text, &tmp_comments);
    if let Some(ref target) = opts.target_language {
        // Match Go's strict semantics: reject even when lang is "" (unknown).
        // Go's `lang != opts.TargetLanguage` rejects undetected language when a target is set.
        if &lang != target {
            return Err(TrafilaturaError::LanguageMismatch {
                expected: target.clone(),
                got: lang.clone(),
            });
        }
    }
    if !lang.is_empty() {
        meta.language = lang;
    }

    // Post-cleaning of content and comments trees.
    post_cleaning(&mut content_doc);
    let mut comments_doc = comments_doc;
    if let Some(ref mut cd) = comments_doc {
        post_cleaning(cd);
    }

    // Serialise to HTML strings for the result.
    let content_body = content_doc.body().unwrap_or_else(|| content_doc.root());
    let content_html = content_doc.inner_html(content_body);

    let comments_html = if let Some(ref cd) = comments_doc {
        let comments_body = cd.body().unwrap_or_else(|| cd.root());
        cd.inner_html(comments_body)
    } else {
        String::new()
    };

    Ok(ExtractResult {
        content_text: tmp_body_text,
        comments_text: tmp_comments,
        content_html,
        comments_html,
        metadata: meta,
    })
}

// ---------------------------------------------------------------------------
// Readable document builder
// ---------------------------------------------------------------------------

/// Creates a complete, self-contained HTML document from an [`ExtractResult`].
///
/// The output has metadata encoded as `<meta>` tags in `<head>`, article
/// content in `<div id="content-body">`, and comments (if any) in
/// `<div id="comments-body">`.
///
/// Port of `CreateReadableDocument`.
pub fn create_readable_document(result: &ExtractResult) -> String {
    let m = &result.metadata;

    let escape = |s: &str| {
        s.replace('&', "&amp;")
            .replace('"', "&quot;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
    };

    let date_str = match m.date {
        Some(d) => d.format("%Y-%m-%d").to_string(),
        None => String::new(),
    };

    let categories = m.categories.join(", ");
    let tags = m.tags.join("; ");

    let mut html = String::with_capacity(1024);
    html.push_str("<html><head>");

    for (name, value) in &[
        ("title", m.title.as_str()),
        ("author", m.author.as_str()),
        ("url", m.url.as_str()),
        ("hostname", m.hostname.as_str()),
        ("description", m.description.as_str()),
        ("sitename", m.sitename.as_str()),
        ("date", date_str.as_str()),
        ("categories", categories.as_str()),
        ("tags", tags.as_str()),
        ("license", m.license.as_str()),
    ] {
        html.push_str(r#"<meta name=""#);
        html.push_str(name);
        html.push_str(r#"" content=""#);
        html.push_str(&escape(value));
        html.push_str(r#""/>"#);
    }

    html.push_str("</head><body>");

    if !result.content_html.is_empty() {
        html.push_str(r#"<div id="content-body">"#);
        html.push_str(&result.content_html);
        html.push_str("</div>");
    }

    if !result.comments_html.is_empty() {
        html.push_str(r#"<div id="comments-body">"#);
        html.push_str(&result.comments_html);
        html.push_str("</div>");
    }

    html.push_str("</body></html>");
    html
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn simple_article(body: &str) -> String {
        format!("<html><head><title>Test</title></head><body>{body}</body></html>")
    }

    #[test]
    fn test_extract_basic_article() {
        let html = simple_article(
            "<article><p>This is the main content of the article. It has enough text to pass \
             the minimum size threshold for extraction and should appear in the result.</p></article>",
        );
        let result = extract(&html, Options::default()).unwrap();
        assert!(!result.content_text.is_empty(), "should extract content text");
        assert!(
            result.content_text.contains("main content"),
            "content should contain article text"
        );
    }

    #[test]
    fn test_extract_strips_scripts_and_nav() {
        let html = simple_article(
            "<nav>Navigation</nav>\
             <script>alert('x')</script>\
             <article><p>Real content here that is long enough to be extracted without \
             any issues from the minimum size requirements.</p></article>",
        );
        let result = extract(&html, Options::default()).unwrap();
        assert!(!result.content_text.contains("Navigation"), "nav should be stripped");
        assert!(!result.content_text.contains("alert"), "script should be stripped");
    }

    #[test]
    fn test_extract_empty_html_returns_error() {
        let result = extract("", Options::default());
        assert!(result.is_err(), "empty HTML should return an error");
    }

    #[test]
    fn test_extract_exclude_comments() {
        let html = simple_article(
            "<article><p>Article content that is long enough to pass the threshold for \
             minimum extracted size in the extractor pipeline.</p></article>\
             <div id=\"comments\"><p>User comment here</p></div>",
        );
        let mut opts = Options::default();
        opts.exclude_comments = true;
        let result = extract(&html, opts).unwrap();
        assert!(result.comments_text.is_empty(), "comments should be excluded");
    }

    #[test]
    fn test_extract_missing_essential_metadata_title() {
        let html = "<html><body><p>Content that is long enough to pass the minimum size \
                    threshold for the extraction algorithm to work properly.</p></body></html>";
        let mut opts = Options::default();
        opts.has_essential_metadata = true;
        let result = extract(html, opts);
        // No <title> in this document → should fail with MissingMetadata
        assert!(
            matches!(result, Err(TrafilaturaError::MissingMetadata(_))),
            "should fail with missing metadata"
        );
    }

    #[test]
    fn test_extract_favor_recall_option() {
        let html = simple_article(
            "<div class='content'><p>Some content in a div that recall mode should pick up \
             even without a standard article tag structure.</p></div>",
        );
        let mut opts = Options::default();
        opts.focus = ExtractionFocus::FavorRecall;
        // Should not error; recall mode is more permissive
        let _ = extract(&html, opts); // result may or may not have content; just check no panic
    }

    #[test]
    fn test_extract_document_returns_metadata() {
        let html = r#"<html>
            <head>
                <title>My Article Title</title>
                <meta name="author" content="Jane Doe" />
            </head>
            <body>
                <article>
                    <p>Article content that is long enough to pass the minimum size threshold
                    for the extraction algorithm to return a valid result without errors.</p>
                </article>
            </body>
        </html>"#;
        let result = extract(html, Options::default()).unwrap();
        assert!(!result.metadata.title.is_empty(), "should extract title");
    }

    #[test]
    fn test_extract_content_html_populated() {
        let html = simple_article(
            "<article><p>Content text that is long enough to pass all minimum size checks \
             and produce a non-empty HTML output in the result struct.</p></article>",
        );
        let result = extract(&html, Options::default()).unwrap();
        assert!(!result.content_html.is_empty(), "content_html should be populated");
    }

    #[test]
    fn test_extract_missing_essential_metadata_url() {
        // Document has title and enough content but no canonical URL → MissingMetadata("url")
        let html = "<html><head><title>My Title</title></head>\
                    <body><article><p>Content that is long enough to pass the minimum \
                    size threshold for the extraction algorithm.</p></article></body></html>";
        let mut opts = Options::default();
        opts.has_essential_metadata = true;
        let result = extract(html, opts);
        assert!(
            matches!(result, Err(TrafilaturaError::MissingMetadata(_))),
            "should fail: no URL in metadata"
        );
    }

    #[test]
    fn test_extract_missing_essential_metadata_date() {
        // Provide title + canonical URL but no date → MissingMetadata("date")
        let html = r#"<html>
            <head>
                <title>My Title</title>
                <link rel="canonical" href="https://example.com/article" />
            </head>
            <body><article><p>Content that is long enough to pass the minimum size
            threshold for the extraction algorithm to work correctly.</p></article></body>
        </html>"#;
        let mut opts = Options::default();
        opts.has_essential_metadata = true;
        let result = extract(html, opts);
        assert!(
            matches!(result, Err(TrafilaturaError::MissingMetadata(_))),
            "should fail: no date in metadata"
        );
    }

    #[test]
    fn test_extract_prune_selector() {
        // The prune_selector should remove matched elements before any extraction.
        let html = simple_article(
            "<article><p>Keep this content that is definitely long enough to \
             pass the minimum size threshold.</p></article>\
             <div class=\"sidebar\"><p>Remove this sidebar text.</p></div>",
        );
        let mut opts = Options::default();
        opts.prune_selector = Some(".sidebar".into());
        let result = extract(&html, opts).unwrap();
        assert!(
            !result.content_text.contains("Remove this sidebar"),
            "pruned element should not appear in output"
        );
        assert!(
            result.content_text.contains("Keep this content"),
            "non-pruned content should survive"
        );
    }

    #[test]
    fn test_extract_max_tree_size_error() {
        // Build a body with many direct children to trigger TreeTooLarge.
        let many_ps: String = (0..200)
            .map(|i| format!("<p>Paragraph number {i} with enough text.</p>"))
            .collect();
        let html = simple_article(&many_ps);
        let mut opts = Options::default();
        opts.max_tree_size = Some(10); // tiny limit → should trigger TreeTooLarge
        let result = extract(&html, opts);
        assert!(
            matches!(result, Err(TrafilaturaError::TreeTooLarge(_))),
            "should return TreeTooLarge when tree exceeds max_tree_size"
        );
    }

    #[test]
    fn test_extract_target_language_rejects_unknown() {
        // When whatlang cannot detect the language (short/ambiguous text) and a target
        // language is set, Go rejects the content. Rust should do the same.
        // Use a very short snippet that whatlang cannot classify reliably.
        let html = simple_article(
            "<article><p>Short text that is just barely long enough to pass the minimum \
             size threshold but may not be long enough to detect a language reliably.</p></article>",
        );
        let mut opts = Options::default();
        opts.target_language = Some("zh".into()); // Chinese — definitely won't match
        let result = extract(&html, opts);
        // Either the language was detected (and mismatched) or empty (and mismatched with "zh").
        // Either way, the result must be an error.
        assert!(
            matches!(result, Err(TrafilaturaError::LanguageMismatch { .. })),
            "should reject content when detected language != target language"
        );
    }

    // ---------------------------------------------------------------------------
    // create_readable_document
    // ---------------------------------------------------------------------------

    #[test]
    fn test_create_readable_document_structure() {
        let result = ExtractResult {
            content_text: "Hello world".into(),
            comments_text: String::new(),
            content_html: "<p>Hello world</p>".into(),
            comments_html: String::new(),
            metadata: crate::result::Metadata {
                title: "My Title".into(),
                author: "Jane Doe".into(),
                url: "https://example.com/article".into(),
                hostname: "example.com".into(),
                description: "A description".into(),
                sitename: "Example".into(),
                date: chrono::NaiveDate::from_ymd_opt(2023, 4, 5),
                categories: vec!["Tech".into(), "News".into()],
                tags: vec!["rust".into(), "web".into()],
                license: "CC BY 4.0".into(),
                ..Default::default()
            },
        };

        let html = create_readable_document(&result);

        // Should have proper HTML shell.
        assert!(html.starts_with("<html><head>"), "should start with html/head");
        assert!(html.ends_with("</body></html>"), "should end with /body/html");

        // Meta tags.
        assert!(html.contains(r#"name="title" content="My Title""#));
        assert!(html.contains(r#"name="author" content="Jane Doe""#));
        assert!(html.contains(r#"name="url" content="https://example.com/article""#));
        assert!(html.contains(r#"name="hostname" content="example.com""#));
        assert!(html.contains(r#"name="description" content="A description""#));
        assert!(html.contains(r#"name="sitename" content="Example""#));
        assert!(html.contains(r#"name="date" content="2023-04-05""#));
        assert!(html.contains(r#"name="categories" content="Tech, News""#));
        assert!(html.contains(r#"name="tags" content="rust; web""#));
        assert!(html.contains(r#"name="license" content="CC BY 4.0""#));

        // Content div.
        assert!(html.contains(r#"<div id="content-body">"#));
        assert!(html.contains("<p>Hello world</p>"));

        // No empty comments div when comments_html is empty.
        assert!(!html.contains(r#"id="comments-body""#));
    }

    #[test]
    fn test_create_readable_document_with_comments() {
        let result = ExtractResult {
            content_html: "<p>Article</p>".into(),
            comments_html: "<p>A comment</p>".into(),
            ..Default::default()
        };

        let html = create_readable_document(&result);
        assert!(html.contains(r#"<div id="comments-body">"#));
        assert!(html.contains("<p>A comment</p>"));
    }

    #[test]
    fn test_create_readable_document_no_date() {
        let result = ExtractResult {
            content_html: "<p>Content</p>".into(),
            ..Default::default()
        };

        let html = create_readable_document(&result);
        // Date meta tag should exist but have empty content.
        assert!(html.contains(r#"name="date" content="""#));
    }

    #[test]
    fn test_create_readable_document_escapes_special_chars() {
        let result = ExtractResult {
            content_html: "<p>Content</p>".into(),
            metadata: crate::result::Metadata {
                title: r#"Title with "quotes" & <tags>"#.into(),
                ..Default::default()
            },
            ..Default::default()
        };

        let html = create_readable_document(&result);
        // The title value in the attribute should be escaped.
        assert!(html.contains("&quot;quotes&quot;"));
        assert!(html.contains("&amp;"));
        assert!(html.contains("&lt;tags&gt;"));
    }
}
