#!/usr/bin/env python3
"""Compare Rust vs Python per-entry comparison results.

Usage:
    python3 scripts/diff_comparison.py comparison-data/rust_results.jsonl comparison-data/python_results.jsonl

Shows entries where Rust and Python disagree on any assertion.
"""

import json
import sys


def load_results(path):
    results = {}
    with open(path) as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            entry = json.loads(line)
            results[entry["file"]] = entry
    return results


def main():
    if len(sys.argv) != 3:
        print(f"Usage: {sys.argv[0]} <rust_results.jsonl> <python_results.jsonl>", file=sys.stderr)
        sys.exit(1)

    rust = load_results(sys.argv[1])
    python = load_results(sys.argv[2])

    all_files = sorted(set(rust.keys()) | set(python.keys()))

    # Categorize differences.
    rust_better = []   # Rust correct, Python wrong
    python_better = [] # Python correct, Rust wrong
    both_wrong = []    # Both wrong on the same assertion
    both_right = 0

    for file in all_files:
        r = rust.get(file)
        p = python.get(file)
        if r is None or p is None:
            continue

        r_details = {(d["type"], d["string"]): d["found"] for d in r["details"]}
        p_details = {(d["type"], d["string"]): d["found"] for d in p["details"]}

        for key in set(r_details.keys()) | set(p_details.keys()):
            r_found = r_details.get(key)
            p_found = p_details.get(key)
            if r_found is None or p_found is None:
                continue

            kind, string = key
            # "with" assertion: found=True is correct
            # "without" assertion: found=False is correct
            r_correct = (r_found if kind == "with" else not r_found)
            p_correct = (p_found if kind == "with" else not p_found)

            if r_correct and p_correct:
                both_right += 1
            elif r_correct and not p_correct:
                rust_better.append((file, kind, string, r.get("text_len", 0), p.get("text_len", 0)))
            elif not r_correct and p_correct:
                python_better.append((file, kind, string, r.get("text_len", 0), p.get("text_len", 0)))
            else:
                both_wrong.append((file, kind, string))

    print(f"=== Summary ===")
    print(f"Both correct:  {both_right}")
    print(f"Rust better:   {len(rust_better)}")
    print(f"Python better: {len(python_better)}")
    print(f"Both wrong:    {len(both_wrong)}")
    print()

    if python_better:
        print(f"=== Python better ({len(python_better)} assertions) ===")
        print(f"{'File':<50} {'Type':<8} {'Rust len':>8} {'Py len':>8}  Assertion")
        print("-" * 120)
        for file, kind, string, r_len, p_len in sorted(python_better):
            short = string[:50] + "..." if len(string) > 50 else string
            print(f"{file:<50} {kind:<8} {r_len:>8} {p_len:>8}  {short}")
        print()

    if rust_better:
        print(f"=== Rust better ({len(rust_better)} assertions) ===")
        print(f"{'File':<50} {'Type':<8} {'Rust len':>8} {'Py len':>8}  Assertion")
        print("-" * 120)
        for file, kind, string, r_len, p_len in sorted(rust_better):
            short = string[:50] + "..." if len(string) > 50 else string
            print(f"{file:<50} {kind:<8} {r_len:>8} {p_len:>8}  {short}")
        print()

    if both_wrong:
        print(f"=== Both wrong ({len(both_wrong)} assertions) ===")
        for file, kind, string in sorted(both_wrong):
            short = string[:60] + "..." if len(string) > 60 else string
            print(f"  {file}: [{kind}] {short}")


if __name__ == "__main__":
    main()
