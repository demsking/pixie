use crate::errors::{CoreError, Result};

pub const MAX_IMAGE_DIMENSION: u32 = 16384; // 16k
pub const MAX_PIXEL_COUNT: u64 = 268_435_456; // 16384 * 16384
pub const MAX_FILE_SIZE: u64 = 100 * 1024 * 1024; // 100 MB
pub const MAX_MEMORY_LIMIT: u64 = 1024 * 1024 * 1024; // 1 GB

pub fn check_dimensions(width: u32, height: u32) -> Result<()> {
  if width == 0 || height == 0 {
    return Err(CoreError::Validation(
      "Dimensions must be greater than zero".into(),
    ));
  }
  if width > MAX_IMAGE_DIMENSION || height > MAX_IMAGE_DIMENSION {
    return Err(CoreError::DimensionLimit(format!(
      "Dimension exceeds limit of {MAX_IMAGE_DIMENSION}px"
    )));
  }
  let pixel_count = u64::from(width).checked_mul(u64::from(height));
  match pixel_count {
    Some(pixels) if pixels <= MAX_PIXEL_COUNT => Ok(()),
    _ => Err(CoreError::DimensionLimit(format!(
      "Pixel count exceeds limit of {MAX_PIXEL_COUNT} pixels"
    ))),
  }
}

pub fn check_memory_allocation(
  width: u32,
  height: u32,
  channels: u8,
  bytes_per_channel: u8,
) -> Result<()> {
  check_dimensions(width, height)?;
  let area = u64::from(width)
    .checked_mul(u64::from(height))
    .ok_or_else(|| CoreError::MemoryLimit("Integer overflow in pixel count calculation".into()))?;
  let pixel_size = u64::from(channels)
    .checked_mul(u64::from(bytes_per_channel))
    .ok_or_else(|| CoreError::MemoryLimit("Integer overflow in pixel size calculation".into()))?;
  let total_bytes = area
    .checked_mul(pixel_size)
    .ok_or_else(|| CoreError::MemoryLimit("Integer overflow in total bytes calculation".into()))?;

  if total_bytes > MAX_MEMORY_LIMIT {
    return Err(CoreError::MemoryLimit(format!(
      "Memory allocation of {total_bytes} bytes exceeds the limit of {MAX_MEMORY_LIMIT} bytes"
    )));
  }
  Ok(())
}
