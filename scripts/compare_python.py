#!/usr/bin/env python3
"""Run the comparison dataset through Python trafilatura and output per-entry results.

Usage: python3 scripts/compare_python.py > comparison-data/python_results.jsonl
"""

import json
import os
import sys

import trafilatura

ENTRIES_PATH = os.path.join(os.path.dirname(__file__), "..", "comparison-data", "entries.json")
TEST_FILES = os.path.join(os.path.dirname(__file__), "..", "test-files")


def load_html(filename):
    for subdir in ("comparison", "mock"):
        path = os.path.join(TEST_FILES, subdir, filename)
        if os.path.exists(path):
            with open(path, "rb") as f:
                raw = f.read()
            # Try UTF-8 first, fall back to latin-1.
            try:
                return raw.decode("utf-8")
            except UnicodeDecodeError:
                return raw.decode("latin-1")
    return None


def evaluate_entry(entry, text):
    tp = fn = fp = tn = 0
    details = []
    if not text:
        fn = len(entry.get("with", []))
        tn = len(entry.get("without", []))
        for s in entry.get("with", []):
            details.append({"type": "with", "string": s, "found": False})
        for s in entry.get("without", []):
            details.append({"type": "without", "string": s, "found": False})
        return tp, fn, fp, tn, details

    for s in entry.get("with", []):
        found = s in text
        details.append({"type": "with", "string": s, "found": found})
        if found:
            tp += 1
        else:
            fn += 1

    for s in entry.get("without", []):
        found = s in text
        details.append({"type": "without", "string": s, "found": found})
        if found:
            fp += 1
        else:
            tn += 1

    return tp, fn, fp, tn, details


def main():
    with open(ENTRIES_PATH) as f:
        entries = json.load(f)

    total_tp = total_fn = total_fp = total_tn = 0
    evaluated = skipped = 0

    for entry in entries:
        html = load_html(entry["file"])
        if html is None:
            skipped += 1
            continue

        url = entry.get("url", "")
        try:
            text = trafilatura.extract(
                html,
                url=url,
                no_fallback=False,
                include_comments=False,
                include_tables=True,
            ) or ""
        except Exception as e:
            text = ""
            print(json.dumps({"file": entry["file"], "error": str(e)}), file=sys.stderr)

        tp, fn, fp, tn, details = evaluate_entry(entry, text)
        total_tp += tp
        total_fn += fn
        total_fp += fp
        total_tn += tn
        evaluated += 1

        # Output per-entry result.
        print(json.dumps({
            "file": entry["file"],
            "url": url,
            "text_len": len(text),
            "tp": tp, "fn": fn, "fp": fp, "tn": tn,
            "details": details,
        }))

    # Summary to stderr.
    p = total_tp / (total_tp + total_fp) if (total_tp + total_fp) > 0 else 0
    r = total_tp / (total_tp + total_fn) if (total_tp + total_fn) > 0 else 0
    a = (total_tp + total_tn) / (total_tp + total_tn + total_fp + total_fn) if (total_tp + total_tn + total_fp + total_fn) > 0 else 0
    f = (2 * total_tp) / (2 * total_tp + total_fp + total_fn) if (2 * total_tp + total_fp + total_fn) > 0 else 0

    print(f"Python trafilatura: precision={p:.3f}  recall={r:.3f}  accuracy={a:.3f}  f-score={f:.3f}  (evaluated={evaluated}, skipped={skipped})", file=sys.stderr)


if __name__ == "__main__":
    main()
