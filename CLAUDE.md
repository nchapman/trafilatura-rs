# trafilatura-rs

Rust port of [trafilatura](https://github.com/adbar/trafilatura), a web content extraction library. Ported from the [Go implementation](https://github.com/markusmobius/go-trafilatura), not the Python original.

## Source References

- **Go source** (port from this): `/Users/nchapman/Code/go-trafilatura`
- **Python source** (behavioral reference): `/Users/nchapman/Code/trafilatura`


When porting a function, always read the Go source first. When behavior is unclear, check the Python original.

## Porting Philosophy

This is a **faithful, careful port**. The goal is correctness first, then idiom, then performance.

### Mirror Go structure for maintainability

The Go codebase will continue to evolve. Our Rust module layout, function names, and file organization should make it easy to find the corresponding Go code and port future changes. Specifically:

- **One Rust file per Go file** where practical. The Go→Rust file mapping is:
  - `core.go` → `src/lib.rs`
  - `core-options.go` → `src/options.rs`
  - `main-extractor.go` → `src/extraction/mod.rs` + `src/extraction/elements.rs`
  - `html-processing.go` → `src/extraction/html_processing.rs`
  - `baseline.go` → `src/extraction/baseline.rs`
  - `external.go` → `src/extraction/external.rs`
  - `metadata.go` → `src/metadata/mod.rs`
  - `metadata-json.go` → `src/metadata/json_ld.rs`
  - `settings.go` + `tag-converter.go` → `src/settings.rs`
  - `helper.go` → `src/lib.rs` (CreateReadableDocument)
  - `utils-common.go` → `src/utils/mod.rs`
  - `utils-extractor.go` → `src/utils/text.rs` + `src/utils/language.rs`
  - `url.go` → `src/utils/url.rs`
  - `internal/etree/` → `src/dom/`
  - `internal/selector/` → `src/selector/`
  - `internal/lru/` → `src/utils/lru.rs`
- **Keep Go function names** as Rust equivalents. `extractContent` → `extract_content`, `handleParagraphs` → `handle_paragraphs`, `pruneUnwantedNodes` → `prune_unwanted_nodes`. When someone reads a Go function name they should be able to find the Rust version instantly.
- **Port functions in the same order they appear in the Go file**. This makes diff-based comparison possible.
- **Add a comment at the top of each Rust file** noting which Go file it ports: `// Port of go-trafilatura/metadata.go`
- **When porting a function**, add a brief doc comment that includes the Go function name: `/// Port of extractDomAuthor`

### Write idiomatic Rust

While mirroring Go's structure, the code itself should be idiomatic Rust:

- Use `Result<T, E>` and the `?` operator instead of Go's `(result, error)` pattern
- Use `Option<T>` instead of nil checks
- Use iterators and combinators where they're clearer than loops
- Use `enum` for tagged unions instead of Go's interface{}/type assertion
- Use `&str` for borrowed strings, `String` for owned
- Use `impl Into<X>` or generics for flexible function parameters
- Prefer returning owned values from functions that create data
- Use `HashSet<&'static str>` or `phf::Set` for tag catalogs, not `map[string]struct{}`
- Use `std::sync::LazyLock` for compiled regex patterns and static sets (stable since Rust 1.80, do not use `once_cell`)
- Derive `Debug`, `Clone`, `Default` on public types where appropriate
- Use `thiserror` for error types
- No `unwrap()` in library code (only in tests and static regex compilation inside `LazyLock`)

### What NOT to do

- Do not refactor Go logic while porting. If the Go code has a weird branch or redundant check, port it faithfully. We can refactor later once tests prove equivalence.
- Do not add features that the Go version doesn't have.
- Do not skip a function because it "seems unnecessary." Port it, test it.
- Do not invent new abstractions that don't exist in the Go code. The DOM wrapper is the one exception (necessary due to Rust's ownership model).
- Do not use `async` in the library. The core extraction is synchronous. Async is only for the CLI's HTTP fetching.

## Workflow

### Cycle for each section of work

1. **Read the Go source** for the module you're porting
2. **Implement** the Rust equivalent
3. **Write tests** — port the corresponding Go tests, add Rust-specific edge cases
4. **Run `cargo test` and `cargo clippy`** — fix all warnings
5. **Request a code review** (use the `code-reviewer` agent)
6. **Fix review findings**, re-run tests
7. **Commit** with a clean, descriptive message

### Commit discipline

- Commit after completing each coherent piece of work (a module, a group of related functions, a test file)
- Do **not** reference plan phases or milestones in commit messages
- Write clear, specific descriptions of what changed:
  - Good: `Add DOM text/tail operations with unit tests`
  - Good: `Port metadata extraction from meta tags and OpenGraph`
  - Good: `Port content selector rules 1-5 from Go`
  - Bad: `Phase 1 complete`
  - Bad: `WIP`
  - Bad: `Add stuff`
- Use imperative mood: "Add", "Port", "Fix", "Implement"
- Include a brief bullet list of changes when the commit touches multiple concerns

### Testing standards

- **Port Go tests first**. Every Go test file has a corresponding Rust test file. Port assertions faithfully.
- **Use the same test fixtures**. HTML files are in `test-files/` (copied from Go repo). Use the same URL→filename mappings from `realworld-mock_test.go`.
- **Test each function in isolation** before integration. Element handlers should have standalone tests with small HTML fragments.
- **Use `pretty_assertions`** for string comparison in tests — the diff output is essential for debugging extraction differences.
- **When a test fails**, compare output against the Go version before assuming the Rust code is wrong. Read the Go source to understand the expected behavior.
- Run `cargo test` after every change. Do not batch up untested work.

## Technical Reference

### DOM abstraction (`src/dom/`)

The DOM layer is the critical foundation. It wraps `scraper::Html` (which uses `ego-tree`) and provides:

- **Text/Tail concept** (from Python's ElementTree, ported to Go in `internal/etree/`):
  - `text(node)` = text content before the first child element
  - `tail(node)` = text after the element's end tag, before the next sibling
  - When removing elements, tail text must be handled (preserved or removed based on `keep_tail`)
  - When appending elements, tail text nodes must move with the element
- **Tree mutation** via `ego_tree::NodeMut`: append, detach, insert_before
- **NodeId handles** to avoid borrow checker issues — pass IDs, not references

### Key crate choices

| Crate | Purpose | Why |
|-------|---------|-----|
| `scraper` | HTML parsing + CSS selectors | Built on html5ever, jQuery-like API |
| `ego-tree` | Tree mutation | Exposed by scraper, supports append/detach/insert |
| `regex` | Pattern matching | Fast, RE2-compatible (same engine family as Go) |
| `whatlang` | Language detection | Direct Rust port of Go's whatlanggo |
| `chrono` | Date handling | Standard Rust date library |
| `serde` + `serde_json` | JSON-LD parsing, JSON output | Standard serialization |
| `url` | URL parsing/validation | Servo's URL parser |
| `clap` | CLI (binary only) | Derive-based argument parsing |
| `tracing` | Logging | Structured, configurable |
| `thiserror` | Error types | Derive Error implementations |
| `pretty_assertions` | Test diffs (dev only) | Readable assertion failures |

### re2go patterns

The Go codebase has 1,826 lines of re2c-generated state machine code. **Do not port the generated code.** Port the source pattern (one line) and compile it as a `LazyLock<Regex>`.

### Comparison data

The 926-entry comparison dataset (`scripts/comparison/data.go`, 8,532 lines) should be converted to JSON and loaded at test time via serde. Do not create 8K+ lines of Rust static data.

## Commands

```bash
cargo test                    # Run all tests
cargo clippy                  # Lint
cargo fmt --check             # Format check
cargo bench                   # Run benchmarks (after Phase 11)
cargo run -- <file-or-url>    # Run CLI (after Phase 10)
```

## File layout

```
src/
├── lib.rs                    # Public API: extract(), extract_from_reader()
├── error.rs                  # TrafilaturaError enum
├── options.rs                # Options, Config, ExtractionFocus
├── result.rs                 # ExtractResult, Metadata
├── settings.rs               # Tag catalogs + tag category lists
├── dom/                      # DOM abstraction (port of internal/etree/)
│   ├── mod.rs                # Document struct, NodeId
│   ├── tree.rs               # Text/tail, clone, remove, append, strip
│   └── query.rs              # CSS selectors, get_elements_by_tag_name
├── selector/                 # Extraction rules (port of internal/selector/)
│   ├── mod.rs                # Rule type, query/query_all, rule list exports
│   ├── content.rs            # 5 content rules
│   ├── comments.rs           # 4 comment rules
│   ├── discard.rs            # All discard rules
│   ├── metadata.rs           # Author, title, categories, tags selectors
│   └── utils.rs              # contains, starts_with, lower helpers
├── extraction/               # Core pipeline (port of main-extractor.go + html-processing.go)
│   ├── mod.rs                # extract_content, extract_comments orchestration
│   ├── elements.rs           # handle_text_elem dispatch + all handlers
│   ├── html_processing.rs    # doc_cleaning, convert_tags, post_cleaning, link density
│   ├── baseline.rs           # Last-resort extraction
│   └── external.rs           # Readability fallback comparison
├── metadata/                 # Metadata extraction (port of metadata.go + metadata-json.go)
│   ├── mod.rs                # extract_metadata orchestration
│   └── json_ld.rs            # JSON-LD schema.org parsing
└── utils/                    # Shared utilities
    ├── mod.rs                # trim, string helpers
    ├── lru.rs                # LRU/FIFO cache for dedup
    ├── text.rs               # text_filter, duplicate_test
    ├── url.rs                # URL utilities
    ├── regex_patterns.rs     # Compiled regex patterns
    └── language.rs           # Language detection + HTML lang check
```
