#![allow(clippy::unwrap_used, clippy::expect_used)]
use image::{DynamicImage, RgbaImage};
use pixie_core::{FilterType, ResizeOperation, calculate_dimensions};

#[test]
fn test_calculate_dimensions_preserve_ratio() {
  // 800x600 is 4:3. If target is 400x400 (which is 1:1), limiting dimension is width (400) -> height becomes 300
  let (w, h) = calculate_dimensions(800, 600, 400, 400, true, false).unwrap();
  assert_eq!(w, 400);
  assert_eq!(h, 300);

  // 800x600 is 4:3. If target is 1000x300, limiting dimension is height (300) -> width becomes 400
  let (w, h) = calculate_dimensions(800, 600, 1000, 300, true, false).unwrap();
  assert_eq!(w, 400);
  assert_eq!(h, 300);
}

#[test]
fn test_calculate_dimensions_prevent_enlargement() {
  // 800x600 target is 1600x1200, but prevent enlargement is true
  let (w, h) = calculate_dimensions(800, 600, 1600, 1200, true, true).unwrap();
  assert_eq!(w, 800);
  assert_eq!(h, 600);
}

#[test]
fn test_resize_operation_success() {
  let img = DynamicImage::ImageRgba8(RgbaImage::new(100, 100));
  let op = ResizeOperation::new(50, 50, FilterType::Lanczos3);
  let res = op.apply(img).unwrap();
  assert_eq!(res.width(), 50);
  assert_eq!(res.height(), 50);
}

#[test]
fn test_resize_operation_exceed_limits() {
  let img = DynamicImage::ImageRgba8(RgbaImage::new(100, 100));
  let op = ResizeOperation::new(50000, 50000, FilterType::Nearest);
  assert!(op.apply(img).is_err());
}
