use crate::i18n::{gettext, gettext_replace};
use gtk4::prelude::*;
use pixie_core::{ExportOptions, ImageFormat};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

pub struct ExportDialog {
  pub window: gtk4::Window,
}

impl ExportDialog {
  const FORMAT_IDS: [&'static str; 7] = ["png", "jpeg", "webp", "avif", "bmp", "tiff", "gif"];
  fn format_labels() -> [String; 7] {
    [
      gettext("PNG Image"),
      gettext("JPEG Image"),
      gettext("WebP Image"),
      gettext("AVIF Image"),
      gettext("BMP Image"),
      gettext("TIFF Image"),
      gettext("GIF (Static)"),
    ]
  }

  fn extension_for_format(format_id: &str) -> &'static str {
    match format_id {
      "jpeg" => "jpg",
      "webp" => "webp",
      "avif" => "avif",
      "bmp" => "bmp",
      "tiff" => "tiff",
      "gif" => "gif",
      _ => "png",
    }
  }

  fn suggested_destination_path(source_path: Option<&PathBuf>, format_id: &str) -> PathBuf {
    let extension = Self::extension_for_format(format_id);

    if let Some(path) = source_path {
      let parent = path.parent().map(|p| p.to_path_buf()).unwrap_or_default();
      let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy())
        .unwrap_or_else(|| "image".into());
      return parent.join(format!("{stem}-converted.{extension}"));
    }

    PathBuf::from(format!("converted.{extension}"))
  }

  fn format_index(format_id: &str) -> u32 {
    Self::FORMAT_IDS
      .iter()
      .position(|id| *id == format_id)
      .unwrap_or(0) as u32
  }

  fn format_id_for_index(index: u32) -> &'static str {
    Self::FORMAT_IDS
      .get(index as usize)
      .copied()
      .unwrap_or("png")
  }

  fn localize_conversion_warning(warning: &str) -> String {
    if let Some(format) = warning
      .strip_prefix("Destination format ")
      .and_then(|rest| {
        rest.strip_suffix(" does not support transparency. Alpha channel will be flattened.")
      })
    {
      gettext_replace(
        "Destination format {format} does not support transparency. Alpha channel will be flattened.",
        &[("format", format.to_string())],
      )
    } else {
      match warning {
        "JPEG is a lossy format. Saving may introduce quality degradation." => {
          gettext("JPEG is a lossy format. Saving may introduce quality degradation.")
        }
        "BMP does not support compressed saving, resulting in large files. Metadata may be lost." => {
          gettext(
            "BMP does not support compressed saving, resulting in large files. Metadata may be lost.",
          )
        }
        "Original image might be an animation. Converting will lose all frames except the first static frame." => {
          gettext(
            "Original image might be an animation. Converting will lose all frames except the first static frame.",
          )
        }
        _ => warning.to_string(),
      }
    }
  }

  pub fn new<F>(
    parent: &gtk4::Window,
    has_transparency: bool,
    original_format: ImageFormat,
    source_path: Option<PathBuf>,
    on_export: F,
  ) -> Self
  where
    F: Fn(PathBuf, ExportOptions) + 'static,
  {
    let window = gtk4::Window::builder()
      .title(gettext("Convert Image"))
      .transient_for(parent)
      .modal(true)
      .destroy_with_parent(true)
      .default_width(360)
      .resizable(false)
      .build();

    let root_box = gtk4::Box::builder()
      .orientation(gtk4::Orientation::Vertical)
      .spacing(12)
      .margin_top(16)
      .margin_bottom(16)
      .margin_start(16)
      .margin_end(16)
      .build();
    window.set_child(Some(&root_box));

    // Format Selector
    let format_label = gtk4::Label::builder()
      .label(gettext("Target Format"))
      .halign(gtk4::Align::Start)
      .build();
    root_box.append(&format_label);

    let format_labels = Self::format_labels();
    let format_label_refs: Vec<&str> = format_labels.iter().map(String::as_str).collect();
    let format_combo = gtk4::DropDown::from_strings(&format_label_refs);
    format_combo.set_hexpand(true);

    // Set default format to match original
    let default_id = match original_format {
      ImageFormat::Png => "png",
      ImageFormat::Jpeg => "jpeg",
      ImageFormat::Webp => "webp",
      ImageFormat::Avif => "avif",
      ImageFormat::Bmp => "bmp",
      ImageFormat::Tiff => "tiff",
      ImageFormat::Gif => "gif",
    };
    format_combo.set_selected(Self::format_index(default_id));
    root_box.append(&format_combo);

    // Destination Selector
    let destination_label = gtk4::Label::builder()
      .label(gettext("Save Destination"))
      .halign(gtk4::Align::Start)
      .build();
    root_box.append(&destination_label);

    let destination_row = gtk4::Box::builder()
      .orientation(gtk4::Orientation::Horizontal)
      .spacing(6)
      .build();

    let destination_entry = gtk4::Entry::builder()
      .editable(false)
      .hexpand(true)
      .placeholder_text(gettext("Select destination file"))
      .build();
    destination_row.append(&destination_entry);

    let destination_browse_btn = gtk4::Button::with_label(&gettext("Browse..."));
    destination_row.append(&destination_browse_btn);

    root_box.append(&destination_row);

    let destination_error_label = gtk4::Label::builder()
      .wrap(true)
      .halign(gtk4::Align::Start)
      .css_classes(["error"])
      .visible(false)
      .build();
    root_box.append(&destination_error_label);

    let selected_destination = Rc::new(RefCell::new(None::<PathBuf>));
    let suggested_path = Self::suggested_destination_path(source_path.as_ref(), default_id);
    destination_entry.set_text(&suggested_path.to_string_lossy());
    *selected_destination.borrow_mut() = Some(suggested_path);

    // Settings stack/box
    let settings_box = gtk4::Box::builder()
      .orientation(gtk4::Orientation::Vertical)
      .spacing(8)
      .build();
    root_box.append(&settings_box);

    // Warning Label
    let warning_label = gtk4::Label::builder()
      .wrap(true)
      .halign(gtk4::Align::Start)
      .css_classes(["error"])
      .build();
    root_box.append(&warning_label);

    // Buttons
    let btn_box = gtk4::Box::builder()
      .orientation(gtk4::Orientation::Horizontal)
      .spacing(6)
      .halign(gtk4::Align::End)
      .margin_top(12)
      .build();
    let cancel_btn = gtk4::Button::with_label(&gettext("Cancel"));
    let export_btn = gtk4::Button::builder()
      .label(gettext("Convert"))
      .css_classes(["suggested-action"])
      .build();
    btn_box.append(&cancel_btn);
    btn_box.append(&export_btn);
    root_box.append(&btn_box);

    // Dynamic Settings UI elements
    let jpeg_quality_scale =
      gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 10.0, 100.0, 1.0);
    jpeg_quality_scale.set_value(90.0);
    let jpeg_progressive_switch = gtk4::Switch::builder().active(true).build();
    let jpeg_bg_color = gtk4::ColorDialogButton::new(Some(gtk4::ColorDialog::new()));
    jpeg_bg_color.set_rgba(&gtk4::gdk::RGBA::WHITE);

    let png_compression_scale =
      gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 0.0, 9.0, 1.0);
    png_compression_scale.set_value(6.0);
    let png_transparency_switch = gtk4::Switch::builder().active(true).build();

    let webp_lossless_switch = gtk4::Switch::builder().active(false).build();
    let webp_quality_scale =
      gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 10.0, 100.0, 1.0);
    webp_quality_scale.set_value(80.0);

    let avif_quality_scale =
      gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 10.0, 100.0, 1.0);
    avif_quality_scale.set_value(75.0);
    let avif_speed_scale = gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 1.0, 10.0, 1.0);
    avif_speed_scale.set_value(6.0);

    let bmp_bg_box = gtk4::Box::builder()
      .orientation(gtk4::Orientation::Horizontal)
      .spacing(6)
      .build();
    bmp_bg_box.append(
      &gtk4::Label::builder()
        .label(gettext("Background color"))
        .hexpand(true)
        .halign(gtk4::Align::Start)
        .build(),
    );
    let bmp_bg_color = gtk4::ColorDialogButton::new(Some(gtk4::ColorDialog::new()));
    bmp_bg_color.set_rgba(&gtk4::gdk::RGBA::WHITE);
    bmp_bg_box.append(&bmp_bg_color);

    let on_format_changed = {
      let settings_box = settings_box.clone();
      let warning_label = warning_label.clone();

      // jpeg inputs
      let jpeg_quality_scale = jpeg_quality_scale.clone();
      let jpeg_progressive_switch = jpeg_progressive_switch.clone();
      let jpeg_bg_color = jpeg_bg_color.clone();

      // png inputs
      let png_compression_scale = png_compression_scale.clone();
      let png_transparency_switch = png_transparency_switch.clone();

      // webp inputs
      let webp_lossless_switch = webp_lossless_switch.clone();
      let webp_quality_scale = webp_quality_scale.clone();

      // avif inputs
      let avif_quality_scale = avif_quality_scale.clone();
      let avif_speed_scale = avif_speed_scale.clone();

      // bmp inputs
      let bmp_bg_box = bmp_bg_box.clone();

      // destination inputs
      let selected_destination = selected_destination.clone();
      let destination_entry = destination_entry.clone();
      let source_path = source_path.clone();

      move |combo_id: &str| {
        // Clear settings
        while let Some(child) = settings_box.first_child() {
          settings_box.remove(&child);
        }

        let target_format = match combo_id {
          "jpeg" => ImageFormat::Jpeg,
          "webp" => ImageFormat::Webp,
          "avif" => ImageFormat::Avif,
          "bmp" => ImageFormat::Bmp,
          "tiff" => ImageFormat::Tiff,
          "gif" => ImageFormat::Gif,
          _ => ImageFormat::Png,
        };

        // Generate warnings
        let warnings =
          ImageFormat::check_conversion_warnings(original_format, target_format, has_transparency);
        if warnings.is_empty() {
          warning_label.set_text("");
          warning_label.set_visible(false);
        } else {
          let warnings = warnings
            .iter()
            .map(|warning| Self::localize_conversion_warning(warning))
            .collect::<Vec<_>>();
          warning_label.set_text(&warnings.join("\n"));
          warning_label.set_visible(true);
        }

        match combo_id {
          "jpeg" => {
            let l1 = gtk4::Label::builder()
              .label(gettext("Quality (10 - 100)"))
              .halign(gtk4::Align::Start)
              .build();
            settings_box.append(&l1);
            settings_box.append(&jpeg_quality_scale);

            let b1 = gtk4::Box::builder()
              .orientation(gtk4::Orientation::Horizontal)
              .spacing(6)
              .build();
            b1.append(
              &gtk4::Label::builder()
                .label(gettext("Progressive Encoding"))
                .hexpand(true)
                .halign(gtk4::Align::Start)
                .build(),
            );
            b1.append(&jpeg_progressive_switch);
            settings_box.append(&b1);

            let b2 = gtk4::Box::builder()
              .orientation(gtk4::Orientation::Horizontal)
              .spacing(6)
              .build();
            b2.append(
              &gtk4::Label::builder()
                .label(gettext("Background for transparent areas"))
                .hexpand(true)
                .halign(gtk4::Align::Start)
                .build(),
            );
            b2.append(&jpeg_bg_color);
            settings_box.append(&b2);
          }
          "png" => {
            let l1 = gtk4::Label::builder()
              .label(gettext("Compression Level (0 - 9)"))
              .halign(gtk4::Align::Start)
              .build();
            settings_box.append(&l1);
            settings_box.append(&png_compression_scale);

            let b1 = gtk4::Box::builder()
              .orientation(gtk4::Orientation::Horizontal)
              .spacing(6)
              .build();
            b1.append(
              &gtk4::Label::builder()
                .label(gettext("Preserve Transparency"))
                .hexpand(true)
                .halign(gtk4::Align::Start)
                .build(),
            );
            b1.append(&png_transparency_switch);
            settings_box.append(&b1);
          }
          "webp" => {
            let b1 = gtk4::Box::builder()
              .orientation(gtk4::Orientation::Horizontal)
              .spacing(6)
              .build();
            b1.append(
              &gtk4::Label::builder()
                .label(gettext("Lossless Mode"))
                .hexpand(true)
                .halign(gtk4::Align::Start)
                .build(),
            );
            b1.append(&webp_lossless_switch);
            settings_box.append(&b1);

            let l1 = gtk4::Label::builder()
              .label(gettext("Quality"))
              .halign(gtk4::Align::Start)
              .build();
            settings_box.append(&l1);
            settings_box.append(&webp_quality_scale);
          }
          "avif" => {
            let l1 = gtk4::Label::builder()
              .label(gettext("Quality"))
              .halign(gtk4::Align::Start)
              .build();
            settings_box.append(&l1);
            settings_box.append(&avif_quality_scale);

            let l2 = gtk4::Label::builder()
              .label(gettext("Speed/Effort (1 - 10)"))
              .halign(gtk4::Align::Start)
              .build();
            settings_box.append(&l2);
            settings_box.append(&avif_speed_scale);
          }
          "bmp" => {
            settings_box.append(&bmp_bg_box);
          }
          _ => {}
        }

        // Keep destination extension in sync with selected target format.
        let current_destination = selected_destination.borrow().clone();
        if let Some(current_path) = current_destination {
          let mut updated_path = current_path;
          updated_path.set_extension(Self::extension_for_format(combo_id));
          destination_entry.set_text(&updated_path.to_string_lossy());
          *selected_destination.borrow_mut() = Some(updated_path);
        } else {
          let suggested = Self::suggested_destination_path(source_path.as_ref(), combo_id);
          destination_entry.set_text(&suggested.to_string_lossy());
          *selected_destination.borrow_mut() = Some(suggested);
        }
      }
    };

    // Initialize Settings
    on_format_changed(Self::format_id_for_index(format_combo.selected()));

    format_combo.connect_selected_notify(move |cb| {
      on_format_changed(Self::format_id_for_index(cb.selected()));
    });

    let win_clone3 = window.clone();
    let format_combo_for_browse = format_combo.clone();
    let destination_entry_for_browse = destination_entry.clone();
    let destination_error_for_browse = destination_error_label.clone();
    let selected_destination_for_browse = selected_destination.clone();
    destination_browse_btn.connect_clicked(move |_| {
      let format_id = Self::format_id_for_index(format_combo_for_browse.selected());

      let dialog = gtk4::FileDialog::new();
      dialog.set_title(&gettext("Select Save Destination"));

      if let Some(path) = selected_destination_for_browse.borrow().clone() {
        if let Some(name) = path.file_name() {
          dialog.set_initial_name(Some(&name.to_string_lossy()));
        }
        if let Some(parent) = path.parent() {
          dialog.set_initial_folder(Some(&gio::File::for_path(parent)));
        }
      }

      let filter_list = gio::ListStore::new::<gtk4::FileFilter>();
      let filter = gtk4::FileFilter::new();
      filter.set_name(Some(&gettext("Selected Format")));

      match format_id {
        "jpeg" => filter.add_mime_type("image/jpeg"),
        "webp" => filter.add_mime_type("image/webp"),
        "avif" => filter.add_mime_type("image/avif"),
        "bmp" => filter.add_mime_type("image/bmp"),
        "tiff" => filter.add_mime_type("image/tiff"),
        "gif" => filter.add_mime_type("image/gif"),
        _ => filter.add_mime_type("image/png"),
      }

      filter_list.append(&filter);
      dialog.set_filters(Some(&filter_list));

      let destination_entry_cl = destination_entry_for_browse.clone();
      let destination_error_cl = destination_error_for_browse.clone();
      let selected_destination_cl = selected_destination_for_browse.clone();
      dialog.save(
        Some(&win_clone3),
        None::<&gio::Cancellable>,
        move |result| {
          if let Ok(file) = result {
            if let Some(path) = file.path() {
              destination_entry_cl.set_text(&path.to_string_lossy());
              destination_error_cl.set_visible(false);
              destination_error_cl.set_text("");
              *selected_destination_cl.borrow_mut() = Some(path);
            }
          }
        },
      );
    });

    // Close window handlers
    let win_clone = window.clone();
    cancel_btn.connect_clicked(move |_| {
      win_clone.destroy();
    });

    let format_combo_clone = format_combo.clone();
    let win_clone2 = window.clone();
    let destination_error_for_export = destination_error_label.clone();
    let selected_destination_for_export = selected_destination.clone();
    let on_export_rc = Rc::new(on_export);

    export_btn.connect_clicked(move |_| {
      let format_id = Self::format_id_for_index(format_combo_clone.selected());

      // Gather export options
      let options = match format_id {
        "jpeg" => {
          let rgba = jpeg_bg_color.rgba();
          ExportOptions::Jpeg {
            quality: jpeg_quality_scale.value() as u8,
            progressive: jpeg_progressive_switch.is_active(),
            background_color: [
              (rgba.red() * 255.0).round() as u8,
              (rgba.green() * 255.0).round() as u8,
              (rgba.blue() * 255.0).round() as u8,
            ],
          }
        }
        "webp" => ExportOptions::Webp {
          lossless: webp_lossless_switch.is_active(),
          quality: webp_quality_scale.value() as f32,
        },
        "avif" => ExportOptions::Avif {
          quality: avif_quality_scale.value() as u8,
          speed: avif_speed_scale.value() as u8,
        },
        "bmp" => {
          let rgba = bmp_bg_color.rgba();
          ExportOptions::Bmp {
            background_color: Some([
              (rgba.red() * 255.0).round() as u8,
              (rgba.green() * 255.0).round() as u8,
              (rgba.blue() * 255.0).round() as u8,
            ]),
          }
        }
        "tiff" => ExportOptions::Tiff,
        "gif" => ExportOptions::Gif,
        _ => ExportOptions::Png {
          compression: png_compression_scale.value() as u8,
          preserve_transparency: png_transparency_switch.is_active(),
        },
      };

      if let Some(path) = selected_destination_for_export.borrow().clone() {
        destination_error_for_export.set_visible(false);
        destination_error_for_export.set_text("");
        on_export_rc(path, options.clone());
        win_clone2.destroy();
      } else {
        destination_error_for_export.set_text(&gettext("Please select a save destination."));
        destination_error_for_export.set_visible(true);
      }
    });

    Self { window }
  }

  pub fn show(&self) {
    self.window.present();
  }
}
