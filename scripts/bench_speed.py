#!/usr/bin/env python3
"""Speed benchmark: single-document extraction time, Python trafilatura.

Companion to `cargo bench --bench extraction -- single_document` and
`scripts/bench_go.go`. Runs the same four mock fixtures with matching
options (fallback enabled, balanced focus, comments excluded) and reports
the median per-document wall time.

Usage:
    python3 scripts/bench_speed.py [--testfiles DIR] [--n-factor N]

Requires the `trafilatura` package (pip install trafilatura).
"""

from __future__ import annotations

import argparse
import statistics
import sys
import time

import trafilatura

PAGES = [
    # (label, file, iterations) — labels match benches/extraction.rs
    ("small  (6KB)", "die-partei.net.luebeck.html", 100),
    ("medium (85KB)", "zeit.de.zugverkehr.html", 50),
    ("large  (382KB)", "reuters.com.parasite.html", 30),
    ("xlarge (906KB)", "pcgamer.com.skyrim.html", 20),
]


def load(file: str, testfiles: str) -> str | None:
    for sub in ("mock", "comparison"):
        try:
            with open(f"{testfiles}/{sub}/{file}", encoding="utf-8") as fh:
                return fh.read()
        except FileNotFoundError:
            continue
    return None


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument(
        "--testfiles",
        default=None,
        help="Path to the test-files directory (default: <repo>/test-files)",
    )
    ap.add_argument(
        "--n-factor",
        type=int,
        default=1,
        help="Scale iteration counts by this factor (e.g. 0.5 for a quick run)",
    )
    args = ap.parse_args()

    testfiles = args.testfiles or str(
        __import__("pathlib").Path(__file__).resolve().parent.parent / "test-files"
    )

    print(f"python trafilatura {trafilatura.__version__}")
    for label, fname, n in PAGES:
        html = load(fname, testfiles)
        if html is None:
            print(f"{label}: file not found: {fname}", file=sys.stderr)
            sys.exit(1)
        n = max(3, int(n * args.n_factor))
        for _ in range(3):  # warmup
            trafilatura.extract(html, include_comments=False)
        durations = []
        for _ in range(n):
            start = time.perf_counter()
            trafilatura.extract(html, include_comments=False)
            durations.append(time.perf_counter() - start)
        med = statistics.median(durations)
        print(f"{label}: median {med * 1000:.3f} ms over {n} runs")


if __name__ == "__main__":
    main()
