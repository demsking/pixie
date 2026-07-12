use super::canvas::{CropMode, PixieCanvas};
use crate::i18n::gettext;
use gtk4::prelude::*;
use std::rc::Rc;

pub struct CropToolView {
  pub container: gtk4::Box,
}

impl CropToolView {
  pub fn new(canvas: &PixieCanvas, on_start_cropping: impl Fn() + 'static) -> Self {
    let container = gtk4::Box::builder()
      .orientation(gtk4::Orientation::Vertical)
      .spacing(12)
      .margin_top(12)
      .margin_bottom(12)
      .margin_start(12)
      .margin_end(12)
      .build();

    let title = gtk4::Label::builder()
      .label(gettext("Crop Image"))
      .halign(gtk4::Align::Start)
      .css_classes(["title-3"])
      .build();
    container.append(&title);

    let orientation_subtitle = gtk4::Label::builder()
      .label(gettext("Orientation"))
      .halign(gtk4::Align::Start)
      .build();
    container.append(&orientation_subtitle);

    let orientation_row = gtk4::Box::builder()
      .orientation(gtk4::Orientation::Horizontal)
      .spacing(6)
      .build();
    let landscape_btn = gtk4::ToggleButton::builder()
      .label(gettext("Landscape"))
      .active(true)
      .hexpand(true)
      .css_classes(["crop-ratio-chip", "crop-orientation-chip"])
      .build();
    let portrait_btn = gtk4::ToggleButton::builder()
      .label(gettext("Portrait"))
      .hexpand(true)
      .css_classes(["crop-ratio-chip", "crop-orientation-chip"])
      .build();
    orientation_row.append(&landscape_btn);
    orientation_row.append(&portrait_btn);
    container.append(&orientation_row);

    let ratio_subtitle = gtk4::Label::builder()
      .label(gettext("Aspect ratio"))
      .halign(gtk4::Align::Start)
      .build();
    container.append(&ratio_subtitle);

    // Aspect ratio chips (2-column grid)
    let ratio_grid = gtk4::Grid::builder()
      .column_spacing(6)
      .row_spacing(6)
      .hexpand(true)
      .build();

    let ratio_buttons: Vec<(CropMode, gtk4::ToggleButton)> = [
      (CropMode::Free, gettext("Free")),
      (CropMode::Original, gettext("Original")),
      (CropMode::Square11, gettext("Square")),
      (CropMode::Circle, gettext("Circle")),
      (CropMode::Ratio54, "5:4".to_string()),
      (CropMode::Ratio43, "4:3".to_string()),
      (CropMode::Ratio32, "3:2".to_string()),
      (CropMode::Ratio169, "16:9".to_string()),
    ]
    .into_iter()
    .map(|(mode, label)| {
      let btn = gtk4::ToggleButton::builder()
        .label(label)
        .hexpand(true)
        .css_classes(["crop-ratio-chip"])
        .build();
      (mode, btn)
    })
    .collect();

    for (index, (_, btn)) in ratio_buttons.iter().enumerate() {
      let col = (index % 2) as i32;
      let row = (index / 2) as i32;
      ratio_grid.attach(btn, col, row, 1, 1);
    }
    container.append(&ratio_grid);

    // Connect ratio chips to canvas
    if let Some((_, btn)) = ratio_buttons.first() {
      btn.set_active(true);
    }

    let ratio_buttons = Rc::new(ratio_buttons);
    let canvas_clone = canvas.state.clone();
    let drawing_area = canvas.drawing_area.clone();
    for (mode, btn) in ratio_buttons.iter() {
      let mode = *mode;
      let all_buttons = ratio_buttons.clone();
      let canvas_state = canvas_clone.clone();
      let area = drawing_area.clone();
      btn.connect_clicked(move |clicked| {
        if !clicked.is_active() {
          clicked.set_active(true);
          return;
        }

        for (_, other) in all_buttons.iter() {
          if other != clicked {
            other.set_active(false);
          }
        }

        let mut state = canvas_state.borrow_mut();
        state.crop_mode = mode;
        if let Some(ref img) = state.image {
          if let Some(rect) = state.crop_rect {
            state.crop_rect = Some(super::canvas::apply_ratio_to_rect(
              rect,
              mode,
              img.width(),
              img.height(),
              state.crop_portrait,
            ));
          }
        }
        drop(state);
        area.queue_draw();
      });
    }

    let set_ratio_labels = |portrait: bool, buttons: &Rc<Vec<(CropMode, gtk4::ToggleButton)>>| {
      for (mode, btn) in buttons.iter() {
        match mode {
          CropMode::Ratio54 => btn.set_label(if portrait { "4:5" } else { "5:4" }),
          CropMode::Ratio43 => btn.set_label(if portrait { "3:4" } else { "4:3" }),
          CropMode::Ratio32 => btn.set_label(if portrait { "2:3" } else { "3:2" }),
          CropMode::Ratio169 => btn.set_label(if portrait { "9:16" } else { "16:9" }),
          _ => {}
        }
      }
    };

    let ratio_buttons_for_landscape = ratio_buttons.clone();
    let canvas_landscape = canvas.state.clone();
    let area_landscape = canvas.drawing_area.clone();
    let portrait_btn_cl = portrait_btn.clone();
    landscape_btn.connect_clicked(move |btn| {
      if !btn.is_active() {
        btn.set_active(true);
      }
      portrait_btn_cl.set_active(false);
      set_ratio_labels(false, &ratio_buttons_for_landscape);

      let mut state = canvas_landscape.borrow_mut();
      state.crop_portrait = false;
      if let Some(ref img) = state.image {
        if let Some(rect) = state.crop_rect {
          state.crop_rect = Some(super::canvas::apply_ratio_to_rect(
            rect,
            state.crop_mode,
            img.width(),
            img.height(),
            false,
          ));
        }
      }
      drop(state);
      area_landscape.queue_draw();
    });

    let ratio_buttons_for_portrait = ratio_buttons.clone();
    let canvas_portrait = canvas.state.clone();
    let area_portrait = canvas.drawing_area.clone();
    let landscape_btn_cl = landscape_btn.clone();
    portrait_btn.connect_clicked(move |btn| {
      if !btn.is_active() {
        btn.set_active(true);
      }
      landscape_btn_cl.set_active(false);
      set_ratio_labels(true, &ratio_buttons_for_portrait);

      let mut state = canvas_portrait.borrow_mut();
      state.crop_portrait = true;
      if let Some(ref img) = state.image {
        if let Some(rect) = state.crop_rect {
          state.crop_rect = Some(super::canvas::apply_ratio_to_rect(
            rect,
            state.crop_mode,
            img.width(),
            img.height(),
            true,
          ));
        }
      }
      drop(state);
      area_portrait.queue_draw();
    });

    let start_cropping_btn = gtk4::Button::builder()
      .label(gettext("Start Cropping"))
      .hexpand(true)
      .css_classes(["suggested-action", "pill"])
      .build();
    start_cropping_btn.connect_clicked(move |_| {
      on_start_cropping();
    });
    container.append(&start_cropping_btn);

    Self { container }
  }
}
