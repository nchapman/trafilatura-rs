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

## Usage

```swift
import Trafilatura

// Simple extraction
let result = try extractSimple(html: html)
print(result.contentText)
print(result.metadata.title)

// With options
var opts = defaultOptions()
opts.includeLinks = true
opts.focus = .favorRecall
let result = try extract(html: html, options: opts)
```

## Supported Platforms

| Platform        | Architecture    |
|-----------------|-----------------|
| macOS           | arm64, x86_64   |
| iOS             | arm64           |
| iOS Simulator   | arm64, x86_64   |

## License

Apache-2.0
