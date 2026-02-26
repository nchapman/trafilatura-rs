// Port of go-trafilatura/core-options.go

/// Controls whether extraction favors precision, recall, or a balance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExtractionFocus {
    #[default]
    Balanced,
    FavorRecall,
    FavorPrecision,
}

/// Controls how date extraction behaves.
/// Port of HtmlDateMode in go-trafilatura.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HtmlDateMode {
    /// Default: use Fast mode (meta + JSON-LD only).
    #[default]
    Default,
    /// Fast: meta elements and JSON-LD only (current behavior).
    Fast,
    /// Extensive: also scan body text for dates (not yet implemented, behaves like Fast).
    Extensive,
    /// Disabled: skip date extraction entirely.
    Disabled,
}

/// User-provided fallback content for when main extraction yields too little.
/// Port of FallbackCandidates in go-trafilatura.
#[derive(Debug, Clone, Default)]
pub struct FallbackCandidates {
    /// Pre-extracted HTML string from Readability or similar.
    pub readability_html: Option<String>,
}

/// Advanced tuning parameters for the extraction algorithm.
#[derive(Debug, Clone)]
pub struct Config {
    pub cache_size: usize,
    pub min_duplicate_check_size: usize,
    pub max_duplicate_count: usize,
    pub min_extracted_size: usize,
    pub min_extracted_comment_size: usize,
    pub min_output_size: usize,
    pub min_output_comment_size: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            cache_size: 4096,
            min_duplicate_check_size: 100,
            max_duplicate_count: 2,
            min_extracted_size: 250,
            min_extracted_comment_size: 1,
            min_output_size: 1,
            min_output_comment_size: 1,
        }
    }
}

/// Options for content extraction.
///
/// All fields default to sensible values via [`Default`]. The most commonly
/// adjusted options are [`enable_fallback`](Self::enable_fallback),
/// [`include_links`](Self::include_links), and [`focus`](Self::focus).
///
/// # Example
///
/// ```rust
/// use trafilatura::{Options, ExtractionFocus};
///
/// let opts = Options {
///     enable_fallback: true,
///     include_links: true,
///     focus: ExtractionFocus::FavorRecall,
///     ..Options::default()
/// };
/// ```
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// Advanced tuning parameters (cache size, minimum lengths, etc.).
    pub config: Config,
    /// The page's original URL, used for resolving relative links.
    pub original_url: Option<url::Url>,
    /// If set, reject documents not matching this ISO 639-1 language code.
    pub target_language: Option<String>,
    /// Enable readability/baseline fallback when primary extraction yields too little.
    pub enable_fallback: bool,
    /// Favor precision, recall, or balance in extraction heuristics.
    pub focus: ExtractionFocus,
    /// Skip comment extraction entirely.
    pub exclude_comments: bool,
    /// Remove tables from extracted content.
    pub exclude_tables: bool,
    /// Preserve `<img>` tags in output HTML.
    pub include_images: bool,
    /// Preserve `<a>` tags (hyperlinks) in output HTML.
    pub include_links: bool,
    /// Author names to exclude from metadata results.
    pub blacklisted_authors: Vec<String>,
    /// Enable cross-document duplicate detection via LRU cache.
    pub deduplicate: bool,
    /// Require title, URL, and date in metadata or return an error.
    pub has_essential_metadata: bool,
    /// Maximum number of DOM elements before returning `TreeTooLarge`.
    pub max_tree_size: Option<usize>,
    /// CSS selector for elements to remove before extraction (user-controlled pruning).
    pub prune_selector: Option<String>,
    /// Enable tracing log output.
    pub enable_log: bool,
    /// Controls date extraction behavior.
    pub html_date_mode: HtmlDateMode,
    /// If set, use this date directly instead of extracting from the document.
    pub html_date_override: Option<chrono::NaiveDate>,
    /// User-provided fallback candidates for content extraction.
    pub fallback_candidates: Option<FallbackCandidates>,
}
