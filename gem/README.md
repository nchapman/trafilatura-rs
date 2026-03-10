# Trafilatura for Ruby

Extract readable content, comments, and metadata from web pages.

A high-performance Rust-based implementation with native bindings for Ruby. See [trafilatura-rs](https://github.com/nchapman/trafilatura-rs) on GitHub.

## Installation

```ruby
gem install trafilatura
```

Or in your Gemfile:

```ruby
gem "trafilatura"
```

## Quick Start

```ruby
require "trafilatura"

html = "<html>...</html>"

result = Trafilatura.extract_simple(html)
puts result.content_text        # Main article text
puts result.metadata.title      # Page title
```

## API Reference

### Functions

| Function | Description |
|----------|-------------|
| `Trafilatura.extract_simple(html)` | Extract with default options. Raises on failure. |
| `Trafilatura.extract_with(html, **opts)` | Extract with keyword options. Raises on failure. |
| `Trafilatura.extract(html, options)` | Extract with an `ExtractionOptions` object. Raises on failure. |
| `Trafilatura.default_options` | Returns default `ExtractionOptions`. |
| `Trafilatura.default_config` | Returns default `ExtractionConfig`. |
| `Trafilatura.create_readable_document(result)` | Wraps an `ExtractResult` in a self-contained HTML document. |

### Extraction with Options

Pass only the options you want to change — everything else uses defaults:

```ruby
result = Trafilatura.extract_with(html,
  # Include links and images in HTML output
  include_links: true,
  include_images: true,

  # Extraction strategy (symbol or constant)
  focus: :favor_recall,        # Extract more (may include noise)
  # focus: :favor_precision,   # Extract less but higher quality
  # focus: :balanced,          # Default

  # Enable readability fallback for difficult pages
  enable_fallback: true,

  # Filter by language (ISO 639-1 code)
  target_language: "en",

  # Provide the source URL (improves metadata extraction)
  original_url: "https://example.com/article",

  # Remove specific elements before extraction
  prune_selector: "aside, .sidebar, .ad",

  # Control comment extraction
  exclude_comments: true,
  exclude_tables: true,

  # Date extraction (symbol or constant)
  html_date_mode: :extensive,  # :automatic, :fast, :extensive, :disabled
  html_date_override: "2024-01-15",  # ISO-8601 format (YYYY-MM-DD)

  # Deduplication
  deduplicate: true,

  # Require title + date + author or skip the page
  require_essential_metadata: true,

  # Limit DOM size (prevents slow extraction on huge pages)
  max_tree_size: 50_000,
)
```

### Result Structure

```ruby
result = Trafilatura.extract_simple(html)

# Content
result.content_text       # Plain text of the main article
result.content_html       # Cleaned HTML of the main article
result.comments_text      # Plain text of user comments
result.comments_html      # Cleaned HTML of user comments

# Metadata
result.metadata.title        # Page title
result.metadata.author       # Author name
result.metadata.date         # Publication date (YYYY-MM-DD string, or nil)
result.metadata.url          # Canonical URL
result.metadata.hostname     # Domain name
result.metadata.description  # Meta description
result.metadata.sitename     # Site name
result.metadata.categories   # Array of strings
result.metadata.tags         # Array of strings
result.metadata.language     # Detected language
result.metadata.image        # Featured image URL
result.metadata.license      # Content license
result.metadata.page_type    # Page type (e.g. "article")
```

### Readable Document

Wrap an extraction result in a self-contained HTML page:

```ruby
result = Trafilatura.extract_simple(html)
doc = Trafilatura.create_readable_document(result)
# Returns a full HTML document with the extracted content
```

### Error Handling

All extraction functions raise subclasses of `StandardError`:

```ruby
begin
  result = Trafilatura.extract_simple(html)
rescue Trafilatura::TrafilaturaError::ParseError => e
  # Invalid HTML or URL
rescue Trafilatura::TrafilaturaError::InsufficientContent => e
  # Not enough content found
rescue Trafilatura::TrafilaturaError::LanguageMismatch => e
  # Content language doesn't match target_language
rescue Trafilatura::TrafilaturaError::DuplicateContent
  # Content was flagged as duplicate (when deduplicate is on)
rescue Trafilatura::TrafilaturaError::MissingMetadata => e
  # Required metadata not found (when require_essential_metadata is on)
rescue Trafilatura::TrafilaturaError::TreeTooLarge => e
  # DOM exceeded max_tree_size limit
end
```

### Advanced Config

Fine-tune minimum content thresholds:

```ruby
result = Trafilatura.extract_with(html,
  config: Trafilatura::ExtractionConfig.new(
    min_extracted_size: 250,          # Min characters for main content (default: 250)
    min_extracted_comment_size: 1,    # Min characters for comments (default: 1)
    min_output_size: 1,               # Min output characters (default: 1)
    min_output_comment_size: 1        # Min output comment characters (default: 1)
  )
)
```

## Supported Platforms

| Platform         | Architecture |
|------------------|-------------|
| Linux (glibc)    | x86_64, arm64 |
| macOS            | x86_64, arm64 |

## License

Apache-2.0
