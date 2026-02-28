# trafilatura

[![Crates.io](https://img.shields.io/crates/v/trafilatura.svg)](https://crates.io/crates/trafilatura)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Rust: 1.80+](https://img.shields.io/badge/rust-1.80%2B-orange.svg)](https://www.rust-lang.org)

Extract readable content, comments, and metadata from web pages.

A Rust port of [go-trafilatura](https://github.com/markusmobius/go-trafilatura),
which itself ports the Python [trafilatura](https://github.com/adbar/trafilatura)
library by Adrien Barbaresi.

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
trafilatura = "0.1"
```

### Library

```rust
use trafilatura::{extract, Options};

let html = r#"<html><body>
  <nav>Menu items</nav>
  <article><p>This is the main article content.</p></article>
  <footer>Copyright 2024</footer>
</body></html>"#;

let result = extract(html, Options::default()).unwrap();
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

let result = extract(html, opts).unwrap();
```

### CLI

```sh
# Extract from a URL
trafilatura https://example.com/article

# Extract from a file
trafilatura path/to/page.html

# Include links in output
trafilatura --links https://example.com/article
```

## What it extracts

- **Content** — main article body as both plain text and cleaned HTML
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

## Benchmarks

### Speed

Extraction time per document, Rust vs Go ([go-trafilatura](https://github.com/markusmobius/go-trafilatura)) vs Python ([trafilatura](https://github.com/adbar/trafilatura)):

| Document | Rust | Go | Python |
|----------|-----:|---:|-------:|
| small (6 KB) | 793 µs | 1.19 ms | 1.1 ms |
| medium (85 KB) | 5.7 ms | 5.6 ms | 6.2 ms |
| large (382 KB) | 3.6 ms | 4.9 ms | 4.7 ms |
| xlarge (906 KB) | 10.4 ms | 13.9 ms | 13.9 ms |

### Extraction quality

Evaluated on a 960-entry dataset (strings expected to be present/absent in extracted text):

| Implementation | Precision | Recall | Accuracy | F-score |
|----------------|----------:|-------:|---------:|--------:|
| **Rust** (balanced + fallback) | 0.908 | 0.919 | 0.913 | 0.913 |
| Python trafilatura | 0.920 | 0.909 | 0.915 | 0.914 |
| Go go-trafilatura | 0.909 | 0.921 | 0.914 | 0.915 |

All three implementations produce near-identical quality scores. Minor differences stem from HTML parser handling and Unicode normalization.

Measured on Apple M4 Max, Rust 1.93, macOS 15.7.

Reproduce:

```sh
cargo bench                                            # speed benchmarks
cargo test --test comparison_test -- --nocapture       # Rust quality scores
python3 scripts/compare_python.py > /dev/null          # Python quality (stderr)
```

## License

Apache-2.0
