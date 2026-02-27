// Port of go-trafilatura/baseline.go

use std::collections::HashSet;

use crate::dom::Document;
use crate::utils::{trim, unescape_html};

/// CSS selector for basic pre-cleaning.
const BASIC_CLEANING_SELECTOR: &str =
    "aside, footer, div[id*=\"footer\"], div[class*=\"footer\"], script, style";

/// Remove footer/aside/script/style elements as a preliminary cleaning step.
///
/// Port of `basicCleaning`.
pub(crate) fn basic_cleaning(doc: &mut Document) {
    let root = doc.root();
    let to_remove = doc.query_selector_all(root, BASIC_CLEANING_SELECTOR);
    // Iterate in reverse so removing a parent doesn't invalidate child IDs.
    for id in to_remove.into_iter().rev() {
        doc.remove(id, false);
    }
}

/// Last-resort extraction using simple heuristics.
///
/// Tries in order: JSON-LD `articleBody` → `<article>` tag → text paragraphs →
/// `<body>` text → full document text.
///
/// Returns a new Document whose `<body>` contains extracted `<p>` elements and
/// the corresponding plain-text string.
///
/// Port of `baseline`.
pub(crate) fn baseline(doc: &mut Document) -> (Document, String) {
    let mut result = Document::parse("<html><body></body></html>");
    let result_body = result.body().expect("parsed result document has <body>");
    let mut tmp_text = String::new();

    // -----------------------------------------------------------------
    // 1. Scrape JSON-LD for articleBody
    // -----------------------------------------------------------------
    let root = doc.root();
    let scripts = doc.query_selector_all(root, r#"script[type="application/ld+json"]"#);
    for script_id in scripts {
        let json_text = trim(&doc.text_content(script_id));
        let json_text = unescape_html(&json_text);
        if json_text.is_empty() {
            continue;
        }

        let data: serde_json::Value = match serde_json::from_str(&json_text) {
            Ok(v) => v,
            Err(_) => continue,
        };

        if let Some(article_body) = find_article_body(&data) {
            let article_body = trim(&article_body);
            if !article_body.is_empty() {
                let p_id = result.sub_element(result_body, "p");
                result.set_text(p_id, &article_body);
                tmp_text.push(' ');
                tmp_text.push_str(&article_body);
            }
        }
    }

    let tmp_trimmed = trim(&tmp_text);
    if tmp_trimmed.chars().count() > 100 {
        return (result, tmp_trimmed);
    }

    // -----------------------------------------------------------------
    // 2. Basic tree cleaning before further fallbacks
    // -----------------------------------------------------------------
    basic_cleaning(doc);

    // -----------------------------------------------------------------
    // 3. Scrape from <article> tag
    // -----------------------------------------------------------------
    let root = doc.root();
    if let Some(article_id) = doc.query_selector(root, "article") {
        let article_text = trim(&doc.text_content(article_id));
        if article_text.chars().count() > 100 {
            let p_id = result.sub_element(result_body, "p");
            result.set_text(p_id, &article_text);
            tmp_text.push(' ');
            tmp_text.push_str(&article_text);
        }
    }

    if !result.children(result_body).is_empty() {
        return (result, trim(&tmp_text));
    }

    // -----------------------------------------------------------------
    // 4. Scrape from text paragraphs (deduplicated)
    // -----------------------------------------------------------------
    let root = doc.root();
    let elements = doc.iter(root, &["blockquote", "pre", "q", "code", "p"]);
    let mut seen: HashSet<String> = HashSet::new();
    for elem_id in elements {
        let entry = trim(&doc.text_content(elem_id));
        if entry.is_empty() {
            continue;
        }
        if seen.insert(entry.clone()) {
            let p_id = result.sub_element(result_body, "p");
            result.set_text(p_id, &entry);
            tmp_text.push(' ');
            tmp_text.push_str(&entry);
        }
    }

    let tmp_trimmed = trim(&tmp_text);
    if tmp_trimmed.chars().count() > 100 {
        return (result, tmp_trimmed);
    }

    // -----------------------------------------------------------------
    // 5. Default strategy: take <body> text with iter_text
    // -----------------------------------------------------------------
    if let Some(body_id) = doc.body() {
        let text = trim(&doc.iter_text(body_id, "\n"));
        if text.chars().count() > 100 {
            let p_id = result.sub_element(result_body, "p");
            result.set_text(p_id, &text);
            return (result, text);
        }
    }

    // -----------------------------------------------------------------
    // 6. Final fallback: full document text content
    // -----------------------------------------------------------------
    let text = trim(&doc.text_content(doc.root()));
    let p_id = result.sub_element(result_body, "p");
    result.set_text(p_id, &text);
    (result, text)
}

/// Recursively search for `"articleBody"` (case-insensitive) in a JSON value.
///
/// When the value contains HTML `<p>` tags, strips them by parsing the HTML
/// and returning the plain-text content.
fn find_article_body(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::Object(map) => {
            for (key, val) in map {
                if key.to_lowercase() == "articlebody" {
                    if let serde_json::Value::String(s) = val {
                        let s = trim(s);
                        if !s.is_empty() {
                            if s.contains("<p>") {
                                // Strip HTML: parse fragment, get plain text.
                                let tmp = Document::parse(&format!(
                                    "<html><body><div>{s}</div></body></html>"
                                ));
                                if let Some(body) = tmp.body() {
                                    if let Some(&div_id) = tmp.children(body).first() {
                                        return Some(trim(&tmp.text_content(div_id)));
                                    }
                                }
                            }
                            return Some(s);
                        }
                    }
                    // Key matched but value was not a usable string: do not recurse into it.
                    // Go's type switch falls into `case string` only, silently ignoring
                    // non-string values for the articleBody key.
                    continue;
                }
                // Recurse into nested objects and arrays for non-articleBody keys.
                if let Some(found) = find_article_body(val) {
                    return Some(found);
                }
            }
            None
        }
        serde_json::Value::Array(arr) => {
            for item in arr {
                if let Some(found) = find_article_body(item) {
                    return Some(found);
                }
            }
            None
        }
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(html: &str) -> Document {
        Document::parse(html)
    }

    #[test]
    fn test_baseline_blank_document() {
        let mut d = doc("");
        let (_, result) = baseline(&mut d);
        assert!(result.is_empty(), "blank document should produce empty text");
    }

    #[test]
    fn test_baseline_invalid_html() {
        let mut d = doc("<invalid html>");
        let (_, result) = baseline(&mut d);
        // html5ever parses <invalid html> as an element with no text → empty
        assert!(result.is_empty(), "invalid HTML should produce empty text");
    }

    #[test]
    fn test_baseline_article_tag() {
        let content = "The article consists of this text.".repeat(10);
        let html = format!("<html><body><article>{content}</article></body></html>");
        let mut d = doc(&html);
        let (_, result) = baseline(&mut d);
        assert!(!result.is_empty(), "should extract text from <article>");
    }

    #[test]
    fn test_baseline_article_tag_bold() {
        let mut d = doc("<html><body><article><b>The article consists of this text.</b></article></body></html>");
        let (_, result) = baseline(&mut d);
        assert!(!result.is_empty());
    }

    #[test]
    fn test_baseline_blockquote() {
        let mut d = doc("<html><body><blockquote>This is only a quote but it is better than nothing.</blockquote></body></html>");
        let (_, result) = baseline(&mut d);
        assert!(!result.is_empty(), "should extract blockquote text");
    }

    #[test]
    fn test_baseline_json_ld_invalid_json() {
        let html = r#"
            <html><body>
                <script type="application/ld+json">
                    {"articleBody": "This is the article body, it has to be long enough."  # invalid JSON
                </script>
            </body></html>"#;
        let mut d = doc(html);
        let (_, result) = baseline(&mut d);
        assert!(result.is_empty(), "invalid JSON should produce empty result");
    }

    #[test]
    fn test_baseline_json_ld_ok() {
        let html = r#"
            <html><body>
                <script type="application/ld+json">
                    {
                        "@type": "Article",
                        "articleBody": "This is the article body, it has to be long enough to fool the length threshold which is set at len 100."
                    }
                </script>
            </body></html>"#;
        let mut d = doc(html);
        let (_, result) = baseline(&mut d);
        assert_eq!(
            result,
            "This is the article body, it has to be long enough to fool the length threshold which is set at len 100."
        );
    }

    #[test]
    fn test_baseline_json_ld_html_stripped() {
        let html = r#"
            <html><body>
                <script type="application/ld+json">
                    {
                        "@type": "Article",
                        "articleBody": "<p>This is the article body, it has to be long enough to fool the length threshold which is set at len 100.</p>"
                    }
                </script>
            </body></html>"#;
        let mut d = doc(html);
        let (_, result) = baseline(&mut d);
        assert_eq!(
            result,
            "This is the article body, it has to be long enough to fool the length threshold which is set at len 100."
        );
    }

    #[test]
    fn test_baseline_body_text_fallback() {
        let mut d = doc("<html><body><div>   Document body...   </div><script> console.log('Hello world') </script></body></html>");
        let (_, result) = baseline(&mut d);
        assert_eq!(result, "Document body...");
    }

    #[test]
    fn test_baseline_json_ld_nested() {
        // articleBody is nested inside another key
        let html = r#"
            <html><body>
                <script type="application/ld+json">
                    {
                        "headline": "Test",
                        "nested": {
                            "articleBody": "Nested body text that is long enough to exceed the threshold of one hundred characters."
                        }
                    }
                </script>
            </body></html>"#;
        let mut d = doc(html);
        let (_, result) = baseline(&mut d);
        assert!(result.contains("Nested body text"), "should find nested articleBody");
    }

    #[test]
    fn test_basic_cleaning_removes_footer() {
        let html = r#"<html><body>
            <p>Content paragraph.</p>
            <footer>Footer text</footer>
            <aside>Sidebar</aside>
            <script>alert('x')</script>
        </body></html>"#;
        let mut d = Document::parse(html);
        basic_cleaning(&mut d);
        let root = d.root();
        assert!(d.query_selector(root, "footer").is_none(), "footer should be removed");
        assert!(d.query_selector(root, "aside").is_none(), "aside should be removed");
        assert!(d.query_selector(root, "script").is_none(), "script should be removed");
        // Content paragraph should survive
        assert!(d.query_selector(root, "p").is_some(), "p should remain");
    }

    #[test]
    fn test_baseline_result_has_p_elements() {
        let content = "Article content. ".repeat(10);
        let html = format!("<html><body><article>{content}</article></body></html>");
        let mut d = doc(&html);
        let (result_doc, _) = baseline(&mut d);
        let body = result_doc.body().expect("result has body");
        let paragraphs = result_doc.get_elements_by_tag_name(body, "p");
        assert!(!paragraphs.is_empty(), "result doc should contain <p> elements");
    }

    #[test]
    fn test_baseline_json_ld_array_root() {
        // Rust can decode array-rooted JSON-LD (Go silently fails since it expects a map).
        // Our find_article_body handles arrays via the Array branch.
        let html = r#"
            <html><body>
                <script type="application/ld+json">
                    [{"@type": "Article", "articleBody": "Array-rooted body content that is definitely longer than one hundred characters of text."}]
                </script>
            </body></html>"#;
        let mut d = doc(html);
        let (_, result) = baseline(&mut d);
        assert!(result.contains("Array-rooted"), "should extract articleBody from array-rooted JSON-LD");
    }

    #[test]
    fn test_baseline_json_ld_realworld_brigitte() {
        // Real-world test ported from Go's Test_Baseline (brigitte.de article).
        let html = r#"<html><body>
            <script type="application/ld+json">
            {
                "description": "In letzter Zeit kam man am Begriff \"Hygge\" nicht vorbei.",
                "articleBody": "In letzter Zeit kam man am Begriff \"Hygge\" (\"gemütlich\" oder \"angenehm\") nicht vorbei. Jetzt macht ihm ein neuer Glücks-Trend Konkurrenz: \"Ikigai\". Bist du glücklich? Schwierige Frage, nicht wahr? Viele von uns müssen da erst mal überlegen.",
                "@type": "NewsArticle"
            }
            </script>
        </body></html>"#;
        let mut d = doc(html);
        let (_, result) = baseline(&mut d);
        assert!(
            result.starts_with("In letzter Zeit kam man"),
            "should start with expected prefix; got: {result}"
        );
        assert!(
            result.ends_with("erst mal überlegen."),
            "should end with expected suffix; got: {result}"
        );
    }
}
