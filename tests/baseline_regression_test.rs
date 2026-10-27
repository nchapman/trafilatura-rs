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
