use crate::operations::ImageOperation;

#[derive(Debug, Clone)]
pub struct EditingHistory {
  operations: Vec<ImageOperation>,
  index: usize,
  revision: u64,
}

impl EditingHistory {
  pub fn new() -> Self {
    Self {
      operations: Vec::new(),
      index: 0,
      revision: 0,
    }
  }

  pub fn push(&mut self, op: ImageOperation) {
    // Invalidate redo history when a new operation is added after an undo
    self.operations.truncate(self.index);
    self.operations.push(op);
    self.index += 1;
    self.revision += 1;
  }

  pub fn undo(&mut self) -> bool {
    if self.index > 0 {
      self.index -= 1;
      self.revision += 1;
      true
    } else {
      false
    }
  }

  pub fn redo(&mut self) -> bool {
    if self.index < self.operations.len() {
      self.index += 1;
      self.revision += 1;
      true
    } else {
      false
    }
  }

  pub fn can_undo(&self) -> bool {
    self.index > 0
  }

  pub fn can_redo(&self) -> bool {
    self.index < self.operations.len()
  }

  pub fn reset(&mut self) {
    if self.operations.is_empty() && self.index == 0 {
      return;
    }
    self.operations.clear();
    self.index = 0;
    self.revision += 1;
  }

  pub fn operations(&self) -> &[ImageOperation] {
    &self.operations[0..self.index]
  }

  pub fn len(&self) -> usize {
    self.index
  }

  pub fn is_empty(&self) -> bool {
    self.index == 0
  }

  pub fn revision(&self) -> u64 {
    self.revision
  }
}

impl Default for EditingHistory {
  fn default() -> Self {
    Self::new()
  }
}
