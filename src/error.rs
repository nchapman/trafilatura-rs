// Port of error handling from go-trafilatura/core.go

use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum TrafilaturaError {
    #[error("failed to parse HTML: {0}")]
    ParseError(String),

    #[error("wrong language: expected {expected}, got {got}")]
    LanguageMismatch { expected: String, got: String },

    #[error("insufficient content: text {text_len} < {min_output_size} and comments {comment_len} < {min_output_comment_size}")]
    InsufficientContent {
        text_len: usize,
        comment_len: usize,
        min_output_size: usize,
        min_output_comment_size: usize,
    },

    #[error("missing required metadata: {0}")]
    MissingMetadata(String),

    #[error("extracted body is a duplicate")]
    DuplicateContent,

    #[error("output tree too large: {0} elements")]
    TreeTooLarge(usize),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
