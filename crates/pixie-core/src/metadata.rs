#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MetadataPolicy {
  #[serde(default)]
  pub remove_all_metadata: bool,
  pub remove_gps: bool,
  pub remove_camera_info: bool,
  pub remove_comments: bool,
  pub remove_timestamps: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MetadataValueEdit {
  Keep,
  Set(String),
  Clear,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MetadataEdits {
  pub image_description: MetadataValueEdit,
  pub artist: MetadataValueEdit,
  pub copyright: MetadataValueEdit,
  pub software: MetadataValueEdit,
}

impl Default for MetadataEdits {
  fn default() -> Self {
    Self {
      image_description: MetadataValueEdit::Keep,
      artist: MetadataValueEdit::Keep,
      copyright: MetadataValueEdit::Keep,
      software: MetadataValueEdit::Keep,
    }
  }
}

impl MetadataEdits {
  pub fn is_noop(&self) -> bool {
    self.image_description == MetadataValueEdit::Keep
      && self.artist == MetadataValueEdit::Keep
      && self.copyright == MetadataValueEdit::Keep
      && self.software == MetadataValueEdit::Keep
  }
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

#[derive(Debug, Clone, Default)]
pub struct MetadataFields {
  pub image_description: Option<String>,
  pub artist: Option<String>,
  pub copyright: Option<String>,
  pub software: Option<String>,
}

/// Read the current string metadata fields from an image file.
/// Returns None for any field not present in the file.
pub fn read_image_metadata_fields(path: &std::path::Path) -> MetadataFields {
  let metadata = match little_exif::metadata::Metadata::new_from_path(path) {
    Ok(m) => m,
    Err(_) => return MetadataFields::default(),
  };

  let mut fields = MetadataFields::default();
  for tag in &metadata {
    let debug_str = format!("{:?}", tag);
    let extract = |prefix: &str| -> Option<String> {
      if debug_str.starts_with(prefix) {
        // The Debug representation is like `Prefix("value")`, extract the inner string.
        let inner = debug_str
          .trim_start_matches(prefix)
          .trim_start_matches('(')
          .trim_end_matches(')');
        // Strip surrounding quotes if present.
        let s = inner.trim_matches('"');
        if s.is_empty() {
          None
        } else {
          Some(s.to_owned())
        }
      } else {
        None
      }
    };
    if let Some(v) = extract("ImageDescription") {
      fields.image_description = Some(v);
    } else if let Some(v) = extract("Artist") {
      fields.artist = Some(v);
    } else if let Some(v) = extract("Copyright") {
      fields.copyright = Some(v);
    } else if let Some(v) = extract("Software") {
      fields.software = Some(v);
    }
  }
  fields
}

/// Apply user-requested metadata edits directly on the exported target image.
pub fn apply_metadata_edits(
  target_path: &std::path::Path,
  edits: &MetadataEdits,
) -> Result<(), String> {
  if edits.is_noop() {
    return Ok(());
  }

  let mut metadata = little_exif::metadata::Metadata::new_from_path(target_path)
    .unwrap_or_else(|_| little_exif::metadata::Metadata::new());

  let apply_str_edit = |metadata: &mut little_exif::metadata::Metadata,
                        edit: &MetadataValueEdit,
                        make_tag: fn(String) -> little_exif::exif_tag::ExifTag,
                        prefix: &str| {
    match edit {
      MetadataValueEdit::Keep => {}
      MetadataValueEdit::Set(value) => {
        metadata.set_tag(make_tag(value.clone()));
      }
      MetadataValueEdit::Clear => {
        let p = prefix.to_owned();
        let tags_to_remove: Vec<_> = (&*metadata)
          .into_iter()
          .filter(|tag| format!("{:?}", tag).starts_with(&p))
          .cloned()
          .collect();
        for tag in tags_to_remove {
          metadata.remove_tag(tag);
        }
      }
    }
  };

  apply_str_edit(
    &mut metadata,
    &edits.image_description,
    little_exif::exif_tag::ExifTag::ImageDescription,
    "ImageDescription",
  );
  apply_str_edit(
    &mut metadata,
    &edits.artist,
    little_exif::exif_tag::ExifTag::Artist,
    "Artist",
  );
  apply_str_edit(
    &mut metadata,
    &edits.copyright,
    little_exif::exif_tag::ExifTag::Copyright,
    "Copyright",
  );
  apply_str_edit(
    &mut metadata,
    &edits.software,
    little_exif::exif_tag::ExifTag::Software,
    "Software",
  );

  metadata
    .write_to_file(target_path)
    .map_err(|e| format!("Failed to write edited metadata to target image: {e}"))?;

  Ok(())
}
