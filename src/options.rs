// Port of go-trafilatura/core-options.go

/// Controls whether extraction favors precision, recall, or a balance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum ExtractionFocus {
    #[default]
    Balanced,
    FavorRecall,
    FavorPrecision,
}

/// Controls how date extraction behaves.
/// Port of HtmlDateMode in go-trafilatura.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum HtmlDateMode {
    /// Default: use Fast mode (meta + JSON-LD only).
    #[default]
    Default,
    /// Fast: meta elements and JSON-LD only (current behavior).
    Fast,
    /// Extensive: also scan body text for dates.
    ///
    /// **Not yet implemented** — currently behaves identically to [`Fast`](Self::Fast).
    /// The variant is kept to match the Go API; it will gain full behavior
    /// when the `htmldate` integration is ported.
    Extensive,
    /// Disabled: skip date extraction entirely.
    Disabled,
}

/// User-provided fallback content for when main extraction yields too little.
/// Port of FallbackCandidates in go-trafilatura.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct FallbackCandidates {
    /// Pre-extracted HTML string from Readability or similar.
    pub readability_html: Option<String>,
}

/// Advanced tuning parameters for the extraction algorithm.
#[derive(Debug, Clone)]
#[non_exhaustive]
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

impl Config {
    /// Set the LRU cache size for duplicate detection.
    pub fn with_cache_size(mut self, size: usize) -> Self {
        self.cache_size = size;
        self
    }

    /// Set the minimum text length before duplicate checking kicks in.
    pub fn with_min_duplicate_check_size(mut self, size: usize) -> Self {
        self.min_duplicate_check_size = size;
        self
    }

    /// Set the maximum number of times a segment may appear before it is considered duplicate.
    pub fn with_max_duplicate_count(mut self, count: usize) -> Self {
        self.max_duplicate_count = count;
        self
    }

    /// Set the minimum extracted text size (below this, fallback strategies are tried).
    pub fn with_min_extracted_size(mut self, size: usize) -> Self {
        self.min_extracted_size = size;
        self
    }

    /// Set the minimum extracted comment size.
    pub fn with_min_extracted_comment_size(mut self, size: usize) -> Self {
        self.min_extracted_comment_size = size;
        self
    }

    /// Set the minimum output text size (below this, `InsufficientContent` is returned).
    pub fn with_min_output_size(mut self, size: usize) -> Self {
        self.min_output_size = size;
        self
    }

    /// Set the minimum output comment size.
    pub fn with_min_output_comment_size(mut self, size: usize) -> Self {
        self.min_output_comment_size = size;
        self
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
/// let mut opts = Options::default();
/// opts.enable_fallback = true;
/// opts.include_links = true;
/// opts.focus = ExtractionFocus::FavorRecall;
/// ```
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
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
    pub excluded_authors: Vec<String>,
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

// ---------------------------------------------------------------------------
// Builder methods
// ---------------------------------------------------------------------------

impl Options {
    /// Enable or disable readability/baseline fallback extraction.
    pub fn with_fallback(mut self, enable: bool) -> Self {
        self.enable_fallback = enable;
        self
    }

    /// Preserve `<a>` tags (hyperlinks) in output HTML.
    pub fn with_links(mut self, include: bool) -> Self {
        self.include_links = include;
        self
    }

    /// Preserve `<img>` tags in output HTML.
    pub fn with_images(mut self, include: bool) -> Self {
        self.include_images = include;
        self
    }

    /// Set the extraction focus (precision, recall, or balanced).
    pub fn with_focus(mut self, focus: ExtractionFocus) -> Self {
        self.focus = focus;
        self
    }

    /// Skip comment extraction entirely.
    pub fn with_exclude_comments(mut self, exclude: bool) -> Self {
        self.exclude_comments = exclude;
        self
    }

    /// Remove tables from extracted content.
    pub fn with_exclude_tables(mut self, exclude: bool) -> Self {
        self.exclude_tables = exclude;
        self
    }

    /// Set the page's original URL for resolving relative links.
    pub fn with_url(mut self, url: url::Url) -> Self {
        self.original_url = Some(url);
        self
    }

    /// Set the target language (ISO 639-1). Documents not matching are rejected.
    pub fn with_target_language(mut self, lang: impl Into<String>) -> Self {
        self.target_language = Some(lang.into());
        self
    }

    /// Enable cross-document duplicate detection.
    pub fn with_deduplicate(mut self, enable: bool) -> Self {
        self.deduplicate = enable;
        self
    }

    /// Set a CSS selector for elements to prune before extraction.
    pub fn with_prune_selector(mut self, selector: impl Into<String>) -> Self {
        self.prune_selector = Some(selector.into());
        self
    }

    /// Require title, URL, and date in metadata or return an error.
    pub fn with_essential_metadata(mut self, require: bool) -> Self {
        self.has_essential_metadata = require;
        self
    }

    /// Set the maximum number of DOM elements before returning `TreeTooLarge`.
    pub fn with_max_tree_size(mut self, max: usize) -> Self {
        self.max_tree_size = Some(max);
        self
    }

    /// Set the advanced tuning configuration.
    pub fn with_config(mut self, config: Config) -> Self {
        self.config = config;
        self
    }

    /// Set the date extraction mode.
    pub fn with_html_date_mode(mut self, mode: HtmlDateMode) -> Self {
        self.html_date_mode = mode;
        self
    }

    /// Override the extracted date with a fixed value.
    pub fn with_html_date_override(mut self, date: chrono::NaiveDate) -> Self {
        self.html_date_override = Some(date);
        self
    }

    /// Set author names to exclude from metadata results.
    pub fn with_excluded_authors(mut self, authors: Vec<String>) -> Self {
        self.excluded_authors = authors;
        self
    }

    /// Provide fallback candidates for content extraction.
    pub fn with_fallback_candidates(mut self, candidates: FallbackCandidates) -> Self {
        self.fallback_candidates = Some(candidates);
        self
    }

    /// Enable or disable tracing log output.
    pub fn with_log(mut self, enable: bool) -> Self {
        self.enable_log = enable;
        self
    }
}
