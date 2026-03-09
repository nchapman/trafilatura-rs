# Trafilatura for .NET

Extract readable content, comments, and metadata from web pages.

A high-performance Rust-based port of the [trafilatura](https://github.com/adbar/trafilatura) library with native bindings for .NET.

## Installation

```bash
dotnet add package Trafilatura
```

## Usage

```csharp
using Trafilatura;

// Simple extraction
var result = TrafilaturaMethods.ExtractSimple(html);
Console.WriteLine(result.contentText);
Console.WriteLine(result.metadata.title);

// With options
var opts = TrafilaturaMethods.DefaultOptions() with {
    includeLinks = true,
    focus = ExtractionFocus.FavorRecall
};
var result = TrafilaturaMethods.Extract(html, opts);
```

## Supported Platforms

| Runtime         | Architecture |
|-----------------|-------------|
| Windows         | x64, arm64  |
| Linux (glibc)   | x64, arm64  |
| macOS           | x64, arm64  |

## License

Apache-2.0
