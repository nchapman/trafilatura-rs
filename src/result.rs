// Port of ExtractResult and Metadata from go-trafilatura/core.go and metadata.go

use serde::Serialize;

/// The result of content extraction.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
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

#[cfg(feature = "markdown")]
use html2markdown::Options as MarkdownOptions;

#[cfg(feature = "markdown")]
impl ExtractResult {
    /// Convert `content_html` to Markdown using default options.
    pub fn content_markdown(&self) -> String {
        html2markdown::convert(&self.content_html)
    }

    /// Convert `content_html` to Markdown with custom options.
    pub fn content_markdown_with(&self, options: &MarkdownOptions) -> String {
        html2markdown::convert_with(&self.content_html, options)
    }

    /// Convert `comments_html` to Markdown using default options.
    pub fn comments_markdown(&self) -> String {
        html2markdown::convert(&self.comments_html)
    }

    /// Convert `comments_html` to Markdown with custom options.
    pub fn comments_markdown_with(&self, options: &MarkdownOptions) -> String {
        html2markdown::convert_with(&self.comments_html, options)
    }
}

/// Metadata extracted from the document via meta tags, JSON-LD, OpenGraph, etc.
#[derive(Debug, Clone, Default, Serialize)]
#[non_exhaustive]
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

#[cfg(test)]
#[cfg(feature = "markdown")]
mod markdown_tests {
    use super::*;

    #[test]
    fn test_content_markdown() {
        let result = ExtractResult {
            content_html: "<h1>Title</h1><p>Hello <strong>world</strong></p>".into(),
            ..Default::default()
        };
        let md = result.content_markdown();
        assert!(md.contains("# Title"));
        assert!(md.contains("**world**"));
    }

    #[test]
    fn test_comments_markdown() {
        let result = ExtractResult {
            comments_html: "<p>A <em>great</em> comment</p>".into(),
            ..Default::default()
        };
        let md = result.comments_markdown();
        assert!(md.contains("*great*"));
    }

    #[test]
    fn test_empty_html_produces_empty_markdown() {
        let result = ExtractResult::default();
        assert_eq!(result.content_markdown(), "");
        assert_eq!(result.comments_markdown(), "");
    }

    #[test]
    fn test_content_markdown_with_custom_options() {
        let result = ExtractResult {
            content_html: "<ul><li>one</li><li>two</li></ul>".into(),
            ..Default::default()
        };
        let opts = html2markdown::Options::new().with_bullet('-');
        let md = result.content_markdown_with(&opts);
        assert!(md.contains("- one"));
        assert!(md.contains("- two"));
    }
}
