# trafilatura-rs Implementation Plan

## Context

[Trafilatura](https://github.com/adbar/trafilatura) is a web content extraction library that pulls readable text, comments, and metadata from HTML pages. The original is written in Python (using lxml/XPath). A Go port ([go-trafilatura](https://github.com/markusmobius/go-trafilatura)) already exists and will serve as our primary source for the Rust port.

**Why port from Go (not Python)**:
- Go already converted 60+ XPath expressions to CSS selectors — Rust has excellent CSS selector support (`scraper` crate) but no mature XPath library
- Go's static typing and explicit error handling map naturally to Rust's type system and `Result<T, E>`
- Go already solved DOM tree mutation patterns without Python's `deepcopy()` — these patterns translate cleanly to Rust
- Go's pre-compiled regex approach matches Rust's `regex` crate pattern

**Primary use case**: Rust library (`trafilatura` crate). Secondary: CLI binary for testing and standalone use.

**Source references**:
- Go codebase: `/Users/nchapman/Code/go-trafilatura` (primary porting source)
- Python codebase: `/Users/nchapman/Code/trafilatura` (reference for behavior, test cases, and edge cases)

**Target directory**: `/Users/nchapman/Code/lessisbetter/trafilatura-rs`

---

## Go Codebase Overview

Understanding the Go source is critical for the port. Here's the structural map:

### File Structure (~4.1K LOC core + ~1.7K LOC selectors + ~3.8K LOC tests)

```
go-trafilatura/
├── core.go (219 lines)              # Main API: Extract(), ExtractDocument()
├── core-options.go (199 lines)      # Options, Config, ExtractionFocus, FallbackCandidates
├── main-extractor.go (852 lines)    # Content/comment extraction pipeline — THE CORE
├── html-processing.go (557 lines)   # Document cleaning, tag conversion, link density, post cleaning
├── baseline.go (152 lines)          # Last-resort fallback extraction
├── external.go (242 lines)          # Readability/DomDistiller fallback comparison
├── metadata.go (774 lines)          # Metadata extraction from meta tags, OpenGraph, etc.
├── metadata-json.go (486 lines)     # JSON-LD schema.org parsing
├── tag-converter.go (25 lines)      # Tag category lists (listXmlListTags, etc.)
├── settings.go (129 lines)          # Tag catalogs (tags_to_clean, tags_to_strip, etc.)
├── helper.go (77 lines)             # CreateReadableDocument helper
├── utils-common.go (99 lines)       # String utilities (trim, isImageFile, uniquifyLists)
├── utils-extractor.go (149 lines)   # Extraction utilities (textFilter, duplicateTest, languageClassifier, checkHtmlLanguage)
├── url.go (120 lines)               # URL utilities (isAbsoluteURL, createAbsoluteURL, validateURL, getDomainURL, getBaseURL)
├── log-utils.go (53 lines)          # Logging utilities (logInfo, logWarn, logDebug, ellipsis)
│
├── internal/
│   ├── etree/ (2 files, ~390 lines) # DOM tree operations — TEXT/TAIL CONCEPT IS CRITICAL
│   │   ├── element.go (249 lines)   # Iter, IterDescendants, Text, SetText, Tail, SetTail, TailNodes, Append, Extend, IterText
│   │   └── etree.go (139 lines)     # Element, SubElement, StripTags, StripElements, Remove, Strip, ToString, FromString
│   │
│   ├── selector/ (16 files, ~1,657 lines) # CSS selector rule system for content/comment/metadata detection
│   │   ├── selector.go (48 lines)   # Rule type definition + Query/QueryAll functions
│   │   ├── content.go (234 lines)   # 5 content extraction rules
│   │   ├── comments.go (138 lines)  # 4 comment extraction rules
│   │   ├── content-discard-overall.go (227 lines) # Elements always discarded
│   │   ├── content-discard-precision.go (65 lines) # Precision-mode extra discards
│   │   ├── teaser-discard.go (54 lines) # Teaser/summary discards
│   │   ├── image-discard.go (54 lines)  # Image discards
│   │   ├── comments-discard.go (87 lines) # Comment-area discards
│   │   ├── comments-removed.go (63 lines) # Already-removed comment patterns
│   │   ├── meta-author.go (132 lines)    # Author metadata selectors
│   │   ├── meta-author-discard.go (93 lines) # Author area discard selectors (MISSED IN ORIGINAL PLAN)
│   │   ├── meta-title.go (93 lines)      # Title metadata selectors
│   │   ├── meta-categories.go (192 lines) # Category metadata selectors
│   │   ├── meta-tags.go (125 lines)       # Tag metadata selectors
│   │   └── utils.go (52 lines)            # String matching helpers (contains, startsWith, lower)
│   │
│   ├── lru/ (92 lines)              # LRU cache for text deduplication
│   └── re2go/ (4 files)             # re2c-generated state machine for text filter regex
│       ├── base.re (18 lines)       # re2c base template
│       ├── base.go (3 lines)        # Generated package declaration
│       ├── utils-extractor.re (32 lines) # Source pattern: RE_FILTER for social media/sharing text
│       └── utils-extractor.go (~1826 lines) # Generated state machine (DO NOT PORT — use Rust regex instead)
│
├── cmd/go-trafilatura/ (8 files)    # CLI tool
│   ├── main.go (231 lines)          # Root command, flags, handlers
│   ├── batch.go                     # Batch URL processing
│   ├── feed.go                      # RSS feed processing
│   ├── sitemap.go                   # Sitemap processing
│   └── output.go                    # Result formatting (HTML, text, JSON)
│
├── scripts/comparison/              # Comparison/benchmark framework
│   ├── root.go                      # CLI with "content" and "author" subcommands
│   ├── content.go                   # Precision/recall/accuracy comparison across extractors
│   ├── author.go                    # Author extraction accuracy comparison
│   ├── data.go (8,532 lines!)       # 900+ comparison entries with expected with/without strings
│   ├── table.go                     # Table formatting for results
│   └── utils.go                     # File loading helpers
│
├── test-files/                      # 1,077 HTML test fixtures
│   ├── mock/ (113 files)            # Real-world website captures for mock tests
│   ├── simple/ (38 files)           # Focused test documents (JSON-LD, etc.)
│   └── comparison/ (926 files)      # Large corpus for precision/recall evaluation
│
└── *_test.go (10 files, ~3,764 LOC)
    ├── trafilatura_test.go (1,518)  # Core extraction tests (95+ subcases)
    ├── metadata_test.go (611)       # Metadata unit tests (13 functions)
    ├── realworld_test.go (642)      # Real-world extraction validation
    ├── metadata-realworld_test.go (337) # Real-world metadata tests (37 pages)
    ├── metadata-json_test.go (176)  # JSON-LD parsing tests (23 cases)
    ├── baseline_test.go (165)       # Baseline/fallback tests
    ├── realworld-mock_test.go (113) # Mock file URL→filename mappings (89 URLs)
    ├── helper_test.go (102)         # Test utilities
    ├── html-processing_test.go (59) # HTML cleaning tests
    └── url_test.go (41)             # URL handling tests
```

### Extraction Pipeline (from `core.go`)

This is the main data flow through `ExtractDocument()`:

```
1.  Set default config (if nil)
2.  Create LRU cache for dedup
3.  HTML language check (if target_language set)
4.  Extract metadata (meta tags, JSON-LD, OpenGraph)      ← REQUIRES metadata module
5.  Check essential metadata (title, URL, date)
6.  Update URL from metadata if not provided
7.  Apply user's PruneSelector (custom CSS pruning)         ← REQUIRES selector module
8.  Clone document 3x (main, fallback backup, baseline backup)
9.  docCleaning(doc) — remove script/style/nav/ads/etc.    ← REQUIRES html_processing module
10. convertTags(doc) — normalize tags (summary→b, div→p, etc.)
11. extractComments(doc) — extract + remove comment sections  ← REQUIRES main-extractor module
12. extractContent(doc) — MAIN content extraction via selector rules
13. compareExternalExtraction() — try readability fallback    ← REQUIRES external module
14. baseline(docBackup2) — last resort: JSON-LD articleBody   ← REQUIRES baseline module
15. Tree size sanity check
16. Size checks + deduplication
17. Language classification
18. postCleaning()
19. Return ExtractResult { ContentNode, CommentsNode, ContentText, CommentsText, Metadata }
```

**Critical dependency insight**: `ExtractDocument()` calls `extractMetadata()` _before_ any content extraction. This means Phase 6 (metadata) is NOT parallel with content extraction — it is a **prerequisite** for the core API integration. The original plan incorrectly showed metadata as a parallel track.

### Selector System (from `internal/selector/`)

The Go code defines extraction rules as predicate functions:

```go
type Rule func(*html.Node) bool
```

Each rule checks tag name + CSS class/ID/attributes. Example (content rule 1):
- Match `<article>`, `<div>`, `<main>`, or `<section>` elements
- Where class contains "post-text", "post-content", "article-body", "entry-content", etc.
- Or where `itemprop="articleBody"`
- 40+ string patterns checked via `contains()`, `startsWith()`, `lower()`

Rules are tried in order; first match wins. Five content rules, four comment rules, multiple discard rule sets.

### Text/Tail Concept (from `internal/etree/element.go`)

This is ported from Python's ElementTree model and is **critical to get right**:

```
<p>Hello <b>world</b> and more</p>
     ↑         ↑      ↑
     text(p)   text(b) tail(b)
```

- `text(element)`: text content before the first child element
- `tail(element)`: text after the element's closing tag, before the next sibling
- When removing an element, its tail text must be preserved (moved to previous sibling or parent)
- When appending, tail nodes must move with the element

### Comparison Framework (from `scripts/comparison/`)

The Go project includes a comparison tool that evaluates extraction quality:

```go
type ComparisonEntry struct {
    File    string   // HTML fixture filename
    With    []string // Strings that SHOULD appear in extraction output
    Without []string // Strings that SHOULD NOT appear
    Title, Date, Sitename, Description, License, Region string
    Authors, Comments, Categories, Tags []string
}
```

926 comparison entries with expected `with`/`without` strings. The tool runs multiple extractors against all entries and calculates precision, recall, accuracy, and F-score:

```
Precision = TP / (TP + FP)
Recall    = TP / (TP + FN)
Accuracy  = (TP + TN) / (TP + TN + FP + FN)
F-Score   = 2*TP / (2*TP + FP + FN)
```

Extractors compared: Readability, DomDistiller, Trafilatura (balanced/precision/recall modes).

### Go Dependencies → Rust Equivalents

| Go Dependency | Purpose | Rust Equivalent |
|---------------|---------|-----------------|
| `golang.org/x/net/html` | HTML parsing | `scraper` (wraps `html5ever`) |
| `go-shiori/dom` | DOM manipulation | Build wrapper over `ego-tree` (exposed by scraper) |
| `andybalholm/cascadia` | CSS selectors | `selectors` (built into scraper) |
| `golang.org/x/text` | Unicode handling | `unicode-segmentation`, std lib |
| `forPelevin/gomoji` | Emoji handling | `unicode-emoji` or `emojis` |
| `RadhiFadlillah/whatlanggo` | Language detection | `whatlang` (direct Rust port) |
| `markusmobius/go-htmldate` | Date extraction | `chrono` + custom patterns (see Phase 6 notes) |
| `go-shiori/go-readability` | Readability fallback | `readability` crate (evaluate quality first) |
| `markusmobius/go-domdistiller` | DomDistiller fallback | Skip initially; readability covers most cases |
| `matoous/go-nanoid` | ID generation | `nanoid` crate |
| `rs/zerolog` | Structured logging | `tracing` + `tracing-subscriber` |
| `spf13/cobra` | CLI framework | `clap` v4 (derive macros) |
| `stretchr/testify` | Test assertions | `assert!` + `pretty_assertions` |
| `yosssi/gohtml` | HTML formatting | Custom serialization |
| `beevik/etree` | XML tree operations | Not needed (we handle trees differently) |

---

## Rust Project Architecture

```
trafilatura-rs/
├── Cargo.toml                       # Workspace with lib + CLI binary
├── PLAN.md                          # This file
├── src/
│   ├── lib.rs                       # Public API: extract(), extract_from_reader()
│   ├── error.rs                     # TrafilaturaError enum
│   ├── options.rs                   # Options, Config, ExtractionFocus enums/structs
│   ├── result.rs                    # ExtractResult, Metadata structs
│   │
│   ├── dom/                         # DOM abstraction layer (wraps scraper/ego-tree)
│   │   ├── mod.rs                   # Document struct, NodeId type
│   │   ├── tree.rs                  # Mutable tree: text/tail, clone, remove, append, strip
│   │   └── query.rs                 # CSS selector helpers, query/query_all
│   │
│   ├── selector/                    # Extraction rule predicates
│   │   ├── mod.rs                   # Rule type definition + query/query_all
│   │   ├── content.rs              # 5 content extraction rules
│   │   ├── comments.rs             # 4 comment extraction rules
│   │   ├── discard.rs              # Overall, precision, teaser, image, comment discard rules
│   │   ├── metadata.rs             # Author, author-discard, title, categories, tags selectors
│   │   └── utils.rs                # String matching helpers
│   │
│   ├── extraction/                  # Core extraction pipeline
│   │   ├── mod.rs                   # extract_content(), extract_comments() orchestration
│   │   ├── elements.rs             # handle_text_elem() dispatch + all element handlers
│   │   ├── html_processing.rs      # doc_cleaning, convert_tags, post_cleaning, link density
│   │   ├── baseline.rs             # Last-resort extraction (JSON-LD body, <article>, <p>)
│   │   └── external.rs             # Readability fallback comparison logic
│   │
│   ├── metadata/                    # Metadata extraction
│   │   ├── mod.rs                   # extract_metadata() orchestration
│   │   └── json_ld.rs              # JSON-LD schema.org parsing
│   │
│   ├── utils/                       # Shared utilities
│   │   ├── mod.rs                   # trim(), string helpers, URL utilities
│   │   ├── lru.rs                  # LRU cache for text deduplication
│   │   ├── text.rs                 # textFilter, textCharsTest, duplicateTest
│   │   ├── url.rs                  # isAbsoluteURL, createAbsoluteURL, validateURL, getDomainURL, getBaseURL
│   │   ├── regex_patterns.rs       # All compiled regex patterns (LazyLock<Regex>)
│   │   └── language.rs             # Language classification wrapper (whatlang) + checkHtmlLanguage
│   │
│   └── settings.rs                  # Tag catalogs as static HashSets + tag category lists
│
├── src/bin/
│   └── trafilatura.rs               # CLI binary entry point (clap)
│
├── tests/
│   ├── common/mod.rs                # Test helpers (load_mock_file, extract_mock_file, etc.)
│   ├── extraction_test.rs           # Core extraction tests (from trafilatura_test.go)
│   ├── metadata_test.rs             # Metadata tests (from metadata_test.go)
│   ├── metadata_json_test.rs        # JSON-LD tests (from metadata-json_test.go)
│   ├── baseline_test.rs             # Baseline/fallback tests
│   ├── realworld_test.rs            # Real-world extraction validation (95+ subcases)
│   ├── html_processing_test.rs      # HTML cleaning tests
│   ├── url_test.rs                  # URL utility tests (from url_test.go)
│   └── comparison_test.rs           # Precision/recall/accuracy framework
│
├── test-files/                      # COPIED from go-trafilatura/test-files (see Practical Decisions)
│   ├── mock/ (113 files)
│   ├── simple/ (38 files)
│   └── comparison/ (926 files)
│
├── comparison-data/
│   └── entries.json                 # Comparison data (generated from Go's data.go — see Phase 10)
│
├── benches/
│   └── extraction.rs                # Criterion benchmarks
│
└── scripts/
    └── convert_comparison_data.py   # Script to convert Go data.go → JSON
```

---

## Practical Decisions

### Test files: Copy, not symlink

The original plan suggests symlinking test-files from the Go repo. **Use `cp -r` instead.**

Reasons:
- Symlinks break in CI (no Go repo available on CI runners)
- Symlinks make the Rust project depend on the Go project's file layout
- The test files are static HTML fixtures (they never change independently)
- 926 HTML files are small (~50MB total) — acceptable in-repo

Add `test-files/` to `.gitignore` and document the copy command. Or better: commit the test files to the Rust repo directly.

### Crate choice: `scraper` is viable but needs care

The `scraper` crate (v0.22) wraps `html5ever` and `ego-tree`. Key facts:
- `ego-tree` uses `NodeId` handles (indices into a `Vec`), which avoids Rust borrow issues
- `scraper::Html` gives mutable access to the underlying `ego_tree::Tree<scraper::Node>`
- The `Node` enum has variants for `Document`, `Element`, `Text`, `Comment`, etc.
- **Mutation is possible**: `ego_tree::NodeMut` allows inserting, appending, detaching children
- **Key limitation**: `scraper` itself provides no mutation API — you must access the raw `ego-tree` methods through `Html::tree`

The text/tail concept maps to: text = first `Text` child before any `Element` child; tail = `Text` siblings immediately after the element. This is exactly how Go's `etree` package works with `html.Node.FirstChild`/`NextSibling`.

**Recommendation**: Use `scraper` for parsing and CSS selectors. Build a `Document` wrapper that adds text/tail methods operating on the underlying `ego-tree` directly. This avoids maintaining a fork.

### Use `std::sync::LazyLock` (stable since Rust 1.80), not `once_cell`

`LazyLock` is now stable in std. No need for the `once_cell` dependency.

### re2go state machines: Use Rust regex instead

The Go codebase uses `re2c`-generated state machines for the `IsTextFilter` function (1,826 lines of generated Go). The _source_ pattern is just:

```
(?i)\W*(Drucken|E-?Mail|Facebook|Flipboard|Google|Instagram|Linkedin|Mail|PDF|Pinterest|Pocket|Print|QQ|Reddit|Twitter|WeChat|WeiBo|Whatsapp|Xing|Mehr zum Thema:?|More on this.{,8})$
```

In Rust, compile this as a `LazyLock<Regex>`. The Rust `regex` crate is fast enough — no need for hand-rolled state machines.

### Date extraction: Start simple, evaluate later

The Go code uses `go-htmldate` (a port of Python's `htmldate`). There is no mature Rust equivalent. For Phase 6, implement only metadata-level date extraction (from `<meta>` tags, JSON-LD, OpenGraph). Skip deep document scanning for dates initially — this can be added as an enhancement. This avoids the need to port an entire date extraction library.

### `FallbackCandidates` and `HtmlDateOverride` options: Defer

The Go `Options` struct has `FallbackCandidates`, `HtmlDateOverride`, and `HtmlDateOptions` fields that let callers inject pre-computed extraction results. These are advanced API features. For the initial port, support only `EnableFallback: bool` (which triggers internal readability extraction). The injection API can be added later if users need it.

---

## Detailed Phase Breakdown

### Phase 1: Project Scaffold & DOM Abstraction

> **Status: ✅ COMPLETED** — commits `2efc601` (scaffold) + `2597c00` (DOM abstraction)

**Goal**: Set up the project structure and build the DOM layer that everything else depends on. This is the most important foundation — get it wrong and everything downstream suffers.

**Dependencies**: None (first phase).

**Outputs**: `Cargo.toml`, `src/dom/` module, unit tests for all DOM operations.

#### 1.1 Project Setup

Create `Cargo.toml` with:
```toml
[package]
name = "trafilatura"
version = "0.1.0"
edition = "2021"
rust-version = "1.80"  # Required for std::sync::LazyLock

[lib]
name = "trafilatura"
path = "src/lib.rs"

[[bin]]
name = "trafilatura"
path = "src/bin/trafilatura.rs"

[dependencies]
scraper = "0.22"          # HTML parsing + CSS selectors
ego-tree = "0.10"         # Tree structure (re-exported by scraper)
regex = "1"               # Regular expressions
chrono = "0.4"            # Date/time handling
url = "2"                 # URL parsing
serde = { version = "1", features = ["derive"] }
serde_json = "1"          # JSON-LD parsing
whatlang = "0.16"         # Language detection
tracing = "0.1"           # Structured logging
tracing-subscriber = "0.3"
thiserror = "2"           # Error type derivation

[dev-dependencies]
pretty_assertions = "1"
criterion = { version = "0.5", features = ["html_reports"] }

[[bench]]
name = "extraction"
harness = false
```

Copy test fixtures:
```bash
cp -r /Users/nchapman/Code/go-trafilatura/test-files test-files
```

Initialize git repo.

#### 1.2 DOM Abstraction Layer

This wraps `scraper::Html` / `ego_tree::Tree<scraper::Node>` to provide the operations needed by the extraction pipeline. The key insight: use `ego_tree::NodeId` as handles to avoid Rust borrow checker issues.

**`src/dom/mod.rs`** — Document struct and NodeId type:

```rust
use scraper::{Html, Node, Selector};
use ego_tree::NodeId;

pub struct Document {
    tree: ego_tree::Tree<Node>,
}

impl Document {
    /// Parse HTML string into a mutable Document
    pub fn parse(html: &str) -> Self;

    /// Get a reference to a node by ID
    pub fn get(&self, id: NodeId) -> &Node;

    /// Get the root element node
    pub fn root(&self) -> NodeId;

    /// Get the <body> element, if any
    pub fn body(&self) -> Option<NodeId>;
}
```

**`src/dom/tree.rs`** — Mutable tree operations (port of `internal/etree/element.go` + `etree.go`):

The text/tail concept must be implemented carefully. In the underlying tree, text nodes are `html::TextNode` children/siblings of element nodes. The Go code accesses them via `Text()` and `Tail()`:

```rust
impl Document {
    // --- Text/Tail (port of etree/element.go:96-192) ---

    /// Get text before the first child element (element's direct text content)
    pub fn text(&self, node: NodeId) -> String;

    /// Set text before the first child element
    pub fn set_text(&mut self, node: NodeId, text: &str);

    /// Get text after element's end tag, before next sibling's start tag
    pub fn tail(&self, node: NodeId) -> String;

    /// Set text after element's end tag
    pub fn set_tail(&mut self, node: NodeId, tail: &str);

    /// Get the list of tail text nodes for an element
    pub fn tail_nodes(&self, node: NodeId) -> Vec<NodeId>;

    // --- Tree Mutation (port of etree/etree.go) ---

    /// Create a new element node with the given tag name
    pub fn create_element(&mut self, tag: &str) -> NodeId;

    /// Create a new text node
    pub fn create_text_node(&mut self, text: &str) -> NodeId;

    /// Append child, preserving tail text nodes
    /// Port of etree.Append — must move tail text nodes along with the element
    pub fn append_child(&mut self, parent: NodeId, child: NodeId);

    /// Append multiple children (port of etree.Extend)
    pub fn extend(&mut self, parent: NodeId, children: Vec<NodeId>);

    /// Create element and append as child (port of etree.SubElement)
    pub fn sub_element(&mut self, parent: NodeId, tag: &str) -> NodeId;

    /// Remove element from tree (optionally preserving tail text)
    /// Port of etree.Remove — when keepTail is false, tail text nodes are also removed
    pub fn remove(&mut self, node: NodeId, keep_tail: bool);

    /// Strip tags: remove tag but keep text content and children
    /// Port of etree.StripTags — processes in reverse order to avoid invalidation
    pub fn strip_tags(&mut self, root: NodeId, tags: &[&str]);

    /// Strip elements: remove tag AND all children
    /// Port of etree.StripElements
    pub fn strip_elements(&mut self, root: NodeId, keep_tail: bool, tags: &[&str]);

    /// Strip single element: remove tag but merge children into parent
    /// Port of etree.Strip — clones children, inserts before element, removes element
    pub fn strip(&mut self, node: NodeId);

    /// Deep clone the entire document
    pub fn clone_document(&self) -> Document;

    // --- Tree Traversal (port of etree/element.go:31-92) ---

    /// Iterate element and all subelements in document order, filtered by tags
    /// Port of etree.Iter — includes element itself if it matches
    pub fn iter(&self, node: NodeId, tags: &[&str]) -> Vec<NodeId>;

    /// Like iter() but excludes the starting element itself
    /// Port of etree.IterDescendants
    pub fn iter_descendants(&self, node: NodeId, tags: &[&str]) -> Vec<NodeId>;

    /// Collect all inner text with level-aware separators
    /// Port of etree.IterText — adds separator when element level changes
    pub fn iter_text(&self, node: NodeId, separator: &str) -> String;

    /// Get all text content (no separators) — equivalent to Go's dom.TextContent
    pub fn text_content(&self, node: NodeId) -> String;

    // --- Element Access ---

    pub fn tag_name(&self, node: NodeId) -> &str;
    pub fn set_tag_name(&mut self, node: NodeId, tag: &str);
    pub fn id(&self, node: NodeId) -> String;
    pub fn class_name(&self, node: NodeId) -> String;
    pub fn get_attribute(&self, node: NodeId, name: &str) -> Option<String>;
    pub fn set_attribute(&mut self, node: NodeId, name: &str, value: &str);
    pub fn remove_attribute(&mut self, node: NodeId, name: &str);
    pub fn clear_attributes(&mut self, node: NodeId);
    pub fn children(&self, node: NodeId) -> Vec<NodeId>;
    pub fn child_nodes(&self, node: NodeId) -> Vec<NodeId>; // includes text nodes
    pub fn parent(&self, node: NodeId) -> Option<NodeId>;
    pub fn next_sibling(&self, node: NodeId) -> Option<NodeId>;
    pub fn prev_element_sibling(&self, node: NodeId) -> Option<NodeId>;
    pub fn next_element_sibling(&self, node: NodeId) -> Option<NodeId>;
    pub fn is_element(&self, node: NodeId) -> bool;
    pub fn is_text(&self, node: NodeId) -> bool;
    pub fn is_void_element(&self, node: NodeId) -> bool;

    // --- Serialization ---

    /// Port of etree.ToString — serializes element + tail text
    pub fn to_string(&self, node: NodeId, pretty: bool) -> String;
    pub fn inner_html(&self, node: NodeId) -> String;
    pub fn outer_html(&self, node: NodeId) -> String;
}
```

**`src/dom/query.rs`** — CSS selector queries:

```rust
impl Document {
    /// Find first element matching CSS selector
    pub fn query_selector(&self, root: NodeId, selector: &str) -> Option<NodeId>;

    /// Find all elements matching CSS selector
    pub fn query_selector_all(&self, root: NodeId, selector: &str) -> Vec<NodeId>;

    /// Get all elements with matching tag name (equivalent to Go's dom.GetElementsByTagName)
    pub fn get_elements_by_tag_name(&self, root: NodeId, tag: &str) -> Vec<NodeId>;

    /// Set inner HTML of an element (used in baseline.go for parsing articleBody HTML)
    pub fn set_inner_html(&mut self, node: NodeId, html: &str);
}
```

**Tests for Phase 1**: Test every DOM operation individually. Key test cases:
- Text/tail on `<p>Hello <b>world</b> and more</p>` → text(p)="Hello ", text(b)="world", tail(b)=" and more"
- Remove element with tail preservation (both keepTail=true and keepTail=false)
- Strip tags keeps content but removes wrapping element
- Append moves tail text nodes along with the element (port of `etree.Append` behavior)
- Clone produces independent subtrees (mutations don't propagate)
- `iter_text` produces correct whitespace-separated output with level-aware separators
- CSS selector queries match expected elements
- `set_tag_name` works (Go uses `element.Data = "done"` and `element.Data = "p"` extensively)
- `strip_tags` processes in reverse order (prevents parent-before-child issues)

#### Acceptance Criteria
- [x] `Document::parse()` can parse any HTML string from test-files/
- [x] All text/tail operations match Go etree behavior on the same HTML input
- [x] `clone_document()` produces a fully independent copy
- [x] `strip_tags`, `strip_elements`, `remove`, `strip` all match Go behavior
- [x] `iter_text` matches Go's `etree.IterText` output on 5+ different HTML structures
- [x] CSS selector queries produce same matches as Go's `dom.QuerySelectorAll`
- [x] At least 30 unit tests covering all DOM operations
- [x] `cargo test` passes, `cargo clippy` clean

---

### Phase 2: Settings, Options, URL & Utility Infrastructure

> **Status: ✅ COMPLETED** — commit `1471f20`

**Goal**: Port configuration types, tag catalogs, tag category lists, regex patterns, LRU cache, string utilities, URL utilities, and text filtering utilities.

**Dependencies**: Phase 1 (DOM abstraction — needed for `textFilter` which uses `etree.Text`/`etree.Tail`).

**Outputs**: `src/options.rs`, `src/result.rs`, `src/error.rs`, `src/settings.rs`, `src/utils/`

#### 2.1 Options & Config (port of `core-options.go`)

```rust
// src/options.rs
#[derive(Debug, Clone)]
pub struct Options {
    pub config: Config,
    pub original_url: Option<url::Url>,
    pub target_language: Option<String>,
    pub enable_fallback: bool,
    pub focus: ExtractionFocus,
    pub exclude_comments: bool,
    pub exclude_tables: bool,
    pub include_images: bool,
    pub include_links: bool,
    pub blacklisted_authors: Vec<String>,
    pub deduplicate: bool,
    pub has_essential_metadata: bool,
    pub max_tree_size: Option<usize>,
    pub prune_selector: Option<String>,
    pub enable_log: bool,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub cache_size: usize,              // default: 4096
    pub min_duplicate_check_size: usize, // default: 100
    pub max_duplicate_count: usize,      // default: 2
    pub min_extracted_size: usize,       // default: 250
    pub min_extracted_comment_size: usize, // default: 1
    pub min_output_size: usize,          // default: 1
    pub min_output_comment_size: usize,  // default: 1
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExtractionFocus {
    Balanced,
    FavorRecall,
    FavorPrecision,
}
```

#### 2.2 Result Types (port of `core.go:49-68` + `metadata.go`)

```rust
// src/result.rs
pub struct ExtractResult {
    pub content_text: String,
    pub comments_text: String,
    pub content_html: String,     // Serialized content DOM
    pub comments_html: String,    // Serialized comments DOM
    pub metadata: Metadata,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Metadata {
    pub title: String,
    pub author: String,
    pub url: String,
    pub hostname: String,
    pub description: String,
    pub sitename: String,
    pub date: Option<chrono::NaiveDate>,
    pub categories: Vec<String>,
    pub tags: Vec<String>,
    pub id: String,
    pub fingerprint: String,
    pub license: String,
    pub language: String,
    pub image: String,
    pub page_type: String,
}
```

#### 2.3 Error Type

```rust
// src/error.rs
#[derive(Debug, thiserror::Error)]
pub enum TrafilaturaError {
    #[error("failed to parse HTML: {0}")]
    ParseError(String),

    #[error("wrong language: expected {expected}, got {got}")]
    LanguageMismatch { expected: String, got: String },

    #[error("text and comments not long enough: {text_len} / {comment_len}")]
    InsufficientContent { text_len: usize, comment_len: usize },

    #[error("missing required metadata: {0}")]
    MissingMetadata(String),

    #[error("extracted body is a duplicate")]
    DuplicateContent,

    #[error("output tree too large: {0} elements")]
    TreeTooLarge(usize),
}
```

#### 2.4 Tag Catalogs & Tag Category Lists (port of `settings.go` + `tag-converter.go`)

```rust
// src/settings.rs
use std::collections::HashSet;
use std::sync::LazyLock;

pub static TAGS_TO_CLEAN: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    ["aside", "embed", "footer", "form", "head", "iframe", "menu", "object", "script",
     "applet", "audio", "canvas", "figure", "map", "picture", "svg", "video",
     "area", "blink", "button", "datalist", "dialog", "frame", "frameset", "fieldset",
     "link", "input", "ins", "label", "legend", "marquee", "math", "menuitem", "nav",
     "noscript", "optgroup", "option", "output", "param", "progress", "rp", "rt", "rtc",
     "select", "source", "style", "track", "textarea", "time", "use"]
        .into_iter().collect()
});

// Similarly for TAGS_TO_STRIP, EMPTY_TAGS_TO_REMOVE, TAG_CATALOG,
// FORMAT_TAG_CATALOG, VALID_TAG_CATALOG, ELEMENT_WITH_SIZE_ATTR, ALLOWED_ATTRIBUTES
// Also the tag category lists from tag-converter.go:
// LIST_XML_LIST_TAGS, LIST_XML_QUOTE_TAGS, LIST_XML_HEAD_TAGS, LIST_XML_LB_TAGS,
// LIST_XML_HI_TAGS, LIST_XML_REF_TAGS, LIST_XML_GRAPHIC_TAGS, LIST_XML_ITEM_TAGS,
// LIST_XML_CELL_TAGS (both as slices and as HashSets for O(1) lookup)
```

**Important**: The plan must include `tag-converter.go`'s tag category lists (not just `settings.go`). These lists (`mapXmlListTags`, `mapXmlQuoteTags`, etc.) are used pervasively in `main-extractor.go` for element dispatch. They are only 25 lines but are load-bearing.

#### 2.5 LRU Cache (port of `internal/lru/cache.go`, 92 lines)

Simple LRU cache that stores text strings and counts how many times each has been seen. Used for deduplication.

```rust
// src/utils/lru.rs
pub struct LruCache {
    capacity: usize,
    keys: Vec<String>,          // Insertion-ordered for FIFO eviction
    data: HashMap<String, usize>,
}

impl LruCache {
    pub fn new(capacity: usize) -> Self;
    pub fn get(&self, key: &str) -> Option<usize>;
    pub fn put(&mut self, key: String, value: usize);
    pub fn clear(&mut self);
}
```

Note: The Go LRU uses simple FIFO eviction (oldest key removed), not true LRU. Match this behavior exactly. No need for `linked-hash-map` — a `Vec<String>` + `HashMap` suffices (matching the Go implementation).

#### 2.6 Regex Patterns (port of `internal/re2go/` + patterns from `metadata.go` + `utils-extractor.go`)

All regex patterns compiled once at startup via `LazyLock<Regex>`:

```rust
// src/utils/regex_patterns.rs
use std::sync::LazyLock;
use regex::Regex;

// From re2go/utils-extractor.re (the text filter pattern):
pub static TEXT_FILTER: LazyLock<Regex> = LazyLock::new(||
    Regex::new(r"(?i)\W*(Drucken|E-?Mail|Facebook|Flipboard|Google|Instagram|Linkedin|Mail|PDF|Pinterest|Pocket|Print|QQ|Reddit|Twitter|WeChat|WeiBo|Whatsapp|Xing|Mehr zum Thema:?|More on this.{0,8})$").unwrap()
);

// From metadata.go (20+ patterns):
pub static AUTHOR_PREFIX: LazyLock<Regex> = LazyLock::new(||
    Regex::new(r"(?i)^([a-zäöüß]+(ed|t))? ?(written by|words by|words|by|von|from) ").unwrap()
);
// ... rx_title_cleaner, rx_json_symbol, rx_name_json, rx_url_check, rx_sitename_finder,
//     rx_html_strip_tag, rx_category_href, rx_tag_href, rx_cc_license, rx_cc_license_text,
//     rx_author_digits, rx_author_social_media, rx_author_space_chars, rx_author_nickname,
//     rx_author_special_chars, rx_author_preposition, rx_author_email, rx_author_separator,
//     rx_author_html

// From utils-extractor.go:
pub static HTML_LANG: LazyLock<Regex> = LazyLock::new(||
    Regex::new(r"(?i)[a-z]{2}").unwrap()
);
```

Total: ~25 regex patterns. Port each one from Go, testing individually.

#### 2.7 URL Utilities (port of `url.go`, 120 lines)

**This was missing from the original plan.** The `url.go` file contains 5 functions used by metadata extraction and tag conversion:

```rust
// src/utils/url.rs
/// Check if URL is absolute HTTP(S)
pub fn is_absolute_url(s: &str) -> bool;

/// Convert relative URL to absolute using base URL
pub fn create_absolute_url(url_str: &str, base: Option<&url::Url>) -> String;

/// Extract hostname from URL
pub fn get_domain_url(url_str: &str) -> String;

/// Extract base URL (scheme + hostname) from URL
pub fn get_base_url(url_str: &str) -> String;

/// Validate and optionally absolutize a URL
pub fn validate_url(url_str: &str, base_url: Option<&url::Url>) -> (String, bool);
```

#### 2.8 Text Utilities (port of `utils-common.go` + `utils-extractor.go`)

```rust
// src/utils/text.rs (port of utils-extractor.go)
/// Filter out lines containing social sharing text (Drucken, Facebook, etc.)
pub fn text_filter(doc: &Document, node: NodeId) -> bool;

/// Check if string has non-whitespace content
pub fn text_chars_test(s: &str) -> bool;

/// Check for duplicate text within cache
pub fn duplicate_test(doc: &Document, node: NodeId, cache: &mut LruCache, opts: &Options) -> bool;

// src/utils/mod.rs (port of utils-common.go)
/// Collapse whitespace and trim
pub fn trim(s: &str) -> String;

/// Count words in string
pub fn str_word_count(s: &str) -> usize;

/// Return first non-empty string
pub fn str_or(args: &[&str]) -> &str;

/// Check if element is a valid img element
pub fn is_image_element(doc: &Document, node: NodeId) -> bool;

/// Check if file path looks like an image
pub fn is_image_file(src: &str) -> bool;

/// Deduplicate a list of strings
pub fn uniquify_lists(items: &[String]) -> Vec<String>;
```

#### 2.9 Language Detection Wrapper (port of `utils-extractor.go`)

```rust
// src/utils/language.rs
/// Detect language from extracted text (port of languageClassifier)
pub fn language_classifier(body_text: &str, comments_text: &str) -> String;

/// Check HTML meta elements for language information (port of checkHtmlLanguage)
pub fn check_html_language(doc: &Document, opts: &Options, strict: bool) -> bool;
```

**Tests for Phase 2**:
- LRU cache insert/evict/count (matching Go FIFO behavior exactly)
- All regex patterns compile and match expected inputs
- Config defaults match Go's DefaultConfig()
- URL utilities: test absolute URL detection, relative-to-absolute conversion, domain extraction
- `trim()` collapses multiple spaces: `"  hello   world  "` → `"hello world"`
- `text_filter` detects social sharing lines ("Facebook", "Drucken", etc.)
- `is_image_file` detects image extensions
- Port `url_test.go` (41 lines)

#### Acceptance Criteria
- [x] All 25+ regex patterns compile and match test inputs from Go source
- [x] LRU cache matches Go behavior (FIFO eviction, count tracking)
- [x] URL utilities pass all cases from `url_test.go`
- [x] `trim()` behavior matches Go's `trim()` (Fields + Join + TrimSpace)
- [x] `text_filter` correctly identifies social media sharing lines
- [x] Config defaults produce same values as Go's `DefaultConfig()`
- [x] `cargo test` passes, `cargo clippy` clean

---

### Phase 3: Selector System

> **Status: ✅ COMPLETED** — commit `a9de1a6`

**Goal**: Port the CSS selector predicate rules that identify content, comments, and metadata regions.

**Dependencies**: Phase 1 (DOM abstraction — selectors need `doc.tag_name()`, `doc.id()`, `doc.class_name()`, `doc.get_attribute()`).

**Outputs**: `src/selector/` module with all rule functions.

#### 3.1 Rule Type & Query Functions (port of `selector/selector.go`)

```rust
// src/selector/mod.rs

/// A selector rule is a predicate function that checks if an element matches
pub type Rule = fn(&Document, NodeId) -> bool;

/// Find first element in tree matching the rule (depth-first)
/// Port of selector.Query — iterates all elements via get_elements_by_tag_name("*")
pub fn query(doc: &Document, root: NodeId, rule: Rule) -> Option<NodeId>;

/// Find all elements in tree matching the rule (depth-first)
/// Port of selector.QueryAll
pub fn query_all(doc: &Document, root: NodeId, rule: Rule) -> Vec<NodeId>;

// Exported rule lists
pub static CONTENT_RULES: &[Rule] = &[
    content_rule_1, content_rule_2, content_rule_3, content_rule_4, content_rule_5,
];
pub static COMMENT_RULES: &[Rule] = &[
    comment_rule_1, comment_rule_2, comment_rule_3, comment_rule_4,
];
pub static OVERALL_DISCARDED: &[Rule] = &[...];
pub static PRECISION_DISCARDED: &[Rule] = &[...];
pub static DISCARDED_TEASER: &[Rule] = &[...];
pub static DISCARDED_IMAGE: &[Rule] = &[...];
pub static DISCARDED_COMMENTS: &[Rule] = &[...];
pub static REMOVED_COMMENTS: &[Rule] = &[...];
pub static META_AUTHOR: &[Rule] = &[...];
pub static META_AUTHOR_DISCARD: &[Rule] = &[...];  // From meta-author-discard.go (MISSED IN ORIGINAL PLAN)
pub static META_TITLE: &[Rule] = &[...];
pub static META_CATEGORIES: &[Rule] = &[...];
pub static META_TAGS: &[Rule] = &[...];
```

#### 3.2 Content Rules (port of `selector/content.go`, 234 lines)

Port all 5 content rules. Each is pure string matching on tag/id/class/attributes.

#### 3.3 Comment Rules (port of `selector/comments.go`, 138 lines)

Port all 4 comment rules.

#### 3.4 Discard Rules (port of 5 files, ~530 lines total)

Port from `content-discard-overall.go`, `content-discard-precision.go`, `teaser-discard.go`, `image-discard.go`, `comments-discard.go`, `comments-removed.go`.

#### 3.5 Metadata Selectors (port of 5 files, ~635 lines total)

Port from `meta-author.go`, `meta-author-discard.go`, `meta-title.go`, `meta-categories.go`, `meta-tags.go`.

#### 3.6 String Matching Utils (port of `selector/utils.go`, 52 lines)

```rust
// src/selector/utils.rs
pub fn contains(haystack: &str, needle: &str) -> bool;
pub fn starts_with(haystack: &str, prefix: &str) -> bool;
pub fn lower(s: &str) -> String;
```

**Tests for Phase 3**: Test each content rule against sample HTML fragments. Verify rule ordering produces expected first-match behavior. Test that discard rules match the expected elements.

#### Acceptance Criteria
- [x] All 16 selector files ported (5 content + 4 comment + 7 discard/metadata)
- [x] Each rule produces same true/false results as Go on sample HTML elements
- [x] `query()` returns first matching element in document order
- [x] `query_all()` returns all matching elements in document order
- [x] Content rules 1-5 match in priority order (first match wins in extraction loop)
- [x] At least 20 unit tests covering representative rules from each category
- [x] `cargo test` passes, `cargo clippy` clean

---

### Phase 4: HTML Processing Pipeline

> **Status: ✅ COMPLETED** — commit `05215a6`

**Goal**: Port document cleaning, tag conversion, link density analysis, and post-processing.

**Dependencies**: Phase 1 (DOM), Phase 2 (settings/tag catalogs/regex/URL utils), Phase 3 (selector rules — `prune_unwanted_nodes` uses `selector.Rule`).

**Outputs**: `src/extraction/html_processing.rs`

#### 4.1 Document Cleaning (port of `html-processing.go` top section, ~140 lines)

```rust
/// Remove unwanted elements (scripts, ads, navigation, etc.)
/// Port of docCleaning — uses TAGS_TO_CLEAN, TAGS_TO_STRIP, EMPTY_TAGS_TO_REMOVE
pub fn doc_cleaning(doc: &mut Document, opts: &Options);

/// Remove HTML comment nodes
fn remove_html_comment_nodes(doc: &mut Document);

/// Remove empty elements from designated tag set
fn prune_html(doc: &mut Document, opts: &Options);
```

Key behaviors to preserve:
- When `ExcludeTables` is true, add table tags to cleaning list
- When `IncludeImages` is true, remove figure/picture/source from cleaning list and img from stripping list
- When `FavorRecall` and there are `<p>` elements, backup document before cleaning; restore if all `<p>` removed
- Process figure→table: if a `<figure>` contains a `<table>`, rename figure to `<div>`

#### 4.2 Prune Unwanted Nodes (port of `html-processing.go:141-188`)

```rust
/// Remove elements matching any of the given selector rules, preserving tail text
/// Port of pruneUnwantedNodes — clones tree, applies rules, optionally reverts if too much removed
pub fn prune_unwanted_nodes(
    doc: &mut Document,
    root: NodeId,
    rules: &[Rule],
    with_backup: bool,
) -> NodeId;
```

Important: This function clones the subtree, applies pruning, and if `with_backup` is true, reverts if the new text length is <= 1/7 of the original.

#### 4.3 Text Node Processing (port of `html-processing.go:191-396`)

```rust
/// Convert, format and probe potential text elements
/// Port of handleTextNode
pub fn handle_text_node(
    doc: &mut Document,
    node: NodeId,
    cache: &mut LruCache,
    fix_comments: bool,
    preserve_spaces: bool,
    opts: &Options,
) -> Option<NodeId>;

/// Convert, format, and probe potential text elements (light format)
/// Port of processNode
pub fn process_node(
    doc: &mut Document,
    node: NodeId,
    cache: &mut LruCache,
    opts: &Options,
) -> Option<NodeId>;
```

Note: `handleTextNode` and `processNode` are in `html-processing.go` in the Go source but called extensively from `main-extractor.go`. They form the bridge between HTML processing and extraction.

#### 4.4 Link Density Analysis (port of `html-processing.go:246-479`)

Critical heuristic for distinguishing content from navigation:

```rust
/// Check whether element is rich in links (probably boilerplate)
/// Port of linkDensityTest
pub fn link_density_test(doc: &Document, node: NodeId, opts: &Options) -> (Vec<NodeId>, bool);

/// Check whether a table is rich in links
/// Port of linkDensityTestTables
pub fn link_density_test_tables(doc: &Document, node: NodeId, opts: &Options) -> bool;

/// Collect heuristics on link text
/// Port of collectLinkInfo
fn collect_link_info(doc: &Document, links: &[NodeId]) -> (usize, usize, Vec<NodeId>);

/// Remove elements with high link density
/// Port of deleteByLinkDensity
pub fn delete_by_link_density(
    doc: &mut Document,
    subtree: NodeId,
    opts: &Options,
    backtracking: bool,
    tag_names: &[&str],
);
```

#### 4.5 Tag Conversion (port of `html-processing.go:481-557`)

```rust
/// Simplify HTML markup, handle links and code detection
/// Port of convertTags
pub fn convert_tags(doc: &mut Document, opts: &Options);
```

Key behaviors:
- When `!IncludeLinks`: temporarily rename important links to "protected-a", strip rest, rename back
- When `IncludeLinks`: convert relative URLs to absolute using `create_absolute_url`
- Detect code blocks: `<pre>` with single `<span>` child, or hljs-prefixed class spans → rename to `<code>`

#### 4.6 Post-Cleaning (port of `html-processing.go:399-448`)

```rust
/// Final cleanup of extraction output
/// Port of postCleaning — removes empty elements and useless attributes
pub fn post_cleaning(doc: &mut Document, body: NodeId);
```

Key behaviors:
- Remove empty non-void elements (no text, no children)
- Strip presentational attributes (id, class, align, background, bgcolor, etc.)
- Keep only allowed attributes (from ALLOWED_ATTRIBUTES set)
- Special handling: width/height only allowed on table/th/td/hr/pre elements

**Tests for Phase 4**: Port `html-processing_test.go` (59 lines). Test:
- `doc_cleaning` removes scripts/styles
- `doc_cleaning` preserves paragraphs in FavorRecall mode
- Link density removes nav blocks
- `prune_unwanted_nodes` reverts when too much text removed
- `convert_tags` handles link stripping/conversion and code detection
- `post_cleaning` removes empty elements and strips attributes

#### Acceptance Criteria
- [x] `doc_cleaning` removes all tags in TAGS_TO_CLEAN and strips all tags in TAGS_TO_STRIP
- [x] `prune_unwanted_nodes` backup/revert logic works (reverts when >6/7 text removed)
- [x] `link_density_test` correctly identifies link-heavy elements
- [x] `delete_by_link_density` removes boilerplate blocks without removing content
- [x] `convert_tags` correctly strips/preserves links based on IncludeLinks option
- [x] `post_cleaning` removes empty elements and strips presentational attributes
- [x] All tests from `html-processing_test.go` pass
- [x] `cargo test` passes, `cargo clippy` clean

---

### Phase 5: Metadata Extraction

> **Status: ✅ COMPLETED** — commit `7008304`

**Goal**: Port metadata extraction from meta tags, JSON-LD, OpenGraph, and HTML attributes.

**Dependencies**: Phase 1 (DOM), Phase 2 (regex patterns, URL utils, string utils), Phase 3 (metadata selector rules), Phase 4 (prune_unwanted_nodes — used by `extractDomAuthor`).

**NOTE**: The original plan placed metadata as Phase 6, parallel to extraction. This is wrong. Looking at `core.go`, `extractMetadata()` is called at step 4 — **before** content extraction. It is also called independently of the extraction pipeline. More importantly, `extractDomAuthor()` calls `pruneUnwantedNodes()` with `selector.MetaAuthorDiscard`, meaning metadata depends on both the selector system AND the pruning infrastructure from html_processing. Moving metadata to Phase 5 correctly sequences it before the core extraction phase.

**Outputs**: `src/metadata/mod.rs`, `src/metadata/json_ld.rs`

#### 5.1 Metadata Orchestration (port of `metadata.go`, 774 lines)

```rust
/// Extract metadata from all available sources
/// Port of extractMetadata
pub fn extract_metadata(doc: &Document, opts: &Options) -> Metadata;
```

Extraction sources (in priority order):
1. OpenGraph tags (`<meta property="og:..." content="...">`) — `extractOpenGraphMeta`
2. Meta tags (`<meta name="..." content="...">`) — `examineMeta`
3. JSON-LD (`<script type="application/ld+json">`) — `extractJsonLd`
4. DOM elements using selector rules — `extractDomTitle`, `extractDomAuthor`, `extractDomURL`
5. Title from `<title>` element — `examineTitleElement`
6. URL/hostname from document or options
7. Categories and tags from DOM — `extractDomCategories`, `extractDomTags`
8. License from links — `extractLicense`

Author post-processing (all the `rxAuthor*` patterns):
- Remove "by", "von", "posted by" prefixes
- Remove social media handles (@username)
- Remove nicknames in parentheses
- Remove email addresses
- Normalize multiple authors (split on /, ;, &)
- Apply blacklist filtering
- Capitalize names

Port these functions from metadata.go:
- `examineMeta` (72 lines)
- `extractOpenGraphMeta` (40 lines)
- `validateMetadataName` (15 lines)
- `examineTitleElement` (15 lines)
- `extractDomTitle` (42 lines)
- `extractDomAuthor` (10 lines) — NOTE: calls `pruneUnwantedNodes` with `MetaAuthorDiscard`
- `extractDomURL` (37 lines) — NOTE: calls `getBaseURL` from url.go
- `extractDomSitename` (10 lines)
- `extractDomCategories` (34 lines)
- `extractDomTags` (20 lines)
- `cleanCatTags` (10 lines)
- `extractDomMetaSelectors` (13 lines)
- `extractLicense` (17 lines)
- `parseLicenseElement` (20 lines)
- `normalizeAuthors` (62 lines)
- `removeBlacklistedAuthors` (23 lines)

#### 5.2 JSON-LD Parsing (port of `metadata-json.go`, 486 lines)

```rust
/// Extract metadata from JSON-LD script elements
/// Port of extractJsonLd
fn extract_json_ld(opts: &Options, doc: &Document, original_metadata: Metadata) -> Metadata;
```

Parse Schema.org structured data:
- Find all `<script type="application/ld+json">` and `<script type="application/settings+json">` elements
- Parse JSON, handle both single objects and arrays
- Recursively find Person, Organization, and Article-type schemas
- Sort by importance (Article > Blog > Page)
- Extract: author (Person), publisher (Organization), headline/name, articleSection, keywords
- Handle nested schemas (author→Person→name, givenName+familyName)
- Priority: article persons over standalone persons; article organizations over standalone

Port these functions:
- `decodeJsonLd` (149 lines)
- `getSchemaNames` (97 lines)
- `getSchemaTypes` (10 lines)
- `getStringValues` (22 lines)
- `getSingleStringValue` (7 lines)
- `schemaInArticle` (42 lines)

#### 5.3 Date Extraction (simplified)

For the initial port, extract dates only from metadata (JSON-LD, OpenGraph, meta tags). Skip deep document scanning (`go-htmldate` equivalent). Add a `// TODO: implement deep date scanning` comment.

```rust
/// Simple date extraction from metadata sources only
fn extract_date_from_metadata(metadata: &Metadata) -> Option<chrono::NaiveDate>;
```

**Tests for Phase 5**: Port from:
- `metadata_test.go` (611 lines, 13 functions)
- `metadata-json_test.go` (176 lines, 23 JSON-LD cases)
- `metadata-realworld_test.go` (337 lines, 37 real pages with expected metadata values)

#### Acceptance Criteria
- [x] `extract_metadata` produces correct title, author, URL, description, sitename for 5+ test HTML files
- [x] OpenGraph metadata extracted correctly
- [x] JSON-LD parsing handles Article, NewsArticle, BlogPosting, Person, Organization types
- [x] Author normalization handles all edge cases (prefixes, social handles, multiple authors)
- [x] License extraction finds Creative Commons links
- [x] All 13 metadata test functions from `metadata_test.go` pass
- [x] All 23 JSON-LD test cases from `metadata-json_test.go` pass
- [x] At least 30 of 37 real-world metadata tests pass (some may differ due to date extraction)
- [x] `cargo test` passes, `cargo clippy` clean

---

### Phase 6A: Content Extraction — Element Handlers

> **Status: ✅ COMPLETED** — commit `01ea4ce`

**Goal**: Port the element handler functions from `main-extractor.go`. These are the building blocks used by the extraction orchestration.

**Dependencies**: Phase 1 (DOM), Phase 2 (settings/utils), Phase 3 (selectors), Phase 4 (html_processing — handleTextNode, processNode, link density).

**NOTE**: The original plan had all of `main-extractor.go` (852 lines) as a single Phase 5. This is too large. It is split into 6A (element handlers) and 6B (orchestration) for manageability. The element handlers are independently testable.

**Outputs**: `src/extraction/elements.rs`

#### 6A.1 Element Handler Dispatch (port of `main-extractor.go:531-564`)

```rust
/// Dispatch to appropriate handler based on element tag
/// Port of handleTextElem
pub fn handle_text_elem(
    doc: &mut Document,
    element: NodeId,
    potential_tags: &HashSet<&str>,
    cache: &mut LruCache,
    opts: &Options,
) -> Option<NodeId>;
```

Handler dispatch table:
- `ul`, `ol`, `dl` → `handle_lists()`
- `blockquote`, `q`, `pre`, `code` → `handle_quotes()`
- `h1`-`h6`, `summary` → `handle_titles()`
- `p` → `handle_paragraphs()`
- `br`, `hr` → tail text processing
- `em`, `i`, `b`, `strong`, `u`, `kbd`, `samp`, `tt`, `var`, `sub`, `sup`, `mark`, `a`, `span` → `handle_formatting()`
- `table` → `handle_table()` (if table in potential_tags)
- `img` → `handle_image()` (if img in potential_tags)
- Everything else → `handle_other_elements()`

#### 6A.2 Individual Handlers (port of `main-extractor.go:16-530`)

Port these functions:
- `handle_titles` (~42 lines) — process headings, summary→b conversion
- `handle_formatting` (~27 lines) — repair orphan formatting elements into `<p>`
- `add_sub_element` (~7 lines)
- `process_nested_element` (~17 lines) — iterate child elements, handle lists and text
- `is_text_element` (~3 lines)
- `define_new_element` (~6 lines)
- `handle_lists` (~62 lines) — process ul/ol/dl with descendants
- `is_code_block_element` (~17 lines) — detect code by lang attr, parent highlight class, or child code element
- `handle_code_blocks` (~13 lines) — clone and rename to `<code>`
- `handle_quotes` (~19 lines) — handle blockquote/pre/q, detect code blocks
- `handle_other_elements` (~31 lines) — handle div, details, unknown tags
- `handle_paragraphs` (~106 lines) — THE MOST COMPLEX: process children, handle nested p/links/images, remove empty, clean trailing br
- `handle_table` (~82 lines) — process table rows/cells, strip thead/tbody/tfoot, nested tables
- `handle_image` (~48 lines) — extract src/data-src/alt/title, validate image URLs

**Key pattern — "done" marking**: The Go code marks processed elements by setting `element.Data = "done"`. In Rust, use `doc.set_tag_name(node, "done")`. This is used to prevent double-processing and is stripped at the end of extraction.

**Tests for Phase 6A**: Write unit tests for each handler with focused HTML fragments:
- `handle_titles`: heading with/without children
- `handle_lists`: nested lists, mixed content
- `handle_quotes`: blockquote, code block detection
- `handle_paragraphs`: paragraph with children, nested `<p>`, links, images, empty elements
- `handle_table`: simple table, nested tables, cells with mixed content
- `handle_image`: src vs data-src, missing src, protocol-relative URLs

#### Acceptance Criteria
- [x] All 14 handler functions ported and individually testable
- [x] `handle_paragraphs` correctly handles nested `<p>` (strips inner, keeps text)
- [x] `handle_table` stops at nested tables (doesn't recurse into them)
- [x] `handle_image` correctly prioritizes data-src over src when both are image URLs
- [x] "done" marking prevents double-processing
- [x] At least 15 unit tests covering all handler types
- [x] `cargo test` passes, `cargo clippy` clean

---

### Phase 6B: Content Extraction — Orchestration

> **Status: ✅ COMPLETED** — commit `01ea4ce`

**Goal**: Port the top-level extraction functions that orchestrate the element handlers.

**Dependencies**: Phase 6A (element handlers), Phase 3 (content/comment selector rules), Phase 4 (pruning, link density).

**Outputs**: `src/extraction/mod.rs`

#### 6B.1 Content Extraction (port of `main-extractor.go:664-788`)

```rust
/// Find the main content using selector rules, prune, process elements
/// Port of extractContent
pub fn extract_content(
    doc: &mut Document,
    cache: &mut LruCache,
    opts: &Options,
) -> (NodeId, String);
```

Algorithm (from Go):
1. Backup document
2. Create empty result `<body>` element
3. Build potential tags map (add table/img/a based on options)
4. For each content rule in `CONTENT_RULES`:
   a. `selector::query()` to find first matching subtree
   b. `prune_unwanted_sections()` — prune discarded content, images, teasers, link density
   c. Skip if subtree now empty
   d. Check paragraph text density; add `div` to potential tags if low
   e. Strip unwanted links/spans
   f. Process all sub-elements via `handle_text_elem()`
   g. Remove trailing titles
   h. If result has >1 children, break (success)
5. If result empty or too short, call `recover_wild_text()`
6. Strip "done" elements and div tags

#### 6B.2 Comment Extraction (port of `main-extractor.go:808-852`)

```rust
/// Extract comments from document
/// Port of extractComments
pub fn extract_comments(
    doc: &mut Document,
    cache: &mut LruCache,
    opts: &Options,
) -> (Option<NodeId>, String);
```

#### 6B.3 Wild Text Recovery (port of `main-extractor.go:569-608`)

```rust
/// Search entire document for orphaned text when extraction yields too little
/// Port of recoverWildText
fn recover_wild_text(
    doc: &Document,
    result_body: NodeId,
    potential_tags: &HashSet<&str>,
    cache: &mut LruCache,
    opts: &Options,
);
```

#### 6B.4 Section Pruning (port of `main-extractor.go:611-662`)

```rust
/// Rule-based deletion of targeted document sections
/// Port of pruneUnwantedSections
fn prune_unwanted_sections(
    doc: &mut Document,
    subtree: NodeId,
    potential_tags: &HashSet<&str>,
    opts: &Options,
) -> NodeId;
```

**Tests for Phase 6B**: Port from `trafilatura_test.go` (1,518 lines):
- Exotic tags test
- Formatting preservation
- Filter/discard tests
- Image extraction
- Link extraction
- Table processing (complex nested tables)
- List processing (nested lists)
- Code block preservation
- Precision vs recall mode behavior

#### Acceptance Criteria
- [x] `extract_content` finds content for all mock HTML files that Go finds content for
- [x] `extract_comments` extracts comments when present
- [x] `recover_wild_text` recovers text from documents with no standard content structure
- [x] `prune_unwanted_sections` correctly applies discard rules and link density filtering
- [x] At least 50 test cases from `trafilatura_test.go` pass
- [x] Precision/recall mode options change extraction behavior as expected
- [x] `cargo test` passes, `cargo clippy` clean

#### Implementation Notes (Phase 6)

Key design decisions made during implementation:

- **Element handlers return `Option<String>`** (HTML fragment strings), not `Option<NodeId>`. This avoids ego-tree's restriction on using NodeIds across different `Document` instances (ego-tree arenas can't share node handles). Returning HTML string fragments sidesteps all cross-document ownership issues.
- **NodeId preservation across `clone_document()`**: `clone_document()` does `self.tree.clone()` which copies the arena `Vec` with all indices intact. A `NodeId` obtained from the original doc is valid in the cloned doc at the same index. This is relied upon in `prune_unwanted_sections`.
- **`prune_unwanted_sections` signature**: Takes `&Document`, clones internally (via `prune_unwanted_nodes` chain), returns new `Document`. The caller's original `doc` is never mutated.
- **`handle_code_blocks` structure preservation**: The handler takes `&mut Document`, strips all attributes from descendants (Go's etree strips attrs), then calls `inner_html` to get the cleaned inner HTML, then marks descendants as "done". Preserves nested structure instead of flattening to text.
- **`handle_titles` child restoration**: Before calling `handle_text_node` on a child, snapshot `text()` and `tail()`. If `handle_text_node` returns `None`, restore from snapshots and mark the child "done" anyway — Go always appends something here.
- **HTML attribute escaping**: `escape_attr()` helper escapes `&`, `"`, `<`, `>` in any string interpolated into a raw HTML attribute. Used in `handle_image` for `src` and `alt` attributes.
- **Paragraph density heuristic**: Must count `<p>` elements from the ORIGINAL unmodified document (not the pruned work copy). Go's `extractContent` counts from `doc` before any pruning.
- **`recover_wild_text` tag lists**: Uses `XML_LB_TAGS` and `XML_LIST_TAGS` from `settings.rs` (not hardcoded slices), matching Go's `xmlLinebreakTags` / `xmlListTags`.
- **Content selector iteration**: `selector::content::CONTENT` is `&[Rule]`. To try each rule independently: `std::slice::from_ref(&rule)` converts a single Rule reference to a `&[Rule]`.
- **Result assembly**: Collect `Vec<(String, String)>` (html_fragment, tag_name), pop trailing head/ref tags, join and parse into result Document, strip "done" elements and "div" wrappers.

---

### Phase 7: Baseline & External Fallback

> **Status: ✅ COMPLETED** — commit `de746c5`

**Goal**: Port the fallback extraction strategies for when the main algorithm doesn't find enough content.

**Dependencies**: Phase 4 (html_processing — `docCleaning` used by `sanitizeTree`), Phase 6B (extraction must exist to be compared against).

**Outputs**: `src/extraction/baseline.rs`, `src/extraction/external.rs`

#### 7.1 Baseline Extraction (port of `baseline.go`, 152 lines)

Last-resort strategies, tried in order:
1. JSON-LD `articleBody` field → parse as HTML, extract text
2. `<article>` tag content
3. All `<p>`, `<pre>`, `<blockquote>`, `<q>`, `<code>` elements (deduplicated)
4. Entire `<body>` text content
5. Entire document text content (final fallback)

```rust
/// Last-resort extraction using simple heuristics
/// Port of baseline
pub fn baseline(doc: &mut Document) -> (NodeId, String);

/// Remove footer/aside/script/style elements
/// Port of basicCleaning
fn basic_cleaning(doc: &mut Document) -> NodeId;
```

#### 7.2 External Fallback (port of `external.go`, 242 lines)

```rust
/// Compare our extraction with readability, use whichever is better
/// Port of compareExternalExtraction
pub fn compare_external_extraction(
    original_doc: &Document,
    extracted: &mut Document,
    extracted_body: NodeId,
    opts: &Options,
) -> (NodeId, String);

/// Check if the fallback candidate is good enough to use
/// Port of candidateIsUsable
fn candidate_is_usable(
    candidate_doc: &Document,
    extracted_doc: &Document,
    len_candidate: usize,
    len_extracted: usize,
    opts: &Options,
) -> bool;

/// Clean and validate external extractor output
/// Port of sanitizeTree
fn sanitize_tree(doc: &mut Document, tree: NodeId, opts: &Options);
```

Comparison heuristics (`candidate_is_usable`):
- If candidate is empty or same length → not usable
- If our extraction is empty but candidate has content → usable
- If our extraction > 2x candidate → not usable
- If candidate > 2x our extraction → usable
- Borderline: check paragraph text length, table-to-paragraph ratio, heading presence

**Readability crate evaluation**: Before implementing, evaluate the `readability` Rust crate against `go-readability` on 5-10 test documents. If results diverge significantly (>10% difference in comparison metrics), consider either:
1. Accepting the difference and documenting it
2. Using a different readability implementation
3. Deferring the external fallback entirely (it only matters when main extraction fails)

Post-fallback: `sanitize_tree()` cleans the readability output:
- Call `docCleaning` on the tree
- Remove aside/audio/button/fieldset/figure/footer/iframe/etc. (`tagsToSanitize` list)
- Strip links if `!IncludeLinks`, always strip spans
- Strip any tags not in `validTagCatalog`

**Tests for Phase 7**: Port `baseline_test.go` (165 lines). Test:
- Baseline finds content from JSON-LD articleBody
- Baseline falls through to article tag, then paragraphs, then body
- Fallback comparison selects better candidate
- `sanitize_tree` removes unsafe elements from readability output

#### Acceptance Criteria
- [x] `baseline` extracts text from JSON-LD articleBody when present
- [x] `baseline` falls through tiers correctly (JSON-LD → article → p → body)
- [x] `candidate_is_usable` matches Go's heuristic decisions on test cases
- [x] `sanitize_tree` removes all tags from `tagsToSanitize`
- [x] All tests from `baseline_test.go` pass
- [x] `cargo test` passes, `cargo clippy` clean

---

### Phase 8: Core API & Integration

> **Status: ✅ COMPLETED** — commit cb0290a

**Goal**: Wire everything together into the public API. Implement `ExtractDocument` pipeline.

**Dependencies**: ALL previous phases (1-7). This is the integration point.

**Outputs**: `src/lib.rs` with `extract()` and `extract_from_reader()`

#### 8.1 Public API

```rust
// src/lib.rs

/// Extract readable content from an HTML string
pub fn extract(html: &str, opts: Options) -> Result<ExtractResult, TrafilaturaError> {
    let mut doc = Document::parse(html);
    extract_document(&mut doc, opts)
}

/// Extract readable content from a reader
pub fn extract_from_reader(
    reader: impl std::io::Read,
    opts: Options,
) -> Result<ExtractResult, TrafilaturaError> {
    let mut html = String::new();
    reader.read_to_string(&mut html)?;
    extract(&html, opts)
}
```

#### 8.2 Pipeline Implementation (port of `core.go:82-219`)

```rust
fn extract_document(
    doc: &mut Document,
    opts: Options,
) -> Result<ExtractResult, TrafilaturaError> {
    let config = opts.config.clone();
    let mut cache = LruCache::new(config.cache_size);

    // 1. Language check
    if let Some(ref lang) = opts.target_language {
        if !check_html_language(doc, &opts, false) {
            return Err(TrafilaturaError::LanguageMismatch { ... });
        }
    }

    // 2. Extract metadata (BEFORE content extraction)
    let mut metadata = extract_metadata(doc, &opts);

    // 3. Essential metadata check
    if opts.has_essential_metadata {
        if metadata.title.is_empty() { return Err(MissingMetadata("title")); }
        if metadata.url.is_empty() { return Err(MissingMetadata("url")); }
        if metadata.date.is_none() { return Err(MissingMetadata("date")); }
    }

    // 4. Update URL from metadata if not provided
    if opts.original_url.is_none() && !metadata.url.is_empty() {
        if let Ok(parsed) = url::Url::parse(&metadata.url) {
            opts.original_url = Some(parsed);
        }
    }

    // 5. Apply prune selector
    if let Some(ref sel) = opts.prune_selector { ... }

    // 6. Clone for fallback/baseline (3 clones as in Go)
    let doc_clone = doc.clone_document();
    let doc_backup1 = doc.clone_document();
    let doc_backup2 = doc.clone_document();
    *doc = doc_clone;

    // 7. Clean + convert
    doc_cleaning(doc, &opts);
    convert_tags(doc, &opts);

    // 8. Extract comments (before content, because comments are removed from doc)
    let (comments_body, comments_text) = if !opts.exclude_comments {
        extract_comments(doc, &mut cache, &opts)
    } else {
        if opts.focus == FavorPrecision {
            prune_unwanted_nodes(doc, REMOVED_COMMENTS);
        }
        (None, String::new())
    };

    // 9. Extract content
    let (mut post_body, mut body_text) = extract_content(doc, &mut cache, &opts);

    // 10. Fallback
    if opts.enable_fallback {
        let (fb, ft) = compare_external_extraction(&doc_backup1, ...);
        post_body = fb; body_text = ft;
    }

    // 11. Baseline rescue
    if body_text.chars().count() < config.min_extracted_size
        && opts.focus != FavorPrecision
    {
        let (bl, bt) = baseline(&doc_backup2);
        post_body = bl; body_text = bt;
    }

    // 12. Tree size sanity check
    // 13. Size checks + deduplication
    // 14. Language classification
    // 15. Post-cleaning
    // 16. Return ExtractResult
}
```

#### 8.3 Helper Function (port of `helper.go`)

```rust
/// Convert ExtractResult to a complete HTML document with metadata in <head>
/// Port of CreateReadableDocument
pub fn create_readable_document(result: &ExtractResult) -> Document;
```

**Tests for Phase 8**: Full integration tests. Feed complete HTML documents through `extract()` and validate:
- Content text matches expected strings (from mock files)
- Comments extracted when enabled, excluded when disabled
- Metadata populated correctly
- Error cases: missing metadata, wrong language, too short → appropriate errors
- Fallback triggers when content too short
- Baseline triggers as last resort
- All three `ExtractionFocus` modes produce different results on the same input
- Port representative tests from `trafilatura_test.go` that test the full pipeline

#### Acceptance Criteria
- [x] `extract()` produces correct output for at least 10 mock HTML files
- [x] All error types returned appropriately (LanguageMismatch, InsufficientContent, etc.)
- [x] Pipeline follows same order as Go: metadata → clone → clean → comments → content → fallback → baseline → checks
- [x] ExcludeComments option works
- [x] ExcludeTables option works
- [x] IncludeImages and IncludeLinks options work
- [x] Deduplicate option works
- [x] `cargo test` passes, `cargo clippy` clean

---

### Phase 9: Real-World Tests & Comparison Suite

> **Status: 🔲 NEXT**

**Goal**: Comprehensive test coverage proving we match Go's extraction quality.

**Dependencies**: Phase 8 (full API must work).

**Outputs**: `tests/` directory completed, `comparison-data/entries.json`, comparison test

#### 9.1 Test Helpers (port of `helper_test.go` + `realworld-mock_test.go`)

```rust
// tests/common/mod.rs

/// URL-to-filename mapping for mock files (port of realworld-mock_test.go, 89 URLs)
pub fn mock_file_map() -> HashMap<&'static str, &'static str>;

/// Load HTML content from mock file by URL
pub fn load_mock_file(url: &str) -> String;

/// Parse and extract from mock file
pub fn extract_mock_file(url: &str, opts: Options) -> ExtractResult;

/// Create Document from raw HTML string
pub fn doc_from_str(html: &str) -> Document;
```

#### 9.2 Real-World Tests (port of `realworld_test.go`, 642 lines)

95+ real-world pages with positive/negative assertions. Each test loads a mock HTML file, extracts content, and checks for expected strings.

#### 9.3 Comparison Data Migration

The Go `scripts/comparison/data.go` contains 926 `ComparisonEntry` structs in 8,532 lines. **Do not manually port this to Rust source code.**

**Strategy: Code-generate JSON, load at test time.**

1. Write a Go script (`scripts/convert_comparison_data.py` or Go main) that imports `data.go` and outputs `comparison-data/entries.json`
2. In Rust tests, load `entries.json` at test time via `serde_json`
3. This avoids 8,532 lines of Rust static data and makes updates easy

```json
// comparison-data/entries.json (generated)
[
  {
    "file": "some-page.html",
    "with": ["expected text 1", "expected text 2"],
    "without": ["unwanted text 1"],
    "title": "Page Title",
    "authors": ["Author Name"],
    "date": "2023-01-15",
    "sitename": "Example.com",
    "categories": ["Tech"],
    "tags": ["rust", "programming"],
    "comments": ["A comment"]
  },
  ...
]
```

Alternative if Go codegen is not practical: Write a Python script that parses `data.go` with regex to extract the struct literals into JSON. The format is regular enough for this.

#### 9.4 Comparison Framework

```rust
// tests/comparison_test.rs

#[derive(Deserialize)]
struct ComparisonEntry {
    file: String,
    with: Vec<String>,
    without: Vec<String>,
    title: Option<String>,
    authors: Vec<String>,
    // ... etc
}

fn evaluate_entry(entry: &ComparisonEntry, result_text: &str) -> (usize, usize, usize, usize) {
    // TP: "with" string present in result
    // FN: "with" string missing from result
    // FP: "without" string present in result
    // TN: "without" string absent from result
}

#[test]
fn comparison_balanced() { ... }

#[test]
fn comparison_precision() { ... }

#[test]
fn comparison_recall() { ... }
```

**Success criteria**: Precision, recall, accuracy, and F-score all within 2% of Go version's numbers.

#### Acceptance Criteria
- [ ] At least 90 of 95+ real-world tests pass
- [ ] Comparison data loaded from JSON (no 8K+ lines of Rust static data)
- [ ] Comparison framework computes precision/recall/accuracy/F-score
- [ ] All four metrics within 2% of Go baseline (balanced mode)
- [ ] Precision mode has higher precision than balanced mode
- [ ] Recall mode has higher recall than balanced mode
- [ ] `cargo test` passes, `cargo clippy` clean

---

### Phase 10: CLI Binary

**Goal**: Build a CLI for standalone usage and testing.

**Dependencies**: Phase 8 (core API).

**Outputs**: `src/bin/trafilatura.rs`

#### 10.1 CLI Structure (port of `cmd/go-trafilatura/main.go`)

```rust
use clap::{Parser, ValueEnum};

#[derive(Parser)]
#[command(name = "trafilatura", about = "Extract readable content from web pages")]
struct Cli {
    /// HTML file path or URL to extract from
    source: Option<String>,

    /// Output format
    #[arg(short, long, default_value = "html")]
    format: OutputFormat,

    /// Target language (ISO 639-1 code)
    #[arg(short, long)]
    language: Option<String>,

    /// Disable fallback extraction
    #[arg(long)]
    no_fallback: bool,

    /// Exclude comments
    #[arg(long)]
    no_comments: bool,

    /// Skip tables
    #[arg(long)]
    no_tables: bool,

    /// Include images
    #[arg(long)]
    images: bool,

    /// Keep hyperlinks
    #[arg(long)]
    links: bool,

    /// Extraction focus mode
    #[arg(long)]
    precision: bool,

    #[arg(long)]
    recall: bool,

    /// Remove duplicate content
    #[arg(long)]
    deduplicate: bool,

    /// Require title/URL/date metadata
    #[arg(long)]
    has_metadata: bool,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[derive(ValueEnum, Clone)]
enum OutputFormat {
    Html,
    Txt,
    Json,
}
```

Defer batch, feed, and sitemap processing. These require HTTP fetching and are not part of the core extraction library.

#### Acceptance Criteria
- [ ] `trafilatura test-files/mock/some-file.html` produces content on stdout
- [ ] `--format txt` outputs plain text
- [ ] `--format json` outputs structured JSON with metadata
- [ ] `--no-comments`, `--no-tables`, `--images`, `--links` flags work
- [ ] `--precision` and `--recall` flags change extraction behavior
- [ ] Reads from stdin when no source specified

---

### Phase 11: Benchmarks & Performance

**Goal**: Establish performance baselines and compare against Go.

**Dependencies**: Phase 9 (comparison framework with all test data).

**Outputs**: `benches/extraction.rs`

#### 11.1 Criterion Benchmarks

```rust
use criterion::{criterion_group, criterion_main, Criterion};

fn bench_single_document(c: &mut Criterion) {
    let html = std::fs::read_to_string("test-files/mock/blog.python.org.html").unwrap();
    c.bench_function("extract_single_doc", |b| {
        b.iter(|| trafilatura::extract(&html, Options::default()))
    });
}

fn bench_comparison_corpus(c: &mut Criterion) {
    // Load all 926 comparison documents, benchmark extracting all of them
}

fn bench_metadata_only(c: &mut Criterion) {
    // Benchmark just metadata extraction
}

fn bench_dom_operations(c: &mut Criterion) {
    // Benchmark text/tail, clone, iter_text — DOM operations that may be bottlenecks
}
```

**Go reference performance**: ~4.25s for 960 documents single-threaded (no fallback), ~8.39s with fallback.

#### Acceptance Criteria
- [ ] Benchmark suite runs without errors
- [ ] Single document extraction < 5ms (median)
- [ ] Full corpus extraction within 2x of Go performance (target: match or beat)
- [ ] No obvious performance regressions from DOM clone operations

---

## Implementation Order & Dependencies

```
Phase 1 ──► Phase 2 ──► Phase 3 ──► Phase 4 ──► Phase 5 ──► Phase 6A ──► Phase 6B ──► Phase 7 ──► Phase 8
  DOM         Config      Selectors   HTML Proc   Metadata    Handlers     Orchestr     Fallback     Core API
  +Utils      +Regex                  +LinkDens                                                       │
              +URL                    +TextProc                                                       │
                                                                                                      │
Phase 9 ◄─────────────────────────────────────────────────────────────────────────────────────────────┘
Tests & Comparison
     │
     ├──► Phase 10 (CLI)
     └──► Phase 11 (Benchmarks)
```

**Strict sequential chain**: 1 → 2 → 3 → 4 → 5 → 6A → 6B → 7 → 8 → 9

The phases are strictly sequential because each builds on the prior. The original plan claimed metadata and fallback could be parallel with extraction — this is technically true for the code modules, but testing either in isolation is impractical without the full pipeline.

Phase 10 (CLI) and Phase 11 (benchmarks) are independent after Phase 9.

**Incremental testing**: Although the test phase is listed as Phase 9, unit tests should be written alongside each phase. Phase 9 adds the real-world and comparison tests that require the full pipeline.

---

## Risk Areas & Mitigations

### 1. DOM Abstraction (Phase 1) — HIGH RISK
**Risk**: The text/tail concept doesn't exist natively in Rust HTML libraries. `scraper` crate's `ego-tree` may not expose enough mutation capabilities through the public API.
**Mitigation**: Prototype the DOM layer first with focused spike tests. The `ego-tree::NodeMut` API supports `append`, `insert_before`, `detach`, which covers our needs. If `scraper::Html`'s wrapper is too restrictive, access `ego-tree` directly via `Html::tree` (which returns `&ego_tree::Tree<Node>`). Worst case: fork `scraper` to expose tree mutation, or use `html5ever` directly with a custom tree sink.
**Spike test**: Before committing to the approach, implement `text()`, `set_text()`, `tail()`, `set_tail()`, `remove()`, and `strip()` on a real HTML document and verify they match Go output.

### 2. Tree Cloning Performance (Phase 8)
**Risk**: The pipeline clones the document 3 times (`dom.Clone(doc, true)` in Go). Deep cloning a large DOM tree is expensive.
**Mitigation**: Profile early. The Go version does this too and achieves acceptable performance (~4.5ms per document). If Rust cloning is slower, consider:
- Lazy cloning (only clone when mutation needed)
- Persistent data structure for the tree
- Reducing clone count (baseline backup can be deferred)

### 3. Comparison Dataset Port (Phase 9)
**Risk**: 8,532 lines of comparison data in Go → Rust. Manual conversion is error-prone and creates a maintenance burden.
**Mitigation**: Code-generate JSON from Go source. Load at test time. Write a conversion script and version-control the JSON output alongside the script.

### 4. Readability Crate Quality (Phase 7)
**Risk**: The `readability` Rust crate may not match `go-readability`'s extraction quality, causing comparison metric differences.
**Mitigation**: Test readability independently on 10 documents before integrating. If results diverge >10%, consider:
- Accepting the difference (fallback is only used when main extraction fails)
- Making the readability crate pluggable (trait-based)
- Deferring fallback entirely for v0.1

### 5. Regex Pattern Compatibility
**Risk**: Some Go regex patterns use features that Rust's `regex` crate handles differently (e.g., Unicode property classes `\pL`, `\pM`, `\pN`).
**Mitigation**: Rust's `regex` crate supports Unicode properties when the `unicode` feature is enabled (it is by default). Test each pattern individually during Phase 2. The `re2go` state machine is just a regex — port the source pattern, not the generated code.

### 6. Date Extraction Gap
**Risk**: Skipping deep date extraction (go-htmldate equivalent) means some pages will have missing dates. This affects comparison metrics.
**Mitigation**: Document this as a known limitation in v0.1. Date extraction from metadata (JSON-LD, OpenGraph, meta tags) covers the majority of cases. Deep date scanning can be added as a v0.2 enhancement or by creating a Rust port of htmldate.

### 7. handleParagraphs Complexity
**Risk**: `handleParagraphs` is the most complex handler (106 lines in Go) with many branches for nested paragraphs, links, images, empty element removal, and line break cleanup. It is easy to introduce subtle bugs.
**Mitigation**: Port this function with extensive test coverage. Write at least 10 targeted tests for handleParagraphs covering: empty paragraph, paragraph with text only, paragraph with children, nested `<p>`, paragraph with links (href/target preservation), paragraph with images, empty child removal, trailing br removal.

---

## Verification Strategy

1. **Per-phase unit tests**: Every module gets tests as it's built
2. **Integration tests**: Full pipeline tests after Phase 8
3. **Real-world validation**: 95+ real-world pages tested (from `realworld_test.go`)
4. **Comparison metrics**: Precision/recall/accuracy/F-score on 926-document corpus
5. **Cross-reference**: Run Go comparison tool side-by-side, compare numbers
6. **Performance**: Criterion benchmarks compared against Go baseline
7. **CI**: `cargo test`, `cargo clippy`, `cargo fmt --check`

**Success criteria**:
- All ported Go tests pass
- Comparison metrics within 2% of Go version on precision/recall/F-score
- No `cargo clippy` warnings
- Performance matches or beats Go
