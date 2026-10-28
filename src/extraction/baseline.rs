// Port of go-trafilatura/baseline.go (v2.2.x), which was realigned with
// Python trafilatura 2.3.1's baseline in the September 2026 upstream update
// (upstream commits 2114bfe/5ecedf5). Key semantics:
//
// - Every strategy builds a fresh body; earlier strategies' paragraphs never
//   leak into a later strategy's result.
// - The final fallback reads `<body>` text nodes only (never `<head>`), with
//   no length gate — the caller's size checks decide acceptance.
// - JSON-LD walking collects schema.org full-text properties (articleBody,
//   reviewBody, recipeInstructions, steps, FAQ answers, @graph/mainEntity),
//   Product/VideoObject descriptions as a teaser tier, and Discourse forum
//   preloads.
// - Dedupe drops substring repeats above `MIN_DUPLICATE_LENGTH` while under
//   `DEDUPE_SCAN_CAP` (nested elements re-collecting container text), not
//   just exact repeats.

use std::sync::LazyLock;

use regex::Regex;
use serde_json::Value;

use crate::dom::{Document, NodeId, MAX_TREE_DEPTH};
use crate::metadata::json_ld::get_schema_types;
use crate::settings::{DEDUPE_SCAN_CAP, MIN_DUPLICATE_LENGTH};
use crate::utils::{remove_control_characters, trim, unescape_html};

/// CSS selector for basic pre-cleaning.
const BASIC_CLEANING_SELECTOR: &str = "aside, footer, div[id*=\"footer\"], div[class*=\"footer\"], script, style, fencedframe, svg, template";

/// Cookie-consent markers matched against class and id attributes.
static RX_COOKIE_CONSENT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)cookie[-_]?(?:banner|bar|consent|law|notice|policy|description)|notice[-_]{0,2}cookie|consent[-_]?(?:banner|manager|sdk)|borlabs|cookiebot|cmplz|onetrust|moove[-_]?gdpr",
    )
    .expect("valid cookie consent regex")
});

/// Cheap pre-filter on raw script text before attempting JSON decode.
static RX_JSON_CONTENT_HOOK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"articleBody|reviewBody|recipeInstructions|acceptedAnswer|"(?:Product|VideoObject|HowTo)""#)
        .expect("valid JSON content hook regex")
});

// Real-tag detection for embedded markup: must not fire on comparison-operator
// prose ("i<b and c>d", b a tag name) — only closing tags, bare tags (<p>),
// self-closing tags (<br/>), or attribute-bearing tags (contain '='). Mirrors
// Python's `_HTML_MARKUP` and Go's `rxEmbeddedHTML`.
const HTML_TAG_NAMES: &str = "a|abbr|address|article|aside|b|blockquote|body|br|caption|cite|code|dd|del|div|dl|dt|em|figcaption|figure|footer|h[1-6]|head|header|hr|html|i|img|ins|kbd|li|main|mark|nav|ol|p|pre|q|quote|s|section|small|span|strong|sub|summary|sup|table|tbody|td|tfoot|th|thead|time|title|tr|u|ul";

static RX_EMBEDDED_HTML: LazyLock<Regex> = LazyLock::new(|| {
    let pattern =
        format!(r"(?i)</(?:{HTML_TAG_NAMES})>|<(?:{HTML_TAG_NAMES})(?:\s[^<>]*=[^<>]*)?/?>");
    Regex::new(&pattern).expect("valid embedded HTML regex")
});

/// Remove footer/aside/script/style/cookie-consent elements as a preliminary
/// cleaning step.
///
/// Port of `basicCleaning`.
pub(crate) fn basic_cleaning(doc: &mut Document) {
    let root = doc.root();
    let mut to_remove = doc.query_selector_all(root, BASIC_CLEANING_SELECTOR);
    for id in doc.iter(root, &[]) {
        let consent = RX_COOKIE_CONSENT.is_match(&doc.class_name(id))
            || doc
                .get_attribute(id, "id")
                .is_some_and(|v| RX_COOKIE_CONSENT.is_match(&v));
        if consent {
            to_remove.push(id);
        }
    }
    // Iterate in reverse so removing a parent doesn't invalidate child IDs;
    // duplicates and already-detached nodes are no-ops in `remove`.
    for id in to_remove.into_iter().rev() {
        doc.remove(id, true);
    }
}

/// Returns the items of a JSON value: arrays as-is, `null` as empty, anything
/// else as a single-element slice.
///
/// Port of `jsonItems`.
fn json_items(value: &Value) -> Vec<&Value> {
    match value {
        Value::Null => Vec::new(),
        Value::Array(items) => items.iter().collect(),
        other => vec![other],
    }
}

/// Decodes JSON, repairing raw control characters inside quoted strings on
/// retry (real-world JSON-LD often carries literal newlines/tabs).
///
/// Port of `decodeJSON`. `serde_json` (like Go's `encoding/json`) rejects
/// control characters in strings; the repair pass re-escapes them as `\u00XX`.
fn decode_json(input: &str) -> Option<Value> {
    if let Ok(value) = serde_json::from_str(input) {
        return Some(value);
    }

    let mut escaped: Vec<u8> = Vec::with_capacity(input.len());
    let mut quoted = false;
    let mut backslash = false;
    let mut changed = false;
    for &character in input.as_bytes() {
        if quoted && !backslash && character < 0x20 {
            escaped.extend_from_slice(b"\\u00");
            const HEX: &[u8; 16] = b"0123456789abcdef";
            escaped.push(HEX[(character >> 4) as usize]);
            escaped.push(HEX[(character & 0xf) as usize]);
            changed = true;
        } else {
            escaped.push(character);
        }
        if backslash {
            backslash = false;
        } else if quoted && character == b'\\' {
            backslash = true;
        } else if character == b'"' {
            quoted = !quoted;
        }
    }
    if !changed {
        return None;
    }
    let repaired = String::from_utf8(escaped).ok()?;
    serde_json::from_str(&repaired).ok()
}

/// Collects schema.org text content from parsed JSON-LD into full-text bodies
/// and teaser descriptions.
///
/// Port of `walkJSONContent`. Recursion into nested objects is deliberately
/// limited to `@graph`/`mainEntity` (matching Python 2.3.1's `_walk_json`);
/// arbitrary-key recursion was removed upstream.
fn walk_json_content(value: &Value, bodies: &mut Vec<String>, teasers: &mut Vec<String>) {
    for item in json_items(value) {
        let Some(object) = item.as_object() else {
            continue;
        };

        for key in ["articleBody", "reviewBody"] {
            if let Some(Value::String(text)) = object.get(key) {
                if !text.is_empty() {
                    bodies.push(text.clone());
                }
            }
        }

        // Recipe / how-to instructions: a string, strings, or step objects
        // carrying "text", possibly one itemListElement level down.
        for key in ["recipeInstructions", "step"] {
            for step in json_items(object.get(key).unwrap_or(&Value::Null)) {
                match step {
                    Value::String(text) => bodies.push(text.clone()),
                    step_object => {
                        let Some(step_map) = step_object.as_object() else {
                            continue;
                        };
                        let mut subs: Vec<&Value> = vec![step_object];
                        subs.extend(json_items(
                            step_map.get("itemListElement").unwrap_or(&Value::Null),
                        ));
                        for sub in subs {
                            if let Some(sub_map) = sub.as_object() {
                                if let Some(Value::String(text)) = sub_map.get("text") {
                                    bodies.push(text.clone());
                                }
                            }
                        }
                    }
                }
            }
        }

        // FAQ answers.
        if let Some(Value::Object(answer)) = object.get("acceptedAnswer") {
            if let Some(Value::String(text)) = answer.get("text") {
                bodies.push(text.clone());
            }
        }

        // Teaser tier: Product/VideoObject descriptions.
        for schema_type in get_schema_types(object, true) {
            if schema_type == "product" || schema_type == "videoobject" {
                if let Some(Value::String(text)) = object.get("description") {
                    teasers.push(text.clone());
                }
                break;
            }
        }

        for key in ["@graph", "mainEntity"] {
            walk_json_content(object.get(key).unwrap_or(&Value::Null), bodies, teasers);
        }
    }
}

/// Gathers raw text content embedded as JSON in the document.
///
/// Port of `collectJSONContent`: JSON-LD script blocks plus Discourse forum
/// posts preloaded into a `data-preloaded` attribute (the rendered page body
/// is empty for those).
fn collect_json_content(doc: &Document) -> (Vec<String>, Vec<String>) {
    let mut bodies = Vec::new();
    let mut teasers = Vec::new();

    let root = doc.root();
    for script_id in doc.query_selector_all(root, r#"script[type="application/ld+json"]"#) {
        let text = doc.text_content(script_id);
        if !RX_JSON_CONTENT_HOOK.is_match(&text) {
            continue;
        }
        if let Some(value) = decode_json(&text) {
            walk_json_content(&value, &mut bodies, &mut teasers);
        }
    }

    if let Some(preload) = doc.query_selector(root, "div[id=\"data-preloaded\"]") {
        if let Some(attr) = doc.get_attribute(preload, "data-preloaded") {
            if let Ok(Value::Object(topics)) = serde_json::from_str::<Value>(&attr) {
                let mut keys: Vec<&String> = topics.keys().collect();
                keys.sort();
                for key in keys {
                    if !key.starts_with("topic_") {
                        continue;
                    }
                    let Value::String(encoded) = &topics[key] else {
                        continue;
                    };
                    let Ok(Value::Object(topic)) = serde_json::from_str::<Value>(encoded) else {
                        continue;
                    };
                    let posts = topic
                        .get("post_stream")
                        .and_then(|stream| stream.as_object())
                        .and_then(|stream| stream.get("posts"))
                        .unwrap_or(&Value::Null);
                    for post in json_items(posts) {
                        if let Some(post_map) = post.as_object() {
                            if let Some(Value::String(cooked)) = post_map.get("cooked") {
                                bodies.push(cooked.clone());
                            }
                        }
                    }
                }
            }
        }
    }

    (bodies, teasers)
}

/// Derives clean text from an embedded-JSON value which may carry (escaped)
/// HTML markup.
///
/// Port of `renderBaselineText`: unescape first so escaped markup is parsed
/// rather than leaked, then strip tags when real ones are detected.
fn render_baseline_text(raw: &str) -> String {
    let raw = remove_control_characters(&unescape_html(raw));
    if RX_EMBEDDED_HTML.is_match(&raw) {
        let tmp = Document::parse(&format!("<html><body><div>{raw}</div></body></html>"));
        if let Some(body) = tmp.body() {
            if let Some(&div_id) = tmp.children(body).first() {
                return trim(&tmp.text_content(div_id));
            }
        }
    }
    trim(&raw)
}

/// Wraps one paragraph per text in a fresh body element, optionally dropping
/// repeated content.
///
/// Port of `buildBaselineBody`. Dedupe drops a text when it is a substring of
/// the accumulated content (nested re-collections), only above
/// `MIN_DUPLICATE_LENGTH` and while under `DEDUPE_SCAN_CAP`; short repeats
/// and cross-paragraph-boundary coincidences are kept.
fn build_baseline_body(texts: &[String], deduplicate: bool) -> (Document, String) {
    let mut result = Document::parse("<html><body></body></html>");
    let result_body = result.body().expect("parsed result document has <body>");
    let mut content = String::new();
    let mut content_length: usize = 0;

    for text in texts {
        let text = remove_control_characters(text);
        let text_length = text.chars().count();
        if text.is_empty()
            || (deduplicate
                && text_length > MIN_DUPLICATE_LENGTH
                && content_length <= DEDUPE_SCAN_CAP
                && content.contains(text.as_str()))
        {
            continue;
        }
        let p_id = result.sub_element(result_body, "p");
        result.set_text(p_id, &text);
        if !content.is_empty() {
            content.push('\n');
            content_length += 1;
        }
        content.push_str(&text);
        content_length += text_length;
    }

    (result, content)
}

/// Last-resort extraction using simple heuristics.
///
/// Tries in order: embedded JSON content → non-nested `<article>` tags →
/// text paragraphs → teaser descriptions → `<body>` text nodes.
///
/// The input document is not mutated (upstream clones before cleaning).
///
/// Port of `baseline`.
pub(crate) fn baseline(doc: &Document) -> (Document, String) {
    let mut doc = doc.clone_document();

    // -----------------------------------------------------------------
    // 1. Embedded JSON content (schema.org properties, Discourse posts)
    // -----------------------------------------------------------------
    let (bodies, teasers) = collect_json_content(&doc);
    let bodies: Vec<String> = bodies.iter().map(|b| render_baseline_text(b)).collect();
    let (body, text) = build_baseline_body(&bodies, true);
    if text.chars().count() > 100 {
        return (body, text);
    }

    // -----------------------------------------------------------------
    // 2. Basic tree cleaning before further fallbacks
    // -----------------------------------------------------------------
    basic_cleaning(&mut doc);

    // -----------------------------------------------------------------
    // 3. Scrape from <article> tags (nested ones counted in their ancestor)
    // -----------------------------------------------------------------
    let root = doc.root();
    let mut articles: Vec<String> = Vec::new();
    let mut max_length: usize = 0;
    for article_id in doc.query_selector_all(root, "article") {
        let mut nested = false;
        let mut parent = doc.parent(article_id);
        while let Some(parent_id) = parent {
            if doc.tag_name(parent_id) == "article" {
                nested = true;
                break;
            }
            parent = doc.parent(parent_id);
        }
        if nested {
            continue;
        }
        let text = trim(&doc.text_content(article_id));
        let length = text.chars().count();
        if length > 100 {
            articles.push(text);
            max_length = max_length.max(length);
        }
    }
    if !articles.is_empty() {
        // A dominant article relegates much smaller siblings to noise
        // (related teasers); similar-sized ones are all content (forum posts).
        articles.retain(|text| text.chars().count() * 5 >= max_length);
        return build_baseline_body(&articles, false);
    }

    // -----------------------------------------------------------------
    // 4. Scrape from text paragraphs, dropping nested repeats
    // -----------------------------------------------------------------
    let root = doc.root();
    let mut paragraphs: Vec<String> = Vec::new();
    for elem_id in doc.iter(root, &["blockquote", "code", "p", "pre", "q", "quote"]) {
        paragraphs.push(trim(&doc.text_content(elem_id)));
    }
    let (body, text) = build_baseline_body(&paragraphs, true);
    if text.chars().count() > 100 {
        return (body, text);
    }

    // -----------------------------------------------------------------
    // 5. Teaser tier: Product/VideoObject descriptions, once every
    //    full-text source came up empty.
    // -----------------------------------------------------------------
    let teasers: Vec<String> = teasers.iter().map(|t| render_baseline_text(t)).collect();
    let (teaser_body, teaser_text) = build_baseline_body(&teasers, true);

    // -----------------------------------------------------------------
    // 6. Default strategy: take every <body> text node.
    //    No length gate — the caller's size checks decide acceptance.
    // -----------------------------------------------------------------
    let mut parts: Vec<String> = Vec::new();
    if let Some(body_id) = doc.body() {
        collect_text_parts(&doc, body_id, 0, &mut parts);
    }
    let (body, text) = build_baseline_body(&[parts.join("\n")], false);

    // A short summary must not shadow a longer body dump — the teaser wins
    // only if the dump is shorter.
    if teaser_text.chars().count() > 100 && teaser_text.chars().count() > text.chars().count() {
        return (teaser_body, teaser_text);
    }
    (body, text)
}

/// Collects trimmed per-text-node content under a subtree in document order.
///
/// Port of the `collectText` closure in Go's `baseline`. Depth-capped like
/// every other recursive walk in this crate (Go grows goroutine stacks
/// dynamically; Rust thread stacks are fixed, so unbounded recursion on
/// pathologically nested markup would overflow).
fn collect_text_parts(doc: &Document, id: NodeId, depth: usize, parts: &mut Vec<String>) {
    if depth >= MAX_TREE_DEPTH {
        return;
    }
    for child in doc.child_nodes(id) {
        if doc.is_text(child) {
            let text = trim(&doc.text_content(child));
            if !text.is_empty() {
                parts.push(text);
            }
        } else if doc.is_element(child) {
            collect_text_parts(doc, child, depth + 1, parts);
        }
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

    fn text_of(html: &str) -> String {
        let (_, text) = baseline(&doc(html));
        text
    }

    #[test]
    fn test_baseline_blank_document() {
        let (_, result) = baseline(&doc(""));
        assert!(
            result.is_empty(),
            "blank document should produce empty text"
        );
    }

    #[test]
    fn test_baseline_invalid_html() {
        let (_, result) = baseline(&doc("<invalid html>"));
        // html5ever parses <invalid html> as an element with no text → empty
        assert!(result.is_empty(), "invalid HTML should produce empty text");
    }

    #[test]
    fn test_baseline_article_tag() {
        let content = "The article consists of this text.".repeat(10);
        let html = format!("<html><body><article>{content}</article></body></html>");
        let (_, result) = baseline(&doc(&html));
        assert!(!result.is_empty(), "should extract text from <article>");
    }

    #[test]
    fn test_baseline_article_tag_bold() {
        let d = doc("<html><body><article><b>The article consists of this text.</b></article></body></html>");
        let (_, result) = baseline(&d);
        assert!(!result.is_empty());
    }

    #[test]
    fn test_baseline_blockquote() {
        let d = doc("<html><body><blockquote>This is only a quote but it is better than nothing.</blockquote></body></html>");
        let (_, result) = baseline(&d);
        assert!(!result.is_empty(), "should extract blockquote text");
    }

    #[test]
    fn test_baseline_quote_tag() {
        // Python 2.2.0 added the non-standard <quote> tag to the scan.
        let d = doc("<html><body><quote>This is only a quote but it is better than nothing.</quote></body></html>");
        let (_, result) = baseline(&d);
        assert!(!result.is_empty(), "should extract <quote> text");
    }

    #[test]
    fn test_baseline_json_ld_invalid_json() {
        let html = r#"
            <html><body>
                <script type="application/ld+json">
                    {"articleBody": "This is the article body, it has to be long enough."  # invalid JSON
                </script>
            </body></html>"#;
        let (_, result) = baseline(&doc(html));
        assert!(
            result.is_empty(),
            "invalid JSON should produce empty result"
        );
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
        let (_, result) = baseline(&doc(html));
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
        let (_, result) = baseline(&doc(html));
        assert_eq!(
            result,
            "This is the article body, it has to be long enough to fool the length threshold which is set at len 100."
        );
    }

    #[test]
    fn test_baseline_body_text_fallback() {
        let d = doc("<html><body><div>   Document body...   </div><script> console.log('Hello world') </script></body></html>");
        let (_, result) = baseline(&d);
        assert_eq!(result, "Document body...");
    }

    #[test]
    fn test_baseline_json_ld_graph_nested() {
        // Nested JSON-LD walking is limited to @graph/mainEntity (upstream
        // removed arbitrary-key recursion in the 2.2.0 realignment).
        let html = r#"
            <html><body>
                <script type="application/ld+json">
                    {
                        "headline": "Test",
                        "@graph": [
                            {
                                "@type": "Article",
                                "articleBody": "Nested body text that is long enough to exceed the threshold of one hundred characters, comfortably padded for the gate."
                            }
                        ]
                    }
                </script>
            </body></html>"#;
        let (_, result) = baseline(&doc(html));
        assert!(
            result.contains("Nested body text"),
            "should find articleBody under @graph; got: {result}"
        );
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
        assert!(
            d.query_selector(root, "footer").is_none(),
            "footer should be removed"
        );
        assert!(
            d.query_selector(root, "aside").is_none(),
            "aside should be removed"
        );
        assert!(
            d.query_selector(root, "script").is_none(),
            "script should be removed"
        );
        // Content paragraph should survive
        assert!(d.query_selector(root, "p").is_some(), "p should remain");
    }

    #[test]
    fn test_basic_cleaning_removes_cookie_consent_and_media() {
        let html = r#"<html><body>
            <div class="CookieConsentBanner">Consent text</div>
            <div id="onetrust-consent-sdk">Consent text</div>
            <svg><title>icon</title></svg>
            <template><p>markup</p></template>
            <p>visible</p>
        </body></html>"#;
        let mut d = Document::parse(html);
        basic_cleaning(&mut d);
        let body_html = d.inner_html(d.body().unwrap());
        assert_eq!(body_html.trim(), "<p>visible</p>");
    }

    #[test]
    fn test_basic_cleaning_keeps_tail_text_of_removed_elements() {
        // Go's etree.Remove(el, true) keeps tail text of removed elements
        // (x/net/html tails are separate sibling nodes that survive removal);
        // the default strategy's body dump then still sees that text.
        let html = r#"<html><body><aside>discard me</aside> tail text survives <p>after</p></body></html>"#;
        let mut d = Document::parse(html);
        basic_cleaning(&mut d);
        let body = d.body().unwrap();
        let body_html = d.inner_html(body);
        assert!(
            body_html.contains("tail text survives"),
            "tail must survive removal; got: {body_html}"
        );
        assert!(!body_html.contains("discard me"));
    }

    #[test]
    fn test_baseline_result_has_p_elements() {
        let content = "Article content. ".repeat(10);
        let html = format!("<html><body><article>{content}</article></body></html>");
        let d = doc(&html);
        let (result_doc, _) = baseline(&d);
        let body = result_doc.body().expect("result has body");
        let paragraphs = result_doc.get_elements_by_tag_name(body, "p");
        assert!(
            !paragraphs.is_empty(),
            "result doc should contain <p> elements"
        );
    }

    #[test]
    fn test_baseline_json_ld_array_root() {
        let html = r#"
            <html><body>
                <script type="application/ld+json">
                    [{"@type": "Article", "articleBody": "Array-rooted body content that is definitely longer than one hundred characters of text, comfortably padded for the gate."}]
                </script>
            </body></html>"#;
        let (_, result) = baseline(&doc(html));
        assert!(
            result.contains("Array-rooted"),
            "should extract articleBody from array-rooted JSON-LD"
        );
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
        let (_, result) = baseline(&doc(html));
        assert!(
            result.starts_with("In letzter Zeit kam man"),
            "should start with expected prefix; got: {result}"
        );
        assert!(
            result.ends_with("erst mal überlegen."),
            "should end with expected suffix; got: {result}"
        );
    }

    // -----------------------------------------------------------------
    // Tests ported from go-trafilatura v2.2.x baseline_test.go
    // -----------------------------------------------------------------

    const FULL_TEXT: &str = "Complete article content. Complete article content. Complete article content. Complete article content. Complete article content. Complete article content. Complete article content. Complete article content. ";

    fn json_ld_doc(payload: &str, body: &str) -> String {
        format!(
            r#"<html><head><script type="application/ld+json">{payload}</script></head><body>{body}</body></html>"#
        )
    }

    #[test]
    fn test_baseline_upstream22_json_shapes() {
        for payload in [
            format!(r#"[{{"articleBody": "{FULL_TEXT}"}}]"#),
            format!(r#"{{"@graph": [{{"reviewBody": "{FULL_TEXT}"}}]}}"#),
            format!(r#"{{"recipeInstructions": [{{"text": "{FULL_TEXT}"}}]}}"#),
            format!(
                r#"{{"@type": "HowTo", "step": [{{"itemListElement": [{{"text": "{FULL_TEXT}"}}]}}]}}"#
            ),
            format!(r#"{{"mainEntity": {{"acceptedAnswer": {{"text": "{FULL_TEXT}"}}}}}}"#),
        ] {
            let text = text_of(&json_ld_doc(&payload, ""));
            assert_eq!(text, trim(FULL_TEXT), "payload: {payload}");
        }
    }

    #[test]
    fn test_baseline_upstream22_hook_not_triggered() {
        // "step" alone is not a hook (too generic); a bare step value must not
        // be picked up by any strategy.
        let text = text_of(&json_ld_doc(&format!(r#"{{"step": "{FULL_TEXT}"}}"#), ""));
        assert!(text.is_empty(), "unexpected text: {text}");
    }

    #[test]
    fn test_baseline_upstream22_render_baseline_text() {
        assert_eq!(render_baseline_text("i<b and c>d"), "i<b and c>d");
        assert_eq!(
            render_baseline_text("&lt;p&gt;Embedded &lt;b&gt;text&lt;/b&gt;&lt;/p&gt;"),
            "Embedded text"
        );
        // p<q and i<b and c>d prose must stay intact.
        let prose = "If p<q then u<s or i<b and c>d holds, with extra padding to comfortably exceed the one hundred character gate.";
        assert_eq!(render_baseline_text(prose), prose);
    }

    #[test]
    fn test_baseline_upstream22_unicode_dedupe() {
        // Unicode length is counted in chars, not bytes: the duplicate is
        // 80_000 chars > DEDUPE_SCAN_CAP/… and must still be dropped.
        let unicode_text = "中".repeat(80_000);
        let (body, text) = build_baseline_body(&[unicode_text.clone(), unicode_text.clone()], true);
        assert_eq!(text, unicode_text);
        let body_children = body.body().map(|b| body.children(b).len()).unwrap_or(0);
        assert_eq!(body_children, 1);
    }

    #[test]
    fn test_baseline_upstream22_short_json_falls_through_to_article() {
        let html = format!(
            r#"<html><body><script type="application/ld+json">{{"articleBody": "Rejected short JSON"}}</script><article>{FULL_TEXT}</article></body></html>"#
        );
        let (body_doc, text) = baseline(&doc(&html));
        assert_eq!(text, trim(FULL_TEXT));
        let body_children = body_doc
            .body()
            .map(|b| body_doc.children(b).len())
            .unwrap_or(0);
        assert_eq!(body_children, 1);
    }

    #[test]
    fn test_baseline_upstream22_nested_paragraph_not_duplicated() {
        let html = format!(
            "<html><body><blockquote>Attribution line: <p>{FULL_TEXT}</p></blockquote></body></html>"
        );
        let text = text_of(&html);
        let expected = trim(FULL_TEXT);
        assert_eq!(text.matches(&expected).count(), 1);
    }

    #[test]
    fn test_baseline_upstream22_nested_article_not_duplicated() {
        let inner =
            "Inner article content sentence repeated to build a properly long body text here. "
                .repeat(3);
        let html = format!(
            "<html><body><article>Outer wrapper. <article>{inner}</article></article></body></html>"
        );
        let text = text_of(&html);
        let expected = trim(&inner);
        assert_eq!(text.matches(expected.as_str()).count(), 1);
    }

    #[test]
    fn test_baseline_upstream22_article_dominance() {
        // A dominant article drops much smaller siblings (related teasers)…
        let main =
            "Main article content sentence repeated to build a properly dominant body. ".repeat(8);
        let teaser = "Related article teaser text that comfortably exceeds the length gate....";
        let teaser_article = format!("{teaser}{}", &teaser[..40]);
        let text = text_of(&format!(
            "<html><body><article>{main}</article><article>{teaser_article}</article></body></html>"
        ));
        assert!(text.contains("Main article content"));
        assert!(!text.contains("Related article teaser"), "got: {text}");

        // …but similar-sized ones are all kept (forum posts).
        let mut body = String::new();
        for index in 0..3 {
            body.push_str(&format!(
                "<article>Forum post number {index} with a comparable amount of discussion text in every single post here.</article>"
            ));
        }
        let text = text_of(&format!("<html><body>{body}</body></html>"));
        for index in 0..3 {
            assert!(
                text.contains(&format!("Forum post number {index}")),
                "post {index} missing: {text}"
            );
        }
    }

    #[test]
    fn test_baseline_upstream22_paragraph_dedup_keeps_short_repeats() {
        let long = "The annual conference will be held in Berlin this coming September, organizers said today.";
        let short = "held in Berlin";
        let text = text_of(&format!(
            "<html><body><p>{long}</p><p>{short}</p></body></html>"
        ));
        assert_eq!(text.matches(short).count(), 2);
    }

    #[test]
    fn test_baseline_upstream22_dedup_keeps_cross_boundary_paragraph() {
        let paragraphs: Vec<String> = [
            "The meeting concluded with remarks about the annual budget",
            "planning process that begins next week according to officials.",
            "annual budget planning process that begins next week",
        ]
        .into_iter()
        .map(String::from)
        .collect();
        let (body_doc, _) = build_baseline_body(&paragraphs, true);
        let body = body_doc.body().expect("body");
        let children = body_doc.children(body);
        assert_eq!(children.len(), 3);
        let last_text = body_doc.text(*children.last().unwrap());
        assert_eq!(last_text, paragraphs[2]);
    }

    #[test]
    fn test_baseline_upstream22_build_body_dedupe_cap() {
        let filler =
            "Opening paragraph content long enough to push the accumulated text past a tiny cap."
                .to_string();
        let duplicate =
            "This exact paragraph repeats and is well above the duplicate length gate for sure."
                .to_string();
        let (_, text) = build_baseline_body(&[filler, duplicate.clone(), duplicate.clone()], true);
        assert_eq!(text.matches(duplicate.as_str()).count(), 1);
    }

    #[test]
    fn test_baseline_upstream22_teaser_does_not_shadow_longer_body() {
        let description = "Short marketing product blurb standing in as a JSON-LD teaser, just past the one hundred character content gate here.";
        let mut paragraphs = String::new();
        for index in 0..5 {
            paragraphs.push_str(&format!(
                "<div>Real article paragraph number {index} with genuine sentence content that only the whole-body dump captures from these bare div elements on the page.</div>"
            ));
        }
        let text = text_of(&json_ld_doc(
            &format!(r#"{{"@type": "Product", "description": "{description}"}}"#),
            &paragraphs,
        ));
        assert!(
            text.contains("Real article paragraph number 3"),
            "teaser: {text}"
        );
    }

    #[test]
    fn test_baseline_upstream22_teaser_used_when_body_shorter() {
        let description = "Full product description carrying the actual page content, comfortably longer than the sparse body below and past the gate.";
        let text = text_of(&json_ld_doc(
            &format!(r#"{{"@type": "Product", "description": "{description}"}}"#),
            "<div>x</div>",
        ));
        assert!(text.contains("Full product description"), "got: {text}");
    }

    #[test]
    fn test_baseline_upstream22_description_teaser_tier() {
        let teaser = "Description teaser standing in for page content, comfortably longer than the one hundred character gate here.";
        let fulltext = "The full body text which must take precedence over the description teaser and clears the gate comfortably.";
        for schema_type in ["Product", "VideoObject"] {
            let payload = format!(r#"{{"@type": "{schema_type}", "description": "{teaser}"}}"#);
            assert!(
                text_of(&json_ld_doc(&payload, "")).contains(teaser),
                "{schema_type} teaser should be used when nothing else exists"
            );

            let both = format!(
                r#"[{{"@type": "{schema_type}", "description": "{teaser}"}}, {{"@type": "Article", "articleBody": "{fulltext}"}}]"#
            );
            for input in [
                json_ld_doc(&both, ""),
                json_ld_doc(&payload, &format!("<p>{fulltext}</p>")),
            ] {
                let text = text_of(&input);
                assert!(text.contains(fulltext), "got: {text}");
                assert!(!text.contains(teaser), "got: {text}");
            }
        }
    }

    #[test]
    fn test_baseline_upstream22_jsonld_double_embed() {
        for (text, payload) in [
            (
                "Article body text embedded twice via duplicated JSON-LD scripts, comfortably longer than the one hundred character strategy gate.",
                r#"{"articleBody": "{}"}"#,
            ),
            (
                "Video description standing in for page content here, comfortably longer than the one hundred character gate.",
                r#"{"@type": "VideoObject", "description": "{}"}"#,
            ),
        ] {
            let script = format!(
                r#"<script type="application/ld+json">{}</script>"#,
                payload.replace("{}", text)
            );
            let html = format!("<html><head>{script}{script}</head><body></body></html>");
            assert_eq!(text_of(&html).matches(text).count(), 1);
        }
    }

    #[test]
    fn test_baseline_upstream22_jsonld_content() {
        for (name, body, expected, forbidden) in [
            (
                "raw controls",
                "First line of the body text\n\twith a raw newline and a tab inside the JSON string, comfortably long enough to pass the gate.",
                "First line of the body text with a raw newline",
                "",
            ),
            (
                "markup",
                "Opening text<br/>more after a break element<div>and inside a div</div> comfortably long enough to pass the one hundred character gate.",
                "Opening text",
                "<",
            ),
            (
                "escaped markup",
                "&lt;p&gt;Escaped paragraph body text that should come out clean, comfortably long enough to pass the one hundred character gate.&lt;/p&gt;",
                "Escaped paragraph body text",
                "&lt;",
            ),
            (
                "comparison",
                "Comparing a < b in plain prose should stay intact, with padding to exceed the one hundred character gate.",
                "a < b",
                "",
            ),
            (
                "expression",
                "For x<y and z>0 the series converges absolutely, with plenty of extra padding to exceed the length gate here.",
                "x<y and z>0",
                "",
            ),
            (
                "tag letters",
                "If p<q then u<s or i<b and c>d holds, with extra padding to comfortably exceed the one hundred character gate.",
                "i<b and c>d holds",
                "",
            ),
        ] {
            let text = text_of(&json_ld_doc(&format!(r#"{{"articleBody": "{body}"}}"#), ""));
            assert!(text.contains(expected), "{name}: {text}");
            if !forbidden.is_empty() {
                assert!(!text.contains(forbidden), "{name}: {text}");
            }
            if name == "tag letters" {
                assert!(text.contains("p<q"), "{name}: {text}");
            }
        }
    }

    #[test]
    fn test_baseline_upstream22_control_chars() {
        let control = "before\u{01}after, with plenty of padding words to pass the one hundred character strategy gate comfortably.";
        for input in [
            json_ld_doc(&format!(r#"{{"articleBody": "{control}"}}"#), ""),
            format!("<html><body><p>{control}</p></body></html>"),
            format!("<html><body><div>{control}</div></body></html>"),
        ] {
            let text = text_of(&input);
            assert!(text.chars().count() > 100, "got: {text}");
            assert!(!text.contains('\u{01}'), "got: {text}");
            assert!(text.contains("before"), "got: {text}");
            assert!(text.contains("after"), "got: {text}");
        }
    }

    #[test]
    fn test_baseline_upstream22_schema_properties() {
        for (payload, expected) in [
            (
                r#"{"@type":"Recipe","recipeInstructions":[{"@type":"HowToStep","text":"Mix the flour with sugar and butter until the dough is smooth and pliable in texture."},{"@type":"HowToStep","text":"Bake for thirty five minutes at one hundred eighty degrees until golden brown on top."}]}"#,
                vec!["Mix the flour", "Bake for thirty five minutes"],
            ),
            (
                r#"{"@type":"Recipe","recipeInstructions":["Combine all the dry ingredients thoroughly in a large mixing bowl before adding any liquids.","Pour the batter into the tin and bake until a skewer inserted in the centre comes out clean."]}"#,
                vec![
                    "Combine all the dry ingredients",
                    "Pour the batter into the tin",
                ],
            ),
            (
                r#"{"@type":"FAQPage","mainEntity":[{"@type":"Question","name":"How?","acceptedAnswer":{"@type":"Answer","text":"This is the accepted answer to the question, written with a comfortable amount of words to pass the one hundred character gate."}}]}"#,
                vec!["This is the accepted answer"],
            ),
        ] {
            let text = text_of(&json_ld_doc(payload, ""));
            for expected in expected {
                assert!(text.contains(expected), "missing {expected} in: {text}");
            }
        }
    }

    #[test]
    fn test_baseline_upstream22_discourse_preload() {
        let topic = r#"{"post_stream":{"posts":[{"cooked":"<p>First forum post with a good amount of substantive discussion content in it.</p>"},{"cooked":"<p>Second forum post continuing the thread with more useful information for everyone.</p>"}]}}"#;
        let preloaded = format!(
            r#"{{"site_settings": "ignored", "topic_12": "{}"}}"#,
            topic.replace('"', "\\\"")
        );
        // Attribute values are HTML-escaped in source.
        let escaped = preloaded
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;");
        let html = format!(
            r#"<html><body><div id="data-preloaded" data-preloaded="{escaped}"></div></body></html>"#
        );
        let text = text_of(&html);
        assert!(text.contains("First forum post"), "got: {text}");
        assert!(text.contains("Second forum post"), "got: {text}");
        assert!(!text.contains("<p>"), "got: {text}");
    }

    #[test]
    fn test_baseline_upstream22_discourse_preload_malformed() {
        let paragraph = "Real paragraph content that should be extracted by the paragraph strategy, comfortably long enough for the gate.";
        // Go: html.EscapeString over `{"topic_12": "[\"not\", \"a\", \"dict\"]"}`.
        let raw_preloaded = "{\"topic_12\": \"[\\\"not\\\", \\\"a\\\", \\\"dict\\\"]\"}";
        for preloaded in [
            "[]".to_string(),
            "&quot;just a string&quot;".to_string(),
            raw_preloaded.replace('"', "&quot;"),
        ] {
            let html = format!(
                r#"<html><body><div id="data-preloaded" data-preloaded="{preloaded}"></div><p>{paragraph}</p></body></html>"#
            );
            let text = text_of(&html);
            assert!(text.contains(paragraph), "got: {text}");
        }
    }

    #[test]
    fn test_baseline_upstream22_element_input_not_mutated() {
        let html = format!(
            "<html><body><aside>side text</aside><p>{}</p></body></html>",
            "Real paragraph content on the page. ".repeat(4)
        );
        let d = doc(&html);
        let (_, text) = baseline(&d);
        assert!(text.contains("Real paragraph content"));
        assert!(!text.contains("side text"), "got: {text}");
        // The input document must be untouched (aside still present).
        assert!(d.query_selector(d.root(), "aside").is_some());
    }

    #[test]
    fn test_baseline_upstream22_strategy_fallthrough() {
        let paragraph = "Real paragraph content that should be extracted by the paragraph strategy, comfortably long enough for the gate.";
        let text = text_of(&json_ld_doc(
            r#"{"articleBody": "Too short."}"#,
            &format!("<p>{paragraph}</p>"),
        ));
        assert!(text.contains(paragraph));
        assert!(text.chars().count() > 100);
    }

    #[test]
    fn test_baseline_upstream22_jsonld_shapes_mixed_array() {
        let body = "Body text from structured data, made comfortably long enough to pass the one hundred character length threshold.";
        for payload in [
            format!(r#"{{"articleBody": "{body}"}}"#),
            format!(
                r#"{{"@context": "https://schema.org", "@graph": [{{"@type": "Article", "articleBody": "{body}"}}]}}"#
            ),
            format!(r#"[{{"@type": "Article", "articleBody": "{body}"}}]"#),
            format!(r#"[42, {{"@type": "Article", "articleBody": "{body}"}}]"#),
        ] {
            assert_eq!(text_of(&json_ld_doc(&payload, "")).matches(body).count(), 1);
        }
    }

    #[test]
    fn test_baseline_upstream22_howto() {
        let step = "Measure the table and add the overhang you want on every side before cutting any fabric at all.";
        let payload = format!(
            r#"{{"@type":"HowTo","step":[{{"@type":"HowToSection","itemListElement":{{"@type":"HowToDirection","text":"{step} Then hem the edges neatly all around the cloth."}}}}]}}"#
        );
        assert!(text_of(&json_ld_doc(&payload, "")).contains(step));

        let payload = r#"{"@type":"HowTo","step":[{"@type":"HowToStep","text":"Own step text: preheat the oven to a moderate temperature first.","itemListElement":[{"@type":"HowToDirection","text":"Set the dial to one hundred and eighty degrees."}]}]}"#;
        let text = text_of(&json_ld_doc(payload, ""));
        assert!(text.contains("preheat the oven"), "got: {text}");
        assert!(text.contains("one hundred and eighty"), "got: {text}");
    }

    #[test]
    fn test_baseline_issue13_cases() {
        // https://github.com/nchapman/trafilatura-rs/issues/13
        let (body_doc, text) = baseline(&doc("<html><body><p>tiny</p></body></html>"));
        assert_eq!(text, "tiny");
        let body_children = body_doc
            .body()
            .map(|b| body_doc.children(b).len())
            .unwrap_or(0);
        assert_eq!(body_children, 1, "no strategy-accumulated paragraphs");

        let (_, text) = baseline(&doc(
            "<html><head><title>Empty</title></head><body></body></html>",
        ));
        assert!(text.is_empty(), "title must not leak; got: {text}");
    }
    #[test]
    fn test_baseline_deeply_nested_markup_no_stack_overflow() {
        // html5ever does not cap nesting; the default-strategy walk must be
        // depth-capped like every other traversal or pathological markup
        // overflows the stack (Go grows goroutine stacks instead).
        //
        // Reachable text (within MAX_TREE_DEPTH) is still collected; content
        // nested deeper is truncated, consistent with text_content/iter_text.
        let shallow = format!(
            "<html><body>{}<p>deep text</p></body></html>",
            "<div>".repeat(400)
        );
        let (_, text) = baseline(&doc(&shallow));
        assert!(text.contains("deep text"), "got: {text}");

        let pathological = format!(
            "<html><body>{}<p>unreachable text</p></body></html>",
            "<div>".repeat(5_000)
        );
        // Must complete without stack overflow; deep content is truncated.
        let (_, text) = baseline(&doc(&pathological));
        assert!(!text.contains("unreachable text"));
    }
}
