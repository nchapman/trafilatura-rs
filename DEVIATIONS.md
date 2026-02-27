# Deviations from Go Source

This document tracks intentional differences between trafilatura-rs and the
[Go implementation](https://github.com/markusmobius/go-trafilatura) it was
ported from.

## Architecture

### Element handlers return HTML strings, not NodeIds

Go element handlers return a cloned `*etree.Element` that gets spliced into
the result tree. Rust handlers return `Option<String>` (serialized HTML
fragments) because ego-tree's `NodeId` is not transferable across different
`Document` instances.

**Files**: `src/extraction/elements.rs`

### In-place mutation with text/tail snapshots

Where Go clones elements before mutation, Rust works in-place and snapshots
text/tail content beforehand. If processing fails, the snapshot is restored.
This avoids unnecessary cloning while preserving identical behavior.

**Files**: `src/extraction/elements.rs`

### DOM text/tail model

The DOM layer uses Python's ElementTree text/tail concept (inherited from the
Go port of `internal/etree/`). This is architecturally faithful to Go but
requires explicit handling in a few places where Go's separate text nodes
behave differently (e.g., leading spaces between elements).

**Files**: `src/dom/tree.rs`

## Readability Fallback

### Different readability crate

trafilatura-rs uses the `readability` Rust crate (readeck port) instead of
go-readability (go-shiori). These produce different extraction results:

- Comment sections may be included where Go excludes them
- Navigation content may be extracted instead of article body
- Ad links may not be filtered as aggressively

This is the largest behavioral difference and accounts for the two ignored
integration tests:

- `test_extract_love_hina`: readability-rs includes comment sections
- `test_extract_rnz_witzel`: readability-rs extracts nav instead of article

### No dom-distiller fallback

Go's fallback pipeline includes dom-distiller as a third option. Rust omits
it — no suitable Rust equivalent is available. Fallback uses user-provided
candidates and readability only.

**Files**: `src/extraction/external.rs`

## Language Detection

### Fewer languages supported

The `whatlang` Rust crate covers ~69 languages; Go's `whatlanggo` covers ~87.
Language detection may not work for ~18 languages supported in Go. An
ISO 639-3 → 639-1 mapping table bridges available languages.

**Files**: `src/utils/language.rs`

## Date Extraction

### `HtmlDateMode::Extensive` not yet implemented

The variant exists (matching Go's API) but behaves identically to `Fast`.
Full body-text date scanning will be added when the `htmldate` integration
is ported.

**Files**: `src/options.rs`

## Output Format

### Apostrophe escaping

Go serializes apostrophes as `&#39;`; html5ever leaves them unescaped. Both
are valid HTML — the difference is cosmetic.

## Test Adjustments

### `test_extract_schleifen_ucoz`

Removed the `"Aufrufe:"` assertion. Go extracts this page-view counter via
go-readability, but the Python original does not. Rust aligns with Python
behavior.
