// Per-entry comparison output for diffing against Python trafilatura.
//
// Usage: cargo test --test comparison_detail_test -- --nocapture > comparison-data/rust_results.jsonl

use serde::{Deserialize, Serialize};
use std::path::Path;
use trafilatura::options::{ExtractionFocus, Options};

#[derive(Deserialize)]
struct ComparisonEntry {
    url: String,
    file: String,
    #[serde(default)]
    with: Vec<String>,
    #[serde(default)]
    without: Vec<String>,
}

#[derive(Serialize)]
struct Detail {
    #[serde(rename = "type")]
    kind: String,
    string: String,
    found: bool,
}

#[derive(Serialize)]
struct EntryResult {
    file: String,
    url: String,
    text_len: usize,
    tp: usize,
    #[serde(rename = "fn")]
    fn_: usize,
    fp: usize,
    tn: usize,
    details: Vec<Detail>,
}

fn load_html(file: &str) -> Option<String> {
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-files");
    for dir in &["comparison", "mock"] {
        let path = base.join(dir).join(file);
        if path.exists() {
            let bytes = std::fs::read(&path).ok()?;
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
                Some(String::from_utf8_lossy(&std::fs::read(&path).ok()?).into_owned())
            });
        }
    }
    None
}

#[test]
fn comparison_detail() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("comparison-data/entries.json");
    let json = std::fs::read_to_string(&path).unwrap();
    let entries: Vec<ComparisonEntry> = serde_json::from_str(&json).unwrap();

    let mut total_tp = 0usize;
    let mut total_fn = 0usize;
    let mut total_fp = 0usize;
    let mut total_tn = 0usize;
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
            let mut o = Options::default()
                .with_fallback(true)
                .with_focus(ExtractionFocus::Balanced)
                .with_exclude_comments(true);
            o.original_url = url;
            o
        };

        let text = trafilatura::extract(&html, &opts)
            .map(|r| r.content_text)
            .unwrap_or_default();

        let mut tp = 0usize;
        let mut fn_ = 0usize;
        let mut fp = 0usize;
        let mut tn = 0usize;
        let mut details = Vec::new();

        for s in &entry.with {
            let found = text.contains(s.as_str());
            details.push(Detail {
                kind: "with".into(),
                string: s.clone(),
                found,
            });
            if found {
                tp += 1;
            } else {
                fn_ += 1;
            }
        }

        for s in &entry.without {
            let found = text.contains(s.as_str());
            details.push(Detail {
                kind: "without".into(),
                string: s.clone(),
                found,
            });
            if found {
                fp += 1;
            } else {
                tn += 1;
            }
        }

        total_tp += tp;
        total_fn += fn_;
        total_fp += fp;
        total_tn += tn;
        evaluated += 1;

        let result = EntryResult {
            file: entry.file.clone(),
            url: entry.url.clone(),
            text_len: text.len(),
            tp,
            fn_,
            fp,
            tn,
            details,
        };
        println!("{}", serde_json::to_string(&result).unwrap());
    }

    let p = if total_tp + total_fp > 0 {
        total_tp as f64 / (total_tp + total_fp) as f64
    } else {
        0.0
    };
    let r = if total_tp + total_fn > 0 {
        total_tp as f64 / (total_tp + total_fn) as f64
    } else {
        0.0
    };
    let a = if total_tp + total_tn + total_fp + total_fn > 0 {
        (total_tp + total_tn) as f64 / (total_tp + total_tn + total_fp + total_fn) as f64
    } else {
        0.0
    };
    let f = if 2 * total_tp + total_fp + total_fn > 0 {
        (2 * total_tp) as f64 / (2 * total_tp + total_fp + total_fn) as f64
    } else {
        0.0
    };

    eprintln!("Rust trafilatura: precision={p:.3}  recall={r:.3}  accuracy={a:.3}  f-score={f:.3}  (evaluated={evaluated}, skipped={skipped})");
}
