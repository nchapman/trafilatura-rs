import XCTest
import Trafilatura

final class TrafilaturaTests: XCTestCase {

    private let articleHtml = """
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
            <p>This is the first paragraph of the article. It contains enough text to \
        exceed the minimum extraction thresholds that the trafilatura algorithm uses \
        to determine whether content is substantial enough to be considered an article. \
        The algorithm analyzes DOM structure and text density to find main content.</p>
            <p>The second paragraph adds more content to ensure the algorithm can properly \
        identify this as the main content of the page. Trafilatura uses multiple \
        extraction strategies including readability and justext as fallbacks to ensure \
        robust content extraction across different page structures and layouts.</p>
            <p>A third paragraph further strengthens the signal that this is genuine article \
        content rather than boilerplate navigation or sidebar material. The algorithm \
        compares candidate nodes and selects the one with the highest content score, \
        while preserving the original structure of the text content.</p>
            <p>Finally, a fourth paragraph ensures we are well above the default minimum \
        extraction size of two hundred and fifty characters, making this a reliable \
        test fixture for the extraction algorithm across all three language bindings \
        used in this test suite.</p>
          </article>
          <footer>Copyright 2024</footer>
        </body>
        </html>
        """

    // MARK: - extract_simple()

    func testExtractSimple() throws {
        let result = try extractSimple(html: articleHtml)
        XCTAssertTrue(result.contentText.contains("first paragraph"))
        XCTAssertTrue(result.contentText.count > 0)
    }

    // MARK: - extract()

    func testExtractWithDefaultOptions() throws {
        let a = try extractSimple(html: articleHtml)
        let b = try extract(html: articleHtml, options: defaultOptions())
        XCTAssertEqual(a.contentText, b.contentText)
    }

    func testExtractWithFocus() throws {
        var opts = defaultOptions()
        opts.focus = .favorRecall
        let result = try extract(html: articleHtml, options: opts)
        XCTAssertTrue(result.contentText.count > 0)
    }

    func testExtractExcludeComments() throws {
        var opts = defaultOptions()
        opts.excludeComments = true
        let result = try extract(html: articleHtml, options: opts)
        XCTAssertEqual(result.commentsText, "")
    }

    func testExtractWithValidUrl() throws {
        var opts = defaultOptions()
        opts.originalUrl = "https://example.com/article"
        let result = try extract(html: articleHtml, options: opts)
        XCTAssertTrue(result.contentText.count > 0)
    }

    func testExtractWithInvalidUrl() {
        var opts = defaultOptions()
        opts.originalUrl = "not a url"
        XCTAssertThrowsError(try extract(html: articleHtml, options: opts)) { error in
            guard case TrafilaturaError.ParseError = error else {
                XCTFail("Expected TrafilaturaError.ParseError, got \(error)")
                return
            }
        }
    }

    func testExtractWithInvalidDate() {
        var opts = defaultOptions()
        opts.htmlDateOverride = "01/01/2024"
        XCTAssertThrowsError(try extract(html: articleHtml, options: opts)) { error in
            guard case TrafilaturaError.ParseError = error else {
                XCTFail("Expected TrafilaturaError.ParseError, got \(error)")
                return
            }
        }
    }

    // MARK: - Metadata

    func testMetadata() throws {
        let result = try extractSimple(html: articleHtml)
        XCTAssertEqual(result.metadata.title, "Test Article Title")
    }

    // MARK: - HTML output

    func testHtmlOutput() throws {
        let result = try extractSimple(html: articleHtml)
        XCTAssertTrue(result.contentHtml.count > 0)
        XCTAssertTrue(result.contentHtml.contains("<"))
    }

    // MARK: - create_readable_document()

    func testCreateReadableDocument() throws {
        let result = try extractSimple(html: articleHtml)
        let doc = createReadableDocument(result: result)
        XCTAssertTrue(doc.contains("<html"))
        XCTAssertTrue(doc.contains("content-body"))
    }

    // MARK: - default_options()

    func testDefaultOptions() {
        let opts = defaultOptions()
        XCTAssertNil(opts.originalUrl)
        XCTAssertNil(opts.targetLanguage)
        XCTAssertFalse(opts.enableFallback)
        XCTAssertEqual(opts.focus, .balanced)
        XCTAssertFalse(opts.excludeComments)
        XCTAssertFalse(opts.excludeTables)
        XCTAssertFalse(opts.includeImages)
        XCTAssertFalse(opts.includeLinks)
        XCTAssertFalse(opts.deduplicate)
        XCTAssertFalse(opts.hasEssentialMetadata)
        XCTAssertNil(opts.maxTreeSize)
        XCTAssertNil(opts.pruneSelector)
        XCTAssertEqual(opts.htmlDateMode, .`default`)
        XCTAssertNil(opts.htmlDateOverride)
    }

    // MARK: - default_config()

    func testDefaultConfig() {
        // Defaults from trafilatura::Config::default()
        let config = defaultConfig()
        XCTAssertEqual(config.minExtractedSize, 250)
        XCTAssertEqual(config.minExtractedCommentSize, 1)
        XCTAssertEqual(config.minOutputSize, 1)
        XCTAssertEqual(config.minOutputCommentSize, 1)
    }
}
