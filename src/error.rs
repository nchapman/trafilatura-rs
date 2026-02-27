// Port of error handling from go-trafilatura/core.go

use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum TrafilaturaError {
    #[error("failed to parse HTML: {0}")]
    ParseError(String),

    #[error("wrong language: expected {expected}, got {got}")]
    LanguageMismatch { expected: String, got: String },

    #[error("text and comments not long enough: {text_len} / {comment_len}")]
    InsufficientContent { text_len: usize, comment_len: usize },

    #[error("missing required metadata: {0}")]
    MissingMetadata(String),

    #[error("extracted body is a duplicate")]
    DuplicateContent,

    #[error("output tree too large: {0} elements")]
    TreeTooLarge(usize),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
