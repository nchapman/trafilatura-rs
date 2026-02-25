// Converts go-trafilatura's comparison dataset (scripts/comparison/data.go)
// to JSON for use by the Rust comparison test suite.
//
// Usage (from trafilatura-rs root):
//   go run ./scripts/convert_comparison_data/ > comparison-data/entries.json

package main

import (
	"encoding/json"
	"fmt"
	"os"
)

// OutputEntry adds the URL (the map key from comparisonData) to ComparisonEntry.
type OutputEntry struct {
	URL string `json:"url"`
	ComparisonEntry
}

func main() {
	entries := make([]OutputEntry, 0, len(comparisonData))
	for url, e := range comparisonData {
		entries = append(entries, OutputEntry{URL: url, ComparisonEntry: e})
	}

	enc := json.NewEncoder(os.Stdout)
	enc.SetIndent("", "  ")
	if err := enc.Encode(entries); err != nil {
		fmt.Fprintf(os.Stderr, "error: %v\n", err)
		os.Exit(1)
	}
}
