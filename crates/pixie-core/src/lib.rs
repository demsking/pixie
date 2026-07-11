pub mod coordinates;
pub mod decoding;
pub mod dimensions;
pub mod document;
pub mod encoding;
pub mod errors;
pub mod formats;
pub mod history;
pub mod limits;
pub mod metadata;
pub mod operations;
pub mod pipeline;

// Re-exports
pub use coordinates::{PixelPoint, PixelRect};
pub use decoding::decode_image;
pub use dimensions::ImageDimensions;
pub use document::PixieDocument;
pub use encoding::{ExportOptions, encode_image};
pub use errors::{CoreError, Result};
pub use formats::ImageFormat;
pub use history::EditingHistory;
pub use metadata::MetadataPolicy;
pub use pipeline::OperationPipeline;

pub use operations::{
  ImageOperation,
  circle_mask::CircleMaskOperation,
  crop::CropOperation,
  flatten::FlattenOperation,
  flip::{FlipDirection, FlipOperation},
  resize::{FilterType, ResizeOperation, calculate_dimensions},
  rotate::{RotateOperation, RotationType},
};
