use thiserror::Error;

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum AppError {
  #[error("Core Error: {0}")]
  Core(#[from] pixie_core::CoreError),

  #[error("IO Error: {0}")]
  Io(#[from] std::io::Error),

  #[error("GTK/GLib Error: {0}")]
  Glib(#[from] glib::Error),

  #[error("File open error: {0}")]
  FileOpen(String),

  #[error("Task channel error: {0}")]
  Channel(String),
}

#[allow(dead_code)]
pub type Result<T> = std::result::Result<T, AppError>;
