use crate::errors::{CoreError, Result};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ImageFormat {
  Png,
  Jpeg,
  Webp,
  Bmp,
  Tiff,
  Gif,
  Avif,
}

impl ImageFormat {
  pub fn detect(data: &[u8]) -> Result<Self> {
    if data.len() < 4 {
      return Err(CoreError::Format(
        "File is too small to identify format".into(),
      ));
    }

    // PNG Signature
    if data.starts_with(&[0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]) {
      return Ok(ImageFormat::Png);
    }

    // JPEG Signature
    if data.starts_with(&[0xff, 0xd8, 0xff]) {
      return Ok(ImageFormat::Jpeg);
    }

    // GIF Signature
    if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
      return Ok(ImageFormat::Gif);
    }

    // BMP Signature
    if data.starts_with(b"BM") {
      return Ok(ImageFormat::Bmp);
    }

    // TIFF Signatures
    if data.starts_with(&[0x49, 0x49, 0x2a, 0x00]) || data.starts_with(&[0x4d, 0x4d, 0x00, 0x2a]) {
      return Ok(ImageFormat::Tiff);
    }

    // WebP Signature: RIFF....WEBP
    if data.len() >= 12 && &data[0..4] == b"RIFF" && &data[8..12] == b"WEBP" {
      return Ok(ImageFormat::Webp);
    }

    // AVIF Signature: ftypavif or ftypavis in the first few bytes (typically bytes 4-12)
    if data.len() >= 12 && &data[4..8] == b"ftyp" {
      let brand = &data[8..12];
      if brand == b"avif" || brand == b"avis" {
        return Ok(ImageFormat::Avif);
      }
    }

    Err(CoreError::Format(
      "Unsupported or unknown image format signature".into(),
    ))
  }

  pub fn from_extension(path: &Path) -> Option<Self> {
    let ext = path.extension()?.to_str()?.to_lowercase();
    match ext.as_str() {
      "png" => Some(ImageFormat::Png),
      "jpg" | "jpeg" => Some(ImageFormat::Jpeg),
      "webp" => Some(ImageFormat::Webp),
      "bmp" => Some(ImageFormat::Bmp),
      "tif" | "tiff" => Some(ImageFormat::Tiff),
      "gif" => Some(ImageFormat::Gif),
      "avif" => Some(ImageFormat::Avif),
      _ => None,
    }
  }

  pub fn to_mime_type(self) -> &'static str {
    match self {
      ImageFormat::Png => "image/png",
      ImageFormat::Jpeg => "image/jpeg",
      ImageFormat::Webp => "image/webp",
      ImageFormat::Bmp => "image/bmp",
      ImageFormat::Tiff => "image/tiff",
      ImageFormat::Gif => "image/gif",
      ImageFormat::Avif => "image/avif",
    }
  }

  pub fn supports_transparency(self) -> bool {
    match self {
      ImageFormat::Png
      | ImageFormat::Webp
      | ImageFormat::Avif
      | ImageFormat::Tiff
      | ImageFormat::Gif => true,
      ImageFormat::Jpeg | ImageFormat::Bmp => false,
    }
  }

  pub fn check_conversion_warnings(
    from: ImageFormat,
    to: ImageFormat,
    has_transparency: bool,
  ) -> Vec<String> {
    let mut warnings = Vec::new();

    if has_transparency && !to.supports_transparency() {
      warnings.push(format!(
        "Destination format {} does not support transparency. Alpha channel will be flattened.",
        to.to_mime_type()
      ));
    }

    match (from, to) {
      (_, ImageFormat::Jpeg) => {
        warnings.push("JPEG is a lossy format. Saving may introduce quality degradation.".into());
      }
      (_, ImageFormat::Bmp) => {
        warnings.push(
          "BMP does not support compressed saving, resulting in large files. Metadata may be lost."
            .into(),
        );
      }
      (ImageFormat::Gif, to_fmt) if to_fmt != ImageFormat::Gif => {
        warnings.push(
          "Original image might be an animation. Converting will lose all frames except the first static frame.".into()
        );
      }
      _ => {}
    }

    warnings
  }

  pub fn to_image_format(self) -> image::ImageFormat {
    match self {
      ImageFormat::Png => image::ImageFormat::Png,
      ImageFormat::Jpeg => image::ImageFormat::Jpeg,
      ImageFormat::Webp => image::ImageFormat::WebP,
      ImageFormat::Bmp => image::ImageFormat::Bmp,
      ImageFormat::Tiff => image::ImageFormat::Tiff,
      ImageFormat::Gif => image::ImageFormat::Gif,
      ImageFormat::Avif => image::ImageFormat::Avif,
    }
  }
}
