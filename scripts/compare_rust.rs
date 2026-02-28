#!/usr/bin/env -S cargo +nightly -Zscript
//! Run the comparison dataset through Rust trafilatura and output per-entry results.
//!
//! Usage: cargo test --test comparison_detail -- --nocapture > comparison-data/rust_results.jsonl
//!
//! This is meant to be run as a test (see below), not as a standalone binary,
//! because it needs access to the trafilatura crate.

// This file documents the approach. The actual runner is in tests/comparison_detail_test.rs.
