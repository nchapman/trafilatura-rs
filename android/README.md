# Trafilatura for Android

Extract readable content, comments, and metadata from web pages.

A high-performance Rust-based implementation with native bindings for Android. See [trafilatura-rs](https://github.com/nchapman/trafilatura-rs) on GitHub.

## Installation

```kotlin
// build.gradle.kts
dependencies {
    implementation("io.github.nchapman:trafilatura:<VERSION>")
}
```

Replace `<VERSION>` with the latest release version.

## Quick Start

```kotlin
import trafilatura.*

val html = "<html>...</html>"

val result = extractSimple(html)
println(result.contentText)       // Main article text
println(result.metadata.title)    // Page title
```

## API Reference

### Functions

| Function | Description |
|----------|-------------|
| `extractSimple(html)` | Extract with default options. Throws on failure. |
| `extract(html, options)` | Extract with custom options. Throws on failure. |
| `defaultOptions()` | Returns default `ExtractionOptions`. |
| `defaultConfig()` | Returns default `ExtractionConfig`. |
| `createReadableDocument(result)` | Wraps an `ExtractResult` in a self-contained HTML document. |

### Extraction with Options

```kotlin
val opts = defaultOptions().copy(
    // Include links and images in HTML output
    includeLinks = true,
    includeImages = true,

    // Extraction strategy
    focus = ExtractionFocus.FAVOR_RECALL,       // Extract more (may include noise)
    // focus = ExtractionFocus.FAVOR_PRECISION,  // Extract less but higher quality
    // focus = ExtractionFocus.BALANCED,          // Default

    // Enable readability fallback for difficult pages
    enableFallback = true,

    // Filter by language (ISO 639-1 code)
    targetLanguage = "en",

    // Provide the source URL (improves metadata extraction)
    originalUrl = "https://example.com/article",

    // Remove specific elements before extraction
    pruneSelector = "aside, .sidebar, .ad",

    // Control comment extraction
    excludeComments = true,
    excludeTables = true,

    // Date extraction
    htmlDateMode = HtmlDateMode.EXTENSIVE,  // AUTOMATIC, FAST, EXTENSIVE, DISABLED
    htmlDateOverride = "2024-01-15",        // ISO-8601 format (YYYY-MM-DD)

    // Deduplication
    deduplicate = true,

    // Require title + date + author or skip the page
    requireEssentialMetadata = true,

    // Limit DOM size (prevents slow extraction on huge pages)
    maxTreeSize = 50000L,
)
val result = extract(html, opts)
```

### Result Structure

```kotlin
val result = extractSimple(html)

// Content
result.contentText       // Plain text of the main article
result.contentHtml       // Cleaned HTML of the main article
result.commentsText      // Plain text of user comments
result.commentsHtml      // Cleaned HTML of user comments

// Metadata
result.metadata.title        // Page title
result.metadata.author       // Author name
result.metadata.date         // Publication date (YYYY-MM-DD string, or null)
result.metadata.url          // Canonical URL
result.metadata.hostname     // Domain name
result.metadata.description  // Meta description
result.metadata.sitename     // Site name
result.metadata.categories   // List<String>
result.metadata.tags         // List<String>
result.metadata.language     // Detected language
result.metadata.image        // Featured image URL
result.metadata.license      // Content license
result.metadata.pageType     // Page type (e.g. "article")
```

### Readable Document

Wrap an extraction result in a self-contained HTML page:

```kotlin
val result = extractSimple(html)
val doc = createReadableDocument(result)
// Returns a full HTML document with the extracted content
```

### Error Handling

All extraction functions throw `TrafilaturaException`:

```kotlin
try {
    val result = extractSimple(html)
} catch (e: TrafilaturaException.ParseException) {
    // Invalid HTML or URL
} catch (e: TrafilaturaException.InsufficientContent) {
    // Not enough content found
} catch (e: TrafilaturaException.LanguageMismatch) {
    // Content language doesn't match targetLanguage
} catch (e: TrafilaturaException.DuplicateContent) {
    // Content was flagged as duplicate (when deduplicate is on)
} catch (e: TrafilaturaException.MissingMetadata) {
    // Required metadata not found (when requireEssentialMetadata is on)
} catch (e: TrafilaturaException.TreeTooLarge) {
    // DOM exceeded maxTreeSize limit
} catch (e: TrafilaturaException) {
    println("Extraction failed: ${e.message}")
}
```

### Advanced Config

Fine-tune minimum content thresholds:

```kotlin
val opts = defaultOptions().copy(
    config = ExtractionConfig(
        minExtractedSize = 250,          // Min characters for main content (default: 250)
        minExtractedCommentSize = 1,     // Min characters for comments (default: 1)
        minOutputSize = 1,               // Min output characters (default: 1)
        minOutputCommentSize = 1         // Min output comment characters (default: 1)
    )
)
```

## Supported ABIs

| ABI | Use |
|-----|-----|
| arm64-v8a | Modern phones/tablets |
| armeabi-v7a | Older 32-bit devices |
| x86_64 | Emulators |

Minimum SDK: 21 (Android 5.0)

## License

Apache-2.0
