#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MetadataPolicy {
  pub remove_gps: bool,
  pub remove_camera_info: bool,
  pub remove_comments: bool,
  pub remove_timestamps: bool,
}

impl MetadataPolicy {
  pub const fn strip_all() -> Self {
    Self {
      remove_gps: true,
      remove_camera_info: true,
      remove_comments: true,
      remove_timestamps: true,
    }
  }

  pub const fn preserve_all() -> Self {
    Self {
      remove_gps: false,
      remove_camera_info: false,
      remove_comments: false,
      remove_timestamps: false,
    }
  }
}

impl Default for MetadataPolicy {
  fn default() -> Self {
    // Default is privacy-first: strip GPS and camera info, preserve other basic metadata if possible
    Self {
      remove_gps: true,
      remove_camera_info: true,
      remove_comments: false,
      remove_timestamps: false,
    }
  }
}
