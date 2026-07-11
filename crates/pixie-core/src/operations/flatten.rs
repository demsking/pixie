use crate::errors::Result;
use image::{DynamicImage, ImageBuffer, Rgb, RgbImage};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FlattenOperation {
  pub background_color: [u8; 3],
}

impl FlattenOperation {
  pub fn new(background_color: [u8; 3]) -> Self {
    Self { background_color }
  }

  pub fn apply(&self, img: DynamicImage) -> Result<DynamicImage> {
    let (w, h) = (img.width(), img.height());
    let rgba = img.to_rgba8();
    let mut out: RgbImage = ImageBuffer::new(w, h);

    let bg = self.background_color;

    for y in 0..h {
      for x in 0..w {
        let pixel = rgba.get_pixel(x, y);
        let alpha = f64::from(pixel[3]) / 255.0;

        let r = (f64::from(pixel[0]) * alpha + f64::from(bg[0]) * (1.0 - alpha)).round() as u8;
        let g = (f64::from(pixel[1]) * alpha + f64::from(bg[1]) * (1.0 - alpha)).round() as u8;
        let b = (f64::from(pixel[2]) * alpha + f64::from(bg[2]) * (1.0 - alpha)).round() as u8;

        out.put_pixel(x, y, Rgb([r, g, b]));
      }
    }

    Ok(DynamicImage::ImageRgb8(out))
  }
}
