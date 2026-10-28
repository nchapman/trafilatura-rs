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

Because those fragments are re-parsed by `extract_content`/`extract_comments`,
any **raw text** interpolated into a fragment string (from `text()`, `tail()`,
`iter_text()`, or `text_content()`) must go through `escape_text` first;
markup obtained from `inner_html()` is already escaped and must not be
re-escaped. Without this, decoded entities in text nodes (e.g.
`Guido &lt;guido at python.org&gt;`) re-enter the tree as real elements on
re-parse (issue #13).

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

## Baseline Extraction

### Realigned to go-trafilatura v2.2.x / Python trafilatura 2.3.1

`src/extraction/baseline.rs` was originally ported from go-trafilatura as of
upstream commit `6b0b475` (Oct 2024). Upstream rewrote `baseline.go` in
September 2026 (commits `2114bfe`/`5ecedf5`) to align with Python trafilatura
2.3.1, fixing two bugs the original port had inherited verbatim (strategies
accumulating into one shared `<body>`, and a whole-document text fallback
that leaked `<head><title>` into results). We ported that rewrite (issue
#13): fresh body per strategy, `<body>`-only final fallback with no length
gate, teaser tier, expanded JSON-LD walking, substring dedupe with caps,
Discourse preload support, and cookie-consent/basic-cleaning updates.

Parts of the same upstream update are **not** ported because nothing else in
this crate uses them yet:

- `html2txt` (block-aware text dump) — used upstream only by the new core
  rescue tier below and tests.
- the core rescue retry that compares extracted length against 20% of
  `html2txt(doc)` and retries with `recallRetry` (`core.go`). Porting it is a
  separate quality feature, not part of baseline parity.

Known approximation: `remove_control_characters` keeps whitespace and drops
`Cc` control characters like upstream, but keeps `Cf` format characters
(e.g. U+200B zero-width space) that Go's `unicode.IsPrint` filter removes —
Rust std has no printable/category predicate. Cosmetic only.

Also matching upstream deliberately: JSON-LD script text is decoded **raw**
(no script-level `unescape_html`) — upstream dropped the old script-level
unescape in the same rewrite; entity decoding happens per value in
`render_baseline_text`. Consequence: a script whose JSON is fully
HTML-escaped (`&quot;…&quot;`) is skipped by both Python 2.3.1 and Go v2.2.x
too. The metadata JSON-LD path still unescapes, because upstream's
metadata extractor does.

The default strategy's text-node walk is depth-capped (`MAX_TREE_DEPTH`)
like every other traversal in this crate; Go relies on growable goroutine
stacks where Rust would overflow, so content nested deeper than the cap is
truncated (other baseline walks were already capped via `text_content`).

**Files**: `src/extraction/baseline.rs`, `src/utils/mod.rs`, `src/settings.rs`

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

### Full htmldate cascade (port of rust-htmldate / htmldate)

Date extraction is a port of the [rust-htmldate](https://crates.io/crates/rust-htmldate)
cascade (itself a port of Python's htmldate, Apache-2.0): meta elements,
JSON-LD, `<abbr>`, date-ish elements, `<title>`/`<h1>`, `<time>`, ISO
timestamps, og:image URLs, idiosyncratic "posted on" patterns, free-text
snippets, and a final raw-HTML scan (incl. copyright-year fallback), all
with newest-wins semantics (`UseOriginalDate = false`, the default).

On the 113-file test corpus this matches rust-htmldate 1.10.2 on every
file but one; the remaining difference (caktusgroup) is intentional:
like Python trafilatura, we feed `og:url`/canonical as the page URL and a
date in that URL path wins over later stages — rust-htmldate only sees
the URL the caller passes it.

`HtmlDateMode::Extensive` (the default) runs the full cascade;
`HtmlDateMode::Fast` restricts to meta + JSON-LD and the fast date-element
subset, as before.

Known approximation (external fallback): the external fallback (upstream
`external_parse`, backed by rust-dateparser) is ported as a **numeric
surface only**:
month-first triples the day-first cascade cannot see (`1-21-2019`),
comma/space triples in either order, mixed separators with a binding
first one (`21-1 2019`), optional trailing clock time, and dateparser's
stripping of a fixed set of English filler words (`on`, `the`, `from`, …),
edge noise punctuation (anything its tokenizer would split into an
ignorable run: `"1-21-2019."`, `"(1-21-2019)"`) and ordinal suffixes
(`"21st"`) before matching — so free text like `"on the 21st of 1 2019"`
parses like `"21 1 2019"`. Not ported:

- interior non-separator punctuation joining the numbers: the tokenizer
  drops any non-alphanumeric run between digits, so upstream resolves
  `"1@21@2019"` or `"1_21_2019"` positionally; our separator grammar
  keeps only `-`, `/`, `.`, space and comma, so those candidates fall
  through to the year scan. Never surfaced in the fuzz corpus (synthetic
  separators were drawn from the realistic set).
- non-ASCII letters whose charset maps to a locale: upstream
  auto-detects the language from the characters and switches the parse
  order (e.g. `é`/`Ж` → day-first), which rejects or reorders triples
  our value-based flip still resolves (`"é1 21 2019"` → None upstream,
  2019-01-21 here). This is the same locale-order deviation as above
  and is independent of position — a standalone `"(é) 1 21 2019"` is
  rejected upstream too. Locale-less non-ASCII characters (`𝑡`, `→`)
  trim at either edge exactly like upstream. Degenerate synthetic
  inputs only.
- dateparser's multi-language month names and locale-dependent orders
  (e.g. `"13 maerz 2019"` or `"Maggio 13, 2019"` resolve upstream via its
  de/it locale tables; our month-name table is exactly upstream
  rust-htmldate's own `regex_parse` table, which lacks them). Affects
  ~36 of 9,227 differential fuzz cases.
- junk-input quirks on day-`00`, year-`0000` and year-month triples,
  where dateparser clamps day 0 to the current day of the month
  (`"1/00/2019"` → `2019-01-16`, i.e. the 16th on fuzz day), resolves
  year 0 to the current year, or fills the missing day of a year-first
  `"2019/06"` with the same current-day rule (→ `2019-06-16`). We
  return the first of the month or reject the year — deterministic,
  where upstream's answer changes with the calendar. Affects the
  remaining ~198 fuzz cases, all synthetic degenerate inputs.

Known approximation (unix stamps): `<abbr data-utime>` stamps
convert through the system timezone on native targets, like upstream's
environment zone (TZ / iana_time_zone) and Python's local
`datetime.fromtimestamp`. On wasm32-unknown-unknown `chrono::Local` is
unavailable (its js-sys glue is stubbed out by the JS loader), so wasm
builds convert as UTC — same gate as the `Local::now()` plausibility
windows elsewhere in the crate.

Known approximation: the date-element selector reads `id`/`class`/
`itemprop` attributes the way upstream does, but upstream takes the first
attribute *present* in source order while our DOM (scraper default
features) sorts attributes alphabetically, losing source order. We check
all present attributes, which is over-inclusive when an element's
non-first attribute carries a date hint; a real corpus shows this loses
fewer dates than any deterministic ordering of the sorted attributes.

**Files**: `src/metadata/dates.rs`, `src/metadata/mod.rs`

## Output Format

### Apostrophe escaping

Go serializes apostrophes as `&#39;`; html5ever leaves them unescaped. Both
are valid HTML — the difference is cosmetic.

## Test Adjustments

### `test_extract_schleifen_ucoz`

Removed the `"Aufrufe:"` assertion. Go extracts this page-view counter via
go-readability, but the Python original does not. Rust aligns with Python
behavior.
