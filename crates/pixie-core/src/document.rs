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
  rendered_cache: RenderedImageCache,
  pub metadata_policy: crate::metadata::MetadataPolicy,
  pub metadata_edits: crate::metadata::MetadataEdits,
}

#[derive(Debug, Clone)]
struct RenderedImageCache {
  history_revision: u64,
  image: DynamicImage,
}

impl std::fmt::Debug for PixieDocument {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("PixieDocument")
      .field("original_format", &self.original_format)
      .field("history", &self.history)
      .field("metadata_policy", &self.metadata_policy)
      .field("metadata_edits", &self.metadata_edits)
      .finish()
  }
}

impl PixieDocument {
  pub fn open(data: &[u8]) -> Result<Self> {
    let (original_image, original_format) = decode_image(data)?;
    Ok(Self::new(original_image, original_format))
  }

  pub fn new(original_image: DynamicImage, original_format: ImageFormat) -> Self {
    let rendered_image = original_image.clone();
    Self {
      original_image,
      original_format,
      history: EditingHistory::new(),
      rendered_cache: RenderedImageCache {
        history_revision: 0,
        image: rendered_image,
      },
      metadata_policy: crate::metadata::MetadataPolicy::default(),
      metadata_edits: crate::metadata::MetadataEdits::default(),
    }
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
    self.commit_rendered_operation(op, next.clone());
    Ok(next)
  }

  pub fn commit_rendered_operation(&mut self, op: ImageOperation, rendered: DynamicImage) {
    self.history.push(op);
    self.update_rendered_cache(rendered);
  }

  pub fn current_image(&mut self) -> Result<DynamicImage> {
    let revision = self.history.revision();
    if self.rendered_cache.history_revision == revision {
      return Ok(self.rendered_cache.image.clone());
    }

    let mut pipeline = OperationPipeline::new();
    for op in self.history.operations() {
      pipeline.add(op.clone());
    }
    let rendered = pipeline.apply(self.original_image.clone())?;
    self.update_rendered_cache(rendered.clone());
    Ok(rendered)
  }

  pub fn undo(&mut self) -> bool {
    let changed = self.history.undo();
    if changed {
      self.invalidate_rendered_cache();
    }
    changed
  }

  pub fn redo(&mut self) -> bool {
    let changed = self.history.redo();
    if changed {
      self.invalidate_rendered_cache();
    }
    changed
  }

  pub fn reset(&mut self) {
    self.history.reset();
    self.update_rendered_cache(self.original_image.clone());
  }

  pub fn export<W: Write + std::io::Seek>(
    &mut self,
    writer: W,
    options: &ExportOptions,
  ) -> Result<()> {
    let current = self.current_image()?;
    encode_image(&current, writer, options)
  }

  fn update_rendered_cache(&mut self, image: DynamicImage) {
    self.rendered_cache = RenderedImageCache {
      history_revision: self.history.revision(),
      image,
    };
  }

  fn invalidate_rendered_cache(&mut self) {
    self.rendered_cache.history_revision = self.history.revision().wrapping_sub(1);
  }
}
