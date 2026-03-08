using Xunit;
using uniffi.trafilatura_uniffi;

namespace TrafilaturaTests;

internal static class TestFixtures
{
    internal const string ArticleHtml = @"
<html>
<head>
  <title>Test Article Title</title>
  <meta name=""author"" content=""Jane Doe"">
  <meta property=""og:description"" content=""A test article about web extraction."">
  <meta property=""og:site_name"" content=""TestSite"">
</head>
<body>
  <nav><a href=""/"">Home</a> | <a href=""/about"">About</a></nav>
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
</html>";
}

public class ExtractSimpleTest
{
    [Fact]
    public void BasicExtraction()
    {
        var result = TrafilaturaUniffiMethods.ExtractSimple(TestFixtures.ArticleHtml);
        Assert.Contains("first paragraph", result.contentText);
        Assert.NotEmpty(result.contentText);
    }
}

public class ExtractTest
{

    [Fact]
    public void DefaultOptionsMatchesExtractSimple()
    {
        var a = TrafilaturaUniffiMethods.ExtractSimple(TestFixtures.ArticleHtml);
        var b = TrafilaturaUniffiMethods.Extract(TestFixtures.ArticleHtml, TrafilaturaUniffiMethods.DefaultOptions());
        Assert.Equal(a.contentText, b.contentText);
    }

    [Fact]
    public void WithFocus()
    {
        var opts = TrafilaturaUniffiMethods.DefaultOptions() with { focus = ExtractionFocus.FavorRecall };
        var result = TrafilaturaUniffiMethods.Extract(TestFixtures.ArticleHtml, opts);
        Assert.NotEmpty(result.contentText);
    }

    [Fact]
    public void ExcludeComments()
    {
        var opts = TrafilaturaUniffiMethods.DefaultOptions() with { excludeComments = true };
        var result = TrafilaturaUniffiMethods.Extract(TestFixtures.ArticleHtml, opts);
        Assert.Equal("", result.commentsText);
    }
}

public class ValidationTest
{

    [Fact]
    public void ValidUrl()
    {
        var opts = TrafilaturaUniffiMethods.DefaultOptions() with { originalUrl = "https://example.com/article" };
        var result = TrafilaturaUniffiMethods.Extract(TestFixtures.ArticleHtml, opts);
        Assert.NotEmpty(result.contentText);
    }

    [Fact]
    public void InvalidUrlThrowsParseError()
    {
        var opts = TrafilaturaUniffiMethods.DefaultOptions() with { originalUrl = "not a url" };
        Assert.Throws<TrafilaturaException.ParseException>(() =>
            TrafilaturaUniffiMethods.Extract(TestFixtures.ArticleHtml, opts));
    }

    [Fact]
    public void InvalidDateThrowsParseError()
    {
        var opts = TrafilaturaUniffiMethods.DefaultOptions() with { htmlDateOverride = "01/01/2024" };
        Assert.Throws<TrafilaturaException.ParseException>(() =>
            TrafilaturaUniffiMethods.Extract(TestFixtures.ArticleHtml, opts));
    }
}

public class MetadataTest
{

    [Fact]
    public void MetadataExtraction()
    {
        var result = TrafilaturaUniffiMethods.ExtractSimple(TestFixtures.ArticleHtml);
        Assert.Equal("Test Article Title", result.metadata.title);
    }
}

public class HtmlOutputTest
{

    [Fact]
    public void ContentHtmlPresent()
    {
        var result = TrafilaturaUniffiMethods.ExtractSimple(TestFixtures.ArticleHtml);
        Assert.NotEmpty(result.contentHtml);
        Assert.Contains("<", result.contentHtml);
    }
}

public class ReadableDocumentTest
{

    [Fact]
    public void CreateReadableDocument()
    {
        var result = TrafilaturaUniffiMethods.ExtractSimple(TestFixtures.ArticleHtml);
        var doc = TrafilaturaUniffiMethods.CreateReadableDocument(result);
        Assert.Contains("<html", doc);
        Assert.Contains("content-body", doc);
    }
}

public class DefaultOptionsTest
{
    [Fact]
    public void DefaultOptionValues()
    {
        var opts = TrafilaturaUniffiMethods.DefaultOptions();
        Assert.Null(opts.originalUrl);
        Assert.Null(opts.targetLanguage);
        Assert.False(opts.enableFallback);
        Assert.Equal(ExtractionFocus.Balanced, opts.focus);
        Assert.False(opts.excludeComments);
        Assert.False(opts.excludeTables);
        Assert.False(opts.includeImages);
        Assert.False(opts.includeLinks);
        Assert.False(opts.deduplicate);
        Assert.False(opts.requireEssentialMetadata);
        Assert.Null(opts.maxTreeSize);
        Assert.Null(opts.pruneSelector);
        Assert.Equal(HtmlDateMode.Automatic, opts.htmlDateMode);
        Assert.Null(opts.htmlDateOverride);
    }
}

public class DefaultConfigTest
{
    [Fact]
    public void DefaultConfigValues()
    {
        var config = TrafilaturaUniffiMethods.DefaultConfig();
        Assert.Equal(250L, config.minExtractedSize);
        Assert.Equal(1L, config.minExtractedCommentSize);
        Assert.Equal(1L, config.minOutputSize);
        Assert.Equal(1L, config.minOutputCommentSize);
    }
}
