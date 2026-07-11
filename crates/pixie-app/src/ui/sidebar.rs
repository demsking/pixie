use gtk4::prelude::*;

pub struct PixieSidebar {
  pub container: gtk4::Box,
}

impl PixieSidebar {
  pub fn new() -> Self {
    let container = gtk4::Box::builder()
      .orientation(gtk4::Orientation::Vertical)
      .width_request(280)
      .css_classes(["sidebar-box"])
      .visible(false)
      .build();

    Self { container }
  }

  pub fn clear(&self) {
    while let Some(child) = self.container.first_child() {
      self.container.remove(&child);
    }
    self.container.set_visible(false);
  }
}
