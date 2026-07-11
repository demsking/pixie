#![allow(clippy::unwrap_used, clippy::expect_used)]
use image::{DynamicImage, RgbaImage};
use pixie_core::{
  EditingHistory, FlipDirection, FlipOperation, ImageOperation, OperationPipeline, RotateOperation,
  RotationType,
};

#[test]
fn test_pipeline_rendering() {
  let img = DynamicImage::ImageRgba8(RgbaImage::new(100, 100));
  let mut pipeline = OperationPipeline::new();
  pipeline.add(ImageOperation::Rotate(RotateOperation::new(
    RotationType::Cw90,
  )));
  pipeline.add(ImageOperation::Flip(FlipOperation::new(
    FlipDirection::Horizontal,
  )));

  let res = pipeline.apply(img).unwrap();
  assert_eq!(res.width(), 100);
  assert_eq!(res.height(), 100);
}

#[test]
fn test_history_undo_redo() {
  let mut history = EditingHistory::new();
  let op1 = ImageOperation::Flip(FlipOperation::new(FlipDirection::Horizontal));
  let op2 = ImageOperation::Flip(FlipOperation::new(FlipDirection::Vertical));

  history.push(op1);
  assert_eq!(history.len(), 1);
  assert!(history.can_undo());
  assert!(!history.can_redo());

  history.push(op2);
  assert_eq!(history.len(), 2);

  assert!(history.undo());
  assert_eq!(history.len(), 1);
  assert!(history.can_redo());

  // Add new operation after undo: should truncate redo history
  let op3 = ImageOperation::Rotate(RotateOperation::new(RotationType::Cw90));
  history.push(op3);
  assert_eq!(history.len(), 2);
  assert!(!history.can_redo());
}
