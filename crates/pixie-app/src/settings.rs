use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct AppSettings {
  pub dark_theme: bool,
  pub save_directory: Option<String>,
}

impl Default for AppSettings {
  fn default() -> Self {
    Self {
      dark_theme: true,
      save_directory: None,
    }
  }
}
