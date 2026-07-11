use crate::errors::{CoreError, Result};
use crate::limits::check_memory_allocation;
use image::{DynamicImage, ImageBuffer, Rgba, RgbaImage};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum RotationType {
  Cw90,
  Ccw90,
  Rotate180,
  Arbitrary {
    angle: f64, // in degrees
    expand: bool,
    background_color: [u8; 4],
  },
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RotateOperation {
  pub rotation: RotationType,
}

impl RotateOperation {
  pub fn new(rotation: RotationType) -> Self {
    Self { rotation }
  }

  pub fn apply(&self, img: DynamicImage) -> Result<DynamicImage> {
    match &self.rotation {
      RotationType::Cw90 => Ok(img.rotate90()),
      RotationType::Ccw90 => Ok(img.rotate270()),
      RotationType::Rotate180 => Ok(img.rotate180()),
      RotationType::Arbitrary {
        angle,
        expand,
        background_color,
      } => rotate_arbitrary(&img, *angle, *expand, *background_color),
    }
  }
}

fn rotate_arbitrary(
  img: &DynamicImage,
  angle_degrees: f64,
  expand: bool,
  bg_color: [u8; 4],
) -> Result<DynamicImage> {
  // Normalise angle to (-180, 180]
  let angle = angle_degrees.rem_euclid(360.0);
  let angle = if angle > 180.0 { angle - 360.0 } else { angle };

  if angle.abs() < 1e-5 {
    return Ok(img.clone());
  }

  // Fast paths for 90, 180, 270
  if (angle - 90.0).abs() < 1e-5 {
    return Ok(img.rotate90());
  } else if (angle - 180.0).abs() < 1e-5 || (angle + 180.0).abs() < 1e-5 {
    return Ok(img.rotate180());
  } else if (angle + 90.0).abs() < 1e-5 {
    return Ok(img.rotate270());
  }

  let rad = angle.to_radians();
  let cos_a = rad.cos();
  let sin_a = rad.sin();

  let orig_w = img.width();
  let orig_h = img.height();

  let (new_w, new_h) = if expand {
    let w_new = (f64::from(orig_w) * cos_a.abs() + f64::from(orig_h) * sin_a.abs()).round() as u32;
    let h_new = (f64::from(orig_w) * sin_a.abs() + f64::from(orig_h) * cos_a.abs()).round() as u32;
    (w_new, h_new)
  } else {
    (orig_w, orig_h)
  };

  if new_w == 0 || new_h == 0 {
    return Err(CoreError::Validation(
      "Rotated image dimensions cannot be zero".into(),
    ));
  }

  check_memory_allocation(new_w, new_h, 4, 1)?;

  let rgba = img.to_rgba8();
  let mut out_img: RgbaImage = ImageBuffer::new(new_w, new_h);

  let cx_orig = f64::from(orig_w) / 2.0;
  let cy_orig = f64::from(orig_h) / 2.0;
  let cx_new = f64::from(new_w) / 2.0;
  let cy_new = f64::from(new_h) / 2.0;

  let bg_pixel = Rgba(bg_color);

  // Bilinear interpolation loop
  for y in 0..new_h {
    for x in 0..new_w {
      // Map destination pixel (x, y) to source coordinates (sx, sy) relative to centers
      let dx = f64::from(x) + 0.5 - cx_new;
      let dy = f64::from(y) + 0.5 - cy_new;

      // Reverse rotate
      let sx = dx * cos_a + dy * sin_a + cx_orig;
      let sy = -dx * sin_a + dy * cos_a + cy_orig;

      // Bilinear sampling
      let x0 = sx.floor();
      let y0 = sy.floor();
      let x1 = x0 + 1.0;
      let y1 = y0 + 1.0;

      let wx1 = sx - x0;
      let wx0 = 1.0 - wx1;
      let wy1 = sy - y0;
      let wy0 = 1.0 - wy1;

      let ix0 = x0 as i32;
      let iy0 = y0 as i32;
      let ix1 = x1 as i32;
      let iy1 = y1 as i32;

      // Get the 4 neighborhood pixels
      let get_pixel = |px: i32, py: i32| -> Rgba<u8> {
        if px >= 0 && px < (orig_w as i32) && py >= 0 && py < (orig_h as i32) {
          *rgba.get_pixel(px as u32, py as u32)
        } else {
          bg_pixel
        }
      };

      let p00 = get_pixel(ix0, iy0);
      let p10 = get_pixel(ix1, iy0);
      let p01 = get_pixel(ix0, iy1);
      let p11 = get_pixel(ix1, iy1);

      // Interpolate each channel
      let mut out_channels = [0u8; 4];
      for c in 0..4 {
        let val = f64::from(p00[c]) * wx0 * wy0
          + f64::from(p10[c]) * wx1 * wy0
          + f64::from(p01[c]) * wx0 * wy1
          + f64::from(p11[c]) * wx1 * wy1;
        out_channels[c] = val.clamp(0.0, 255.0).round() as u8;
      }

      out_img.put_pixel(x, y, Rgba(out_channels));
    }
  }

  Ok(DynamicImage::ImageRgba8(out_img))
}
