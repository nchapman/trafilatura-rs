// Extraction benchmarks using Criterion.
//
// Run with:
//   cargo bench
//   cargo bench -- --output-format verbose
//
// Three benchmark groups:
//   1. single_document — individual pages spanning the size spectrum
//   2. corpus          — all 960 comparison-suite entries in one pass
//   3. modes           — per-mode throughput (balanced, precision, recall)

use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion, Throughput};
use std::path::Path;
use trafilatura::options::{ExtractionFocus, Options};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn test_files_dir() -> &'static Path {
    static DIR: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
    DIR.get_or_init(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("test-files"))
}

fn load_mock(filename: &str) -> String {
    let path = test_files_dir().join("mock").join(filename);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("Failed to read {filename}: {e}"))
}

fn load_comparison_html(filename: &str) -> Option<String> {
    for dir in &["comparison", "mock"] {
        let path = test_files_dir().join(dir).join(filename);
        if path.exists() {
            return std::fs::read_to_string(path).ok();
        }
    }
    None
}

#[derive(serde::Deserialize)]
struct ComparisonEntry {
    file: String,
}

fn load_corpus() -> Vec<String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("comparison-data/entries.json");
    let json = std::fs::read_to_string(path).expect("comparison-data/entries.json not found");
    let entries: Vec<ComparisonEntry> =
        serde_json::from_str(&json).expect("failed to parse entries.json");
    entries
        .into_iter()
        .filter_map(|e| load_comparison_html(&e.file))
        .collect()
}

fn default_opts() -> Options {
    Options::default()
        .with_fallback(true)
        .with_focus(ExtractionFocus::Balanced)
        .with_exclude_comments(true)
}

// ---------------------------------------------------------------------------
// 1. Single-document benchmarks
// ---------------------------------------------------------------------------

fn bench_single_document(c: &mut Criterion) {
    // Representative pages from the mock set, spanning the size spectrum.
    let pages: &[(&str, &str)] = &[
        ("small  (6KB)", "die-partei.net.luebeck.html"),
        ("medium (85KB)", "zeit.de.zugverkehr.html"),
        ("large  (382KB)", "reuters.com.parasite.html"),
        ("xlarge (906KB)", "pcgamer.com.skyrim.html"),
    ];

    let mut group = c.benchmark_group("single_document");

    for &(label, filename) in pages {
        let html = load_mock(filename);
        let bytes = html.len() as u64;
        group.throughput(Throughput::Bytes(bytes));
        group.bench_with_input(BenchmarkId::new("extract", label), &html, |b, html| {
            b.iter(|| trafilatura::extract(html, &default_opts()))
        });
    }

    group.finish();
}

// ---------------------------------------------------------------------------
// 2. Full corpus benchmark (960 documents, single-threaded)
// ---------------------------------------------------------------------------

fn bench_corpus(c: &mut Criterion) {
    let corpus = load_corpus();
    let total_bytes: u64 = corpus.iter().map(|h| h.len() as u64).sum();

    let mut group = c.benchmark_group("corpus");
    group.sample_size(10); // corpus run is slow; 10 samples is enough
    group.throughput(Throughput::Bytes(total_bytes));

    group.bench_function("extract_all_960", |b| {
        b.iter_batched(
            || corpus.clone(),
            |docs| {
                for html in docs {
                    let _ = trafilatura::extract(&html, &default_opts());
                }
            },
            BatchSize::LargeInput,
        )
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// 3. Mode comparison (on a single representative document)
// ---------------------------------------------------------------------------

fn bench_modes(c: &mut Criterion) {
    let html = load_mock("reuters.com.parasite.html");
    let bytes = html.len() as u64;

    let modes: &[(&str, bool, ExtractionFocus)] = &[
        ("balanced_no_fallback", false, ExtractionFocus::Balanced),
        ("balanced_fallback", true, ExtractionFocus::Balanced),
        ("favor_precision", true, ExtractionFocus::FavorPrecision),
        ("favor_recall", true, ExtractionFocus::FavorRecall),
    ];

    let mut group = c.benchmark_group("modes");
    group.throughput(Throughput::Bytes(bytes));

    for &(label, enable_fallback, focus) in modes {
        let opts = Options::default()
            .with_fallback(enable_fallback)
            .with_focus(focus)
            .with_exclude_comments(true);
        group.bench_with_input(BenchmarkId::new("extract", label), &html, |b, html| {
            b.iter(|| trafilatura::extract(html, &opts))
        });
    }

    group.finish();
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

criterion_group!(benches, bench_single_document, bench_corpus, bench_modes,);
criterion_main!(benches);
