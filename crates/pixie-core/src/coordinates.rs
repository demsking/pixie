use crate::errors::{CoreError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct PixelPoint {
  pub x: u32,
  pub y: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct PixelRect {
  pub x: u32,
  pub y: u32,
  pub width: u32,
  pub height: u32,
}

impl PixelRect {
  pub const fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
    Self {
      x,
      y,
      width,
      height,
    }
  }

  pub fn validate_within(&self, max_width: u32, max_height: u32) -> Result<()> {
    if self.width == 0 || self.height == 0 {
      return Err(CoreError::Validation(
        "Crop rectangle cannot have zero dimensions".into(),
      ));
    }
    let end_x = self
      .x
      .checked_add(self.width)
      .ok_or_else(|| CoreError::Validation("Crop coordinate overflow (x + width)".into()))?;
    let end_y = self
      .y
      .checked_add(self.height)
      .ok_or_else(|| CoreError::Validation("Crop coordinate overflow (y + height)".into()))?;

    if end_x > max_width || end_y > max_height {
      return Err(CoreError::Validation(format!(
        "Crop area exceeds image bounds: [0, 0, {max_width}, {max_height}], requested x={}+w={}, y={}+h={}",
        self.x, self.width, self.y, self.height
      )));
    }

    Ok(())
  }

  pub fn clamp_to(self, bounds: PixelRect) -> Self {
    let x = self.x.clamp(bounds.x, bounds.x + bounds.width);
    let y = self.y.clamp(bounds.y, bounds.y + bounds.height);

    let max_w = (bounds.x + bounds.width).saturating_sub(x);
    let max_h = (bounds.y + bounds.height).saturating_sub(y);

    let width = self.width.clamp(0, max_w);
    let height = self.height.clamp(0, max_h);

    Self {
      x,
      y,
      width,
      height,
    }
  }
}
