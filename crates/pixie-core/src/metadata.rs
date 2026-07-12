#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MetadataPolicy {
  #[serde(default)]
  pub remove_all_metadata: bool,
  pub remove_gps: bool,
  pub remove_camera_info: bool,
  pub remove_comments: bool,
  pub remove_timestamps: bool,
}

impl MetadataPolicy {
  pub const fn strip_all() -> Self {
    Self {
      remove_all_metadata: true,
      remove_gps: true,
      remove_camera_info: true,
      remove_comments: true,
      remove_timestamps: true,
    }
  }

  pub const fn preserve_all() -> Self {
    Self {
      remove_all_metadata: false,
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
      remove_all_metadata: false,
      remove_gps: true,
      remove_camera_info: true,
      remove_comments: false,
      remove_timestamps: false,
    }
  }
}

/// Clean metadata from the target image based on the original image's metadata and the policy.
pub fn clean_image_metadata(
  original_path: &std::path::Path,
  target_path: &std::path::Path,
  policy: &MetadataPolicy,
) -> Result<(), String> {
  if policy.remove_all_metadata {
    // Encoding starts from image pixels only, so skipping metadata transfer
    // ensures the output keeps no original EXIF/XMP/IPTC blocks.
    return Ok(());
  }

  if !original_path.exists() {
    return Ok(());
  }

  // Load metadata from the original image file
  let mut metadata = match little_exif::metadata::Metadata::new_from_path(original_path) {
    Ok(m) => m,
    Err(_) => return Ok(()),
  };

  // Collect tags to remove using the public IntoIterator implementation
  let mut tags_to_remove = Vec::new();
  let camera_prefixes = [
    "Make",
    "Model",
    "Software",
    "Lens",
    "BodySerialNumber",
    "CameraSerialNumber",
    "MakerNote",
    "ExifVersion",
    "Exposure",
    "FNumber",
    "ISOSpeed",
    "PhotographicSensitivity",
    "ShutterSpeed",
    "ApertureValue",
    "FocalLength",
    "Flash",
    "WhiteBalance",
    "MeteringMode",
    "ColorSpace",
  ];
  let timestamp_prefixes = [
    "DateTime",
    "CreateDate",
    "ModifyDate",
    "OffsetTime",
    "SubSecTime",
  ];

  for tag in &metadata {
    let debug_str = format!("{:?}", tag);
    let mut remove = false;

    if policy.remove_gps && debug_str.starts_with("GPS") {
      remove = true;
    }

    if policy.remove_camera_info
      && camera_prefixes
        .iter()
        .any(|prefix| debug_str.starts_with(prefix))
    {
      remove = true;
    }

    if policy.remove_timestamps
      && timestamp_prefixes
        .iter()
        .any(|prefix| debug_str.starts_with(prefix))
    {
      remove = true;
    }

    if policy.remove_comments
      && (debug_str.starts_with("UserComment")
        || debug_str.starts_with("ImageDescription")
        || debug_str.starts_with("DocumentName"))
    {
      remove = true;
    }

    if remove {
      tags_to_remove.push(tag.clone());
    }
  }

  // If the original had no tags, we don't need to do anything
  if (&metadata).into_iter().next().is_none() {
    return Ok(());
  }

  // Remove the selected tags
  for tag in tags_to_remove {
    metadata.remove_tag(tag);
  }

  // Write the filtered metadata back to the target file
  metadata
    .write_to_file(target_path)
    .map_err(|e| format!("Failed to write metadata to target image: {e}"))?;

  Ok(())
}
