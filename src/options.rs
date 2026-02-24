// Port of go-trafilatura/core-options.go

/// Controls whether extraction favors precision, recall, or a balance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExtractionFocus {
    #[default]
    Balanced,
    FavorRecall,
    FavorPrecision,
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
}
