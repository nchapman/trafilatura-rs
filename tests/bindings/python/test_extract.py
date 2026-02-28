"""Tests for extract() and extract_simple()."""

import pytest

from trafilatura_uniffi import (
    TrafilaturaError,
    ExtractionFocus,
    extract,
    extract_simple,
    default_options,
)

ARTICLE_HTML = """
<html>
<head>
  <title>Test Article Title</title>
  <meta name="author" content="Jane Doe">
  <meta property="og:description" content="A test article about web extraction.">
  <meta property="og:site_name" content="TestSite">
</head>
<body>
  <nav><a href="/">Home</a> | <a href="/about">About</a></nav>
  <article>
    <h1>Test Article Title</h1>
    <p>This is the first paragraph of the article. It contains enough text to
    exceed the minimum extraction thresholds that the trafilatura algorithm uses
    to determine whether content is substantial enough to be considered an article.
    The algorithm analyzes DOM structure and text density to find main content.</p>
    <p>The second paragraph adds more content to ensure the algorithm can properly
    identify this as the main content of the page. Trafilatura uses multiple
    extraction strategies including readability and justext as fallbacks to ensure
    robust content extraction across different page structures and layouts.</p>
    <p>A third paragraph further strengthens the signal that this is genuine article
    content rather than boilerplate navigation or sidebar material. The algorithm
    compares candidate nodes and selects the one with the highest content score,
    while preserving the original structure of the text content.</p>
    <p>Finally, a fourth paragraph ensures we are well above the default minimum
    extraction size of two hundred and fifty characters, making this a reliable
    test fixture for the extraction algorithm across all three language bindings
    used in this test suite.</p>
  </article>
  <footer>Copyright 2024</footer>
</body>
</html>
"""


def test_extract_simple():
    result = extract_simple(ARTICLE_HTML)
    assert "first paragraph" in result.content_text
    assert len(result.content_text) > 0


def test_extract_with_default_options():
    result_a = extract_simple(ARTICLE_HTML)
    result_b = extract(ARTICLE_HTML, default_options())
    assert result_a.content_text == result_b.content_text


def test_extract_with_focus():
    opts = default_options()
    opts.focus = ExtractionFocus.FAVOR_RECALL
    result = extract(ARTICLE_HTML, opts)
    assert len(result.content_text) > 0


def test_extract_exclude_comments():
    opts = default_options()
    opts.exclude_comments = True
    result = extract(ARTICLE_HTML, opts)
    assert result.comments_text == ""


def test_extract_metadata():
    result = extract_simple(ARTICLE_HTML)
    assert result.metadata.title == "Test Article Title"
    assert isinstance(result.metadata.author, str)
    assert isinstance(result.metadata.categories, list)
    assert isinstance(result.metadata.tags, list)


def test_extract_html_output():
    result = extract_simple(ARTICLE_HTML)
    assert len(result.content_html) > 0
    # HTML should contain markup
    assert "<" in result.content_html


def test_extract_with_valid_url():
    opts = default_options()
    opts.original_url = "https://example.com/article"
    result = extract(ARTICLE_HTML, opts)
    assert len(result.content_text) > 0


def test_extract_with_invalid_url():
    opts = default_options()
    opts.original_url = "not a url"
    with pytest.raises(TrafilaturaError.ParseError):
        extract(ARTICLE_HTML, opts)


def test_extract_with_invalid_date():
    opts = default_options()
    opts.html_date_override = "01/01/2024"  # wrong format, expects YYYY-MM-DD
    with pytest.raises(TrafilaturaError.ParseError):
        extract(ARTICLE_HTML, opts)
