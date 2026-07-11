use crate::errors::{CoreError, Result};
use crate::formats::ImageFormat;
use crate::limits::{MAX_FILE_SIZE, check_dimensions, check_memory_allocation};
use image::DynamicImage;
use std::io::Cursor;

pub fn decode_image(data: &[u8]) -> Result<(DynamicImage, ImageFormat)> {
  if (data.len() as u64) > MAX_FILE_SIZE {
    return Err(CoreError::MemoryLimit(format!(
      "Input file size of {} bytes exceeds limit of {} bytes",
      data.len(),
      MAX_FILE_SIZE
    )));
  }

  let format = ImageFormat::detect(data)?;

  let reader = image::ImageReader::with_format(Cursor::new(data), format.to_image_format());

  // Pre-verify dimensions using header decoding only
  let (w, h) = reader
    .into_dimensions()
    .map_err(|e| CoreError::Decode(format!("Failed to read image dimensions header: {e}")))?;

  check_dimensions(w, h)?;
  // Assume 4 channels of 8-bit color for conservative buffer limits
  check_memory_allocation(w, h, 4, 1)?;

  // Perform actual decoding
  let reader = image::ImageReader::with_format(Cursor::new(data), format.to_image_format());
  let img = reader
    .decode()
    .map_err(|e| CoreError::Decode(format!("Image decoding failed: {e}")))?;

  Ok((img, format))
}
