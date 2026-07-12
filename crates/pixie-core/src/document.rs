use crate::decoding::decode_image;
use crate::encoding::{ExportOptions, encode_image};
use crate::errors::Result;
use crate::formats::ImageFormat;
use crate::history::EditingHistory;
use crate::operations::ImageOperation;
use crate::pipeline::OperationPipeline;
use image::DynamicImage;
use std::io::Write;

pub struct PixieDocument {
  original_image: DynamicImage,
  original_format: ImageFormat,
  history: EditingHistory,
  pub metadata_policy: crate::metadata::MetadataPolicy,
}

impl std::fmt::Debug for PixieDocument {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("PixieDocument")
      .field("original_format", &self.original_format)
      .field("history", &self.history)
      .field("metadata_policy", &self.metadata_policy)
      .finish()
  }
}

impl PixieDocument {
  pub fn open(data: &[u8]) -> Result<Self> {
    let (original_image, original_format) = decode_image(data)?;
    Ok(Self {
      original_image,
      original_format,
      history: EditingHistory::new(),
      metadata_policy: crate::metadata::MetadataPolicy::default(),
    })
  }

  pub const fn original_image(&self) -> &DynamicImage {
    &self.original_image
  }

  pub const fn original_format(&self) -> ImageFormat {
    self.original_format
  }

  pub const fn history(&self) -> &EditingHistory {
    &self.history
  }

  pub fn history_mut(&mut self) -> &mut EditingHistory {
    &mut self.history
  }

  pub fn apply_operation(&mut self, op: ImageOperation) -> Result<DynamicImage> {
    let current = self.current_image()?;
    let next = op.apply(current)?;
    self.history.push(op);
    Ok(next)
  }

  pub fn current_image(&self) -> Result<DynamicImage> {
    let mut pipeline = OperationPipeline::new();
    for op in self.history.operations() {
      pipeline.add(op.clone());
    }
    pipeline.apply(self.original_image.clone())
  }

  pub fn undo(&mut self) -> bool {
    self.history.undo()
  }

  pub fn redo(&mut self) -> bool {
    self.history.redo()
  }

  pub fn reset(&mut self) {
    self.history.reset();
  }

  pub fn export<W: Write + std::io::Seek>(&self, writer: W, options: &ExportOptions) -> Result<()> {
    let current = self.current_image()?;
    encode_image(&current, writer, options)
  }
}
