# trafilatura

[![Crates.io](https://img.shields.io/crates/v/trafilatura.svg)](https://crates.io/crates/trafilatura)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Rust: 1.88+](https://img.shields.io/badge/rust-1.88%2B-orange.svg)](https://www.rust-lang.org)
[![NuGet](https://img.shields.io/nuget/v/Trafilatura)](https://www.nuget.org/packages/Trafilatura)

Extract readable content, comments, and metadata from web pages.

A Rust port of [go-trafilatura](https://github.com/markusmobius/go-trafilatura),
which itself ports the Python [trafilatura](https://github.com/adbar/trafilatura)
library by Adrien Barbaresi.

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
trafilatura = "0.3"
```

### Library

```rust
use trafilatura::{extract, Options};

let html = r#"<html><body>
  <nav>Menu items</nav>
  <article><p>This is the main article content.</p></article>
  <footer>Copyright 2024</footer>
</body></html>"#;

let result = extract(html, &Options::default()).unwrap();
println!("{}", result.content_text);   // "This is the main article content."
println!("{}", result.metadata.title); // extracted <title> or og:title
```

### With options

```rust
use trafilatura::{extract, Options, ExtractionFocus};

let opts = Options::default()
    .with_fallback(true)              // use readability fallback
    .with_links(true)                 // preserve <a> tags in HTML output
    .with_focus(ExtractionFocus::FavorRecall); // extract more content

let result = extract(html, &opts).unwrap();
```

### Markdown output

Enable the `markdown` feature to convert extracted content to Markdown:

```toml
[dependencies]
trafilatura = { version = "0.4", features = ["markdown"] }
```

```rust
use trafilatura::{extract, create_markdown_document, Options};

let result = extract(html, &Options::default()).unwrap();

// Just the content as markdown:
let md = result.content_markdown();

// Full document with YAML front matter + content + comments:
let doc = create_markdown_document(&result);
```

### CLI

```sh
# Extract from a URL
trafilatura https://example.com/article

# Extract as markdown (with front matter)
trafilatura --format md --links https://example.com/article

# Extract from a file
trafilatura path/to/page.html

# Include links in output
trafilatura --links https://example.com/article
```

## What it extracts

- **Content** — main article body as plain text, cleaned HTML, or Markdown
- **Comments** — user comments, separately from article content
- **Metadata** — title, author, date, description, site name, categories,
  tags, license, language, and image URL (from meta tags, OpenGraph, JSON-LD)

## How it works

1. Parse HTML and extract metadata from `<meta>`, OpenGraph, and JSON-LD
2. Clean the DOM (remove scripts, styles, hidden elements, boilerplate)
3. Score and select content using CSS selector rules and paragraph heuristics
4. If primary extraction yields too little, fall back to readability-based
   extraction or baseline (last-resort) extraction
5. Filter duplicates and check language constraints

## Language bindings

Native bindings are available via [UniFFI](https://mozilla.github.io/uniffi-rs/) for Swift, Kotlin, Ruby, Dart, C#, and JavaScript/TypeScript.

```sh
make test-bindings   # run all binding test suites
make test-swift      # Swift only (XCTest)
make test-kotlin     # Kotlin only (JUnit 5)
make test-ruby       # Ruby only (Minitest)
make test-dart       # Dart only
make test-cs         # C# only (xUnit)
make test-js         # JS/TS only (Vitest, WASM)
```

### Swift ([full API docs](swift/README.md))

```swift
// Package.swift
.package(url: "https://github.com/nchapman/trafilatura-swift", from: "0.3.5"),
```

```swift
import Trafilatura

let result = try extractSimple(html: html)
print(result.contentText, result.metadata.title)
```

### Kotlin / Android ([full API docs](android/README.md))

```kotlin
// build.gradle.kts
implementation("io.github.nchapman:trafilatura:0.3.5")
```

```kotlin
import trafilatura.*

val result = extractSimple(html)
println("${result.contentText} ${result.metadata.title}")
```

### Ruby ([full API docs](gem/README.md))

```sh
gem install trafilatura
```

```ruby
require "trafilatura"

result = Trafilatura.extract_simple(html)
puts result.content_text, result.metadata.title
```

### Dart

```dart
import 'package:trafilatura/trafilatura.dart';

final result = extractSimple(html: html);
print('${result.contentText} ${result.metadata.title}');
```

### C# (.NET) ([full API docs](nuget/README.md))

```sh
dotnet add package Trafilatura
```

```csharp
using Trafilatura;

var result = Extractor.ExtractSimple(html);
Console.WriteLine($"{result.contentText} {result.metadata.title}");
```

### JavaScript / TypeScript (WASM)

```typescript
import { Trafilatura } from "./trafilatura.js";

const { extractSimple } = Trafilatura;
const result = extractSimple(html);
console.log(result.contentText, result.metadata.title);
```

## Benchmarks

### Speed

Extraction time per document, Rust vs Go ([go-trafilatura](https://github.com/markusmobius/go-trafilatura)) vs Python ([trafilatura](https://github.com/adbar/trafilatura)):

| Document | Rust | Go | Python |
|----------|-----:|---:|-------:|
| small (6 KB) | 606 µs | 885 µs | 1.12 ms |
| medium (85 KB) | 3.84 ms | 4.43 ms | 4.54 ms |
| large (382 KB) | 2.90 ms | 4.47 ms | 3.81 ms |
| xlarge (906 KB) | 8.81 ms | 12.0 ms | 11.5 ms |

### Extraction quality

Evaluated on a 960-entry dataset (strings expected to be present/absent in extracted text):

| Implementation | Precision | Recall | Accuracy | F-score |
|----------------|----------:|-------:|---------:|--------:|
| **Rust** (balanced + fallback) | 0.907 | 0.919 | 0.912 | 0.913 |
| Python trafilatura 2.3.1 | 0.920 | 0.920 | 0.920 | 0.920 |
| Go go-trafilatura v2.2.6 | 0.916 | 0.917 | 0.916 | 0.916 |

All three implementations produce near-identical quality scores. Minor differences stem from HTML parser handling and Unicode normalization.

Measured on Apple M4 Max, Rust 1.99, go-trafilatura v2.2.6, Python trafilatura 2.3.1, macOS 15.7.

Reproduce:

```sh
cargo bench --bench extraction -- single_document    # Rust speed (criterion medians)
cargo test --test comparison_test -- --nocapture     # Rust quality
cd /path/to/go-trafilatura && \
  go run /path/to/trafilatura-rs/scripts/bench_go.go \
    -testfiles /path/to/trafilatura-rs/test-files     # Go speed (single-doc medians)
cd /path/to/go-trafilatura && \
  go run /path/to/trafilatura-rs/scripts/compare_go.go \
    -corpus /path/to/trafilatura-rs/comparison-data/entries.json \
    -testfiles /path/to/trafilatura-rs/test-files     # Go quality
python3 scripts/bench_speed.py                        # Python speed (median)
python3 scripts/compare_python.py > /dev/null         # Python quality (stderr)
```

## Acknowledgements

- **[Adrien Barbaresi](https://github.com/adbar)** for [trafilatura](https://github.com/adbar/trafilatura) and [htmldate](https://github.com/adbar/htmldate) — the original Python libraries this crate is ultimately ported from, and the source of the date-extraction algorithm ported into `src/metadata/dates.rs`.
- **[Markus Mobius](https://github.com/markusmobius)** for [go-trafilatura](https://github.com/markusmobius/go-trafilatura) (the direct porting target), [go-htmldate](https://github.com/markusmobius/go-htmldate)/[rust-htmldate](https://github.com/markusmobius/rust-htmldate) (the Apache-2.0 Rust reference for the date cascade), and the test corpora: the 960-entry comparison dataset (serialized from go-trafilatura's `comparisonData`, itself derived from trafilatura's evaluation data) and the cached-page fixtures in `test-files/mock` used for metadata and date testing.

The date-extraction port is released under Apache-2.0, matching rust-htmldate and htmldate.

## License

Apache-2.0
