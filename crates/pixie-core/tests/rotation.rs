#![allow(clippy::unwrap_used, clippy::expect_used)]
use image::{DynamicImage, RgbaImage};
use pixie_core::{RotateOperation, RotationType};

#[test]
fn test_rotation_90() {
  let img = DynamicImage::ImageRgba8(RgbaImage::new(100, 50));
  let op = RotateOperation::new(RotationType::Cw90);
  let res = op.apply(img.clone()).unwrap();
  assert_eq!(res.width(), 50);
  assert_eq!(res.height(), 100);

  let op_ccw = RotateOperation::new(RotationType::Ccw90);
  let res_ccw = op_ccw.apply(img).unwrap();
  assert_eq!(res_ccw.width(), 50);
  assert_eq!(res_ccw.height(), 100);
}

#[test]
fn test_rotation_180() {
  let img = DynamicImage::ImageRgba8(RgbaImage::new(100, 50));
  let op = RotateOperation::new(RotationType::Rotate180);
  let res = op.apply(img).unwrap();
  assert_eq!(res.width(), 100);
  assert_eq!(res.height(), 50);
}

#[test]
fn test_rotation_arbitrary_no_expand() {
  let img = DynamicImage::ImageRgba8(RgbaImage::new(100, 50));
  let op = RotateOperation::new(RotationType::Arbitrary {
    angle: 45.0,
    expand: false,
    background_color: [0, 0, 0, 0],
  });
  let res = op.apply(img).unwrap();
  assert_eq!(res.width(), 100);
  assert_eq!(res.height(), 50);
}

#[test]
fn test_rotation_arbitrary_expand() {
  let img = DynamicImage::ImageRgba8(RgbaImage::new(100, 100));
  let op = RotateOperation::new(RotationType::Arbitrary {
    angle: 45.0,
    expand: true,
    background_color: [0, 0, 0, 0],
  });
  let res = op.apply(img).unwrap();
  // For 100x100 square rotated by 45 degrees, the bounding box width/height should be ~141.4
  assert!(res.width() > 135);
  assert!(res.width() < 145);
}
