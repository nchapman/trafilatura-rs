// Port of go-trafilatura/external.go

use std::collections::HashSet;
use std::sync::LazyLock;

use crate::dom::Document;
use crate::options::{ExtractionFocus, Options};
use crate::settings::VALID_TAG_CATALOG;
use crate::utils::trim;

use super::html_processing::doc_cleaning;

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

/// Compare our extraction with external fallback candidates and return the best result.
///
/// Mirrors Go's `compareExternalExtraction`: tries the readability fallback generator
/// in order, using `candidate_is_usable` to decide whether to replace the result.
///
/// Port of `compareExternalExtraction`.
pub fn compare_external_extraction(
    original_doc: &Document,
    extracted_doc: Document,
    opts: &Options,
) -> (Document, String) {
    // The extracted_doc is the full HTML document returned by extract_content.
    // Use the <body> as the text source to avoid counting <title>/<head> text,
    // matching Go's behaviour where extractedDoc is the <body> fragment.
    let text_root = extracted_doc.body().unwrap_or_else(|| extracted_doc.root());
    let extracted_text = trim(&extracted_doc.iter_text(text_root, " "));
    let len_extracted = extracted_text.chars().count();
    let mut extracted_doc = extracted_doc;

    // Bypass for FavorRecall when we already have plenty of text.
    if opts.focus == ExtractionFocus::FavorRecall
        && len_extracted > opts.config.min_extracted_size * 10
    {
        return (extracted_doc, extracted_text);
    }

    // Serialize the original doc to an HTML string for readability input.
    // In Go this is `dom.Clone(originalDoc, true)` + optional `pruneUnwantedNodes`.
    // We must get the <html> element (not the document root, which is not an element node).
    let html_root = original_doc.get_elements_by_tag_name(original_doc.root(), "html")
        .into_iter()
        .next()
        .unwrap_or_else(|| original_doc.root());
    let cleaned_html = original_doc.outer_html(html_root);

    // Try readability fallback generator only when our own extraction is empty.
    //
    // Note: go-trafilatura uses go-readability which produces cleaner, more accurate
    // output than the Rust readable-readability crate. The Rust crate sometimes picks
    // the wrong section of the page, and if we allowed it to replace a non-empty
    // trafilatura extraction, it would cause regressions. We therefore only apply it
    // when trafilatura produced nothing (len_extracted == 0), which is the case where
    // the original doc has malformed HTML that strips away in doc_cleaning.
    if len_extracted == 0 {
        if let Some(mut candidate_doc) = generate_readability_candidate(&cleaned_html) {
            // Pre-strip elements that sanitize_tree would remove (destroying their children).
            // readable-readability may keep original structural wrappers like <aside>,
            // whereas go-readability creates clean article nodes. Stripping preserves content.
            let cand_root = candidate_doc.root();
            candidate_doc.strip_tags(cand_root, &["aside", "figure", "footer", "nav"]);

            let cand_root = candidate_doc.body().unwrap_or_else(|| candidate_doc.root());
            let candidate_text = trim(&candidate_doc.iter_text(cand_root, " "));
            let len_candidate = candidate_text.chars().count();

            if candidate_is_usable(&candidate_doc, &extracted_doc, len_candidate, len_extracted, opts) {
                extracted_doc = candidate_doc;
            }
        }
    }

    // Final cleaning of the extraction result.
    sanitize_tree(&mut extracted_doc, opts);
    let text_root = extracted_doc.body().unwrap_or_else(|| extracted_doc.root());
    let extracted_text = trim(&extracted_doc.iter_text(text_root, " "));
    (extracted_doc, extracted_text)
}

/// Run the `readable-readability` algorithm on the provided HTML string and return the
/// extracted content as a `Document`.
///
/// Returns `None` if readability produces an empty result.
///
/// Port of the readability generator in `createFallbackGenerators`.
fn generate_readability_candidate(html: &str) -> Option<Document> {
    let mut readability = readable_readability::Readability::new();
    let (node, _meta) = readability.parse(html);

    // Serialize the kuchiki NodeRef to an HTML string, then parse into our Document.
    let mut output = Vec::new();
    node.serialize(&mut output).ok()?;
    let html_string = String::from_utf8(output).ok()?;

    if html_string.is_empty() {
        return None;
    }

    let doc = Document::parse(&html_string);
    let body = doc.body().unwrap_or_else(|| doc.root());
    let text = doc.text_content(body);
    if trim(&text).is_empty() {
        return None;
    }

    Some(doc)
}

/// Check if a fallback candidate is better than the current extraction result.
///
/// Port of `candidateIsUsable`.
pub fn candidate_is_usable(
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
        (candidate_big && (p_text_len == 0 || extracted_tables.len() > extracted_paragraphs.len()))
            || (opts.focus == ExtractionFocus::FavorRecall
                && extracted_heads.is_empty()
                && !candidate_headings.is_empty()
                && len_candidate > len_extracted)
    };

    let must_favor_recall = len_extracted < opts.config.min_extracted_size
        && opts.focus == ExtractionFocus::FavorRecall;
    candidate_usable || must_favor_recall
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
pub fn sanitize_tree(doc: &mut Document, opts: &Options) {
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

    // Step 4: strip any non-standard tags (e.g. custom/web-component elements) not in
    // VALID_TAG_CATALOG. Most standard HTML tags survive steps 1–3; this is a safety net
    // for unknown elements. (VALID_TAG_CATALOG intentionally covers all standard HTML tags,
    // so this step is rarely triggered in practice — equivalent to Go's validTagCatalog.)
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
}
