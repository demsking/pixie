use crate::errors::{CoreError, Result};
use crate::limits::check_dimensions;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ImageDimensions {
  width: u32,
  height: u32,
}

impl ImageDimensions {
  pub fn new(width: u32, height: u32) -> Result<Self> {
    check_dimensions(width, height)?;
    Ok(Self { width, height })
  }

  pub const fn width(&self) -> u32 {
    self.width
  }

  pub const fn height(&self) -> u32 {
    self.height
  }

  pub fn aspect_ratio(&self) -> f64 {
    f64::from(self.width) / f64::from(self.height)
  }

  pub fn pixel_count(&self) -> u64 {
    u64::from(self.width) * u64::from(self.height)
  }
}

impl TryFrom<(u32, u32)> for ImageDimensions {
  type Error = CoreError;

  fn try_from(value: (u32, u32)) -> Result<Self> {
    Self::new(value.0, value.1)
  }
}
