// Port of go-trafilatura/core.go

pub mod dom;
pub mod error;
pub mod extraction;
pub mod metadata;
pub mod options;
pub mod result;
pub mod selector;
pub mod settings;
pub mod utils;

use crate::dom::Document;
use crate::error::TrafilaturaError;
use crate::extraction::{
    baseline::baseline,
    external::compare_external_extraction,
    html_processing::{convert_tags, doc_cleaning, post_cleaning, prune_unwanted_nodes},
    {extract_comments, extract_content},
};
use crate::options::{ExtractionFocus, Options};
use crate::result::ExtractResult;
use crate::settings::FORMAT_TAG_CATALOG;
use crate::utils::{
    language::{check_html_language, language_classifier},
    lru::LruCache,
    text::duplicate_test,
};

/// Parse an HTML string and extract its main readable content.
///
/// Port of `Extract` (which reads from `io.Reader`; here we accept `&str` directly).
pub fn extract(html: &str, opts: Options) -> Result<ExtractResult, TrafilaturaError> {
    let doc = Document::parse(html);
    extract_document(doc, opts)
}

/// Extract readable content from an already-parsed `Document`.
///
/// This is the core pipeline, faithfully porting Go's `ExtractDocument`.
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
    if let Some(sel) = &opts.prune_selector.clone() {
        let root = doc.root();
        let to_remove = doc.query_selector_all(root, sel);
        for id in to_remove.into_iter().rev() {
            doc.remove(id, false);
        }
    }

    // Clone the document for fallback strategies before any destructive cleaning.
    let doc_backup1 = doc.clone_document(); // for external fallback
    let doc_backup2 = doc.clone_document(); // for baseline rescue

    // Clean and normalise tags on the main work document.
    doc_cleaning(&mut doc, &opts);
    convert_tags(&mut doc, &opts);

    // Extract comments first (comments sections are removed from `doc` as a side-effect).
    let (comments_doc, tmp_comments) = if !opts.exclude_comments {
        extract_comments(&mut doc, &mut cache, &opts)
    } else {
        if opts.focus == ExtractionFocus::FavorPrecision {
            let cleaned = prune_unwanted_nodes(&doc, selector::discard::REMOVED_COMMENTS, false);
            doc = cleaned;
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
        let mut backup = doc_backup2;
        (content_doc, tmp_body_text) = baseline(&mut backup);
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
}
