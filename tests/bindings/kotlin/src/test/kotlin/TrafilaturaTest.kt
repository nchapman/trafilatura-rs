import trafilatura.*
import kotlin.test.*
import org.junit.jupiter.api.Nested

class TrafilaturaTest {

    private val articleHtml = """
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
    """.trimIndent()

    @Nested inner class ExtractSimple {

        @Test fun `basic extraction`() {
            val result = extractSimple(articleHtml)
            assertTrue(result.contentText.contains("first paragraph"))
            assertTrue(result.contentText.isNotEmpty())
        }
    }

    @Nested inner class Extract {

        @Test fun `default options matches extract_simple`() {
            val a = extractSimple(articleHtml)
            val b = extract(articleHtml, defaultOptions())
            assertEquals(a.contentText, b.contentText)
        }

        @Test fun `with focus`() {
            val opts = defaultOptions().copy(focus = ExtractionFocus.FAVOR_RECALL)
            val result = extract(articleHtml, opts)
            assertTrue(result.contentText.isNotEmpty())
        }

        @Test fun `exclude comments`() {
            val opts = defaultOptions().copy(excludeComments = true)
            val result = extract(articleHtml, opts)
            assertEquals("", result.commentsText)
        }
    }

    @Nested inner class Validation {

        @Test fun `valid url`() {
            val opts = defaultOptions().copy(originalUrl = "https://example.com/article")
            val result = extract(articleHtml, opts)
            assertTrue(result.contentText.isNotEmpty())
        }

        @Test fun `invalid url throws ParseError`() {
            val opts = defaultOptions().copy(originalUrl = "not a url")
            assertFailsWith<TrafilaturaException.ParseException> {
                extract(articleHtml, opts)
            }
        }

        @Test fun `invalid date throws ParseError`() {
            val opts = defaultOptions().copy(htmlDateOverride = "01/01/2024")
            assertFailsWith<TrafilaturaException.ParseException> {
                extract(articleHtml, opts)
            }
        }
    }

    @Nested inner class MetadataFields {

        @Test fun `metadata extraction`() {
            val result = extractSimple(articleHtml)
            assertEquals("Test Article Title", result.metadata.title)
        }
    }

    @Nested inner class HtmlOutput {

        @Test fun `content html present`() {
            val result = extractSimple(articleHtml)
            assertTrue(result.contentHtml.isNotEmpty())
            assertTrue(result.contentHtml.contains("<"))
        }
    }

    @Nested inner class ReadableDocument {

        @Test fun `create readable document`() {
            val result = extractSimple(articleHtml)
            val doc = createReadableDocument(result)
            assertTrue(doc.contains("<html"))
            assertTrue(doc.contains("content-body"))
        }
    }

    @Nested inner class DefaultOptions {

        @Test fun `default option values`() {
            val opts = defaultOptions()
            assertNull(opts.originalUrl)
            assertNull(opts.targetLanguage)
            assertFalse(opts.enableFallback)
            assertEquals(ExtractionFocus.BALANCED, opts.focus)
            assertFalse(opts.excludeComments)
            assertFalse(opts.excludeTables)
            assertFalse(opts.includeImages)
            assertFalse(opts.includeLinks)
            assertFalse(opts.deduplicate)
            assertFalse(opts.requireEssentialMetadata)
            assertNull(opts.maxTreeSize)
            assertNull(opts.pruneSelector)
            assertEquals(HtmlDateMode.AUTOMATIC, opts.htmlDateMode)
            assertNull(opts.htmlDateOverride)
        }
    }

    @Nested inner class DefaultConfig {

        @Test fun `default config values`() {
            // Defaults from trafilatura::Config::default()
            val config = defaultConfig()
            assertEquals(250, config.minExtractedSize)
            assertEquals(1, config.minExtractedCommentSize)
            assertEquals(1, config.minOutputSize)
            assertEquals(1, config.minOutputCommentSize)
        }
    }
}
