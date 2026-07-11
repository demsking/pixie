pub mod circle_mask;
pub mod crop;
pub mod flatten;
pub mod flip;
pub mod resize;
pub mod rotate;

pub use circle_mask::CircleMaskOperation;
pub use crop::CropOperation;
pub use flatten::FlattenOperation;
pub use flip::FlipOperation;
pub use resize::{FilterType, ResizeOperation};
pub use rotate::RotateOperation;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ImageOperation {
  Crop(CropOperation),
  Resize(ResizeOperation),
  Rotate(RotateOperation),
  Flip(FlipOperation),
  CircleMask(CircleMaskOperation),
  Flatten(FlattenOperation),
}

impl ImageOperation {
  pub fn apply(&self, img: image::DynamicImage) -> crate::errors::Result<image::DynamicImage> {
    match self {
      ImageOperation::Crop(op) => op.apply(img),
      ImageOperation::Resize(op) => op.apply(img),
      ImageOperation::Rotate(op) => op.apply(img),
      ImageOperation::Flip(op) => op.apply(img),
      ImageOperation::CircleMask(op) => op.apply(img),
      ImageOperation::Flatten(op) => op.apply(img),
    }
  }
}
