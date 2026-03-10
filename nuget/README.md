# Trafilatura for .NET

Extract readable content, comments, and metadata from web pages.

A high-performance Rust-based implementation with native bindings for .NET. See [trafilatura-rs](https://github.com/nchapman/trafilatura-rs) on GitHub.

## Installation

```bash
dotnet add package Trafilatura
```

## Quick Start

```csharp
using Trafilatura;

var html = "<html>...</html>";

var result = Extractor.ExtractSimple(html);
Console.WriteLine(result.contentText);       // Main article text
Console.WriteLine(result.metadata.title);    // Page title
```

## API Reference

### Functions

| Function | Description |
|----------|-------------|
| `Extractor.ExtractSimple(html)` | Extract with default options. Throws on failure. |
| `Extractor.Extract(html, options)` | Extract with custom options. Throws on failure. |
| `Extractor.DefaultOptions()` | Returns default `ExtractionOptions`. |
| `Extractor.DefaultConfig()` | Returns default `ExtractionConfig`. |
| `Extractor.CreateReadableDocument(result)` | Wraps an `ExtractResult` in a self-contained HTML document. |

### Extraction with Options

```csharp
var opts = Extractor.DefaultOptions() with {
    // Include links and images in HTML output
    includeLinks = true,
    includeImages = true,

    // Extraction strategy
    focus = ExtractionFocus.FavorRecall,       // Extract more (may include noise)
    // focus = ExtractionFocus.FavorPrecision,  // Extract less but higher quality
    // focus = ExtractionFocus.Balanced,         // Default

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
    htmlDateMode = HtmlDateMode.Extensive,  // Automatic, Fast, Extensive, Disabled
    htmlDateOverride = "2024-01-15",        // ISO-8601 format (YYYY-MM-DD)

    // Deduplication
    deduplicate = true,

    // Require title + date + author or skip the page
    requireEssentialMetadata = true,

    // Limit DOM size (prevents slow extraction on huge pages)
    maxTreeSize = 50000L,
};
var result = Extractor.Extract(html, opts);
```

### Result Structure

```csharp
var result = Extractor.ExtractSimple(html);

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
result.metadata.categories   // List<string>
result.metadata.tags         // List<string>
result.metadata.language     // Detected language
result.metadata.image        // Featured image URL
result.metadata.license      // Content license
result.metadata.pageType     // Page type (e.g. "article")
```

### Readable Document

Wrap an extraction result in a self-contained HTML page:

```csharp
var result = Extractor.ExtractSimple(html);
var doc = Extractor.CreateReadableDocument(result);
// Returns a full HTML document with the extracted content
```

### Error Handling

All extraction functions throw `TrafilaturaException`:

```csharp
try {
    var result = Extractor.ExtractSimple(html);
} catch (TrafilaturaException.ParseException e) {
    // Invalid HTML or URL
} catch (TrafilaturaException.InsufficientContent e) {
    // Not enough content found
} catch (TrafilaturaException.LanguageMismatch e) {
    // Content language doesn't match targetLanguage
} catch (TrafilaturaException.DuplicateContent e) {
    // Content was flagged as duplicate (when deduplicate is on)
} catch (TrafilaturaException.MissingMetadata e) {
    // Required metadata not found (when requireEssentialMetadata is on)
} catch (TrafilaturaException.TreeTooLarge e) {
    // DOM exceeded maxTreeSize limit
} catch (TrafilaturaException e) {
    Console.WriteLine($"Extraction failed: {e.Message}");
}
```

### Advanced Config

Fine-tune minimum content thresholds:

```csharp
var opts = Extractor.DefaultOptions() with {
    config = Extractor.DefaultConfig() with {
        minExtractedSize = 250,          // Min characters for main content (default: 250)
        minExtractedCommentSize = 1,     // Min characters for comments (default: 1)
        minOutputSize = 1,               // Min output characters (default: 1)
        minOutputCommentSize = 1         // Min output comment characters (default: 1)
    }
};
```

## Supported Platforms

| Runtime         | Architecture |
|-----------------|-------------|
| Windows         | x64, arm64  |
| Linux (glibc)   | x64, arm64  |
| macOS           | x64, arm64  |

## License

Apache-2.0
