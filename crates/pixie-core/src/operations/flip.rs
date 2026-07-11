use crate::errors::Result;
use image::DynamicImage;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FlipDirection {
  Horizontal,
  Vertical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FlipOperation {
  pub direction: FlipDirection,
}

impl FlipOperation {
  pub fn new(direction: FlipDirection) -> Self {
    Self { direction }
  }

  pub fn apply(&self, img: DynamicImage) -> Result<DynamicImage> {
    match self.direction {
      FlipDirection::Horizontal => Ok(img.fliph()),
      FlipDirection::Vertical => Ok(img.flipv()),
    }
  }
}
