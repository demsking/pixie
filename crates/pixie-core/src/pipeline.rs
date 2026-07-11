use crate::errors::Result;
use crate::operations::ImageOperation;
use image::DynamicImage;

#[derive(Debug, Clone)]
pub struct OperationPipeline {
  pub operations: Vec<ImageOperation>,
}

impl OperationPipeline {
  pub fn new() -> Self {
    Self {
      operations: Vec::new(),
    }
  }

  pub fn add(&mut self, operation: ImageOperation) {
    self.operations.push(operation);
  }

  pub fn apply(&self, mut img: DynamicImage) -> Result<DynamicImage> {
    for op in &self.operations {
      img = op.apply(img)?;
    }
    Ok(img)
  }
}

impl Default for OperationPipeline {
  fn default() -> Self {
    Self::new()
  }
}
