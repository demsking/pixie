use little_exif::exif_tag::ExifTag;
use little_exif::metadata::Metadata;
use pixie_core::{
  MetadataEdits, MetadataPolicy, MetadataValueEdit, apply_metadata_edits, clean_image_metadata,
};

#[test]
fn test_metadata_policy_cleaning() -> Result<(), Box<dyn std::error::Error>> {
  // Create a valid PNG source file.
  let original_file = tempfile::Builder::new().suffix(".png").tempfile()?;
  let source_image = image::RgbaImage::from_pixel(2, 2, image::Rgba([255, 0, 0, 255]));
  source_image.save_with_format(original_file.path(), image::ImageFormat::Png)?;

  // Create and set tags
  let mut metadata = Metadata::new();
  metadata.set_tag(ExifTag::GPSLatitude(vec![
    37.0.into(),
    45.0.into(),
    10.0.into(),
  ]));
  metadata.set_tag(ExifTag::Model("PixieCamera".to_string()));
  metadata.set_tag(ExifTag::ImageDescription("My Comment".to_string()));
  metadata.set_tag(ExifTag::DateTimeOriginal("2026:07:12 12:00:00".to_string()));

  // Write metadata to original file
  metadata.write_to_file(original_file.path())?;

  // Verify metadata is written
  let loaded_meta = Metadata::new_from_path(original_file.path())?;
  let has_gps = loaded_meta
    .into_iter()
    .any(|t| format!("{:?}", t).starts_with("GPS"));
  assert!(has_gps);

  // Create a valid PNG target file.
  let target_file = tempfile::Builder::new().suffix(".png").tempfile()?;
  let target_image = image::RgbaImage::from_pixel(2, 2, image::Rgba([0, 0, 255, 255]));
  target_image.save_with_format(target_file.path(), image::ImageFormat::Png)?;

  // Apply policy: remove GPS and camera model, preserve comments and timestamps
  let policy = MetadataPolicy {
    remove_all_metadata: false,
    remove_gps: true,
    remove_camera_info: true,
    remove_comments: false,
    remove_timestamps: false,
  };

  clean_image_metadata(original_file.path(), target_file.path(), &policy)
    .map_err(std::io::Error::other)?;

  // Read target file's metadata and assert
  let target_meta = Metadata::new_from_path(target_file.path())?;

  let mut gps_present = false;
  let mut camera_present = false;
  let mut comment_present = false;
  let mut timestamp_present = false;

  for tag in &target_meta {
    let debug_str = format!("{:?}", tag);
    if debug_str.starts_with("GPS") {
      gps_present = true;
    }
    if debug_str.starts_with("Model") {
      camera_present = true;
    }
    if debug_str.starts_with("ImageDescription") {
      comment_present = true;
    }
    if debug_str.starts_with("DateTime") {
      timestamp_present = true;
    }
  }

  assert!(!gps_present, "GPS should have been stripped");
  assert!(!camera_present, "Camera model should have been stripped");
  assert!(comment_present, "Comments should have been preserved");
  assert!(timestamp_present, "Timestamps should have been preserved");

  Ok(())
}

#[test]
fn test_apply_metadata_description_edits() -> Result<(), Box<dyn std::error::Error>> {
  let image_file = tempfile::Builder::new().suffix(".png").tempfile()?;
  let source_image = image::RgbaImage::from_pixel(2, 2, image::Rgba([255, 255, 255, 255]));
  source_image.save_with_format(image_file.path(), image::ImageFormat::Png)?;

  let mut metadata = Metadata::new();
  metadata.set_tag(ExifTag::ImageDescription("Before".to_string()));
  metadata.write_to_file(image_file.path())?;

  let set_edits = MetadataEdits {
    image_description: MetadataValueEdit::Set("After".to_string()),
    artist: MetadataValueEdit::Keep,
    copyright: MetadataValueEdit::Keep,
    software: MetadataValueEdit::Keep,
  };
  apply_metadata_edits(image_file.path(), &set_edits).map_err(std::io::Error::other)?;

  let after_set = Metadata::new_from_path(image_file.path())?;
  let mut found_after = false;
  for tag in &after_set {
    let debug_str = format!("{:?}", tag);
    if debug_str.starts_with("ImageDescription") {
      found_after = debug_str.contains("After");
    }
  }
  assert!(found_after, "ImageDescription should be updated to 'After'");

  let clear_edits = MetadataEdits {
    image_description: MetadataValueEdit::Clear,
    artist: MetadataValueEdit::Keep,
    copyright: MetadataValueEdit::Keep,
    software: MetadataValueEdit::Keep,
  };
  apply_metadata_edits(image_file.path(), &clear_edits).map_err(std::io::Error::other)?;

  let after_clear = Metadata::new_from_path(image_file.path())?;
  let has_description = (&after_clear)
    .into_iter()
    .any(|tag| format!("{:?}", tag).starts_with("ImageDescription"));
  assert!(!has_description, "ImageDescription should be removed");

  Ok(())
}
