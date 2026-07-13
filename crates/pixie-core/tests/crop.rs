#![allow(clippy::unwrap_used, clippy::expect_used)]
use image::{DynamicImage, Rgba, RgbaImage};
use pixie_core::{CircleMaskOperation, CropOperation, PixelRect, TransparentBackgroundOperation};

#[test]
fn test_crop_success() {
  let img = DynamicImage::ImageRgba8(RgbaImage::new(100, 100));
  let crop = CropOperation::new(PixelRect::new(10, 10, 50, 50));
  let res = crop.apply(img).unwrap();
  assert_eq!(res.width(), 50);
  assert_eq!(res.height(), 50);
}

#[test]
fn test_crop_invalid_bounds() {
  let img = DynamicImage::ImageRgba8(RgbaImage::new(100, 100));
  let crop = CropOperation::new(PixelRect::new(90, 90, 20, 20));
  assert!(crop.apply(img).is_err());
}

#[test]
fn test_crop_clamping() {
  let bounds = PixelRect::new(0, 0, 100, 100);
  let rect = PixelRect::new(50, 50, 80, 80);
  let clamped = rect.clamp_to(bounds);
  assert_eq!(clamped.x, 50);
  assert_eq!(clamped.y, 50);
  assert_eq!(clamped.width, 50);
  assert_eq!(clamped.height, 50);
}

#[test]
fn test_circle_crop_success() {
  let mut rgba_img = RgbaImage::new(100, 100);
  for p in rgba_img.pixels_mut() {
    *p = Rgba([255, 255, 255, 255]);
  }
  let img = DynamicImage::ImageRgba8(rgba_img);
  let circle = CircleMaskOperation::new(PixelRect::new(10, 10, 50, 50));
  let res = circle.apply(img).unwrap();
  assert_eq!(res.width(), 50);
  assert_eq!(res.height(), 50);

  // Check that pixels outside the circle are transparent (alpha = 0)
  let rgba = res.to_rgba8();
  let p_corner = rgba.get_pixel(0, 0); // (0,0) of cropped image is outside the circle of radius 25 centered at (25, 25)
  assert_eq!(p_corner[3], 0);

  let p_center = rgba.get_pixel(25, 25); // center is inside
  assert_ne!(p_center[3], 0);
}

#[test]
fn test_circle_crop_non_square() {
  let img = DynamicImage::ImageRgba8(RgbaImage::new(100, 100));
  let circle = CircleMaskOperation::new(PixelRect::new(10, 10, 50, 40));
  assert!(circle.apply(img).is_err());
}

#[test]
fn test_transparent_background_removes_edge_connected_background() {
  let mut rgba_img = RgbaImage::from_pixel(5, 5, Rgba([250, 250, 250, 255]));
  for y in 1..4 {
    for x in 1..4 {
      rgba_img.put_pixel(x, y, Rgba([10, 20, 30, 255]));
    }
  }

  let img = DynamicImage::ImageRgba8(rgba_img);
  let operation = TransparentBackgroundOperation::new(8);
  let res = operation.apply(img).unwrap().to_rgba8();

  assert_eq!(res.get_pixel(0, 0)[3], 0);
  assert_eq!(res.get_pixel(4, 4)[3], 0);
  assert_eq!(res.get_pixel(2, 2), &Rgba([10, 20, 30, 255]));
}

#[test]
fn test_transparent_background_preserves_enclosed_matching_pixels() {
  let mut rgba_img = RgbaImage::from_pixel(5, 5, Rgba([255, 255, 255, 255]));
  for y in 1..4 {
    for x in 1..4 {
      rgba_img.put_pixel(x, y, Rgba([0, 0, 0, 255]));
    }
  }
  rgba_img.put_pixel(2, 2, Rgba([255, 255, 255, 255]));

  let img = DynamicImage::ImageRgba8(rgba_img);
  let operation = TransparentBackgroundOperation::new(0);
  let res = operation.apply(img).unwrap().to_rgba8();

  assert_eq!(res.get_pixel(0, 0)[3], 0);
  assert_eq!(res.get_pixel(2, 2), &Rgba([255, 255, 255, 255]));
}
