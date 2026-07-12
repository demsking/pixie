use super::canvas::PixieCanvas;
use gtk4::prelude::*;
use pixie_core::{FilterType, ImageOperation, ResizeOperation};
use std::cell::Cell;
use std::rc::Rc;

pub struct ResizeToolView {
  pub container: gtk4::Box,
}

impl ResizeToolView {
  const FILTER_IDS: [&'static str; 5] = ["nearest", "triangle", "catmull", "gaussian", "lanczos3"];
  const FILTER_LABELS: [&'static str; 5] = [
    "Nearest Neighbor",
    "Triangle (Bilinear)",
    "Catmull-Rom (Bicubic)",
    "Gaussian",
    "Lanczos3 (High Quality)",
  ];

  fn filter_from_index(index: u32) -> FilterType {
    match Self::FILTER_IDS.get(index as usize).copied() {
      Some("nearest") => FilterType::Nearest,
      Some("triangle") => FilterType::Triangle,
      Some("catmull") => FilterType::CatmullRom,
      Some("gaussian") => FilterType::Gaussian,
      _ => FilterType::Lanczos3,
    }
  }

  fn format_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;

    let b = bytes as f64;
    if b >= GB {
      format!("{:.2} GB", b / GB)
    } else if b >= MB {
      format!("{:.2} MB", b / MB)
    } else if b >= KB {
      format!("{:.2} KB", b / KB)
    } else {
      format!("{bytes} B")
    }
  }

  pub fn new<F>(
    canvas: &PixieCanvas,
    current_file_size: Option<u64>,
    on_apply: F,
    on_cancel: impl Fn() + 'static,
  ) -> Self
  where
    F: Fn(ImageOperation) + 'static,
  {
    let container = gtk4::Box::builder()
      .orientation(gtk4::Orientation::Vertical)
      .spacing(12)
      .margin_top(12)
      .margin_bottom(12)
      .margin_start(12)
      .margin_end(12)
      .build();

    let title = gtk4::Label::builder()
      .label("Resize Image")
      .halign(gtk4::Align::Start)
      .css_classes(["title-3"])
      .build();
    container.append(&title);

    // Get original dimensions
    let (orig_w, orig_h) = {
      let state = canvas.state.borrow();
      state
        .image
        .as_ref()
        .map(|img| (img.width(), img.height()))
        .unwrap_or((800, 600))
    };

    // Aspect ratio lock state
    let aspect_ratio_locked = Rc::new(Cell::new(true));

    // SpinButtons
    let width_label = gtk4::Label::builder()
      .label("Width (pixels)")
      .halign(gtk4::Align::Start)
      .build();
    container.append(&width_label);

    let width_spin = gtk4::SpinButton::with_range(1.0, 16384.0, 1.0);
    width_spin.set_value(f64::from(orig_w));
    width_spin.set_hexpand(true);
    container.append(&width_spin);

    let height_label = gtk4::Label::builder()
      .label("Height (pixels)")
      .halign(gtk4::Align::Start)
      .build();
    container.append(&height_label);

    let height_spin = gtk4::SpinButton::with_range(1.0, 16384.0, 1.0);
    height_spin.set_value(f64::from(orig_h));
    height_spin.set_hexpand(true);
    container.append(&height_spin);

    // Percentage SpinButton
    let pct_label = gtk4::Label::builder()
      .label("Percentage (%)")
      .halign(gtk4::Align::Start)
      .build();
    container.append(&pct_label);

    let pct_spin = gtk4::SpinButton::with_range(1.0, 1000.0, 1.0);
    pct_spin.set_value(100.0);
    pct_spin.set_hexpand(true);
    container.append(&pct_spin);

    // Lock Aspect Ratio Switch
    let aspect_lock_box = gtk4::Box::builder()
      .orientation(gtk4::Orientation::Horizontal)
      .spacing(6)
      .build();
    let aspect_lock_label = gtk4::Label::builder()
      .label("Lock Aspect Ratio")
      .hexpand(true)
      .halign(gtk4::Align::Start)
      .build();
    let aspect_lock_switch = gtk4::Switch::builder()
      .active(true)
      .valign(gtk4::Align::Center)
      .build();
    aspect_lock_box.append(&aspect_lock_label);
    aspect_lock_box.append(&aspect_lock_switch);
    container.append(&aspect_lock_box);

    // Prevent Enlargement Switch
    let enlarge_box = gtk4::Box::builder()
      .orientation(gtk4::Orientation::Horizontal)
      .spacing(6)
      .build();
    let enlarge_label = gtk4::Label::builder()
      .label("Prevent Enlargement")
      .hexpand(true)
      .halign(gtk4::Align::Start)
      .build();
    let enlarge_switch = gtk4::Switch::builder()
      .active(false)
      .valign(gtk4::Align::Center)
      .build();
    enlarge_box.append(&enlarge_label);
    enlarge_box.append(&enlarge_switch);
    container.append(&enlarge_box);

    // Interpolation
    let filter_label = gtk4::Label::builder()
      .label("Interpolation Method")
      .halign(gtk4::Align::Start)
      .build();
    container.append(&filter_label);

    let filter_combo = gtk4::DropDown::from_strings(&Self::FILTER_LABELS);
    filter_combo.set_selected(4);
    filter_combo.set_hexpand(true);
    container.append(&filter_combo);

    // Info Display: resulting dimensions & estimated memory
    let info_box = gtk4::Box::builder()
      .orientation(gtk4::Orientation::Vertical)
      .spacing(4)
      .margin_top(8)
      .css_classes(["card"])
      .build();

    let info_dims = gtk4::Label::builder()
      .label(&format!("Output Dimensions: {orig_w} × {orig_h}"))
      .halign(gtk4::Align::Start)
      .build();
    let info_mem = gtk4::Label::builder()
      .label(&format!(
        "Estimated New File Size: {}",
        current_file_size
          .map(Self::format_size)
          .unwrap_or_else(|| "Unknown".to_string())
      ))
      .halign(gtk4::Align::Start)
      .build();
    let info_current_size = gtk4::Label::builder()
      .label(&format!(
        "Current File Size: {}",
        current_file_size
          .map(Self::format_size)
          .unwrap_or_else(|| "Unknown".to_string())
      ))
      .halign(gtk4::Align::Start)
      .build();

    info_box.append(&info_dims);
    info_box.append(&info_current_size);
    info_box.append(&info_mem);
    container.append(&info_box);

    // Mutually updating spins
    let updating = Rc::new(Cell::new(false));

    let update_info = {
      let info_dims = info_dims.clone();
      let info_mem = info_mem.clone();
      move |w: u32, h: u32| {
        info_dims.set_text(&format!("Output Dimensions: {w} × {h}"));
        let estimated_size = current_file_size.map(|size| {
          let scale = (f64::from(w) * f64::from(h)) / (f64::from(orig_w) * f64::from(orig_h));
          let scaled = ((size as f64) * scale).round();
          if scaled.is_sign_negative() {
            0
          } else {
            scaled as u64
          }
        });

        info_mem.set_text(&format!(
          "Estimated File Size: {}",
          estimated_size
            .map(Self::format_size)
            .unwrap_or_else(|| "Unknown".to_string())
        ));
      }
    };

    // Aspect lock callback
    {
      let aspect_locked = aspect_ratio_locked.clone();
      aspect_lock_switch.connect_active_notify(move |sw| {
        aspect_locked.set(sw.is_active());
      });
    }

    // Width spin handler
    {
      let updating = updating.clone();
      let height_spin = height_spin.clone();
      let pct_spin = pct_spin.clone();
      let aspect_locked = aspect_ratio_locked.clone();
      let update_info = update_info.clone();

      width_spin.connect_value_changed(move |spin| {
        if updating.get() {
          return;
        }
        updating.set(true);

        let w = spin.value() as u32;
        let h = if aspect_locked.get() {
          let calc_h = ((f64::from(w) * f64::from(orig_h)) / f64::from(orig_w)).round();
          height_spin.set_value(calc_h);
          calc_h as u32
        } else {
          height_spin.value() as u32
        };

        // Sync percentage
        let pct = (f64::from(w) / f64::from(orig_w)) * 100.0;
        pct_spin.set_value(pct);

        update_info(w, h);
        updating.set(false);
      });
    }

    // Height spin handler
    {
      let updating = updating.clone();
      let width_spin = width_spin.clone();
      let pct_spin = pct_spin.clone();
      let aspect_locked = aspect_ratio_locked.clone();
      let update_info = update_info.clone();

      height_spin.connect_value_changed(move |spin| {
        if updating.get() {
          return;
        }
        updating.set(true);

        let h = spin.value() as u32;
        let w = if aspect_locked.get() {
          let calc_w = ((f64::from(h) * f64::from(orig_w)) / f64::from(orig_h)).round();
          width_spin.set_value(calc_w);
          calc_w as u32
        } else {
          width_spin.value() as u32
        };

        // Sync percentage
        let pct = (f64::from(h) / f64::from(orig_h)) * 100.0;
        pct_spin.set_value(pct);

        update_info(w, h);
        updating.set(false);
      });
    }

    // Percentage spin handler
    {
      let updating = updating.clone();
      let width_spin = width_spin.clone();
      let height_spin = height_spin.clone();
      let update_info = update_info.clone();

      pct_spin.connect_value_changed(move |spin| {
        if updating.get() {
          return;
        }
        updating.set(true);

        let factor = spin.value() / 100.0;
        let w = (f64::from(orig_w) * factor).round() as u32;
        let h = (f64::from(orig_h) * factor).round() as u32;

        width_spin.set_value(f64::from(w));
        height_spin.set_value(f64::from(h));

        update_info(w, h);
        updating.set(false);
      });
    }

    // Actions box
    let btn_box = gtk4::Box::builder()
      .orientation(gtk4::Orientation::Horizontal)
      .spacing(6)
      .margin_top(12)
      .build();

    let apply_btn = gtk4::Button::builder()
      .label("Resize")
      .hexpand(true)
      .css_classes(["suggested-action"])
      .build();

    btn_box.append(&apply_btn);
    container.append(&btn_box);

    let cancel_cb = Rc::new(on_cancel);

    let width_spin_clone = width_spin.clone();
    let height_spin_clone = height_spin.clone();
    let enlarge_switch_clone = enlarge_switch.clone();
    apply_btn.connect_clicked(move |_| {
      let w = width_spin_clone.value() as u32;
      let h = height_spin_clone.value() as u32;

      let filter = Self::filter_from_index(filter_combo.selected());

      let prevent_enlargement = enlarge_switch_clone.is_active();
      let (target_w, target_h) = pixie_core::calculate_dimensions(
        orig_w,
        orig_h,
        w,
        h,
        aspect_ratio_locked.get(),
        prevent_enlargement,
      )
      .unwrap_or((w, h));

      let op = ImageOperation::Resize(ResizeOperation::new(target_w, target_h, filter));
      on_apply(op);
      cancel_cb();
    });

    Self { container }
  }
}
