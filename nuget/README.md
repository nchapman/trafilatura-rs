# Trafilatura for .NET

Extract readable content, comments, and metadata from web pages.

A high-performance Rust-based implementation with native bindings for .NET. See [trafilatura-rs](https://github.com/nchapman/trafilatura-rs) on GitHub.

## Installation

```bash
dotnet add package Trafilatura
```

## Usage

```csharp
using Trafilatura;

// Simple extraction
var result = Extractor.ExtractSimple(html);
Console.WriteLine(result.contentText);
Console.WriteLine(result.metadata.title);

// With options
var opts = Extractor.DefaultOptions() with {
    includeLinks = true,
    focus = ExtractionFocus.FavorRecall
};
var result = Extractor.Extract(html, opts);
```

## Supported Platforms

| Runtime         | Architecture |
|-----------------|-------------|
| Windows         | x64, arm64  |
| Linux (glibc)   | x64, arm64  |
| macOS           | x64, arm64  |

## License

Apache-2.0
