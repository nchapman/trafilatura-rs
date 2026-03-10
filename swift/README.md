# Trafilatura for Swift

Extract readable content, comments, and metadata from web pages.

A high-performance Rust-based implementation with native bindings for Swift. See [trafilatura-rs](https://github.com/nchapman/trafilatura-rs) on GitHub.

## Installation

### Xcode

File → Add Package Dependencies → paste:

```
https://github.com/nchapman/trafilatura-swift
```

### Package.swift

```swift
dependencies: [
    .package(url: "https://github.com/nchapman/trafilatura-swift", from: "<VERSION>"),
]
```

Replace `<VERSION>` with the latest release version (see [releases](https://github.com/nchapman/trafilatura-swift/tags)).

Then add `"Trafilatura"` to your target's dependencies.

## Quick Start

```swift
import Trafilatura

let html = "<html>...</html>"

let result = try extractSimple(html: html)
print(result.contentText)       // Main article text
print(result.metadata.title)    // Page title
```

## API Reference

### Functions

| Function | Description |
|----------|-------------|
| `extractSimple(html:)` | Extract with default options. Throws on failure. |
| `extract(html:options:)` | Extract with custom options. Throws on failure. |
| `defaultOptions()` | Returns default `ExtractionOptions`. |
| `defaultConfig()` | Returns default `ExtractionConfig`. |
| `createReadableDocument(result:)` | Wraps an `ExtractResult` in a self-contained HTML document. |

### Extraction with Options

```swift
var opts = defaultOptions()

// Include links and images in HTML output
opts.includeLinks = true
opts.includeImages = true

// Extraction strategy
opts.focus = .favorRecall       // Extract more content (may include some noise)
// opts.focus = .favorPrecision // Extract less but higher quality
// opts.focus = .balanced       // Default

// Enable readability fallback for difficult pages
opts.enableFallback = true

// Filter by language (ISO 639-1 code)
opts.targetLanguage = "en"

// Provide the source URL (improves metadata extraction)
opts.originalUrl = "https://example.com/article"

// Remove specific elements before extraction
opts.pruneSelector = "aside, .sidebar, .ad"

// Control comment extraction
opts.excludeComments = true
opts.excludeTables = true

// Date extraction
opts.htmlDateMode = .extensive  // .automatic, .fast, .extensive, .disabled
opts.htmlDateOverride = "2024-01-15"  // ISO-8601 format (YYYY-MM-DD)

// Deduplication
opts.deduplicate = true

// Require title + date + author or skip the page
opts.requireEssentialMetadata = true

// Limit DOM size (prevents slow extraction on huge pages)
opts.maxTreeSize = 50000

let result = try extract(html: html, options: opts)
```

### Result Structure

```swift
let result = try extractSimple(html: html)

// Content
result.contentText       // Plain text of the main article
result.contentHtml       // Cleaned HTML of the main article
result.commentsText      // Plain text of user comments
result.commentsHtml      // Cleaned HTML of user comments

// Metadata
result.metadata.title        // Page title
result.metadata.author       // Author name
result.metadata.date         // Publication date (YYYY-MM-DD string, or nil)
result.metadata.url          // Canonical URL
result.metadata.hostname     // Domain name
result.metadata.description  // Meta description
result.metadata.sitename     // Site name
result.metadata.categories   // [String]
result.metadata.tags         // [String]
result.metadata.language     // Detected language
result.metadata.image        // Featured image URL
result.metadata.license      // Content license
result.metadata.pageType     // Page type (e.g. "article")
```

### Readable Document

Wrap an extraction result in a self-contained HTML page:

```swift
let result = try extractSimple(html: html)
let doc = createReadableDocument(result: result)
// Returns a full HTML document with the extracted content
```

### Error Handling

All extraction functions throw `TrafilaturaError`:

```swift
do {
    let result = try extractSimple(html: html)
} catch TrafilaturaError.ParseError(let reason) {
    // Invalid HTML or URL
} catch TrafilaturaError.InsufficientContent(let reason) {
    // Not enough content found
} catch TrafilaturaError.LanguageMismatch(let expected, let got) {
    // Content language doesn't match targetLanguage
} catch TrafilaturaError.DuplicateContent {
    // Content was flagged as duplicate (when deduplicate is on)
} catch TrafilaturaError.MissingMetadata(let reason) {
    // Required metadata not found (when requireEssentialMetadata is on)
} catch TrafilaturaError.TreeTooLarge(let size) {
    // DOM exceeded maxTreeSize limit
} catch {
    print("Extraction failed: \(error)")
}
```

### Advanced Config

Fine-tune minimum content thresholds:

```swift
var opts = defaultOptions()
opts.config = ExtractionConfig(
    minExtractedSize: 250,          // Min characters for main content (default: 250)
    minExtractedCommentSize: 1,     // Min characters for comments (default: 1)
    minOutputSize: 1,               // Min output characters (default: 1)
    minOutputCommentSize: 1         // Min output comment characters (default: 1)
)
```

## Supported Platforms

| Platform        | Architecture    |
|-----------------|-----------------|
| macOS           | arm64, x86_64   |
| iOS             | arm64           |
| iOS Simulator   | arm64, x86_64   |

## License

Apache-2.0
