# Scripts

Tooling for cross-implementation benchmarking against upstream
[go-trafilatura](https://github.com/markusmobius/go-trafilatura) and
[trafilatura](https://github.com/adbar/trafilatura) (Python). The Go/Python
numbers in the README's benchmark tables come from these — re-run them
whenever a new upstream version ships.

The speed and quality harnesses all use the same fixtures and options
(fallback enabled, balanced focus, comments excluded) so the columns are
comparable:

- fixtures: `test-files/mock/{die-partei.net.luebeck.html, zeit.de.zugverkehr.html,
  reuters.com.parasite.html, pcgamer.com.skyrim.html}`
- corpus: `comparison-data/entries.json` (960 entries)
- the Rust side is `benches/extraction.rs` (`cargo bench`) and
  `tests/comparison_test.rs`

## Speed (single-document, median per page)

### Rust

```sh
cargo bench --bench extraction -- single_document
```

### Go

From inside a go-trafilatura checkout (v2 module path):

```sh
go run /path/to/trafilatura-rs/scripts/bench_go.go \
    -testfiles /path/to/trafilatura-rs/test-files
```

Without `-corpus` only the single-document benchmarks run; add
`-corpus <entries.json>` for the 960-doc corpus throughput pass too.
The `-single <file>` / `-single-n <N>` / `-no-fallback` flags benchmark one
file with mean timing instead.

### Python

```sh
python3 scripts/bench_speed.py            # add --testfiles DIR / --n-factor 0.5
```

## Quality (960-entry corpus)

### Rust

```sh
cargo test --test comparison_test -- --nocapture
```

### Go

```sh
cd /path/to/go-trafilatura
go run /path/to/trafilatura-rs/scripts/compare_go.go \
    -corpus /path/to/trafilatura-rs/comparison-data/entries.json \
    -testfiles /path/to/trafilatura-rs/test-files
```

### Python

```sh
python3 scripts/compare_python.py          # scores on stderr
```

## Other

- `compare_rust.rs` — standalone Rust quality scorer (same evaluation as
  `comparison_test.rs`, usable outside `cargo test`).
- `diff_comparison.py` — per-file diff of Rust vs Go/Python extraction
  output, for drilling into quality differences.
- `convert_comparison_data` — rebuilds `comparison-data/` from
  go-trafilatura's `comparisonData` dump.
