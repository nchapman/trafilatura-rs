// Port of go-trafilatura/cmd/go-trafilatura/main.go and output.go

use std::io::{self, Read, Write};

use clap::{Parser, ValueEnum};
use serde::Serialize;
use trafilatura::{
    create_readable_document, extract, ExtractResult, ExtractionFocus, Metadata, Options,
};

const DEFAULT_USER_AGENT: &str =
    "Mozilla/5.0 (X11; Linux x86_64; rv:88.0) Gecko/20100101 Firefox/88.0";

// ---------------------------------------------------------------------------
// CLI definition
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[command(
    name = "trafilatura",
    about = "Extract readable content from a HTML file or URL",
    long_about = "Extract readable content, comments and metadata from a specified source \
                  which can be either a HTML file or URL. Reads from stdin when no source \
                  is given."
)]
struct Cli {
    /// HTML file path or URL to extract from (reads stdin if omitted)
    source: Option<String>,

    /// Output format
    #[arg(short, long, value_enum, default_value = "html")]
    format: OutputFormat,

    /// Target language filter (ISO 639-1 code, e.g. "en")
    #[arg(short, long)]
    language: Option<String>,

    /// Disable fallback extraction using readability
    #[arg(long)]
    no_fallback: bool,

    /// Exclude comments from extraction result
    #[arg(long)]
    no_comments: bool,

    /// Exclude tables from extraction result
    #[arg(long)]
    no_tables: bool,

    /// Include images in extraction result
    #[arg(long)]
    images: bool,

    /// Keep hyperlinks in extraction result
    #[arg(long)]
    links: bool,

    /// Favor precision over recall
    #[arg(long)]
    precision: bool,

    /// Favor recall over precision
    #[arg(long)]
    recall: bool,

    /// Filter out duplicate segments and sections
    #[arg(long)]
    deduplicate: bool,

    /// Only output documents that have title, URL and date
    #[arg(long)]
    has_metadata: bool,

    /// Enable verbose log output
    #[arg(short, long)]
    verbose: bool,

    /// Timeout for downloading web pages (seconds)
    #[arg(short, long, default_value = "30")]
    timeout: u64,

    /// Skip TLS certificate verification
    #[arg(long)]
    skip_tls: bool,

    /// Custom user-agent string
    #[arg(short, long, default_value = DEFAULT_USER_AGENT)]
    user_agent: String,
}

#[derive(ValueEnum, Clone)]
enum OutputFormat {
    Html,
    Txt,
    Json,
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() {
    let cli = Cli::parse();

    if cli.verbose {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .with_writer(std::io::stderr)
            .init();
    }

    let opts = build_options(&cli);

    let result = match cli.source.as_deref() {
        Some(src) if std::path::Path::new(src).exists() => process_file(src, opts),
        Some(src) if is_valid_url(src) => {
            let url = url::Url::parse(src).expect("already validated");
            process_url(&cli, url, opts)
        }
        Some(src) => {
            eprintln!("error: '{}' is neither a file nor a valid URL", src);
            std::process::exit(1);
        }
        None => process_stdin(opts),
    };

    let result = match result {
        Ok(r) if !r.content_text.is_empty() => r,
        Ok(_) => {
            eprintln!("error: no readable content found");
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    };

    if let Err(e) = write_output(&mut io::stdout(), &result, &cli.format) {
        eprintln!("error writing output: {}", e);
        std::process::exit(1);
    }
}

// ---------------------------------------------------------------------------
// Input processing — port of processFile / processURL in main.go
// ---------------------------------------------------------------------------

fn process_file(path: &str, opts: Options) -> Result<ExtractResult, String> {
    let html =
        std::fs::read_to_string(path).map_err(|e| format!("cannot read '{}': {}", path, e))?;
    extract(&html, &opts).map_err(|e| e.to_string())
}

fn process_url(cli: &Cli, url: url::Url, mut opts: Options) -> Result<ExtractResult, String> {
    if cli.verbose {
        eprintln!("downloading {:?}", url.as_str());
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(cli.timeout))
        .danger_accept_invalid_certs(cli.skip_tls)
        .user_agent(&cli.user_agent)
        .build()
        .map_err(|e| format!("failed to build HTTP client: {}", e))?;

    let resp = client
        .get(url.as_str())
        .send()
        .map_err(|e| format!("failed to download '{}': {}", url, e))?;

    let content_type = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_owned();

    if !content_type.contains("text/html") {
        return Err(format!("page is not HTML: \"{}\"", content_type));
    }

    let html = resp
        .text()
        .map_err(|e| format!("failed to read response body: {}", e))?;

    opts.original_url = Some(url);
    extract(&html, &opts).map_err(|e| e.to_string())
}

fn process_stdin(opts: Options) -> Result<ExtractResult, String> {
    let mut html = String::new();
    io::stdin()
        .read_to_string(&mut html)
        .map_err(|e| format!("failed to read stdin: {}", e))?;
    extract(&html, &opts).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Options — port of createExtractorOptions in main.go
// ---------------------------------------------------------------------------

fn build_options(cli: &Cli) -> Options {
    let focus = if cli.precision {
        ExtractionFocus::FavorPrecision
    } else if cli.recall {
        ExtractionFocus::FavorRecall
    } else {
        ExtractionFocus::Balanced
    };

    let mut o = Options::default();
    o.enable_fallback = !cli.no_fallback;
    o.target_language = cli.language.clone();
    o.exclude_comments = cli.no_comments;
    o.exclude_tables = cli.no_tables;
    o.include_images = cli.images;
    o.include_links = cli.links;
    o.focus = focus;
    o.deduplicate = cli.deduplicate;
    o.has_essential_metadata = cli.has_metadata;
    o
}

// ---------------------------------------------------------------------------
// Output formatting — port of output.go
// ---------------------------------------------------------------------------

fn write_output(
    w: &mut dyn Write,
    result: &ExtractResult,
    format: &OutputFormat,
) -> io::Result<()> {
    match format {
        OutputFormat::Html => write_html(w, result),
        OutputFormat::Txt => write_text(w, result),
        OutputFormat::Json => write_json(w, result),
    }
}

/// Port of writeHTML in output.go.
fn write_html(w: &mut dyn Write, result: &ExtractResult) -> io::Result<()> {
    let html = create_readable_document(result);
    writeln!(w, "{}", html)
}

/// Port of writeText in output.go.
/// Note: the Go source has a bug — it appends ContentText twice instead of
/// CommentsText. This port fixes that.
fn write_text(w: &mut dyn Write, result: &ExtractResult) -> io::Result<()> {
    let mut out = result.content_text.clone();
    if !result.comments_text.is_empty() {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(&result.comments_text);
    }
    if !out.is_empty() {
        out.push('\n');
    }
    w.write_all(out.as_bytes())
}

/// Port of writeJSON in output.go.
fn write_json(w: &mut dyn Write, result: &ExtractResult) -> io::Result<()> {
    let output = JsonOutput::from(result);
    serde_json::to_writer(&mut *w, &output).map_err(io::Error::other)?;
    writeln!(w)
}

// ---------------------------------------------------------------------------
// JSON output type — port of jsonExtractResult in output.go
// ---------------------------------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct JsonOutput<'a> {
    content_html: &'a str,
    content_text: &'a str,
    #[serde(skip_serializing_if = "str::is_empty")]
    comments_html: &'a str,
    #[serde(skip_serializing_if = "str::is_empty")]
    comments_text: &'a str,
    metadata: &'a Metadata,
}

impl<'a> From<&'a ExtractResult> for JsonOutput<'a> {
    fn from(r: &'a ExtractResult) -> Self {
        Self {
            content_html: &r.content_html,
            content_text: &r.content_text,
            comments_html: &r.comments_html,
            comments_text: &r.comments_text,
            metadata: &r.metadata,
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn is_valid_url(s: &str) -> bool {
    match url::Url::parse(s) {
        Ok(u) => u.scheme() == "http" || u.scheme() == "https",
        Err(_) => false,
    }
}
