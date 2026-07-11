use crate::coordinates::PixelRect;
use crate::errors::{CoreError, Result};
use image::DynamicImage;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CircleMaskOperation {
  pub rect: PixelRect,
}

impl CircleMaskOperation {
  pub fn new(rect: PixelRect) -> Self {
    Self { rect }
  }

  pub fn apply(&self, img: DynamicImage) -> Result<DynamicImage> {
    self.rect.validate_within(img.width(), img.height())?;
    if self.rect.width != self.rect.height {
      return Err(CoreError::Validation(
        "Circle crop bounding box must be a square".into(),
      ));
    }

    let cropped = img.crop_imm(self.rect.x, self.rect.y, self.rect.width, self.rect.height);
    let mut rgba = cropped.to_rgba8();

    let size = self.rect.width;
    let r = f64::from(size) / 2.0;
    let cx = r;
    let cy = r;

    for y in 0..size {
      for x in 0..size {
        let dx = f64::from(x) + 0.5 - cx;
        let dy = f64::from(y) + 0.5 - cy;
        if dx * dx + dy * dy > r * r {
          let pixel = rgba.get_pixel_mut(x, y);
          pixel[3] = 0; // Make transparent
        }
      }
    }

    Ok(DynamicImage::ImageRgba8(rgba))
  }
}
