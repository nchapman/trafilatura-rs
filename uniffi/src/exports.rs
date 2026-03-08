// Shared UniFFI type definitions and exported functions.
// Included by both the native and WASM crate lib.rs files.

// --- Error ---

/// Errors returned by extraction functions.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum TrafilaturaError {
    #[error("{reason}")]
    ParseError { reason: String },
    #[error("wrong language: expected {expected}, got {got}")]
    LanguageMismatch { expected: String, got: String },
    #[error("{reason}")]
    InsufficientContent { reason: String },
    #[error("{reason}")]
    MissingMetadata { reason: String },
    #[error("extracted body is a duplicate")]
    DuplicateContent,
    #[error("output tree too large: {size} elements")]
    TreeTooLarge { size: i64 },
}

impl From<trafilatura::TrafilaturaError> for TrafilaturaError {
    fn from(e: trafilatura::TrafilaturaError) -> Self {
        match e {
            trafilatura::TrafilaturaError::ParseError(msg) => {
                TrafilaturaError::ParseError { reason: msg }
            }
            trafilatura::TrafilaturaError::LanguageMismatch { expected, got } => {
                TrafilaturaError::LanguageMismatch { expected, got }
            }
            trafilatura::TrafilaturaError::InsufficientContent { .. } => {
                TrafilaturaError::InsufficientContent {
                    reason: e.to_string(),
                }
            }
            trafilatura::TrafilaturaError::MissingMetadata(field) => {
                TrafilaturaError::MissingMetadata { reason: field }
            }
            trafilatura::TrafilaturaError::DuplicateContent => TrafilaturaError::DuplicateContent,
            trafilatura::TrafilaturaError::TreeTooLarge(n) => {
                TrafilaturaError::TreeTooLarge { size: n as i64 }
            }
            // non_exhaustive catch-all
            _ => TrafilaturaError::ParseError {
                reason: e.to_string(),
            },
        }
    }
}

// --- Enums ---

/// Extraction strategy.
#[derive(uniffi::Enum)]
pub enum ExtractionFocus {
    Balanced,
    FavorRecall,
    FavorPrecision,
}

/// Date extraction mode.
#[derive(uniffi::Enum)]
pub enum HtmlDateMode {
    Automatic,
    Fast,
    Extensive,
    Disabled,
}

// --- Records ---

/// Advanced tuning parameters.
#[derive(uniffi::Record)]
pub struct ExtractionConfig {
    pub min_extracted_size: i64,
    pub min_extracted_comment_size: i64,
    pub min_output_size: i64,
    pub min_output_comment_size: i64,
}

/// Extraction options.
#[derive(uniffi::Record)]
pub struct ExtractionOptions {
    pub config: ExtractionConfig,
    /// Page URL as string, or empty.
    pub original_url: Option<String>,
    /// ISO 639-1 language code to enforce.
    pub target_language: Option<String>,
    pub enable_fallback: bool,
    pub focus: ExtractionFocus,
    pub exclude_comments: bool,
    pub exclude_tables: bool,
    pub include_images: bool,
    pub include_links: bool,
    pub deduplicate: bool,
    pub require_essential_metadata: bool,
    pub max_tree_size: Option<i64>,
    /// CSS selector for pre-pruning elements.
    pub prune_selector: Option<String>,
    pub html_date_mode: HtmlDateMode,
    /// Date override as ISO-8601 string (YYYY-MM-DD), or empty.
    pub html_date_override: Option<String>,
}

/// Document metadata extracted from HTML.
#[derive(uniffi::Record)]
pub struct Metadata {
    pub title: String,
    pub author: String,
    pub url: String,
    pub hostname: String,
    pub description: String,
    pub sitename: String,
    /// ISO-8601 date string (YYYY-MM-DD) or empty.
    pub date: Option<String>,
    pub categories: Vec<String>,
    pub tags: Vec<String>,
    pub id: String,
    pub fingerprint: String,
    pub license: String,
    pub language: String,
    pub image: String,
    pub page_type: String,
}

/// Extraction result containing content, comments, and metadata.
#[derive(uniffi::Record)]
pub struct ExtractResult {
    pub content_text: String,
    pub comments_text: String,
    pub content_html: String,
    pub comments_html: String,
    pub metadata: Metadata,
}

// --- Exported functions ---

/// Returns the default extraction options.
#[uniffi::export]
pub fn default_options() -> ExtractionOptions {
    let opts = trafilatura::Options::default();
    to_ffi_options(&opts)
}

/// Returns the default extraction config.
#[uniffi::export]
pub fn default_config() -> ExtractionConfig {
    let config = trafilatura::Config::default();
    to_ffi_config(&config)
}

/// Extract content from HTML with default options.
#[uniffi::export]
pub fn extract_simple(html: String) -> Result<ExtractResult, TrafilaturaError> {
    let opts = trafilatura::Options::default();
    let result = trafilatura::extract(&html, &opts)?;
    Ok(to_ffi_result(result))
}

/// Extract content from HTML with custom options.
#[uniffi::export]
pub fn extract(
    html: String,
    options: ExtractionOptions,
) -> Result<ExtractResult, TrafilaturaError> {
    let opts = to_core_options(&options)?;
    let result = trafilatura::extract(&html, &opts)?;
    Ok(to_ffi_result(result))
}

/// Create a self-contained HTML document from an extraction result.
#[uniffi::export]
pub fn create_readable_document(result: ExtractResult) -> String {
    let core_result = to_core_result(&result);
    trafilatura::create_readable_document(&core_result)
}

// --- Internal conversion helpers ---

fn to_ffi_config(c: &trafilatura::Config) -> ExtractionConfig {
    ExtractionConfig {
        min_extracted_size: c.min_extracted_size as i64,
        min_extracted_comment_size: c.min_extracted_comment_size as i64,
        min_output_size: c.min_output_size as i64,
        min_output_comment_size: c.min_output_comment_size as i64,
    }
}

fn to_ffi_options(opts: &trafilatura::Options) -> ExtractionOptions {
    ExtractionOptions {
        config: to_ffi_config(&opts.config),
        original_url: opts.original_url.as_ref().map(|u| u.to_string()),
        target_language: opts.target_language.clone(),
        enable_fallback: opts.enable_fallback,
        focus: match opts.focus {
            trafilatura::ExtractionFocus::Balanced => ExtractionFocus::Balanced,
            trafilatura::ExtractionFocus::FavorRecall => ExtractionFocus::FavorRecall,
            trafilatura::ExtractionFocus::FavorPrecision => ExtractionFocus::FavorPrecision,
            // New upstream variants fall back to Balanced; update when core adds variants.
            _ => ExtractionFocus::Balanced,
        },
        exclude_comments: opts.exclude_comments,
        exclude_tables: opts.exclude_tables,
        include_images: opts.include_images,
        include_links: opts.include_links,
        deduplicate: opts.deduplicate,
        require_essential_metadata: opts.has_essential_metadata,
        max_tree_size: opts.max_tree_size.map(|n| n as i64),
        prune_selector: opts.prune_selector.clone(),
        html_date_mode: match opts.html_date_mode {
            trafilatura::HtmlDateMode::Default => HtmlDateMode::Automatic,
            trafilatura::HtmlDateMode::Fast => HtmlDateMode::Fast,
            trafilatura::HtmlDateMode::Extensive => HtmlDateMode::Extensive,
            trafilatura::HtmlDateMode::Disabled => HtmlDateMode::Disabled,
            // New upstream variants fall back to Default; update when core adds variants.
            _ => HtmlDateMode::Automatic,
        },
        html_date_override: opts.html_date_override.map(|d| d.to_string()),
    }
}

fn to_core_options(
    opts: &ExtractionOptions,
) -> Result<trafilatura::Options, TrafilaturaError> {
    let mut core = trafilatura::Options::default();

    core.config = trafilatura::Config::default()
        .with_min_extracted_size(opts.config.min_extracted_size.max(0) as usize)
        .with_min_extracted_comment_size(opts.config.min_extracted_comment_size.max(0) as usize)
        .with_min_output_size(opts.config.min_output_size.max(0) as usize)
        .with_min_output_comment_size(opts.config.min_output_comment_size.max(0) as usize);

    if let Some(ref url_str) = opts.original_url {
        core.original_url = Some(
            url::Url::parse(url_str).map_err(|e| TrafilaturaError::ParseError {
                reason: format!("invalid URL '{url_str}': {e}"),
            })?,
        );
    }

    core.target_language = opts.target_language.clone();
    core.enable_fallback = opts.enable_fallback;
    core.focus = match opts.focus {
        ExtractionFocus::Balanced => trafilatura::ExtractionFocus::Balanced,
        ExtractionFocus::FavorRecall => trafilatura::ExtractionFocus::FavorRecall,
        ExtractionFocus::FavorPrecision => trafilatura::ExtractionFocus::FavorPrecision,
    };
    core.exclude_comments = opts.exclude_comments;
    core.exclude_tables = opts.exclude_tables;
    core.include_images = opts.include_images;
    core.include_links = opts.include_links;
    core.deduplicate = opts.deduplicate;
    core.has_essential_metadata = opts.require_essential_metadata;
    core.max_tree_size = opts.max_tree_size.map(|n| n.max(0) as usize);
    core.prune_selector = opts.prune_selector.clone();
    core.html_date_mode = match opts.html_date_mode {
        HtmlDateMode::Automatic => trafilatura::HtmlDateMode::Default,
        HtmlDateMode::Fast => trafilatura::HtmlDateMode::Fast,
        HtmlDateMode::Extensive => trafilatura::HtmlDateMode::Extensive,
        HtmlDateMode::Disabled => trafilatura::HtmlDateMode::Disabled,
    };

    if let Some(ref date_str) = opts.html_date_override {
        core.html_date_override = Some(
            chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d").map_err(|e| {
                TrafilaturaError::ParseError {
                    reason: format!("invalid date '{date_str}': {e}"),
                }
            })?,
        );
    }

    Ok(core)
}

fn to_ffi_metadata(m: &trafilatura::Metadata) -> Metadata {
    Metadata {
        title: m.title.clone(),
        author: m.author.clone(),
        url: m.url.clone(),
        hostname: m.hostname.clone(),
        description: m.description.clone(),
        sitename: m.sitename.clone(),
        date: m.date.map(|d| d.to_string()),
        categories: m.categories.clone(),
        tags: m.tags.clone(),
        id: m.id.clone(),
        fingerprint: m.fingerprint.clone(),
        license: m.license.clone(),
        language: m.language.clone(),
        image: m.image.clone(),
        page_type: m.page_type.clone(),
    }
}

fn to_ffi_result(r: trafilatura::ExtractResult) -> ExtractResult {
    ExtractResult {
        content_text: r.content_text,
        comments_text: r.comments_text,
        content_html: r.content_html,
        comments_html: r.comments_html,
        metadata: to_ffi_metadata(&r.metadata),
    }
}

fn to_core_metadata(m: &Metadata) -> trafilatura::Metadata {
    let mut core = trafilatura::Metadata::default();
    core.title = m.title.clone();
    core.author = m.author.clone();
    core.url = m.url.clone();
    core.hostname = m.hostname.clone();
    core.description = m.description.clone();
    core.sitename = m.sitename.clone();
    // Date strings that don't conform to YYYY-MM-DD are silently ignored here.
    // create_readable_document is best-effort; the date field is cosmetic only.
    core.date = m.date.as_ref().and_then(|d| {
        chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").ok()
    });
    core.categories = m.categories.clone();
    core.tags = m.tags.clone();
    core.id = m.id.clone();
    core.fingerprint = m.fingerprint.clone();
    core.license = m.license.clone();
    core.language = m.language.clone();
    core.image = m.image.clone();
    core.page_type = m.page_type.clone();
    core
}

fn to_core_result(r: &ExtractResult) -> trafilatura::ExtractResult {
    let mut core = trafilatura::ExtractResult::default();
    core.content_text = r.content_text.clone();
    core.comments_text = r.comments_text.clone();
    core.content_html = r.content_html.clone();
    core.comments_html = r.comments_html.clone();
    core.metadata = to_core_metadata(&r.metadata);
    core
}
