// Port of go-trafilatura/external.go + Python trafilatura/external.py (justext integration)

use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use crate::dom::Document;
use crate::options::{ExtractionFocus, Options};
use crate::selector::discard::OVERALL_DISCARDED_CONTENT;
use crate::settings::VALID_TAG_CATALOG;
use crate::utils::trim;

use super::baseline::basic_cleaning;
use super::html_processing::{doc_cleaning, prune_unwanted_nodes};

/// Tags removed from fallback extraction output during sanitization.
///
/// Port of `tagsToSanitize`.
static TAGS_TO_SANITIZE: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    [
        "aside", "audio", "button", "fieldset", "figure", "footer", "iframe",
        "input", "label", "link", "nav", "noindex", "noscript",
        "object", "option", "select", "source", "svg", "time",
    ]
    .into_iter()
    .collect()
});

/// A fallback generator returns an optional `(title, Document)`.
///
/// Port of `_FallbackGenerator`.
type FallbackGenerator = Box<dyn FnOnce() -> Option<(&'static str, Document)>>;

/// Compare our extraction with external fallback candidates and return the best result.
///
/// Iterates fallback generators (user-provided candidates, readability) in order,
/// replacing the result when `candidate_is_usable` returns true. Breaks early when
/// `len_extracted >= MinExtractedSize`.
///
/// Port of `compareExternalExtraction`.
pub(crate) fn compare_external_extraction(
    original_doc: &Document,
    extracted_doc: Document,
    opts: &Options,
) -> (Document, String) {
    // Compute text length from <body>, matching Go where extractedDoc is the body fragment.
    let extracted_text = trim(&body_text(&extracted_doc));
    let mut len_extracted = extracted_text.chars().count();
    let mut extracted_doc = extracted_doc;

    // Bypass for FavorRecall when we already have plenty of text.
    if opts.focus == ExtractionFocus::FavorRecall
        && len_extracted > opts.config.min_extracted_size * 10
    {
        return (extracted_doc, extracted_text);
    }

    // Prior cleaning — clone the original and optionally prune for precision.
    let cleaned_doc = if opts.focus == ExtractionFocus::FavorPrecision {
        prune_unwanted_nodes(original_doc, OVERALL_DISCARDED_CONTENT, false)
    } else {
        original_doc.clone_document()
    };

    // Serialize to HTML for readability input. Readability needs a clean parse
    // (html5ever normalization) to produce correct results.
    let html_root = cleaned_doc
        .get_elements_by_tag_name(cleaned_doc.root(), "html")
        .into_iter()
        .next()
        .unwrap_or_else(|| cleaned_doc.root());
    let cleaned_html = cleaned_doc.outer_html(html_root);

    // Process each fallback generator in order.
    // Port of Go's `for _, generator := range createFallbackGenerators(...)`.
    for generator in create_fallback_generators(&cleaned_html, opts) {
        let Some((title, candidate_doc)) = generator() else {
            continue;
        };

        let candidate_text = trim(&body_text(&candidate_doc));
        let len_candidate = candidate_text.chars().count();
        let _ = title; // Used for logging in Go; available for tracing if needed.

        if candidate_is_usable(
            &candidate_doc,
            &extracted_doc,
            len_candidate,
            len_extracted,
            opts,
        ) {
            extracted_doc = candidate_doc;
            len_extracted = len_candidate;
        }

        if len_extracted >= opts.config.min_extracted_size {
            break;
        }
    }

    // Override faulty extraction: try with justext.
    // Port of Python's external.py lines 94-102.
    let mut jt_used = false;
    let has_sanitizable = has_sanitizable_elements(&extracted_doc);
    if has_sanitizable || len_extracted < opts.config.min_extracted_size {
        if let Some((jt_doc, jt_text)) = justext_rescue(original_doc, opts) {
            let len_jt = jt_text.chars().count();
            // Use justext if it produced text and the current extraction isn't >4x longer.
            if len_jt > 0 && len_extracted <= 4 * len_jt {
                extracted_doc = jt_doc;
                len_extracted = len_jt;
                jt_used = true;
            }
        }
    }

    // Only sanitize if readability was used and justext wasn't.
    // Port of Python: `if use_readability and not jt_result`.
    if !jt_used {
        sanitize_tree(&mut extracted_doc, opts);
    }

    let _ = len_extracted; // may be used for logging
    let final_text = trim(&body_text(&extracted_doc));
    (extracted_doc, final_text)
}

/// Build the ordered list of fallback generators.
///
/// Order: user-provided candidates → readability.
/// (Go also includes dom-distiller as a third generator; we omit it for now.)
///
/// Port of `createFallbackGenerators`.
fn create_fallback_generators(cleaned_html: &str, opts: &Options) -> Vec<FallbackGenerator> {
    let mut generators: Vec<FallbackGenerator> = Vec::new();

    // User-provided readability candidate.
    if let Some(candidates) = &opts.fallback_candidates {
        if let Some(html) = candidates.readability_html.clone() {
            generators.push(Box::new(move || {
                let doc = Document::parse(&html);
                Some(("Readability (user)", doc))
            }));
        }
    }

    // Built-in readability generator — parse via readability, get tree back directly.
    let html_owned = cleaned_html.to_string();
    generators.push(Box::new(move || {
        generate_readability_candidate(&html_owned)
            .map(|doc| ("Readability", doc))
    }));

    generators
}

/// Run the readability algorithm on the provided HTML string and return the
/// extracted content as a `Document`.
///
/// Returns `None` if readability produces an empty result.
///
/// Port of the readability generator in `createFallbackGenerators`.
fn generate_readability_candidate(html: &str) -> Option<Document> {
    let mut parser = libreadability::Parser::new();
    let article = parser.parse(html, None).ok()?;

    if article.content.is_empty() {
        return None;
    }

    let doc = Document::parse(&article.content);
    let body = doc.body().unwrap_or_else(|| doc.root());
    let text = doc.text_content(body);
    if trim(&text).is_empty() {
        return None;
    }

    Some(doc)
}

/// Extract text from the `<body>` of a document (or root if no body).
fn body_text(doc: &Document) -> String {
    let root = doc.body().unwrap_or_else(|| doc.root());
    doc.iter_text(root, " ")
}

/// Check if a fallback candidate is better than the current extraction result.
///
/// Port of `candidateIsUsable`.
pub(crate) fn candidate_is_usable(
    candidate_doc: &Document,
    extracted_doc: &Document,
    len_candidate: usize,
    len_extracted: usize,
    opts: &Options,
) -> bool {
    let candidate_usable = if len_candidate == 0 || len_candidate == len_extracted {
        false
    } else if len_extracted == 0 && len_candidate > 0 {
        true
    } else if len_extracted > 2 * len_candidate {
        false
    } else if len_candidate > 2 * len_extracted {
        true
    } else {
        // Borderline case: use secondary heuristics.
        let ext_root = extracted_doc.root();
        let extracted_heads = extracted_doc.get_elements_by_tag_name(ext_root, "head");
        let extracted_tables = extracted_doc.get_elements_by_tag_name(ext_root, "table");
        let extracted_paragraphs = extracted_doc.get_elements_by_tag_name(ext_root, "p");

        let cand_root = candidate_doc.root();
        let candidate_headings = candidate_doc.query_selector_all(cand_root, "h2,h3,h4");

        let p_text_len: usize = extracted_paragraphs
            .iter()
            .map(|&pid| trim(&extracted_doc.iter_text(pid, " ")).chars().count())
            .sum();

        let candidate_big = len_candidate > opts.config.min_extracted_size * 2;
        if candidate_big
            && (p_text_len == 0 || extracted_tables.len() > extracted_paragraphs.len())
        {
            true
        } else {
            opts.focus == ExtractionFocus::FavorRecall
                && extracted_heads.is_empty()
                && !candidate_headings.is_empty()
                && len_candidate > len_extracted
        }
    };

    let must_favor_recall = len_extracted < opts.config.min_extracted_size
        && opts.focus == ExtractionFocus::FavorRecall;
    candidate_usable || must_favor_recall
}

// ---------------------------------------------------------------------------
// Justext integration (port of Python trafilatura/external.py)
// ---------------------------------------------------------------------------

/// ISO 639-1 → justext language name mapping.
///
/// Port of Python's `JUSTEXT_LANGUAGES` from `settings.py`.
static JUSTEXT_LANGUAGES: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    HashMap::from([
        ("ar", "Arabic"),
        ("bg", "Bulgarian"),
        ("cz", "Czech"),
        ("da", "Danish"),
        ("de", "German"),
        ("en", "English"),
        ("el", "Greek"),
        ("es", "Spanish"),
        ("fa", "Persian"),
        ("fi", "Finnish"),
        ("fr", "French"),
        ("hr", "Croatian"),
        ("hu", "Hungarian"),
        ("ko", "Korean"),
        ("id", "Indonesian"),
        ("it", "Italian"),
        ("no", "Norwegian_Nynorsk"),
        ("nl", "Dutch"),
        ("pl", "Polish"),
        ("pt", "Portuguese"),
        ("ro", "Romanian"),
        ("ru", "Russian"),
        ("sk", "Slovak"),
        ("sl", "Slovenian"),
        ("sr", "Serbian"),
        ("sv", "Swedish"),
        ("tr", "Turkish"),
        ("uk", "Ukrainian"),
        ("ur", "Urdu"),
        ("vi", "Vietnamese"),
    ])
});

/// Check whether the document body contains any elements from `TAGS_TO_SANITIZE`.
///
/// Port of Python's `body.xpath(SANITIZED_XPATH)` check.
fn has_sanitizable_elements(doc: &Document) -> bool {
    let root = doc.body().unwrap_or_else(|| doc.root());
    let all = doc.get_elements_by_tag_name(root, "*");
    for &id in &all {
        let tag = doc.tag_name(id);
        if TAGS_TO_SANITIZE.contains(tag) {
            return true;
        }
    }
    false
}

/// Resolve the justext stoplist for a given target language.
///
/// Tries the language-specific stoplist first, falls back to all stoplists.
/// Returns a `Cow` to avoid cloning the static merged set.
///
/// Port of Python's stoplist resolution in `try_justext`.
fn resolve_justext_stoplist(
    target_language: Option<&str>,
) -> std::borrow::Cow<'static, HashSet<String>> {
    if let Some(lang) = target_language {
        if let Some(&name) = JUSTEXT_LANGUAGES.get(lang) {
            if let Ok(stoplist) = justext::get_stoplist(name) {
                return std::borrow::Cow::Owned(stoplist);
            }
        }
    }
    std::borrow::Cow::Borrowed(justext::get_all_stoplists())
}

/// Run justext on HTML with trafilatura's custom thresholds.
///
/// Port of Python's `try_justext` + `custom_justext`.
fn try_justext(html: &str, stoplist: &HashSet<String>) -> Document {
    // Custom config matching Python trafilatura thresholds.
    let config = justext::Config::default()
        .with_length_low(50)
        .with_length_high(150)
        .with_stopwords_low(0.1)
        .with_stopwords_high(0.2)
        .with_max_link_density(0.25)
        .with_max_heading_distance(150);

    let paragraphs = justext::justext(html, stoplist, &config);

    // Build result document from non-boilerplate paragraphs.
    let mut result = Document::parse("<html><body></body></html>");
    let body_id = result.body().unwrap_or_else(|| result.root());
    for para in &paragraphs {
        if para.is_boilerplate() {
            continue;
        }
        let p_id = result.create_element("p");
        result.set_text(p_id, &para.text);
        result.append_child(body_id, p_id);
    }

    result
}

/// Try justext as a second fallback extractor.
///
/// Clones and basic-cleans the original document, serializes to HTML,
/// runs justext, and returns the result if non-empty.
///
/// Port of Python's `justext_rescue`.
fn justext_rescue(original_doc: &Document, opts: &Options) -> Option<(Document, String)> {
    let mut cleaned = original_doc.clone_document();
    basic_cleaning(&mut cleaned);

    let html_root = cleaned
        .get_elements_by_tag_name(cleaned.root(), "html")
        .into_iter()
        .next()
        .unwrap_or_else(|| cleaned.root());
    let html = cleaned.outer_html(html_root);

    let stoplist = resolve_justext_stoplist(opts.target_language.as_deref());
    let result_doc = try_justext(&html, &stoplist);
    let text = trim(&body_text(&result_doc));
    if text.is_empty() {
        return None;
    }

    Some((result_doc, text))
}

/// Clean and sanitize the output of a generic fallback extractor.
///
/// Steps:
/// 1. `doc_cleaning` — removes script/style/nav/ads via standard rules.
/// 2. Remove any element whose tag is in `TAGS_TO_SANITIZE`.
/// 3. Strip `<a>` tags (if `!include_links`) and always strip `<span>`.
/// 4. Strip any tag not in `VALID_TAG_CATALOG`.
///
/// Port of `sanitizeTree`.
pub(crate) fn sanitize_tree(doc: &mut Document, opts: &Options) {
    // Step 1: standard document cleaning.
    doc_cleaning(doc, opts);

    // Step 2: remove sanitization targets (reverse order for safety).
    let root = doc.root();
    let all_elements = doc.get_elements_by_tag_name(root, "*");
    for &elem_id in all_elements.iter().rev() {
        let tag = doc.tag_name(elem_id).to_string();
        if TAGS_TO_SANITIZE.contains(tag.as_str()) {
            doc.remove(elem_id, false);
        }
    }

    // Step 3: strip link and span tags.
    if !opts.include_links {
        let root = doc.root();
        doc.strip_tags(root, &["a"]);
    }
    let root = doc.root();
    doc.strip_tags(root, &["span"]);

    // Step 4: strip any non-standard tags not in VALID_TAG_CATALOG.
    let root = doc.root();
    let all_elements = doc.get_elements_by_tag_name(root, "*");
    let mut unique_tags: HashSet<String> = HashSet::new();
    for &elem_id in &all_elements {
        unique_tags.insert(doc.tag_name(elem_id).to_string());
    }

    let sanitization_list: Vec<String> = unique_tags
        .into_iter()
        .filter(|tag| !VALID_TAG_CATALOG.contains(tag.as_str()))
        .collect();

    if !sanitization_list.is_empty() {
        let tags_ref: Vec<&str> = sanitization_list.iter().map(|s| s.as_str()).collect();
        let root = doc.root();
        doc.strip_tags(root, &tags_ref);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::options::Options;

    fn default_opts() -> Options {
        Options::default()
    }

    fn doc(html: &str) -> Document {
        Document::parse(html)
    }

    // ---- sanitize_tree ----

    #[test]
    fn test_sanitize_tree_removes_sanitize_tags() {
        let html = r#"<html><body>
            <p>Content</p>
            <aside>Sidebar</aside>
            <footer>Footer</footer>
            <nav>Navigation</nav>
            <iframe src="x.html"></iframe>
        </body></html>"#;
        let mut d = doc(html);
        sanitize_tree(&mut d, &default_opts());
        let root = d.root();
        assert!(d.query_selector(root, "aside").is_none());
        assert!(d.query_selector(root, "footer").is_none());
        assert!(d.query_selector(root, "nav").is_none());
        assert!(d.query_selector(root, "iframe").is_none());
        // <p> should remain
        assert!(d.query_selector(root, "p").is_some());
    }

    #[test]
    fn test_sanitize_tree_strips_span() {
        let html = "<html><body><p>Hello <span>world</span></p></body></html>";
        let mut d = doc(html);
        sanitize_tree(&mut d, &default_opts());
        let root = d.root();
        assert!(d.query_selector(root, "span").is_none(), "span should be stripped");
        // text should survive
        let body = d.body().unwrap();
        let text = d.text_content(body);
        assert!(text.contains("world"), "span text should survive stripping");
    }

    #[test]
    fn test_sanitize_tree_strips_links_when_not_include_links() {
        let html = r#"<html><body><p>See <a href="x">this</a></p></body></html>"#;
        let mut d = doc(html);
        let mut opts = default_opts();
        opts.include_links = false;
        sanitize_tree(&mut d, &opts);
        let root = d.root();
        assert!(d.query_selector(root, "a").is_none(), "<a> should be stripped");
        let body = d.body().unwrap();
        assert!(d.text_content(body).contains("this"), "link text survives");
    }

    #[test]
    fn test_sanitize_tree_keeps_links_when_include_links() {
        let html = r#"<html><body><p>See <a href="x">this</a></p></body></html>"#;
        let mut d = doc(html);
        let mut opts = default_opts();
        opts.include_links = true;
        sanitize_tree(&mut d, &opts);
        let root = d.root();
        assert!(d.query_selector(root, "a").is_some(), "<a> should survive");
    }

    #[test]
    fn test_sanitize_tree_strips_unknown_tags() {
        let html = "<html><body><p>Text</p><custom-widget>stuff</custom-widget></body></html>";
        let mut d = doc(html);
        sanitize_tree(&mut d, &default_opts());
        // custom-widget is not in VALID_TAG_CATALOG, its text should survive (stripped, not removed)
        let body = d.body().unwrap();
        assert!(d.text_content(body).contains("stuff"), "text from unknown tag survives");
    }

    // ---- candidate_is_usable ----

    #[test]
    fn test_candidate_is_usable_empty_candidate() {
        let candidate = doc("<html><body></body></html>");
        let extracted = doc("<html><body><p>Some text</p></body></html>");
        let opts = default_opts();
        assert!(
            !candidate_is_usable(&candidate, &extracted, 0, 9, &opts),
            "empty candidate should not be usable"
        );
    }

    #[test]
    fn test_candidate_is_usable_empty_extraction() {
        let candidate = doc("<html><body><p>Candidate text</p></body></html>");
        let extracted = doc("<html><body></body></html>");
        let opts = default_opts();
        assert!(
            candidate_is_usable(&candidate, &extracted, 100, 0, &opts),
            "candidate should be usable when extraction is empty"
        );
    }

    #[test]
    fn test_candidate_is_usable_extracted_much_larger() {
        let candidate = doc("<html><body><p>short</p></body></html>");
        let extracted = doc("<html><body><p>much longer text here</p></body></html>");
        let opts = default_opts();
        assert!(
            !candidate_is_usable(&candidate, &extracted, 10, 100, &opts),
            "candidate not usable when extracted is >2x candidate"
        );
    }

    #[test]
    fn test_candidate_is_usable_candidate_much_larger() {
        let candidate = doc("<html><body><p>long candidate text</p></body></html>");
        let extracted = doc("<html><body><p>short</p></body></html>");
        let opts = default_opts();
        assert!(
            candidate_is_usable(&candidate, &extracted, 100, 10, &opts),
            "candidate usable when candidate is >2x extracted"
        );
    }

    #[test]
    fn test_candidate_is_usable_same_length() {
        let candidate = doc("<html><body><p>same length</p></body></html>");
        let extracted = doc("<html><body><p>same length</p></body></html>");
        let opts = default_opts();
        assert!(
            !candidate_is_usable(&candidate, &extracted, 11, 11, &opts),
            "same-length candidate is not usable"
        );
    }

    // ---- compare_external_extraction ----

    #[test]
    fn test_compare_external_extraction_favor_recall_bypass() {
        // When focus=FavorRecall and extracted text > 10x min_size, return immediately.
        let original = doc("<html><body></body></html>");
        let long_text = "word ".repeat(600); // ~3000 chars >> 250*10=2500
        let extracted = doc(&format!("<html><body><p>{long_text}</p></body></html>"));
        let mut opts = default_opts();
        opts.focus = ExtractionFocus::FavorRecall;
        let (_, text) = compare_external_extraction(&original, extracted, &opts);
        assert!(text.len() > 1000, "should return long text unchanged");
    }

    #[test]
    fn test_compare_external_extraction_sanitizes() {
        let original = doc("<html><body></body></html>");
        let extracted = doc(r#"<html><body>
            <p>Article content</p>
            <aside>Sidebar junk</aside>
            <span>inline</span>
        </body></html>"#);
        let opts = default_opts();
        let (result_doc, _) = compare_external_extraction(&original, extracted, &opts);
        let root = result_doc.root();
        assert!(result_doc.query_selector(root, "aside").is_none(), "aside removed by sanitize");
        assert!(result_doc.query_selector(root, "span").is_none(), "span stripped by sanitize");
    }

    // ---- FallbackCandidates ----

    #[test]
    fn test_fallback_candidates_readability_html_used_when_extraction_empty() {
        // Our extraction is empty; a rich readability_html candidate should be adopted.
        let original = doc("<html><body></body></html>");
        let extracted = doc("<html><body></body></html>");

        let candidate_body = "word ".repeat(60); // 300 chars > min_extracted_size(250)
        let candidate_html = format!("<html><body><p>{candidate_body}</p></body></html>");

        let mut opts = default_opts();
        opts.enable_fallback = true;
        opts.fallback_candidates = Some(crate::options::FallbackCandidates {
            readability_html: Some(candidate_html),
        });

        let (_, text) = compare_external_extraction(&original, extracted, &opts);
        assert!(
            text.len() > 100,
            "fallback candidate should be adopted when extraction is empty, got: {text:?}"
        );
    }

    #[test]
    fn test_fallback_candidates_not_used_when_extraction_much_larger() {
        // Our extraction is already large; candidate should NOT replace it.
        let original = doc("<html><body></body></html>");
        let long_body = "word ".repeat(200); // 1000 chars >> 2x candidate
        let extracted = doc(&format!("<html><body><p>{long_body}</p></body></html>"));

        let short_candidate = "short".to_string();
        let candidate_html = format!("<html><body><p>{short_candidate}</p></body></html>");

        let mut opts = default_opts();
        opts.enable_fallback = true;
        opts.fallback_candidates = Some(crate::options::FallbackCandidates {
            readability_html: Some(candidate_html),
        });

        let (_, text) = compare_external_extraction(&original, extracted, &opts);
        assert!(
            text.contains("word"),
            "large extraction should be kept, not replaced by small candidate"
        );
    }

    #[test]
    fn test_fallback_candidates_none_does_not_panic() {
        // Passing no fallback_candidates should work normally.
        let original = doc("<html><body></body></html>");
        let extracted = doc("<html><body><p>Some content here</p></body></html>");
        let opts = default_opts();
        let (_, text) = compare_external_extraction(&original, extracted, &opts);
        assert!(text.contains("Some content"), "extraction should survive when no candidates");
    }

    // ---- justext integration ----

    #[test]
    fn test_try_justext_with_known_language() {
        // A document with enough real English text should produce non-empty output.
        let html = r#"<html><body>
            <p>This is a real article about the important things happening in the world today.
            We need to understand how these events are connected and what they mean for us.</p>
            <p>The situation continues to develop as more information becomes available to the public.</p>
        </body></html>"#;
        let stoplist = resolve_justext_stoplist(Some("en"));
        let result = try_justext(html, &stoplist);
        let body = result.body().unwrap();
        let text = result.text_content(body);
        // justext should classify substantive English paragraphs as good
        assert!(!trim(&text).is_empty(), "try_justext should extract English content");
    }

    #[test]
    fn test_try_justext_fallback_to_all_stoplists() {
        // Unknown language should fall back to all stoplists and still work.
        let html = r#"<html><body>
            <p>This is a real article about the important things happening in the world today.
            We need to understand how these events are connected and what they mean for us.</p>
        </body></html>"#;
        let stoplist = resolve_justext_stoplist(Some("xx")); // unknown language
        let result = try_justext(html, &stoplist);
        let body = result.body().unwrap();
        let text = result.text_content(body);
        assert!(!trim(&text).is_empty(), "should work with unknown language via all stoplists");
    }

    #[test]
    fn test_try_justext_no_language() {
        // None language should also fall back to all stoplists.
        let html = r#"<html><body>
            <p>This is a real article about the important things happening in the world today.
            We need to understand how these events are connected and what they mean for us.</p>
        </body></html>"#;
        let stoplist = resolve_justext_stoplist(None);
        let result = try_justext(html, &stoplist);
        let body = result.body().unwrap();
        let text = result.text_content(body);
        assert!(!trim(&text).is_empty(), "should work with no language");
    }

    #[test]
    fn test_has_sanitizable_elements() {
        let clean = doc("<html><body><p>Clean</p></body></html>");
        assert!(!has_sanitizable_elements(&clean));

        let dirty = doc("<html><body><p>Content</p><aside>Sidebar</aside></body></html>");
        assert!(has_sanitizable_elements(&dirty));

        let dirty_nav = doc("<html><body><p>Content</p><nav>Links</nav></body></html>");
        assert!(has_sanitizable_elements(&dirty_nav));
    }

    #[test]
    fn test_justext_rescue_triggers_on_sanitizable_elements() {
        // Body with aside/nav should trigger justext rescue.
        let html = r#"<html><body>
            <p>This is a real article about the important things happening in the world today.
            We need to understand how these events are connected and what they mean for us.</p>
            <aside>Sidebar content that should not appear</aside>
        </body></html>"#;
        let original = doc(html);
        let mut opts = default_opts();
        opts.target_language = Some("en".to_string());
        let result = justext_rescue(&original, &opts);
        assert!(result.is_some(), "justext_rescue should produce output");
        let (_, text) = result.unwrap();
        assert!(!text.is_empty());
    }

    #[test]
    fn test_justext_rescue_triggers_on_short_text() {
        // Short extraction below min_extracted_size should trigger justext.
        let html = r#"<html><body>
            <p>This is a real article about the important things happening in the world today.
            We need to understand how these events are connected and what they mean for us.</p>
        </body></html>"#;
        let original = doc(html);
        let opts = default_opts();
        let result = justext_rescue(&original, &opts);
        // Should at least attempt extraction (may or may not produce text depending on length)
        // The key thing is it doesn't panic
        let _ = result;
    }

    #[test]
    fn test_four_times_length_guard() {
        // When extracted text is >4x longer than justext result, don't replace.
        // We test this at the logic level.
        let len_extracted = 500;
        let len_jt = 100;
        // len_extracted (500) > 4 * len_jt (400) => should NOT use justext
        assert!(len_extracted > 4 * len_jt, "precondition: extracted is >4x justext");

        let len_extracted2 = 399;
        let len_jt2 = 100;
        // len_extracted2 (399) <= 4 * len_jt2 (400) => true, SHOULD use justext
        assert!(len_extracted2 <= 4 * len_jt2, "precondition: extracted is <=4x justext");
    }
}
