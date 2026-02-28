"""Tests for create_readable_document()."""

from trafilatura_uniffi import extract_simple, create_readable_document

ARTICLE_HTML = """
<html>
<head><title>Test Article</title></head>
<body>
  <article>
    <h1>Test Article</h1>
    <p>This is the first paragraph of the article. It contains enough text to
    exceed the minimum extraction thresholds that the trafilatura algorithm uses
    to determine whether content is substantial enough to be considered an article.
    The algorithm analyzes DOM structure and text density to find main content.</p>
    <p>The second paragraph adds more content to ensure the algorithm can properly
    identify this as the main content of the page. Trafilatura uses multiple
    extraction strategies including readability and justext as fallbacks.</p>
    <p>A third paragraph further strengthens the signal that this is genuine article
    content rather than boilerplate navigation or sidebar material.</p>
  </article>
</body>
</html>
"""


def test_create_readable_document():
    result = extract_simple(ARTICLE_HTML)
    doc = create_readable_document(result)
    assert "<html" in doc
    assert "content-body" in doc
    assert len(doc) > 0
