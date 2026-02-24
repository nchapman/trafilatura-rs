// Port of ExtractResult and Metadata from go-trafilatura/core.go and metadata.go

use serde::Serialize;

/// The result of content extraction.
#[derive(Debug, Clone, Default)]
pub struct ExtractResult {
    /// Extracted content as plain text.
    pub content_text: String,

    /// Extracted comments as plain text.
    /// Empty if `ExcludeComments` was set in `Options`.
    pub comments_text: String,

    /// Extracted content as serialized HTML.
    pub content_html: String,

    /// Extracted comments as serialized HTML.
    pub comments_html: String,

    /// Metadata extracted from the document.
    pub metadata: Metadata,
}

/// Metadata extracted from the document via meta tags, JSON-LD, OpenGraph, etc.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Metadata {
    pub title: String,
    pub author: String,
    pub url: String,
    pub hostname: String,
    pub description: String,
    pub sitename: String,
    pub date: Option<chrono::NaiveDate>,
    pub categories: Vec<String>,
    pub tags: Vec<String>,
    pub id: String,
    pub fingerprint: String,
    pub license: String,
    pub language: String,
    pub image: String,
    pub page_type: String,
}
