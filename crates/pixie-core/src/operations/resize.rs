use crate::errors::{CoreError, Result};
use crate::limits::{check_dimensions, check_memory_allocation};
use image::DynamicImage;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FilterType {
  Nearest,
  Triangle,
  CatmullRom,
  Gaussian,
  Lanczos3,
}

impl FilterType {
  pub fn to_image_filter(self) -> image::imageops::FilterType {
    match self {
      FilterType::Nearest => image::imageops::FilterType::Nearest,
      FilterType::Triangle => image::imageops::FilterType::Triangle,
      FilterType::CatmullRom => image::imageops::FilterType::CatmullRom,
      FilterType::Gaussian => image::imageops::FilterType::Gaussian,
      FilterType::Lanczos3 => image::imageops::FilterType::Lanczos3,
    }
  }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ResizeOperation {
  pub width: u32,
  pub height: u32,
  pub filter: FilterType,
}

impl ResizeOperation {
  pub fn new(width: u32, height: u32, filter: FilterType) -> Self {
    Self {
      width,
      height,
      filter,
    }
  }

  pub fn apply(&self, img: DynamicImage) -> Result<DynamicImage> {
    check_dimensions(self.width, self.height)?;

    // 4 channels, 1 byte per channel (RGBA8) or whatever the input has.
    // Let's assume standard 4 channels of 8-bit to be safe and conservative.
    check_memory_allocation(self.width, self.height, 4, 1)?;

    let filter = self.filter.to_image_filter();
    let resized = img.resize_exact(self.width, self.height, filter);
    Ok(resized)
  }
}

pub fn calculate_dimensions(
  orig_w: u32,
  orig_h: u32,
  mut target_w: u32,
  mut target_h: u32,
  preserve_aspect_ratio: bool,
  prevent_enlargement: bool,
) -> Result<(u32, u32)> {
  if orig_w == 0 || orig_h == 0 {
    return Err(CoreError::Validation(
      "Original dimensions cannot be zero".into(),
    ));
  }

  if preserve_aspect_ratio {
    let orig_ratio = f64::from(orig_w) / f64::from(orig_h);
    let target_ratio = f64::from(target_w) / f64::from(target_h);

    if target_ratio > orig_ratio {
      // Target is wider than original ratio: height is limiting
      target_w = (f64::from(target_h) * orig_ratio).round() as u32;
    } else {
      // Target is taller than original ratio: width is limiting
      target_h = (f64::from(target_w) / orig_ratio).round() as u32;
    }
  }

  if prevent_enlargement {
    if target_w > orig_w {
      target_w = orig_w;
      if preserve_aspect_ratio {
        target_h = orig_h;
      }
    }
    if target_h > orig_h {
      target_h = orig_h;
      if preserve_aspect_ratio {
        target_w = orig_w;
      }
    }
  }

  if target_w == 0 {
    target_w = 1;
  }
  if target_h == 0 {
    target_h = 1;
  }

  check_dimensions(target_w, target_h)?;
  Ok((target_w, target_h))
}
