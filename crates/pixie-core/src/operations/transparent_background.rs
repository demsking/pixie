use crate::errors::Result;
use image::{DynamicImage, Rgba};
use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TransparentBackgroundOperation {
  pub tolerance: u8,
}

impl TransparentBackgroundOperation {
  pub const fn new(tolerance: u8) -> Self {
    Self { tolerance }
  }

  pub fn apply(&self, img: DynamicImage) -> Result<DynamicImage> {
    let mut rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();

    if width == 0 || height == 0 {
      return Ok(DynamicImage::ImageRgba8(rgba));
    }

    let background = *rgba.get_pixel(0, 0);
    let mut visited = vec![false; (width as usize) * (height as usize)];
    let mut queue = VecDeque::new();

    for x in 0..width {
      queue.push_back((x, 0));
      if height > 1 {
        queue.push_back((x, height - 1));
      }
    }

    for y in 1..height.saturating_sub(1) {
      queue.push_back((0, y));
      if width > 1 {
        queue.push_back((width - 1, y));
      }
    }

    while let Some((x, y)) = queue.pop_front() {
      let index = pixel_index(x, y, width);
      if visited[index] {
        continue;
      }
      visited[index] = true;

      if !matches_background(*rgba.get_pixel(x, y), background, self.tolerance) {
        continue;
      }

      rgba.get_pixel_mut(x, y)[3] = 0;

      if x > 0 {
        queue.push_back((x - 1, y));
      }
      if x + 1 < width {
        queue.push_back((x + 1, y));
      }
      if y > 0 {
        queue.push_back((x, y - 1));
      }
      if y + 1 < height {
        queue.push_back((x, y + 1));
      }
    }

    Ok(DynamicImage::ImageRgba8(rgba))
  }
}

fn pixel_index(x: u32, y: u32, width: u32) -> usize {
  (y as usize) * (width as usize) + (x as usize)
}

fn matches_background(pixel: Rgba<u8>, background: Rgba<u8>, tolerance: u8) -> bool {
  let tolerance = i16::from(tolerance);
  pixel[3] > 0
    && (i16::from(pixel[0]) - i16::from(background[0])).abs() <= tolerance
    && (i16::from(pixel[1]) - i16::from(background[1])).abs() <= tolerance
    && (i16::from(pixel[2]) - i16::from(background[2])).abs() <= tolerance
}
