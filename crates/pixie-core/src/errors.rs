use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum CoreError {
  #[error("Validation error: {0}")]
  Validation(String),

  #[error("Dimension limit exceeded: {0}")]
  DimensionLimit(String),

  #[error("Memory limit exceeded: {0}")]
  MemoryLimit(String),

  #[error("Decoding failed: {0}")]
  Decode(String),

  #[error("Encoding failed: {0}")]
  Encode(String),

  #[error("Format error: {0}")]
  Format(String),

  #[error("Metadata processing failed: {0}")]
  Metadata(String),

  #[error("Operation cancelled")]
  Cancelled,

  #[error("Internal application error: {0}")]
  Internal(String),
}

pub type Result<T> = std::result::Result<T, CoreError>;
