// Go benchmark for comparing trafilatura extraction speed against Rust.
//
// Usage:
//   cd /path/to/go-trafilatura
//   go run /path/to/this/bench_go.go -corpus /path/to/trafilatura-rs/comparison-data/entries.json \
//       -testfiles /path/to/trafilatura-rs/test-files
//
// Runs the same 960-entry corpus through go-trafilatura and reports timing.

package main

import (
	"encoding/json"
	"flag"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"time"

	trafilatura "github.com/markusmobius/go-trafilatura"
)

type ComparisonEntry struct {
	File string `json:"file"`
	URL  string `json:"url"`
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
	iterations := flag.Int("n", 3, "Number of iterations for corpus benchmark")
	singleFile := flag.String("single", "", "Benchmark a single file instead of corpus")
	singleN := flag.Int("single-n", 100, "Number of iterations for single-file benchmark")
	noFallback := flag.Bool("no-fallback", false, "Disable fallback for single-file mode")
	flag.Parse()

	if *corpusPath == "" || *testFilesDir == "" {
		fmt.Fprintln(os.Stderr, "Usage: go run bench_go.go -corpus <entries.json> -testfiles <test-files-dir>")
		os.Exit(1)
	}

	// Single-file benchmark mode
	if *singleFile != "" {
		html, ok := loadHTML(*singleFile, *testFilesDir)
		if !ok {
			fmt.Fprintf(os.Stderr, "File not found: %s\n", *singleFile)
			os.Exit(1)
		}
		fmt.Printf("Single file: %s (%d bytes, fallback=%t)\n", *singleFile, len(html), !*noFallback)
		fmt.Printf("Running %d iterations...\n", *singleN)

		opts := trafilatura.Options{
			EnableFallback: !*noFallback,
			Focus:          trafilatura.Balanced,
			ExcludeComments: true,
			ExcludeTables:  false,
		}

		// Warmup
		for i := 0; i < 5; i++ {
			trafilatura.Extract(strings.NewReader(html), opts)
		}

		start := time.Now()
		for i := 0; i < *singleN; i++ {
			trafilatura.Extract(strings.NewReader(html), opts)
		}
		elapsed := time.Since(start)
		perIter := elapsed / time.Duration(*singleN)
		throughput := float64(len(html)) * float64(*singleN) / elapsed.Seconds() / 1024 / 1024
		fmt.Printf("Total: %v  Per iteration: %v  Throughput: %.1f MiB/s\n", elapsed, perIter, throughput)
		return
	}

	// Load corpus
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

	// Load all HTML files
	var corpus []string
	var totalBytes int64
	for _, e := range entries {
		if html, ok := loadHTML(e.File, *testFilesDir); ok {
			corpus = append(corpus, html)
			totalBytes += int64(len(html))
		}
	}
	fmt.Printf("Loaded %d documents (%d bytes total)\n", len(corpus), totalBytes)

	opts := trafilatura.Options{
		EnableFallback: true,
		Focus:          trafilatura.Balanced,
		ExcludeComments: true,
		ExcludeTables:  false,
	}

	// Warmup
	fmt.Println("Warming up...")
	for _, html := range corpus[:min(10, len(corpus))] {
		trafilatura.Extract(strings.NewReader(html), opts)
	}

	// Corpus benchmark
	fmt.Printf("Running %d iterations over %d documents...\n", *iterations, len(corpus))
	var times []time.Duration
	for iter := 0; iter < *iterations; iter++ {
		start := time.Now()
		for _, html := range corpus {
			trafilatura.Extract(strings.NewReader(html), opts)
		}
		elapsed := time.Since(start)
		times = append(times, elapsed)
		throughput := float64(totalBytes) / elapsed.Seconds() / 1024 / 1024
		fmt.Printf("  Iteration %d: %v (%.1f MiB/s)\n", iter+1, elapsed, throughput)
	}

	// Summary
	var total time.Duration
	best := times[0]
	for _, t := range times {
		total += t
		if t < best {
			best = t
		}
	}
	avg := total / time.Duration(len(times))
	avgThroughput := float64(totalBytes) / avg.Seconds() / 1024 / 1024
	bestThroughput := float64(totalBytes) / best.Seconds() / 1024 / 1024
	fmt.Printf("\nCorpus: %d docs, %d bytes\n", len(corpus), totalBytes)
	fmt.Printf("Average: %v (%.1f MiB/s)\n", avg, avgThroughput)
	fmt.Printf("Best:    %v (%.1f MiB/s)\n", best, bestThroughput)

	// Single-document benchmarks for the same files as Rust
	singlePages := []struct {
		label string
		file  string
	}{
		{"small  (6KB)", "die-partei.net.luebeck.html"},
		{"medium (85KB)", "zeit.de.zugverkehr.html"},
		{"large  (382KB)", "reuters.com.parasite.html"},
		{"xlarge (906KB)", "pcgamer.com.skyrim.html"},
	}

	fmt.Println("\n--- Single document benchmarks ---")
	for _, p := range singlePages {
		html, ok := loadHTML(p.file, *testFilesDir)
		if !ok {
			fmt.Printf("  %s: file not found\n", p.label)
			continue
		}

		n := 50
		// Warmup
		for i := 0; i < 3; i++ {
			trafilatura.Extract(strings.NewReader(html), opts)
		}

		start := time.Now()
		for i := 0; i < n; i++ {
			trafilatura.Extract(strings.NewReader(html), opts)
		}
		elapsed := time.Since(start)
		perIter := elapsed / time.Duration(n)
		throughput := float64(len(html)) * float64(n) / elapsed.Seconds() / 1024 / 1024
		fmt.Printf("  %-20s %6d bytes  %10v/iter  %.1f MiB/s\n", p.label, len(html), perIter, throughput)
	}
}
