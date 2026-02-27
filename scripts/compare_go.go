// Run the comparison dataset through go-trafilatura and report quality scores.
//
// Usage:
//   cd /path/to/go-trafilatura
//   go run /path/to/trafilatura-rs/scripts/compare_go.go \
//       -corpus /path/to/trafilatura-rs/comparison-data/entries.json \
//       -testfiles /path/to/trafilatura-rs/test-files

package main

import (
	"encoding/json"
	"flag"
	"fmt"
	"os"
	"path/filepath"
	"strings"

	trafilatura "github.com/markusmobius/go-trafilatura"
)

type ComparisonEntry struct {
	File    string   `json:"file"`
	URL     string   `json:"url"`
	With    []string `json:"with"`
	Without []string `json:"without"`
}

func loadHTML(file string, testFilesDir string) (string, bool) {
	for _, dir := range []string{"comparison", "mock"} {
		path := filepath.Join(testFilesDir, dir, file)
		data, err := os.ReadFile(path)
		if err == nil {
			return string(data), true
		}
	}
	return "", false
}

func main() {
	corpusPath := flag.String("corpus", "", "Path to entries.json")
	testFilesDir := flag.String("testfiles", "", "Path to test-files directory")
	flag.Parse()

	if *corpusPath == "" || *testFilesDir == "" {
		fmt.Fprintln(os.Stderr, "Usage: go run compare_go.go -corpus <entries.json> -testfiles <test-files-dir>")
		os.Exit(1)
	}

	data, err := os.ReadFile(*corpusPath)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Failed to read %s: %v\n", *corpusPath, err)
		os.Exit(1)
	}

	var entries []ComparisonEntry
	if err := json.Unmarshal(data, &entries); err != nil {
		fmt.Fprintf(os.Stderr, "Failed to parse entries: %v\n", err)
		os.Exit(1)
	}

	var totalTP, totalFN, totalFP, totalTN float64
	evaluated, skipped := 0, 0

	opts := trafilatura.Options{
		EnableFallback:  true,
		Focus:           trafilatura.Balanced,
		ExcludeComments: true,
		ExcludeTables:   false,
	}

	for _, entry := range entries {
		html, ok := loadHTML(entry.File, *testFilesDir)
		if !ok {
			skipped++
			continue
		}

		result, err := trafilatura.Extract(strings.NewReader(html), opts)
		text := ""
		if err == nil && result != nil {
			text = result.ContentText
		}

		tp, fn, fp, tn := evaluate(entry, text)
		totalTP += tp
		totalFN += fn
		totalFP += fp
		totalTN += tn
		evaluated++
	}

	p := totalTP / (totalTP + totalFP)
	r := totalTP / (totalTP + totalFN)
	a := (totalTP + totalTN) / (totalTP + totalTN + totalFP + totalFN)
	f := (2 * totalTP) / (2*totalTP + totalFP + totalFN)

	fmt.Printf("Go trafilatura: precision=%.3f  recall=%.3f  accuracy=%.3f  f-score=%.3f  (evaluated=%d, skipped=%d)\n",
		p, r, a, f, evaluated, skipped)
}

func evaluate(entry ComparisonEntry, text string) (tp, fn, fp, tn float64) {
	for _, s := range entry.With {
		if strings.Contains(text, s) {
			tp++
		} else {
			fn++
		}
	}
	for _, s := range entry.Without {
		if strings.Contains(text, s) {
			fp++
		} else {
			tn++
		}
	}
	return
}
