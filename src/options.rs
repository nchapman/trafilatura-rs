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
#[derive(Debug, Clone, Default)]
pub struct Options {
    pub config: Config,
    pub original_url: Option<url::Url>,
    pub target_language: Option<String>,
    pub enable_fallback: bool,
    pub focus: ExtractionFocus,
    pub exclude_comments: bool,
    pub exclude_tables: bool,
    pub include_images: bool,
    pub include_links: bool,
    pub blacklisted_authors: Vec<String>,
    pub deduplicate: bool,
    pub has_essential_metadata: bool,
    pub max_tree_size: Option<usize>,
    pub prune_selector: Option<String>,
    pub enable_log: bool,
    /// Controls date extraction behavior.
    pub html_date_mode: HtmlDateMode,
    /// If set, use this date directly instead of extracting.
    pub html_date_override: Option<chrono::NaiveDate>,
    /// User-provided fallback candidates for content extraction.
    pub fallback_candidates: Option<FallbackCandidates>,
}
