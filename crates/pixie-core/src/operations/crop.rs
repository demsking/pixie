use crate::coordinates::PixelRect;
use crate::errors::Result;
use image::DynamicImage;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CropOperation {
  pub rect: PixelRect,
}

impl CropOperation {
  pub fn new(rect: PixelRect) -> Self {
    Self { rect }
  }

  pub fn apply(&self, mut img: DynamicImage) -> Result<DynamicImage> {
    self.rect.validate_within(img.width(), img.height())?;
    let cropped = img.crop(self.rect.x, self.rect.y, self.rect.width, self.rect.height);
    Ok(cropped)
  }
}
