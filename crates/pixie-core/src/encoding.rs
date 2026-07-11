use crate::errors::{CoreError, Result};
use crate::formats::ImageFormat;
use crate::operations::flatten::FlattenOperation;
use image::{DynamicImage, ImageEncoder};
use std::io::Write;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ExportOptions {
  Png {
    compression: u8, // 0..10 (clamped/mapped to PNG compression levels)
    preserve_transparency: bool,
  },
  Jpeg {
    quality: u8, // 1..100
    progressive: bool,
    background_color: [u8; 3],
  },
  Webp {
    lossless: bool,
    quality: f32, // 1..100
  },
  Avif {
    quality: u8, // 1..100
    speed: u8,   // 1..10
  },
  Bmp {
    background_color: Option<[u8; 3]>,
  },
  Tiff,
  Gif,
}

impl ExportOptions {
  pub fn format(&self) -> ImageFormat {
    match self {
      ExportOptions::Png { .. } => ImageFormat::Png,
      ExportOptions::Jpeg { .. } => ImageFormat::Jpeg,
      ExportOptions::Webp { .. } => ImageFormat::Webp,
      ExportOptions::Avif { .. } => ImageFormat::Avif,
      ExportOptions::Bmp { .. } => ImageFormat::Bmp,
      ExportOptions::Tiff => ImageFormat::Tiff,
      ExportOptions::Gif => ImageFormat::Gif,
    }
  }
}

pub fn encode_image<W: Write + std::io::Seek>(
  img: &DynamicImage,
  writer: W,
  options: &ExportOptions,
) -> Result<()> {
  let mut writer = writer;

  match options {
    ExportOptions::Png {
      preserve_transparency,
      ..
    } => {
      let processed_img = if !*preserve_transparency && img.color().has_alpha() {
        FlattenOperation::new([255, 255, 255]).apply(img.clone())?
      } else {
        img.clone()
      };
      let encoder = image::codecs::png::PngEncoder::new(&mut writer);
      encoder
        .write_image(
          processed_img.as_bytes(),
          processed_img.width(),
          processed_img.height(),
          processed_img.color().into(),
        )
        .map_err(|e| CoreError::Encode(format!("PNG encoding error: {e}")))?;
    }
    ExportOptions::Jpeg {
      quality,
      background_color,
      ..
    } => {
      // Flatten transparency onto selected background color for JPEG
      let processed_img = if img.color().has_alpha() {
        FlattenOperation::new(*background_color).apply(img.clone())?
      } else {
        img.clone()
      };
      let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut writer, *quality);
      encoder
        .write_image(
          processed_img.as_bytes(),
          processed_img.width(),
          processed_img.height(),
          processed_img.color().into(),
        )
        .map_err(|e| CoreError::Encode(format!("JPEG encoding error: {e}")))?;
    }
    ExportOptions::Webp { lossless, .. } => {
      // WebP encoding in the native 'image' crate is currently lossless-only.
      if !*lossless {
        tracing::warn!(
          "WebP lossy encoding is not supported by the underlying crate; falling back to lossless."
        );
      }
      let encoder = image::codecs::webp::WebPEncoder::new_lossless(&mut writer);
      encoder
        .write_image(
          img.as_bytes(),
          img.width(),
          img.height(),
          img.color().into(),
        )
        .map_err(|e| CoreError::Encode(format!("WebP encoding error: {e}")))?;
    }
    ExportOptions::Avif { quality, speed } => {
      let encoder =
        image::codecs::avif::AvifEncoder::new_with_speed_quality(&mut writer, *speed, *quality);
      encoder
        .write_image(
          img.as_bytes(),
          img.width(),
          img.height(),
          img.color().into(),
        )
        .map_err(|e| CoreError::Encode(format!("AVIF encoding error: {e}")))?;
    }
    ExportOptions::Bmp { background_color } => {
      let processed_img = if let Some(bg) = background_color {
        if img.color().has_alpha() {
          FlattenOperation::new(*bg).apply(img.clone())?
        } else {
          img.clone()
        }
      } else {
        img.clone()
      };
      let encoder = image::codecs::bmp::BmpEncoder::new(&mut writer);
      encoder
        .write_image(
          processed_img.as_bytes(),
          processed_img.width(),
          processed_img.height(),
          processed_img.color().into(),
        )
        .map_err(|e| CoreError::Encode(format!("BMP encoding error: {e}")))?;
    }
    ExportOptions::Tiff => {
      let encoder = image::codecs::tiff::TiffEncoder::new(&mut writer);
      encoder
        .write_image(
          img.as_bytes(),
          img.width(),
          img.height(),
          img.color().into(),
        )
        .map_err(|e| CoreError::Encode(format!("TIFF encoding error: {e}")))?;
    }
    ExportOptions::Gif => {
      let mut encoder = image::codecs::gif::GifEncoder::new(&mut writer);
      encoder
        .encode_frame(image::Frame::new(img.to_rgba8()))
        .map_err(|e| CoreError::Encode(format!("GIF encoding error: {e}")))?;
    }
  }

  Ok(())
}
