// Port of go-trafilatura/trafilatura_test.go (Test_HtmlProcessing, Test_ExoticTags gaps,
// Test_Images, Test_Links, Test_PruneSelector, Test_External gaps,
// Test_NonStdHtmlEntities, Test_MixedContentExtraction, Test_LargeDocPerformance)

use trafilatura::options::{Config, ExtractionFocus, Options};
use trafilatura::result::ExtractResult;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn zero_config() -> Config {
    let mut c = Config::default();
    c.min_extracted_size = 0;
    c.min_output_size = 0;
    c
}

fn zero_opts() -> Options {
    let mut o = Options::default();
    o.config = zero_config();
    o.enable_fallback = true;
    o
}

fn extract(html: &str, opts: &Options) -> Option<ExtractResult> {
    trafilatura::extract(html, opts).ok()
}

fn read_simple_fixture(name: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("test-files/simple")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("Failed to read fixture {name}: {e}"))
}

// ---------------------------------------------------------------------------
// Test_HtmlProcessing — paywall (integration-testable portion only)
// ---------------------------------------------------------------------------

/// Port of Test_HtmlProcessing paywall case.
///
/// Internal processNode/handleTextNode tests are omitted because those are
/// package-private Go functions not accessible from integration tests.
#[test]
fn test_paywall_removal() {
    let opts = {
        let mut o = Options::default();
        o.config = zero_config();
        o
    };
    let html = r#"<html><body><main><p>1</p><p id="premium">2</p><p>3</p></main></body></html>"#;
    let result = extract(html, &opts).expect("extraction should succeed");
    assert_eq!("1 3", result.content_text);
}

// ---------------------------------------------------------------------------
// Test_ExoticTags gaps
// ---------------------------------------------------------------------------

/// Misformed HTML declaration should still extract content.
#[test]
fn test_exotic_misformed_html() {
    let html = r#"<!DOCTYPE HTML PUBLIC "-//W3C//DTD HTML 4.01 Transitional//EN" 2012"http://www.w3.org/TR/html4/loose.dtd"><html><head></head><body><p>ABC</p></body></html>"#;
    let result = extract(html, &zero_opts()).expect("extraction should succeed");
    assert!(
        result.content_text.contains("ABC"),
        "Expected 'ABC' in content, got: {:?}",
        result.content_text
    );
}

/// Empty blockquote → empty content (handleQuotes returns nil).
#[test]
fn test_exotic_empty_blockquote() {
    let html = r#"<html><body><article><blockquote></blockquote></article></body></html>"#;
    let opts = {
        let mut o = Options::default();
        o.config = zero_config();
        o
    };
    let result = extract(html, &opts);
    let text = result.map(|r| r.content_text).unwrap_or_default();
    assert!(
        text.is_empty(),
        "Expected empty content for empty blockquote, got: {text:?}"
    );
}

/// Empty table → empty/nil content (handleTable returns nil).
#[test]
fn test_exotic_empty_table() {
    let html = r#"<html><body><article><table></table></article></body></html>"#;
    let opts = {
        let mut o = Options::default();
        o.config = zero_config();
        o
    };
    let result = extract(html, &opts);
    let text = result.map(|r| r.content_text).unwrap_or_default();
    assert!(
        text.is_empty(),
        "Expected empty content for empty table, got: {text:?}"
    );
}

/// Nested `<p>` tags — both parts should be extracted.
#[test]
fn test_exotic_nested_p() {
    // HTML5 parsers split nested <p>; both parts should appear in extraction.
    let html = r#"<html><body><article><p>1st part. <p>2nd part.</p></p></article></body></html>"#;
    let opts = {
        let mut o = Options::default();
        o.config = zero_config();
        o
    };
    let result = extract(html, &opts).expect("extraction should succeed");
    assert!(
        result.content_text.contains("1st part"),
        "Expected '1st part' in content, got: {:?}",
        result.content_text
    );
    assert!(
        result.content_text.contains("2nd part"),
        "Expected '2nd part' in content, got: {:?}",
        result.content_text
    );
}

/// HTML5 `<details>/<summary>` elements should be extracted.
#[test]
fn test_exotic_details_summary() {
    let html = r#"<html><body><article><details><summary>Epcot Center</summary><p>Epcot is a theme park at Walt Disney World Resort featuring exciting attractions, international pavilions, award-winning fireworks and seasonal special events.</p></details></article></body></html>"#;
    let opts = {
        let mut o = Options::default();
        o.config = zero_config();
        o
    };
    let result = extract(html, &opts).expect("extraction should succeed");
    assert!(
        result.content_text.contains("Epcot Center"),
        "Expected 'Epcot Center' in content, got: {:?}",
        result.content_text
    );
    assert!(
        result.content_text.contains("award-winning fireworks"),
        "Expected 'award-winning fireworks' in content, got: {:?}",
        result.content_text
    );
}

/// Edge case: `<strong><a></a></strong>` with headings/paragraphs → not empty.
#[test]
fn test_exotic_strong_empty_anchor() {
    let html = r#"
    <!DOCTYPE html>
    <html>
    <head>
        <meta charset="UTF-8">
        <title>A weird bug</title>
    </head>
    <body>
        <div>
            <h1>Lorem ipsum dolor sit amet, consectetur adipiscing elit.</h1>
            <h2>Sed et interdum lectus.</h2>
            <p>Quisque molestie nunc eu arcu condimentum fringilla.</p>
            <!-- strong can be changed to b, em, i, u, or kbd -->
            <strong><a></a></strong>
            <h2>Aliquam eget interdum elit, id posuere ipsum.</h2>
            <p>Phasellus lectus erat, hendrerit sed tortor ac, dignissim vehicula metus.<br/></p>
        </div>
    </body>
    </html>"#;
    let opts = {
        let mut o = Options::default();
        o.include_links = true;
        o.include_images = true;
        o
    };
    let result = extract(html, &opts).expect("extraction should succeed");
    assert!(
        !result.content_text.is_empty(),
        "Expected non-empty content, got empty"
    );
}

/// `<em>` wrapping `<p>`: both the em text and trailing paragraph text extracted.
#[test]
fn test_exotic_em_wrapping_p() {
    let html = r#"
    <html>
    <head>
        <meta charset="UTF-8">
        <title>A weird bug</title>
    </head>
    <body>
        <div id="content">
            <h1>A header</h1>
            <h2>Very specific bug so odd</h2>
            <h3>Nested header</h3>
            <p>Some "hyphenated-word quote" followed by a bit more text line.</p>
            <em>
                <p>em improperly wrapping p here</p>
            </em>
            <p>Text here<br/></p>
            <h3>More articles</h3>
        </div>
    </body>
    </html>"#;

    for focus in [
        ExtractionFocus::Balanced,
        ExtractionFocus::FavorRecall,
        ExtractionFocus::FavorPrecision,
    ] {
        let opts = {
            let mut o = Options::default();
            o.include_links = true;
            o.include_images = true;
            o.focus = focus;
            o
        };
        let result = extract(html, &opts).expect("extraction should succeed");
        assert!(
            result
                .content_text
                .contains("em improperly wrapping p here"),
            "focus={focus:?}: Expected 'em improperly wrapping p here' in content, got: {:?}",
            result.content_text
        );
        assert!(
            result.content_text.ends_with("Text here"),
            "focus={focus:?}: Expected content to end with 'Text here', got: {:?}",
            result.content_text
        );
    }
}

// ---------------------------------------------------------------------------
// Test_Images
// ---------------------------------------------------------------------------

/// With include_images=false (default), image is not in output.
#[test]
fn test_images_excluded_by_default() {
    let html = read_simple_fixture("http_sample.html");
    let opts = {
        let mut o = Options::default();
        o.config = zero_config();
        o
    };
    let result = extract(&html, &opts).expect("extraction should succeed");
    assert!(
        !result
            .content_html
            .contains(r#"<img src="test.jpg" title="Example image"/>"#),
        "Image should not appear when include_images=false"
    );
}

/// With include_images=true, image element is preserved in output.
#[test]
fn test_images_included_when_opted_in() {
    let html = read_simple_fixture("http_sample.html");
    let opts = {
        let mut o = Options::default();
        o.config = zero_config();
        o.include_images = true;
        o
    };
    let result = extract(&html, &opts).expect("extraction should succeed");
    // The scraper serializer may reorder attributes and drop the XML self-closing slash.
    assert!(
        result.content_html.contains("test.jpg") && result.content_html.contains("<img"),
        "Image should appear when include_images=true; html was: {}",
        result.content_html
    );
}

/// `data-src` attribute is promoted to `src` in the output.
#[test]
fn test_images_data_src_promoted() {
    let html = r#"<html><body><article><p><img data-src="test.jpg" alt="text" title="a title"/></p></article></body></html>"#;
    let opts = {
        let mut o = Options::default();
        o.config = zero_config();
        o.include_images = true;
        o
    };
    let result = extract(html, &opts).expect("extraction should succeed");
    // data-src must be promoted to src; attribute order may vary.
    assert!(
        result.content_html.contains(r#"src="test.jpg""#) && result.content_html.contains("<img"),
        "data-src should be promoted to src; html was: {}",
        result.content_html
    );
}

/// `data-src-small` attribute is promoted to `src` in the output.
#[test]
fn test_images_data_src_small_promoted() {
    let html = r#"<html><body><article><div><p><img data-src-small="test.jpg" alt="text" title="a title"/></p></div></article></body></html>"#;
    let opts = {
        let mut o = Options::default();
        o.config = zero_config();
        o.include_images = true;
        o
    };
    let result = extract(html, &opts).expect("extraction should succeed");
    // data-src-small must be promoted to src; attribute order may vary.
    assert!(
        result.content_html.contains(r#"src="test.jpg""#) && result.content_html.contains("<img"),
        "data-src-small should be promoted to src; html was: {}",
        result.content_html
    );
}

/// Image with `other=` attribute (no valid src) → not extracted; result is empty.
#[test]
fn test_images_no_valid_src_attr() {
    let html = r#"<html><body><article><p><img other="test.jpg" alt="text" title="a title"/></p></article></body></html>"#;
    let opts = {
        let mut o = Options::default();
        o.config = zero_config();
        o.include_images = true;
        o
    };
    let result = extract(html, &opts);
    // Image-only content with no valid src → extraction returns nothing.
    let html_out = result.map(|r| r.content_html).unwrap_or_default();
    assert!(
        html_out.is_empty() || html_out == "<body></body>",
        "Image with no valid src should yield empty; got: {html_out:?}"
    );
}

/// data:image/jpeg;base64 URIs are rejected.
#[test]
fn test_images_data_uri_rejected() {
    let html = r#"<html><body><article><p><img src="data:image/jpeg;base64,iVBORw0KGgoAAAANSUhEUgAAAAUAAAAFCAYAAACNbyblAAAAHElEQVQI12P4//8/w38GIAXDIBKE0DHxgljNBAAO9TXL0Y4OHwAAAABJRU5ErkJggg==" alt="text"/></p></article></body></html>"#;
    let opts = {
        let mut o = Options::default();
        o.config = zero_config();
        o.include_images = true;
        o
    };
    let result = extract(html, &opts);
    // data: URI images are rejected — result is empty (None or empty body).
    let html_out = result.map(|r| r.content_html).unwrap_or_default();
    assert!(
        html_out.is_empty() || html_out == "<body></body>",
        "data: URI images should be rejected; got: {html_out:?}"
    );
}

/// data-src inside a nested div is also promoted to src.
#[test]
fn test_images_nested_div_data_src() {
    let html = r#"<html><body><article><div><p><img data-src="test.jpg" alt="text" title="a title"/></p></div></article></body></html>"#;
    let opts = {
        let mut o = Options::default();
        o.config = zero_config();
        o.include_images = true;
        o
    };
    let result = extract(html, &opts).expect("extraction should succeed");
    // Attribute order may vary; verify src is promoted.
    assert!(
        result.content_html.contains(r#"src="test.jpg""#) && result.content_html.contains("<img"),
        "Nested data-src should be promoted; html was: {}",
        result.content_html
    );
}

// ---------------------------------------------------------------------------
// Test_Links
// ---------------------------------------------------------------------------

/// With include_links=false (default), links are stripped but text is kept.
#[test]
fn test_links_excluded_by_default() {
    let html = r#"<html><body><p><a href="testlink.html">Test link text.</a>This part of the text has to be long enough.</p></body></html>"#;
    let opts = zero_opts();
    let result = extract(html, &opts).expect("extraction should succeed");
    assert!(
        !result.content_html.contains("testlink.html"),
        "href should not appear when include_links=false"
    );
}

/// With include_links=true, link href is preserved.
#[test]
fn test_links_included_when_opted_in() {
    let html = r#"<html><body><p><a href="testlink.html">Test link text.</a>This part of the text has to be long enough.</p></body></html>"#;
    let opts = {
        let mut o = Options::default();
        o.include_links = true;
        o.config = zero_config();
        o
    };
    let result = extract(html, &opts).expect("extraction should succeed");
    assert!(
        result
            .content_html
            .contains(r#"<a href="testlink.html">Test link text.</a>"#),
        "href should appear when include_links=true; html was: {}",
        result.content_html
    );
}

/// Link-heavy content with FavorPrecision focus → high link-density paragraph excluded.
#[test]
fn test_links_high_density_excluded_precision() {
    let html = format!(
        r#"<html><body><article><p><a>{}</a></p></article></body></html>"#,
        "abcd".repeat(20)
    );
    let opts = {
        let mut o = Options::default();
        o.config = zero_config();
        o.focus = ExtractionFocus::FavorPrecision;
        o
    };
    let result = extract(&html, &opts);
    let text = result.map(|r| r.content_text).unwrap_or_default();
    assert!(
        text.is_empty(),
        "High link-density content should be excluded in FavorPrecision mode, got: {text:?}"
    );
}

/// Same link-heavy content with Balanced mode → included.
#[test]
fn test_links_high_density_included_balanced() {
    let html = format!(
        r#"<html><body><article><p><a>{}</a></p></article></body></html>"#,
        "abcd".repeat(20)
    );
    let opts = {
        let mut o = Options::default();
        o.config = zero_config();
        o.focus = ExtractionFocus::Balanced;
        o
    };
    let result = extract(&html, &opts);
    let text = result.map(|r| r.content_text).unwrap_or_default();
    assert!(
        text.contains("abcd"),
        "Balanced mode should include high link-density content, got: {text:?}"
    );
}

/// Anchor without href: extracted as-is when include_links=true.
#[test]
fn test_links_without_href() {
    let html = r#"<html><body><p><a>Test link text.</a>This part of the text has to be long enough.</p></body></html>"#;
    let opts = {
        let mut o = Options::default();
        o.include_links = true;
        o.config = zero_config();
        o
    };
    let result = extract(html, &opts).expect("extraction should succeed");
    assert!(
        result.content_html.contains("<a>Test link text.</a>"),
        "Anchor without href should be preserved; html was: {}",
        result.content_html
    );
}

/// Links in various positions (standalone, in h1, in p) all contribute text.
#[test]
fn test_links_various_positions() {
    let html = r#"<html><body><article><a>Segment 1</a><h1><a>Segment 2</a></h1><p>Segment 3</p></article></body></html>"#;
    let opts = {
        let mut o = Options::default();
        o.include_links = true;
        o.config = zero_config();
        o
    };
    let result = extract(html, &opts).expect("extraction should succeed");
    assert!(
        result.content_text.contains('1'),
        "Expected '1' in content text, got: {:?}",
        result.content_text
    );
    assert!(
        result.content_text.contains('2'),
        "Expected '2' in content text, got: {:?}",
        result.content_text
    );
    assert!(
        result.content_text.contains('3'),
        "Expected '3' in content text, got: {:?}",
        result.content_text
    );
}

/// From fixture file: testlink.html absent without include_links, present with it.
#[test]
fn test_links_from_fixture() {
    let html = read_simple_fixture("http_sample.html");

    let result_no_links = extract(&html, &zero_opts()).expect("extraction should succeed");
    assert!(
        !result_no_links.content_html.contains("testlink.html"),
        "testlink.html should not appear when include_links=false"
    );

    let opts_with_links = {
        let mut o = Options::default();
        o.include_links = true;
        o.config = zero_config();
        o
    };
    let result_with_links = extract(&html, &opts_with_links).expect("extraction should succeed");
    assert!(
        result_with_links.content_html.contains("testlink.html"),
        "testlink.html should appear when include_links=true; html was: {}",
        result_with_links.content_html
    );
}

/// `rel="license"` links have their href stripped.
#[test]
fn test_links_license_rel_stripped() {
    let html = r#"<html><body><p>Test text under <a rel="license" href="">CC BY-SA license</a>.</p></body></html>"#;
    let opts = {
        let mut o = Options::default();
        o.include_links = true;
        o.config = zero_config();
        o
    };
    let result = extract(html, &opts).expect("extraction should succeed");
    assert!(
        result.content_html.contains("<a>CC BY-SA license</a>"),
        "License links should have href stripped; html was: {}",
        result.content_html
    );
}

/// Relative link conversion: when OriginalURL is set, relative hrefs are made absolute.
#[test]
fn test_links_relative_url_conversion() {
    let html = r#"<html><body><p><a href="testlink.html">Test link text.</a>This part of the text has to be long enough.</p></body></html>"#;
    let original_url = url::Url::parse("https://www.example.com").expect("valid URL");
    let opts = {
        let mut o = Options::default();
        o.include_links = true;
        o.config = zero_config();
        o.original_url = Some(original_url);
        o
    };
    let result = extract(html, &opts).expect("extraction should succeed");
    assert!(
        result
            .content_html
            .contains(r#"<a href="https://www.example.com/testlink.html">"#),
        "Relative link should be made absolute; html was: {}",
        result.content_html
    );
}

// ---------------------------------------------------------------------------
// Test_PruneSelector
// ---------------------------------------------------------------------------

fn prune_opts(selector: &str) -> Options {
    let mut o = Options::default();
    o.config = zero_config();
    o.enable_fallback = true;
    o.prune_selector = Some(selector.to_string());
    o
}

/// 50 `<p>abc</p>` elements with PruneSelector="p" → empty.
#[test]
fn test_prune_selector_all_p_removed() {
    let html = format!("<html><body>{}</body></html>", "<p>abc</p>".repeat(50));
    let result = extract(&html, &prune_opts("p"));
    let text = result.map(|r| r.content_text).unwrap_or_default();
    assert_eq!("", text, "All <p> elements should be pruned, got: {text:?}");
}

/// h1 + 50×`<p>abc</p>` with PruneSelector="p" → only h1 text "ABC" remains.
///
/// In Go, this relies on the readability external fallback which can extract the lone h1.
/// Our Rust port uses `readable-readability` which does not extract h1-only content the
/// same way, so the result is empty. Behaviour is consistent with how the Spiegel test
/// is ignored (see MEMORY.md).
#[test]
fn test_prune_selector_p_keeps_h1() {
    let html = format!(
        "<html><body><h1>ABC</h1>{}</body></html>",
        "<p>abc</p>".repeat(50)
    );
    let result = extract(&html, &prune_opts("p")).expect("extraction should succeed");
    assert_eq!(
        "ABC", result.content_text,
        "Only h1 text should remain after pruning <p>"
    );
}

/// PruneSelector="p, h1" with h1+50×`<p>` → completely empty.
#[test]
fn test_prune_selector_p_and_h1_empty() {
    let html = format!(
        "<html><body><h1>ABC</h1>{}</body></html>",
        "<p>abc</p>".repeat(50)
    );
    let result = extract(&html, &prune_opts("p, h1"));
    let text = result.map(|r| r.content_text).unwrap_or_default();
    assert_eq!(
        "", text,
        "Pruning both p and h1 should yield empty, got: {text:?}"
    );
}

/// PruneSelector="p, h1" with h1+h2+50×`<p>` → only h2 text "42" remains.
#[test]
fn test_prune_selector_p_and_h1_keeps_h2() {
    let html = format!(
        "<html><body><h1>ABC</h1><h2>42</h2>{}</body></html>",
        "<p>abc</p>".repeat(50)
    );
    let result = extract(&html, &prune_opts("p, h1")).expect("extraction should succeed");
    assert_eq!(
        "42", result.content_text,
        "Only h2 text should remain after pruning p and h1"
    );
}

// ---------------------------------------------------------------------------
// Test_External gaps
// ---------------------------------------------------------------------------

/// apache.html: table content ("localhost:80") included when ExcludeTables=false.
#[test]
fn test_external_exclude_tables_false() {
    let html = read_simple_fixture("apache.html");
    let opts = {
        let mut o = Options::default();
        o.exclude_tables = false;
        o
    };
    let result = extract(&html, &opts).expect("extraction should succeed");
    assert!(
        result.content_text.contains("localhost:80"),
        "Table content should be present when exclude_tables=false; text was: {}",
        result.content_text
    );
}

/// apache.html: table content ("localhost:80") excluded when ExcludeTables=true.
#[test]
fn test_external_exclude_tables_true() {
    let html = read_simple_fixture("apache.html");
    let opts = {
        let mut o = Options::default();
        o.exclude_tables = true;
        o
    };
    let result = extract(&html, &opts);
    let text = result.map(|r| r.content_text).unwrap_or_default();
    assert!(
        !text.contains("localhost:80"),
        "Table content should be absent when exclude_tables=true; text was: {text}"
    );
}

/// scam.html: with ExcludeTables=true and no fallback → empty.
#[test]
fn test_external_scam_no_fallback_empty() {
    let html = read_simple_fixture("scam.html");
    let opts = {
        let mut o = Options::default();
        o.exclude_tables = true;
        o.config = zero_config();
        o
    };
    let result = extract(&html, &opts);
    let text = result.map(|r| r.content_text).unwrap_or_default();
    assert!(
        text.is_empty(),
        "scam.html with exclude_tables=true and no fallback should yield empty, got: {text:?}"
    );
}

/// scam.html: with ExcludeTables=true and enable_fallback=true → non-empty and
/// does not contain ad/nav strings.
///
/// In Go, the readability fallback for scam.html filters out the ad links ("Uncensored
/// Hosting", "ChooseBetter"). The Rust `readable-readability` crate extracts the link
/// table instead, so the content check for those strings cannot pass. Marked as ignored
/// pending a higher-quality readability fallback.
#[test]
fn test_external_scam_with_fallback_nonempty() {
    let html = read_simple_fixture("scam.html");
    let opts = {
        let mut o = Options::default();
        o.exclude_tables = true;
        o.enable_fallback = true;
        o.config = zero_config();
        o
    };
    let result = extract(&html, &opts).expect("extraction should succeed");
    assert!(
        !result.content_text.is_empty(),
        "scam.html with fallback should yield non-empty content"
    );
    assert!(
        !result.content_text.contains("Uncensored Hosting"),
        "Ad text should not appear; got: {}",
        result.content_text
    );
    assert!(
        !result.content_text.contains("ChooseBetter"),
        "Ad text should not appear; got: {}",
        result.content_text
    );
}

// ---------------------------------------------------------------------------
// Test_NonStdHtmlEntities
// ---------------------------------------------------------------------------

/// Non-standard HTML entities should be preserved as-is.
#[test]
fn test_non_std_html_entities() {
    let html = r#"<html><body><p>Text &customentity; more text</p></body></html>"#;
    let result = extract(html, &zero_opts()).expect("extraction should succeed");
    assert_eq!(
        "Text &customentity; more text", result.content_text,
        "Non-standard entity should round-trip unchanged"
    );
}

// ---------------------------------------------------------------------------
// Test_MixedContentExtraction
// ---------------------------------------------------------------------------

/// Mixed content (p + img + video) → only text extracted.
#[test]
fn test_mixed_content_extraction() {
    let html =
        r#"<html><body><p>Text here</p><img src="img.jpg"/><video src="video.mp4"/></body></html>"#;
    let result = extract(html, &zero_opts()).expect("extraction should succeed");
    assert_eq!(
        "Text here", result.content_text,
        "Only text should be extracted from mixed content"
    );
}

// ---------------------------------------------------------------------------
// Test_LargeDocPerformance
// ---------------------------------------------------------------------------

/// 1000 repeated `<p>` elements should extract without hanging.
#[test]
fn test_large_doc_performance() {
    let html = format!(
        "<html><body>{}</body></html>",
        "<p>Sample text</p>".repeat(1000)
    );
    // We simply verify the call returns (no timeout needed in Rust tests).
    let _ = extract(&html, &zero_opts());
}
