// Regression tests for https://github.com/nchapman/trafilatura-rs/issues/13
//
// Baseline fallback and engine fragment escaping, diffed against Python
// trafilatura 2.3.1 and go-trafilatura v2.2.x (which aligned its baseline with
// Python's in the September 2026 upstream update).

use trafilatura::{extract, Options};

fn filler_words(n: usize) -> String {
    "word ".repeat(n)
}

#[test]
fn escaped_angle_bracket_text_survives_extraction() {
    // Decoded entities in text nodes (`a &lt;b at c&gt; d`) must round-trip:
    // content_text keeps the raw characters, content_html re-escapes them.
    // Real-world hit: PEP 8's "Guido van Rossum &lt;guido at python.org&gt;".
    let html = format!(
        "<html><body><article><p>a &lt;b at c&gt; d {}</p></article></body></html>",
        filler_words(60)
    );
    let result = extract(&html, &Options::default()).expect("escaped text extracts");

    assert!(
        result.content_text.contains("a <b at c> d"),
        "text must keep the decoded address; got: {}",
        &result.content_text[..80.min(result.content_text.len())]
    );
    assert!(
        result.content_html.contains("a &lt;b at c&gt; d"),
        "html must re-escape the address; got: {}",
        &result.content_html[..80.min(result.content_html.len())]
    );
    assert!(
        !result.content_html.contains("<b>"),
        "escaped text must not become a real <b> element; got: {}",
        &result.content_html[..80.min(result.content_html.len())]
    );
}

#[test]
fn escaped_angle_bracket_text_in_plain_paragraph() {
    // Same as above but without an <article> wrapper so the paragraph goes
    // through the regular engine path.
    let html = format!(
        "<html><body><p>a &lt;b at c&gt; d {}</p></body></html>",
        filler_words(60)
    );
    let result = extract(&html, &Options::default()).expect("escaped text extracts");

    assert!(result.content_text.contains("a <b at c> d"));
    assert!(result.content_html.contains("a &lt;b at c&gt; d"));
    assert!(!result.content_html.contains("<b>"));
}

#[test]
fn short_page_html_is_not_duplicated_across_strategies() {
    // Paragraph strategy's <p>tiny</p> must not survive into the default
    // strategy's result: each strategy builds a fresh body (Python 2.3.1
    // `_build_body`, go-trafilatura `buildBaselineBody`).
    let html = "<html><body><p>tiny</p></body></html>";
    let result = extract(html, &Options::default()).expect("short page extracts");

    assert_eq!(result.content_text, "tiny");
    assert_eq!(result.content_html, "<p>tiny</p>");
}

#[test]
fn empty_body_does_not_return_title() {
    // The final fallback reads <body> text only; a page whose body is empty
    // yields no content even when <head><title> carries text (Python: None).
    let html = "<html><head><title>Empty</title></head><body></body></html>";
    let result = extract(html, &Options::default());

    assert!(
        result.is_err(),
        "empty body must not return the <title> text"
    );
}
