// Comparison test suite — port of go-trafilatura's scripts/comparison/content.go
//
// Loads 926 entries from comparison-data/entries.json, extracts content from
// the corresponding HTML fixtures, and scores precision/recall/accuracy/F-score.
// Used to verify that our extraction quality is on par with the Go implementation.

use serde::Deserialize;
use std::path::Path;
use trafilatura::options::{ExtractionFocus, Options};

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct ComparisonEntry {
    url: String,
    file: String,
    #[serde(default)]
    with: Vec<String>,
    #[serde(default)]
    without: Vec<String>,
}

#[derive(Debug, Default, Clone, Copy)]
struct EvaluationResult {
    true_positives: usize,
    false_negatives: usize,
    false_positives: usize,
    true_negatives: usize,
}

impl EvaluationResult {
    fn merge(self, other: Self) -> Self {
        Self {
            true_positives: self.true_positives + other.true_positives,
            false_negatives: self.false_negatives + other.false_negatives,
            false_positives: self.false_positives + other.false_positives,
            true_negatives: self.true_negatives + other.true_negatives,
        }
    }
}

#[derive(Debug)]
struct Performance {
    title: String,
    evaluated: usize,
    skipped: usize,
    precision: f64,
    recall: f64,
    accuracy: f64,
    fscore: f64,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn load_entries() -> Vec<ComparisonEntry> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("comparison-data/entries.json");
    let json = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to read comparison-data/entries.json: {e}"));
    serde_json::from_str(&json)
        .unwrap_or_else(|e| panic!("Failed to parse comparison-data/entries.json: {e}"))
}

/// Load HTML from test-files/comparison/ or test-files/mock/, mirroring Go's openDataFile.
fn load_html(file: &str) -> Option<String> {
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-files");

    for dir in &["comparison", "mock"] {
        let path = base.join(dir).join(file);
        if path.exists() {
            let bytes = std::fs::read(&path).ok()?;
            // Detect and handle ISO-8859-1 / Windows-1252 encoded files.
            let lower: Vec<u8> = bytes[..bytes.len().min(4096)]
                .iter()
                .map(|&b| if b < 128 { b.to_ascii_lowercase() } else { 0 })
                .collect();
            let lower_str = std::str::from_utf8(&lower).unwrap_or("");
            if lower_str.contains("charset=iso-8859")
                || lower_str.contains("charset=\"iso-8859")
                || lower_str.contains("charset=windows-1252")
                || lower_str.contains("charset=\"windows-1252")
            {
                let (decoded, _, _) = encoding_rs::WINDOWS_1252.decode(&bytes);
                return Some(decoded.into_owned());
            }
            return String::from_utf8(bytes).ok().or_else(|| {
                std::fs::read(path.clone())
                    .ok()
                    .map(|b| String::from_utf8_lossy(&b).into_owned())
            });
        }
    }
    None
}

fn evaluate_entry(entry: &ComparisonEntry, result: &str) -> EvaluationResult {
    let mut ev = EvaluationResult::default();

    if result.is_empty() {
        ev.false_negatives += entry.with.len();
        ev.true_negatives += entry.without.len();
        return ev;
    }

    for expected in &entry.with {
        if result.contains(expected.as_str()) {
            ev.true_positives += 1;
        } else {
            ev.false_negatives += 1;
        }
    }

    for unwanted in &entry.without {
        if result.contains(unwanted.as_str()) {
            ev.false_positives += 1;
        } else {
            ev.true_negatives += 1;
        }
    }

    ev
}

fn calculate_performance(
    title: &str,
    ev: EvaluationResult,
    evaluated: usize,
    skipped: usize,
) -> Performance {
    let tp = ev.true_positives as f64;
    let fn_ = ev.false_negatives as f64;
    let fp = ev.false_positives as f64;
    let tn = ev.true_negatives as f64;

    let precision = if tp + fp > 0.0 { tp / (tp + fp) } else { 0.0 };
    let recall = if tp + fn_ > 0.0 { tp / (tp + fn_) } else { 0.0 };
    let accuracy = if tp + tn + fp + fn_ > 0.0 {
        (tp + tn) / (tp + tn + fp + fn_)
    } else {
        0.0
    };
    let fscore = if 2.0 * tp + fp + fn_ > 0.0 {
        (2.0 * tp) / (2.0 * tp + fp + fn_)
    } else {
        0.0
    };

    Performance {
        title: title.to_string(),
        evaluated,
        skipped,
        precision,
        recall,
        accuracy,
        fscore,
    }
}

fn run_comparison(title: &str, enable_fallback: bool, focus: ExtractionFocus) -> Performance {
    let entries = load_entries();
    let mut total = EvaluationResult::default();
    let mut evaluated = 0usize;
    let mut skipped = 0usize;

    for entry in &entries {
        let html = match load_html(&entry.file) {
            Some(h) => h,
            None => {
                skipped += 1;
                continue;
            }
        };

        let url = url::Url::parse(&entry.url).ok();
        let opts = {
            let mut o = Options::default();
            o.original_url = url;
            o.enable_fallback = enable_fallback;
            o.focus = focus;
            o.exclude_comments = true;
            o.exclude_tables = false;
            o
        };

        let result = trafilatura::extract(&html, opts)
            .map(|r| r.content_text)
            .unwrap_or_default();

        total = total.merge(evaluate_entry(entry, &result));
        evaluated += 1;
    }

    calculate_performance(title, total, evaluated, skipped)
}

fn print_performance(p: &Performance) {
    println!(
        "{}: precision={:.3}  recall={:.3}  accuracy={:.3}  f-score={:.3}  \
         (evaluated={}, skipped={})",
        p.title, p.precision, p.recall, p.accuracy, p.fscore, p.evaluated, p.skipped
    );
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Full comparison across all extraction modes.
///
/// Actual measured scores on 960 entries:
///   Balanced (no fallback): precision=0.911  recall=0.895  accuracy=0.904  f-score=0.903
///   Balanced + Fallback:    precision=0.911  recall=0.896  accuracy=0.904  f-score=0.903
///   FavorPrecision:         precision=0.922  recall=0.873  accuracy=0.900  f-score=0.897
///   FavorRecall:            precision=0.904  recall=0.903  accuracy=0.904  f-score=0.903
///
/// Thresholds are set 2pp below measured values to catch regressions without
/// being brittle to minor extraction changes.
#[test]
fn comparison_all_modes() {
    let no_fallback = run_comparison("Balanced (no fallback)", false, ExtractionFocus::Balanced);
    let balanced = run_comparison("Balanced + Fallback", true, ExtractionFocus::Balanced);
    let favor_precision = run_comparison("FavorPrecision", true, ExtractionFocus::FavorPrecision);
    let favor_recall = run_comparison("FavorRecall", true, ExtractionFocus::FavorRecall);

    print_performance(&no_fallback);
    print_performance(&balanced);
    print_performance(&favor_precision);
    print_performance(&favor_recall);

    // Balanced + Fallback — primary mode, tightest thresholds.
    assert!(
        balanced.precision >= 0.89,
        "balanced precision {:.3} below 0.89",
        balanced.precision
    );
    assert!(
        balanced.recall >= 0.87,
        "balanced recall {:.3} below 0.87",
        balanced.recall
    );
    assert!(
        balanced.accuracy >= 0.88,
        "balanced accuracy {:.3} below 0.88",
        balanced.accuracy
    );
    assert!(
        balanced.fscore >= 0.88,
        "balanced f-score {:.3} below 0.88",
        balanced.fscore
    );

    // FavorPrecision should have higher (or equal) precision than balanced.
    assert!(
        favor_precision.precision >= balanced.precision - 0.01,
        "FavorPrecision ({:.3}) should not be worse than Balanced ({:.3})",
        favor_precision.precision,
        balanced.precision,
    );

    // FavorRecall should have higher (or equal) recall than balanced.
    assert!(
        favor_recall.recall >= balanced.recall - 0.01,
        "FavorRecall ({:.3}) should not be worse than Balanced ({:.3})",
        favor_recall.recall,
        balanced.recall,
    );
}
