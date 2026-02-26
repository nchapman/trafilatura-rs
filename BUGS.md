# Known Bugs & Ignored Tests

## Status Summary

- **495 tests passing, 0 failing, 8 ignored**
- All ignored tests have root causes identified below
- **No extraction pipeline bugs** — all failures are readability library quality differences

## Reference Implementations

| Label | Path | Notes |
|-------|------|-------|
| **Python trafilatura** | `/Users/nchapman/Code/trafilatura` | Original. Behavioral gold standard. |
| **Go trafilatura** | `/Users/nchapman/Code/go-trafilatura` | Port we're porting from. Uses go-shiori readability (deprecated). |
| **readability-rs** | `../readability-rs` | Our readability lib. Ported from readeck/go-readability (maintained fork). |
| **Readability.js** | `/Users/nchapman/Code/lessisbetter/refs/readability` | Mozilla's JS original. Tested via jsdom. |

### Readability library landscape

- **go-shiori/go-readability**: Used by go-trafilatura. **Deprecated** in favor of readeck.
- **readeck/go-readability/v2**: Maintained fork. Our readability-rs is ported from this.
- **Readability.js**: Mozilla's canonical JS implementation.
- readeck intentionally adds `+1` to comma scoring (`contentScore += numCommas + 1`) to match a Readability.js quirk. go-shiori does not.
- This scoring difference causes readeck and go-shiori to diverge on pages where alternative ancestor heuristics trigger.

---

## Category A: Readability-Dependent Tests (go-shiori vs readability-rs)

These tests pass in Go only because go-shiori readability fallback produces the right output. **Go's own extraction pipeline also returns empty for these inputs** (verified with `EnableFallback=false`). They are NOT extraction pipeline bugs.

### A1. Single-link paragraph

**Test**: `test_formatting_links_stripped_by_default`
**Input**: `<p><a href="">Link text</a></p>`

| Implementation | Own extraction | With fallback |
|----------------|---------------|---------------|
| Go trafilatura | `""` (empty) | go-shiori: `"Link text"` ✓ |
| Rust trafilatura | `""` (empty) | readability-rs: `""` ✗ |

**Root cause**: A paragraph where 100% of text comes from links is intentionally dropped by trafilatura's link-density filter. Go only passes because go-shiori extracts it. This is working as designed — not a bug.

### A2. Bare div content

**Test**: `test_formatting_empty_div_then_content_div`
**Input**: `<div>\t\n</div><div>There is text here.</div>`

| Implementation | Own extraction | With fallback |
|----------------|---------------|---------------|
| Go trafilatura | `""` (empty) | go-shiori: `"There is text here."` ✓ |
| Rust trafilatura | `""` (empty) | readability-rs: `""` ✗ |

**Root cause**: Same — Go's own extraction also returns empty. go-shiori fallback saves it.

### A3. Lone h2 after pruning

**Test**: `test_prune_selector_p_and_h1_keeps_h2`
**Input**: `<h1>ABC</h1><h2>42</h2>` + 50×`<p>abc</p>`, with `prune_selector="p, h1"`

| Implementation | Own extraction | With fallback |
|----------------|---------------|---------------|
| Go trafilatura | `""` (empty) | go-shiori: `"42"` ✓ |
| Rust trafilatura | `""` (empty) | readability-rs: `""` ✗ |

**Root cause**: Same pattern. After pruning, only `<h2>42</h2>` remains — not enough for the extraction pipeline.

### Resolution

These are all readability-rs quality issues, not trafilatura-rs bugs. Options:
1. **Accept as-is** — readability-rs is correct per readeck/Readability.js
2. **Improve readability-rs** — but that means diverging from readeck
3. **Improve trafilatura extraction** — if Python handles these without fallback, we could too

---

## Category B: Readability Quality on Real Pages

### B1. love-hina: readability includes comment junk

**Test**: `test_extract_love_hina`
**Assertion**: content should NOT contain `"Kommentare schreiben"`

Rust trafilatura extracts 317 chars (above min_extracted_size=250). Readability-rs produces 1060 chars including comment form text. Since 1060 > 2×317, `candidate_is_usable` accepts it.

| Implementation | Own extraction | Readability output |
|----------------|---------------|-------------------|
| Go trafilatura | 317 chars | go-shiori: clean (no comment junk) ✓ |
| Rust trafilatura | 317 chars | readability-rs: includes junk ✗ |

### B2. spiegel-albtraum: readability drops intro paragraph

**Test**: `test_extract_spiegel_albtraum`
**Assertion**: content should contain `"Wie konnte es dazu kommen?"`

The intro text is in `<div class="dig-vorspann">`. go-shiori preserves it, readability-rs drops it.

### B3. rnz-witzel: Python succeeds without fallback

**Test**: `test_extract_rnz_witzel`
**Assertion**: content should contain `"Für einen Roman"`

| Implementation | Own extraction | Readability fallback |
|----------------|---------------|---------------------|
| Python trafilatura | 603 chars ✓ | not needed |
| Go trafilatura | nav junk ✗ | go-shiori: 608 chars ✓ |
| Rust trafilatura | 0 chars ✗ | readability-rs: 0 chars ✗ |
| Readability.js | — | null (fails) |

**This is the only actionable bug.** Python extracts the article correctly without any readability fallback. Both Go and Rust fail at the content extraction stage. This means Python's extraction logic handles something the Go port missed. Fixing this in our pipeline would make us resilient regardless of readability quality.

### B4. Other

- `test_external_scam_with_fallback_nonempty` — go-shiori filters ad links, readability-rs doesn't
- `test_prune_selector_p_keeps_h1` — same class as A3

---

## Priority

1. **B3 (rnz-witzel)**: Only real extraction pipeline bug. Python succeeds without fallback. Compare Python's extraction logic to find what Go/Rust miss.
2. **B1-B2, B4, A1-A3**: All readability-rs quality differences. Can only be fixed by improving readability-rs or adding compensating logic in trafilatura-rs.

---

## Investigation Log

### 2025-02-25: readability-rs integration

- Confirmed readability-rs is a correct port of readeck/go-readability
- Confirmed go-shiori diverges from Readability.js (deprecated for a reason)
- Aligned external.rs with Go's generator loop pattern
- Commit: 1300c44

### 2025-02-25: Category A reclassification

- Tested A1, A2, A3 with Go `EnableFallback=false` — **Go also returns empty**
- All three are readability-dependent, not extraction pipeline bugs
- A1: link-only paragraphs intentionally dropped (link-density filter)
- A2: bare div text not enough for extraction pipeline
- A3: lone h2 after pruning not enough for extraction pipeline
- Reclassified from "extraction pipeline bugs" to "readability quality differences"
