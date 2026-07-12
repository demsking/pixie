use gtk4::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;
use pixie_core::{
  CircleMaskOperation, CropOperation, ExportOptions, FlipDirection, FlipOperation, ImageOperation,
  PixieDocument, RotateOperation, RotationType,
};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use super::about_dialog::show_about_dialog;
use super::canvas::PixieCanvas;
use super::crop_tool::CropToolView;
use super::export_dialog::ExportDialog;
use super::resize_tool::ResizeToolView;
use super::sidebar::PixieSidebar;
use crate::workers::{ImageWorker, WorkerOutput};

pub struct PixieWindow {
  pub window: adw::ApplicationWindow,
  canvas: Rc<PixieCanvas>,
  sidebar: Rc<PixieSidebar>,
  active_tool_popover: Rc<RefCell<Option<gtk4::Popover>>>,

  // State
  document: Rc<RefCell<Option<PixieDocument>>>,
  current_file_path: Rc<RefCell<Option<PathBuf>>>,
  worker: Rc<ImageWorker>,
  saved_history_index: Rc<std::cell::Cell<Option<usize>>>,
  saved_metadata_policy: Rc<std::cell::Cell<pixie_core::MetadataPolicy>>,
  force_close: Rc<std::cell::Cell<bool>>,
  alt_pressed: std::cell::Cell<bool>,
  pub resize_to_image_on_load: std::cell::Cell<bool>,

  // Folder navigation state
  folder_images: Rc<RefCell<Vec<PathBuf>>>,
  folder_index: Rc<RefCell<usize>>,

  // Toast/Notifications tracking
  active_toasts: Rc<RefCell<Vec<adw::Toast>>>,

  // UI components
  progress_bar: gtk4::ProgressBar,
  toast_overlay: adw::ToastOverlay,
  crop_btn: gtk4::ToggleButton,
  resize_btn: gtk4::ToggleButton,
  metadata_btn: gtk4::MenuButton,
  rotate_btn: gtk4::Button,
  mirror_btn: gtk4::Button,
  flip_btn: gtk4::Button,
  crop_actions_bar: gtk4::Box,
  undo_btn: gtk4::Button,
  redo_btn: gtk4::Button,
  save_btn: gtk4::Button,
  export_btn: gtk4::Button,
  nav_prev_btn: gtk4::Button,
  nav_next_btn: gtk4::Button,
  zoom_badge_btn: gtk4::Button,
  scrolled: gtk4::ScrolledWindow,
  stack: gtk4::Stack,
}

impl PixieWindow {
  pub fn new(app: &adw::Application) -> Rc<Self> {
    let window = adw::ApplicationWindow::builder()
      .application(app)
      .title("Pixie")
      .icon_name("io.gitlab.demsking.Pixie")
      .default_width(1000)
      .default_height(700)
      .build();

    // Main layout containing ToastOverlay
    let toast_overlay = adw::ToastOverlay::new();
    window.set_content(Some(&toast_overlay));

    // Leaflet/Box container for sidebar and canvas
    let main_box = gtk4::Box::builder()
      .orientation(gtk4::Orientation::Vertical)
      .build();
    toast_overlay.set_child(Some(&main_box));

    // ── Header Bar ──────────────────────────────────────────────────────
    let header_bar = adw::HeaderBar::new();
    main_box.append(&header_bar);

    // Left: Toggle fullscreen button
    let fullscreen_btn = gtk4::Button::builder()
      .icon_name("view-fullscreen-symbolic")
      .tooltip_text("Toggle Fullscreen")
      .build();
    header_bar.pack_start(&fullscreen_btn);

    let update_fullscreen_btn = |btn: &gtk4::Button, is_fullscreen: bool| {
      if is_fullscreen {
        btn.set_icon_name("view-restore-symbolic");
        btn.set_tooltip_text(Some("Exit Fullscreen"));
      } else {
        btn.set_icon_name("view-fullscreen-symbolic");
        btn.set_tooltip_text(Some("Enter Fullscreen"));
      }
    };
    update_fullscreen_btn(&fullscreen_btn, window.is_fullscreen());

    // Left: Undo / Redo
    let undo_btn = gtk4::Button::builder()
      .icon_name("edit-undo-symbolic")
      .tooltip_text("Undo (Ctrl+Z)")
      .sensitive(false)
      .build();
    let redo_btn = gtk4::Button::builder()
      .icon_name("edit-redo-symbolic")
      .tooltip_text("Redo (Ctrl+Shift+Z)")
      .sensitive(false)
      .build();
    header_bar.pack_start(&undo_btn);
    header_bar.pack_start(&redo_btn);

    // File actions remain available through the app menu.
    let save_btn = gtk4::Button::builder()
      .icon_name("document-save-symbolic")
      .tooltip_text("Save (Ctrl+S)")
      .sensitive(false)
      .build();
    let export_btn = gtk4::Button::builder()
      .icon_name("document-send-symbolic")
      .tooltip_text("Convert… (Ctrl+Alt+S)")
      .sensitive(false)
      .build();

    // Floating tool buttons
    let crop_btn = gtk4::ToggleButton::builder()
      .icon_name("edit-cut-symbolic")
      .tooltip_text("Crop Image")
      .sensitive(false)
      .css_classes(["floating-toolbar__button"])
      .build();

    let resize_btn = gtk4::ToggleButton::builder()
      .icon_name("zoom-fit-best-symbolic")
      .tooltip_text("Resize Image")
      .sensitive(false)
      .css_classes(["floating-toolbar__button"])
      .build();

    let metadata_btn = gtk4::MenuButton::builder()
      .icon_name("security-high-symbolic")
      .tooltip_text("Metadata Cleaner")
      .sensitive(false)
      .css_classes(["floating-toolbar__button"])
      .build();

    let rotate_btn = gtk4::Button::builder()
      .icon_name("object-rotate-right-symbolic")
      .tooltip_text("Rotate image to the right")
      .sensitive(false)
      .css_classes(["floating-toolbar__button"])
      .build();

    let mirror_btn = gtk4::Button::builder()
      .icon_name("object-flip-horizontal-symbolic")
      .tooltip_text("Mirror")
      .sensitive(false)
      .css_classes(["floating-toolbar__button"])
      .build();

    let flip_btn = gtk4::Button::builder()
      .icon_name("object-flip-vertical-symbolic")
      .tooltip_text("Flip")
      .sensitive(false)
      .css_classes(["floating-toolbar__button"])
      .build();

    // Right: App menu button
    let app_menu = gio::Menu::new();

    // Section 1: file ops
    let file_section = gio::Menu::new();
    file_section.append(Some("New Window"), Some("app.new-window"));
    file_section.append(Some("Open…"), Some("win.open"));
    app_menu.append_section(None, &file_section);

    // Section 2: image ops
    let image_section = gio::Menu::new();
    image_section.append(Some("Open With…"), Some("win.open-with"));
    image_section.append(Some("Print…"), Some("win.print"));
    app_menu.append_section(None, &image_section);

    // Section 3: save/export ops
    let save_section = gio::Menu::new();
    save_section.append(Some("Save"), Some("win.save"));
    save_section.append(Some("Save As…"), Some("win.save-as"));
    save_section.append(Some("Convert…"), Some("win.export"));
    app_menu.append_section(None, &save_section);

    // Section 4: view/fullscreen ops
    let view_section = gio::Menu::new();
    view_section.append(Some("Toggle Fullscreen"), Some("win.toggle-fullscreen"));
    app_menu.append_section(None, &view_section);

    // Section 5: help
    let help_section = gio::Menu::new();
    help_section.append(Some("Keyboard Shortcuts"), Some("win.show-help-overlay"));
    help_section.append(Some("About Pixie"), Some("win.about"));
    app_menu.append_section(None, &help_section);

    let menu_btn = gtk4::MenuButton::builder()
      .icon_name("open-menu-symbolic")
      .tooltip_text("Main Menu")
      .menu_model(&app_menu)
      .build();
    header_bar.pack_end(&menu_btn);

    let zoom_badge_btn = gtk4::Button::builder()
      .label("100%")
      .visible(false)
      .css_classes(["flat", "zoom-badge"])
      .build();
    header_bar.pack_end(&zoom_badge_btn);

    // ── Central area: Canvas (with overlay) & Sidebar ───────────────────
    let center_box = gtk4::Box::builder()
      .orientation(gtk4::Orientation::Horizontal)
      .hexpand(true)
      .vexpand(true)
      .build();
    main_box.append(&center_box);

    let canvas = Rc::new(PixieCanvas::new());
    let scrolled = gtk4::ScrolledWindow::builder()
      .hexpand(true)
      .vexpand(true)
      .child(&canvas.drawing_area)
      .build();

    // Overlay wrapping the scrolled canvas so we can place floating bars
    let overlay = gtk4::Overlay::new();
    overlay.set_child(Some(&scrolled));
    overlay.set_hexpand(true);
    overlay.set_vexpand(true);

    let open_btn = gtk4::Button::builder()
      .label("Open Files…")
      .halign(gtk4::Align::Center)
      .css_classes(["suggested-action", "pill"])
      .build();

    let status_page = adw::StatusPage::builder()
      .icon_name("image-x-generic-symbolic")
      .title("View and Edit Images")
      .description("Drag and drop images here")
      .child(&open_btn)
      .build();

    let stack = gtk4::Stack::builder()
      .hexpand(true)
      .vexpand(true)
      .transition_type(gtk4::StackTransitionType::Crossfade)
      .transition_duration(200)
      .build();

    stack.add_named(&overlay, Some("canvas"));
    stack.add_named(&status_page, Some("placeholder"));
    stack.set_visible_child_name("placeholder");

    center_box.append(&stack);

    // ── Floating navigation bar (bottom-left) ───────────────────────────
    let nav_bar = gtk4::Box::builder()
      .orientation(gtk4::Orientation::Horizontal)
      .spacing(0)
      .margin_bottom(16)
      .margin_start(16)
      .halign(gtk4::Align::Start)
      .valign(gtk4::Align::End)
      .css_classes(["floating-toolbar"])
      .build();

    let nav_prev_btn = gtk4::Button::builder()
      .icon_name("go-previous-symbolic")
      .tooltip_text("Previous Image")
      .sensitive(false)
      .css_classes(["floating-toolbar__button"])
      .build();
    let nav_next_btn = gtk4::Button::builder()
      .icon_name("go-next-symbolic")
      .tooltip_text("Next Image")
      .sensitive(false)
      .css_classes(["floating-toolbar__button"])
      .build();
    nav_bar.append(&nav_prev_btn);
    nav_bar.append(&nav_next_btn);
    overlay.add_overlay(&nav_bar);

    // ── Floating tool bar (bottom-right) ─────────────────────────────
    let tool_bar = gtk4::Box::builder()
      .orientation(gtk4::Orientation::Horizontal)
      .spacing(6)
      .margin_bottom(16)
      .margin_end(16)
      .halign(gtk4::Align::End)
      .valign(gtk4::Align::End)
      .css_classes(["floating-toolbar"])
      .build();

    tool_bar.append(&crop_btn);
    tool_bar.append(&resize_btn);
    tool_bar.append(&metadata_btn);
    tool_bar.append(&rotate_btn);
    tool_bar.append(&mirror_btn);
    tool_bar.append(&flip_btn);
    overlay.add_overlay(&tool_bar);

    // ── Floating crop actions bar (bottom-center) ─────────────────────
    let crop_actions_bar = gtk4::Box::builder()
      .orientation(gtk4::Orientation::Horizontal)
      .spacing(6)
      .margin_bottom(16)
      .halign(gtk4::Align::Center)
      .valign(gtk4::Align::End)
      .css_classes(["floating-toolbar"])
      .visible(false)
      .build();
    let crop_cancel_btn = gtk4::Button::builder()
      .label("Cancel")
      .css_classes(["floating-toolbar__button", "crop-action-btn"])
      .build();
    let crop_apply_btn = gtk4::Button::builder()
      .label("Apply")
      .css_classes([
        "floating-toolbar__button",
        "crop-action-btn",
        "suggested-action",
      ])
      .build();
    crop_actions_bar.append(&crop_cancel_btn);
    crop_actions_bar.append(&crop_apply_btn);
    overlay.add_overlay(&crop_actions_bar);

    // ── Progress bar (top-left, full width) ─────────────────────────────
    let progress_bar = gtk4::ProgressBar::builder()
      .valign(gtk4::Align::Start)
      .halign(gtk4::Align::Fill)
      .hexpand(true)
      .height_request(6)
      .visible(false)
      .build();

    overlay.add_overlay(&progress_bar);

    let sidebar = Rc::new(PixieSidebar::new());
    sidebar.container.set_halign(gtk4::Align::End);
    sidebar.container.set_valign(gtk4::Align::Fill);
    sidebar.container.set_margin_end(16);
    sidebar.container.set_margin_top(16);
    // Keep sidebar options panel above floating bottom controls.
    sidebar.container.set_margin_bottom(84);
    sidebar.container.set_css_classes(&["floating-sidebar"]);
    overlay.add_overlay(&sidebar.container);

    // ── Background worker channel ────────────────────────────────────────
    let (tx, rx) = async_channel::unbounded::<(u64, WorkerOutput)>();
    let worker = Rc::new(ImageWorker::new(tx));

    let document = Rc::new(RefCell::new(None::<PixieDocument>));
    let current_file_path = Rc::new(RefCell::new(None::<PathBuf>));
    let active_tool_popover = Rc::new(RefCell::new(None::<gtk4::Popover>));
    let folder_images: Rc<RefCell<Vec<PathBuf>>> = Rc::new(RefCell::new(Vec::new()));
    let folder_index: Rc<RefCell<usize>> = Rc::new(RefCell::new(0));
    let saved_history_index = Rc::new(std::cell::Cell::new(None));
    let saved_metadata_policy =
      Rc::new(std::cell::Cell::new(pixie_core::MetadataPolicy::default()));
    let force_close = Rc::new(std::cell::Cell::new(false));
    let active_toasts = Rc::new(RefCell::new(Vec::<adw::Toast>::new()));

    let win_struct = Rc::new(Self {
      window: window.clone(),
      canvas: canvas.clone(),
      sidebar: sidebar.clone(),
      active_tool_popover: active_tool_popover.clone(),
      document: document.clone(),
      current_file_path: current_file_path.clone(),
      worker: worker.clone(),
      saved_history_index: saved_history_index.clone(),
      saved_metadata_policy: saved_metadata_policy.clone(),
      force_close: force_close.clone(),
      alt_pressed: std::cell::Cell::new(false),
      resize_to_image_on_load: std::cell::Cell::new(false),
      folder_images: folder_images.clone(),
      folder_index: folder_index.clone(),
      active_toasts: active_toasts.clone(),
      progress_bar: progress_bar.clone(),
      toast_overlay: toast_overlay.clone(),
      crop_btn: crop_btn.clone(),
      resize_btn: resize_btn.clone(),
      metadata_btn: metadata_btn.clone(),
      rotate_btn: rotate_btn.clone(),
      mirror_btn: mirror_btn.clone(),
      flip_btn: flip_btn.clone(),
      crop_actions_bar: crop_actions_bar.clone(),
      undo_btn: undo_btn.clone(),
      redo_btn: redo_btn.clone(),
      save_btn: save_btn.clone(),
      export_btn: export_btn.clone(),
      nav_prev_btn: nav_prev_btn.clone(),
      nav_next_btn: nav_next_btn.clone(),
      zoom_badge_btn: zoom_badge_btn.clone(),
      scrolled: scrolled.clone(),
      stack: stack.clone(),
    });

    // ── Metadata Cleaner Popover Setup ──────────────────────────────────
    let metadata_popover = gtk4::Popover::new();

    let popover_box = gtk4::Box::builder()
      .orientation(gtk4::Orientation::Vertical)
      .spacing(8)
      .margin_top(12)
      .margin_bottom(12)
      .margin_start(12)
      .margin_end(12)
      .width_request(240)
      .build();

    let title_label = gtk4::Label::builder()
      .label("Metadata Cleaner")
      .halign(gtk4::Align::Start)
      .css_classes(["title-4"])
      .build();
    popover_box.append(&title_label);

    let all_metadata_check = gtk4::CheckButton::builder()
      .label("All EXIF and related metadata")
      .active(false)
      .build();

    let gps_check = gtk4::CheckButton::builder()
      .label("GPS coordinates")
      .active(true)
      .build();

    let camera_check = gtk4::CheckButton::builder()
      .label("Camera models & specs")
      .active(true)
      .build();

    let timestamp_check = gtk4::CheckButton::builder()
      .label("Timestamps & dates")
      .active(true)
      .build();

    let comment_check = gtk4::CheckButton::builder()
      .label("Comments & software")
      .active(true)
      .build();

    popover_box.append(&all_metadata_check);
    popover_box.append(&gps_check);
    popover_box.append(&camera_check);
    popover_box.append(&timestamp_check);
    popover_box.append(&comment_check);

    let clean_action_btn = gtk4::Button::builder()
      .label("Clean Metadata")
      .css_classes(["suggested-action", "pill"])
      .build();
    popover_box.append(&clean_action_btn);

    metadata_popover.set_child(Some(&popover_box));
    win_struct.metadata_btn.set_popover(Some(&metadata_popover));

    // When the popover opens, sync checkboxes with the document's current policy
    let gps_check_map = gps_check.clone();
    let camera_check_map = camera_check.clone();
    let timestamp_check_map = timestamp_check.clone();
    let comment_check_map = comment_check.clone();
    let all_metadata_check_map = all_metadata_check.clone();
    let win_map = win_struct.clone();
    metadata_popover.connect_map(move |_| {
      let doc_borrow = win_map.document.borrow();
      if let Some(ref doc) = *doc_borrow {
        all_metadata_check_map.set_active(doc.metadata_policy.remove_all_metadata);
        gps_check_map.set_active(doc.metadata_policy.remove_gps);
        camera_check_map.set_active(doc.metadata_policy.remove_camera_info);
        timestamp_check_map.set_active(doc.metadata_policy.remove_timestamps);
        comment_check_map.set_active(doc.metadata_policy.remove_comments);

        let per_category_sensitive = !doc.metadata_policy.remove_all_metadata;
        gps_check_map.set_sensitive(per_category_sensitive);
        camera_check_map.set_sensitive(per_category_sensitive);
        timestamp_check_map.set_sensitive(per_category_sensitive);
        comment_check_map.set_sensitive(per_category_sensitive);
      }
    });

    // "Strip all" disables granular toggles, while preserving their selected state.
    let gps_check_all = gps_check.clone();
    let camera_check_all = camera_check.clone();
    let timestamp_check_all = timestamp_check.clone();
    let comment_check_all = comment_check.clone();
    all_metadata_check.connect_toggled(move |btn| {
      let per_category_sensitive = !btn.is_active();
      gps_check_all.set_sensitive(per_category_sensitive);
      camera_check_all.set_sensitive(per_category_sensitive);
      timestamp_check_all.set_sensitive(per_category_sensitive);
      comment_check_all.set_sensitive(per_category_sensitive);
    });

    // When clicking the clean button, update the policy and toast the user
    let win_clean = win_struct.clone();
    let all_metadata_check_clean = all_metadata_check.clone();
    let gps_check_clean = gps_check.clone();
    let camera_check_clean = camera_check.clone();
    let timestamp_check_clean = timestamp_check.clone();
    let comment_check_clean = comment_check.clone();
    let popover_clean = metadata_popover.clone();

    clean_action_btn.connect_clicked(move |_| {
      let mut doc_borrow = win_clean.document.borrow_mut();
      if let Some(ref mut doc) = *doc_borrow {
        let policy = pixie_core::MetadataPolicy {
          remove_all_metadata: all_metadata_check_clean.is_active(),
          remove_gps: gps_check_clean.is_active(),
          remove_camera_info: camera_check_clean.is_active(),
          remove_comments: comment_check_clean.is_active(),
          remove_timestamps: timestamp_check_clean.is_active(),
        };
        doc.metadata_policy = policy;

        if policy.remove_all_metadata {
          win_clean.show_toast("All EXIF and related metadata will be removed on Save/Convert.");
        } else {
          let mut parts = Vec::new();
          if policy.remove_gps {
            parts.push("GPS");
          }
          if policy.remove_camera_info {
            parts.push("Camera Info");
          }
          if policy.remove_timestamps {
            parts.push("Timestamps");
          }
          if policy.remove_comments {
            parts.push("Comments");
          }

          if parts.is_empty() {
            win_clean.show_toast("Metadata policy updated: preserve all.");
          } else {
            win_clean.show_toast(&format!(
              "Metadata marked for removal: {}. Save to apply.",
              parts.join(", ")
            ));
          }
        }
      }
      popover_clean.popdown();
    });

    let win_open_btn = win_struct.clone();
    open_btn.connect_clicked(move |_| {
      win_open_btn.trigger_open_dialog();
    });

    // Wire background worker outputs
    let win_struct_clone = Rc::downgrade(&win_struct);
    glib::MainContext::default().spawn_local(async move {
      while let Ok((task_gen, output)) = rx.recv().await {
        if let Some(win) = win_struct_clone.upgrade() {
          win.handle_worker_output(task_gen, output);
        }
      }
    });

    // Set up button actions
    let win_zoom_badge = win_struct.clone();
    zoom_badge_btn.connect_clicked(move |_| {
      win_zoom_badge.canvas.set_zoom(None);
      win_zoom_badge.scrolled.hadjustment().set_value(0.0);
      win_zoom_badge.scrolled.vadjustment().set_value(0.0);
      win_zoom_badge.update_zoom_badge();
    });

    let win_cl2 = win_struct.clone();
    save_btn.connect_clicked(move |_| {
      win_cl2.trigger_save();
    });

    let win_cl3 = win_struct.clone();
    export_btn.connect_clicked(move |_| {
      win_cl3.trigger_export_dialog();
    });

    let win_fullscreen = win_struct.clone();
    fullscreen_btn.connect_clicked(move |_| {
      let is_fullscreen = win_fullscreen.window.is_fullscreen();
      win_fullscreen.window.set_fullscreened(!is_fullscreen);
    });

    let fullscreen_btn_state = fullscreen_btn.clone();
    window.connect_notify_local(Some("fullscreened"), move |w, _| {
      if w.is_fullscreen() {
        fullscreen_btn_state.set_icon_name("view-restore-symbolic");
        fullscreen_btn_state.set_tooltip_text(Some("Exit Fullscreen"));
      } else {
        fullscreen_btn_state.set_icon_name("view-fullscreen-symbolic");
        fullscreen_btn_state.set_tooltip_text(Some("Enter Fullscreen"));
      }
    });

    // Zoom on Ctrl+Scroll
    let win_scroll = win_struct.clone();
    let scrolled_cl = scrolled.clone();
    let scroll_controller =
      gtk4::EventControllerScroll::new(gtk4::EventControllerScrollFlags::VERTICAL);
    scroll_controller.connect_scroll(move |controller, _dx, dy| {
      let state = controller.current_event_state();
      if state.contains(gtk4::gdk::ModifierType::CONTROL_MASK) {
        if dy < 0.0 {
          win_scroll.canvas.zoom_in();
        } else if dy > 0.0 {
          win_scroll.canvas.zoom_out();
        }
        win_scroll.update_zoom_badge();
        glib::Propagation::Stop
      } else {
        glib::Propagation::Proceed
      }
    });
    scrolled_cl.add_controller(scroll_controller);

    let win_cl4 = win_struct.clone();
    undo_btn.connect_clicked(move |_| {
      win_cl4.trigger_undo();
    });

    let win_cl5 = win_struct.clone();
    redo_btn.connect_clicked(move |_| {
      win_cl5.trigger_redo();
    });

    // Nav buttons
    let win_nav_prev = win_struct.clone();
    nav_prev_btn.connect_clicked(move |_| {
      win_nav_prev.navigate_prev();
    });
    let win_nav_next = win_struct.clone();
    nav_next_btn.connect_clicked(move |_| {
      win_nav_next.navigate_next();
    });

    // Mirror & Flip click handlers
    let win_mirror = win_struct.clone();
    mirror_btn.connect_clicked(move |_| {
      win_mirror.apply_image_operation(ImageOperation::Flip(FlipOperation::new(
        FlipDirection::Horizontal,
      )));
    });

    let win_flip = win_struct.clone();
    flip_btn.connect_clicked(move |_| {
      win_flip.apply_image_operation(ImageOperation::Flip(FlipOperation::new(
        FlipDirection::Vertical,
      )));
    });

    let win_rotate = win_struct.clone();
    rotate_btn.connect_clicked(move |_| {
      let rotation = if win_rotate.alt_pressed.get() {
        RotationType::Ccw90
      } else {
        RotationType::Cw90
      };
      win_rotate.apply_image_operation(ImageOperation::Rotate(RotateOperation::new(rotation)));
    });

    // Sidebar tools switches
    let win_cl6 = win_struct.clone();
    let resize_btn_clone = resize_btn.clone();
    crop_btn.connect_toggled(move |btn| {
      if btn.is_active() {
        resize_btn_clone.set_active(false);
        win_cl6.crop_actions_bar.set_visible(false);
        win_cl6.show_crop_tool();
      } else {
        win_cl6.crop_actions_bar.set_visible(false);
        win_cl6.close_sidebar_if_inactive();
      }
    });

    let win_cl7 = win_struct.clone();
    let crop_btn_clone = crop_btn.clone();
    resize_btn.connect_toggled(move |btn| {
      if btn.is_active() {
        crop_btn_clone.set_active(false);
        win_cl7.crop_actions_bar.set_visible(false);
        win_cl7.show_resize_tool();
      } else {
        win_cl7.close_sidebar_if_inactive();
      }
    });

    // Close sidebar when the canvas area is clicked.
    let click_close_sidebar = gtk4::GestureClick::new();
    let win_canvas_click = win_struct.clone();
    click_close_sidebar.connect_pressed(move |_, _, _, _| {
      if win_canvas_click.active_tool_popover.borrow().is_some() {
        win_canvas_click.close_tool_popover();
        // Keep crop mode active if crop tool is selected.
        if !win_canvas_click.crop_btn.is_active() {
          win_canvas_click.canvas.set_crop_active(false);
        }
      }
    });
    canvas.drawing_area.add_controller(click_close_sidebar);

    // Show floating crop actions once user finishes a crop drag interaction.
    let crop_drag_done = gtk4::GestureDrag::new();
    let win_crop_drag_done = win_struct.clone();
    crop_drag_done.connect_drag_end(move |_, _, _| {
      if win_crop_drag_done.crop_btn.is_active() {
        win_crop_drag_done.crop_actions_bar.set_visible(true);
      }
    });
    canvas.drawing_area.add_controller(crop_drag_done);

    // Floating crop action buttons.
    let win_crop_cancel = win_struct.clone();
    crop_cancel_btn.connect_clicked(move |_| {
      win_crop_cancel.crop_actions_bar.set_visible(false);
      win_crop_cancel.crop_btn.set_active(false);
    });

    let win_crop_apply = win_struct.clone();
    crop_apply_btn.connect_clicked(move |_| {
      let state = win_crop_apply.canvas.state.borrow();
      if let Some(rect) = state.crop_rect {
        let op = if state.crop_mode == super::canvas::CropMode::Circle {
          ImageOperation::CircleMask(CircleMaskOperation::new(rect))
        } else {
          ImageOperation::Crop(CropOperation::new(rect))
        };
        drop(state);
        win_crop_apply.apply_image_operation(op);
      } else {
        drop(state);
      }
      win_crop_apply.crop_actions_bar.set_visible(false);
      win_crop_apply.crop_btn.set_active(false);
    });

    // Drag & Drop
    let drop_target = gtk4::DropTarget::new(gio::File::static_type(), gtk4::gdk::DragAction::COPY);
    let win_cl9 = win_struct.clone();
    drop_target.connect_drop(move |_, value, _, _| {
      if let Ok(file) = value.get::<gio::File>() {
        if let Some(path) = file.path() {
          win_cl9.open_file_with_confirmation(path);
          true
        } else {
          false
        }
      } else {
        false
      }
    });
    window.add_controller(drop_target);

    // Key controller to handle ALT key for reverse rotation
    let key_controller = gtk4::EventControllerKey::new();
    let win_key = win_struct.clone();
    key_controller.connect_key_pressed(move |_controller, key, _keycode, _state| {
      if key == gtk4::gdk::Key::Alt_L || key == gtk4::gdk::Key::Alt_R {
        win_key.alt_pressed.set(true);
        win_key
          .rotate_btn
          .set_icon_name("object-rotate-left-symbolic");
        win_key
          .rotate_btn
          .set_tooltip_text(Some("Rotate image to the left"));
      }
      glib::Propagation::Proceed
    });

    let win_key_release = win_struct.clone();
    key_controller.connect_key_released(move |_controller, key, _keycode, _state| {
      if key == gtk4::gdk::Key::Alt_L || key == gtk4::gdk::Key::Alt_R {
        win_key_release.alt_pressed.set(false);
        win_key_release
          .rotate_btn
          .set_icon_name("object-rotate-right-symbolic");
        win_key_release
          .rotate_btn
          .set_tooltip_text(Some("Rotate image to the right"));
      }
    });
    window.add_controller(key_controller);

    // Actions mapping
    let act_about = gio::SimpleAction::new("about", None);
    let win_cl_about = win_struct.clone();
    act_about.connect_activate(move |_, _| {
      show_about_dialog(win_cl_about.window.upcast_ref());
    });
    window.add_action(&act_about);

    // open-with action
    let act_open_with = gio::SimpleAction::new("open-with", None);
    act_open_with.set_enabled(false);
    let win_cl_ow = win_struct.clone();
    act_open_with.connect_activate(move |_, _| {
      if let Some(path) = win_cl_ow.current_file_path.borrow().clone() {
        let uri = glib::filename_to_uri(&path, None)
          .unwrap_or_else(|_| glib::GString::from(format!("file://{}", path.display())));
        match gio::AppInfo::launch_default_for_uri(&uri, None::<&gio::AppLaunchContext>) {
          Ok(_) => {}
          Err(err) => {
            win_cl_ow.show_toast(&format!("Could not open file: {err}"));
          }
        }
      }
    });
    window.add_action(&act_open_with);

    // print action
    let act_print = gio::SimpleAction::new("print", None);
    let win_cl_print = win_struct.clone();
    act_print.connect_activate(move |_, _| {
      if let Some(path) = win_cl_print.current_file_path.borrow().clone() {
        let file_name = path.file_name().unwrap_or_default().to_string_lossy();
        let print_op = gtk4::PrintOperation::new();
        print_op.set_job_name(&format!("{file_name} - Pixie"));
        print_op.set_allow_async(false);

        match print_op.run(
          gtk4::PrintOperationAction::PrintDialog,
          Some(&win_cl_print.window),
        ) {
          Ok(gtk4::PrintOperationResult::Apply) => {
            win_cl_print.show_toast("Print job sent");
          }
          Ok(gtk4::PrintOperationResult::Cancel) => {
            win_cl_print.show_toast("Printing cancelled");
          }
          Ok(gtk4::PrintOperationResult::InProgress) => {
            win_cl_print.show_toast("Printing in progress");
          }
          Ok(gtk4::PrintOperationResult::Error) => {
            win_cl_print.show_toast("Printing failed");
          }
          Ok(_) => {
            win_cl_print.show_toast("Printing finished");
          }
          Err(err) => {
            win_cl_print.show_toast(&format!("Printing failed: {err}"));
          }
        }
      } else {
        win_cl_print.show_toast("Open an image first");
      }
    });
    window.add_action(&act_print);

    // Setup shortcuts
    win_struct.setup_shortcuts(app, &crop_btn, &resize_btn);

    // Close confirmation handling
    let win_cl_close = win_struct.clone();
    window.connect_close_request(move |_| {
      if win_cl_close.force_close.get() {
        glib::Propagation::Proceed
      } else if win_cl_close.has_unsaved_changes() {
        win_cl_close.prompt_close_confirmation();
        glib::Propagation::Stop
      } else {
        glib::Propagation::Proceed
      }
    });

    window.present();
    win_struct
  }

  fn setup_shortcuts(
    self: &Rc<Self>,
    app: &adw::Application,
    crop_btn: &gtk4::ToggleButton,
    resize_btn: &gtk4::ToggleButton,
  ) {
    // Register Gio Actions
    let win = &self.window;

    let act_open = gio::SimpleAction::new("open", None);
    let self_cl = self.clone();
    act_open.connect_activate(move |_, _| {
      self_cl.trigger_open_dialog();
    });
    win.add_action(&act_open);
    app.set_accels_for_action("win.open", &["<Control>o"]);
    app.set_accels_for_action("win.open-with", &["<Control><Shift>o"]);

    let act_save = gio::SimpleAction::new("save", None);
    act_save.set_enabled(false);
    let self_cl = self.clone();
    act_save.connect_activate(move |_, _| {
      self_cl.trigger_save();
    });
    win.add_action(&act_save);
    app.set_accels_for_action("win.save", &["<Control>s"]);

    let act_save_as = gio::SimpleAction::new("save-as", None);
    act_save_as.set_enabled(false);
    let self_cl = self.clone();
    act_save_as.connect_activate(move |_, _| {
      self_cl.trigger_save_as();
    });
    win.add_action(&act_save_as);
    app.set_accels_for_action("win.save-as", &["<Control><Shift>s"]);

    let act_export = gio::SimpleAction::new("export", None);
    act_export.set_enabled(false);
    let self_cl = self.clone();
    act_export.connect_activate(move |_, _| {
      self_cl.trigger_export_dialog();
    });
    win.add_action(&act_export);
    app.set_accels_for_action("win.export", &["<Control><Alt>s"]);

    let act_undo = gio::SimpleAction::new("undo", None);
    let self_cl = self.clone();
    act_undo.connect_activate(move |_, _| {
      self_cl.trigger_undo();
    });
    win.add_action(&act_undo);
    app.set_accels_for_action("win.undo", &["<Control>z"]);

    let act_redo = gio::SimpleAction::new("redo", None);
    let self_cl = self.clone();
    act_redo.connect_activate(move |_, _| {
      self_cl.trigger_redo();
    });
    win.add_action(&act_redo);
    app.set_accels_for_action("win.redo", &["<Control><Shift>z"]);

    let act_fit = gio::SimpleAction::new("fit", None);
    let self_cl = self.clone();
    act_fit.connect_activate(move |_, _| {
      self_cl.canvas.set_zoom(None);
      self_cl.scrolled.hadjustment().set_value(0.0);
      self_cl.scrolled.vadjustment().set_value(0.0);
      self_cl.update_zoom_badge();
    });
    win.add_action(&act_fit);
    app.set_accels_for_action("win.fit", &["<Control>0"]);

    let act_zoom_in = gio::SimpleAction::new("zoom-in", None);
    let self_cl = self.clone();
    act_zoom_in.connect_activate(move |_, _| {
      self_cl.canvas.zoom_in();
      self_cl.update_zoom_badge();
    });
    win.add_action(&act_zoom_in);
    app.set_accels_for_action("win.zoom-in", &["<Control>plus", "<Control>equal"]);

    let act_zoom_out = gio::SimpleAction::new("zoom-out", None);
    let self_cl = self.clone();
    act_zoom_out.connect_activate(move |_, _| {
      self_cl.canvas.zoom_out();
      self_cl.update_zoom_badge();
    });
    win.add_action(&act_zoom_out);
    app.set_accels_for_action("win.zoom-out", &["<Control>minus"]);

    let act_rot_cw = gio::SimpleAction::new("rot-cw", None);
    let self_cl = self.clone();
    act_rot_cw.connect_activate(move |_, _| {
      self_cl.apply_image_operation(ImageOperation::Rotate(RotateOperation::new(
        RotationType::Cw90,
      )));
    });
    win.add_action(&act_rot_cw);
    app.set_accels_for_action("win.rot-cw", &["<Control>r"]);

    let act_rot_ccw = gio::SimpleAction::new("rot-ccw", None);
    let self_cl = self.clone();
    act_rot_ccw.connect_activate(move |_, _| {
      self_cl.apply_image_operation(ImageOperation::Rotate(RotateOperation::new(
        RotationType::Ccw90,
      )));
    });
    win.add_action(&act_rot_ccw);
    app.set_accels_for_action("win.rot-ccw", &["<Control><Shift>r"]);

    let act_flip_h = gio::SimpleAction::new("flip-h", None);
    let self_cl = self.clone();
    act_flip_h.connect_activate(move |_, _| {
      self_cl.apply_image_operation(ImageOperation::Flip(FlipOperation::new(
        FlipDirection::Horizontal,
      )));
    });
    win.add_action(&act_flip_h);

    let act_flip_v = gio::SimpleAction::new("flip-v", None);
    let self_cl = self.clone();
    act_flip_v.connect_activate(move |_, _| {
      self_cl.apply_image_operation(ImageOperation::Flip(FlipOperation::new(
        FlipDirection::Vertical,
      )));
    });
    win.add_action(&act_flip_v);

    let act_toggle_fullscreen = gio::SimpleAction::new("toggle-fullscreen", None);
    let self_cl = self.clone();
    act_toggle_fullscreen.connect_activate(move |_, _| {
      let is_fullscreen = self_cl.window.is_fullscreen();
      self_cl.window.set_fullscreened(!is_fullscreen);
    });
    win.add_action(&act_toggle_fullscreen);
    app.set_accels_for_action("win.toggle-fullscreen", &["F11"]);

    let act_cancel = gio::SimpleAction::new("cancel", None);
    let crop_cl = crop_btn.clone();
    let resize_cl = resize_btn.clone();
    act_cancel.connect_activate(move |_, _| {
      crop_cl.set_active(false);
      resize_cl.set_active(false);
    });
    win.add_action(&act_cancel);
    app.set_accels_for_action("win.cancel", &["Escape"]);
    app.set_accels_for_action("win.print", &["<Control>p"]);
  }

  pub fn open_file(&self, path: PathBuf) {
    self.clear_active_toasts();
    self.show_toast(&format!(
      "Opening file: {}",
      path.file_name().unwrap_or_default().to_string_lossy()
    ));

    // Scan sibling images in the same folder
    if let Some(dir) = path.parent() {
      let supported: &[&str] = &["png", "jpg", "jpeg", "webp", "avif", "bmp", "tiff", "gif"];
      let siblings: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|entries| {
          let mut v: Vec<PathBuf> = entries
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| {
              p.extension()
                .and_then(|e| e.to_str())
                .map(|e| supported.contains(&e.to_lowercase().as_str()))
                .unwrap_or(false)
            })
            .collect();
          v.sort();
          v
        })
        .unwrap_or_default();

      let idx = siblings.iter().position(|p| p == &path).unwrap_or(0);
      *self.folder_images.borrow_mut() = siblings;
      *self.folder_index.borrow_mut() = idx;
    }

    // Read file bytes in background
    match std::fs::read(&path) {
      Ok(bytes) => {
        *self.current_file_path.borrow_mut() = Some(path);
        self.progress_bar.set_visible(true);
        self.progress_bar.set_fraction(0.0);
        self.stack.set_visible_child_name("canvas");
        self.worker.spawn_decode(bytes);
      }
      Err(e) => {
        self.show_error_dialog(&format!("Could not read file: {e}"));
        if self.document.borrow().is_none() {
          self.stack.set_visible_child_name("placeholder");
        }
      }
    }
  }

  fn handle_worker_output(&self, _gen: u64, output: WorkerOutput) {
    match output {
      WorkerOutput::Progress(fraction) => {
        self.progress_bar.set_fraction(f64::from(fraction));
      }
      WorkerOutput::Decoded { img, format } => {
        self.progress_bar.set_visible(false);
        self.stack.set_visible_child_name("canvas");
        let path = self.current_file_path.borrow();
        let file_name = path
          .as_ref()
          .and_then(|p| p.file_name())
          .map(|n| n.to_string_lossy().into_owned())
          .unwrap_or_else(|| "Unknown".to_string());

        let doc = PixieDocument::open(&std::fs::read(path.as_ref().unwrap()).unwrap()).unwrap(); // safe re-decoding context
        self.undo_btn.set_sensitive(doc.history().can_undo());
        self.redo_btn.set_sensitive(doc.history().can_redo());
        self.saved_metadata_policy.set(doc.metadata_policy);
        *self.document.borrow_mut() = Some(doc);
        self.saved_history_index.set(Some(0));
        self.force_close.set(false);
        self.clear_active_toasts();

        self.canvas.set_zoom(None);
        self.update_zoom_badge();
        self.canvas.set_image(img.clone());

        if self.resize_to_image_on_load.get() {
          self.resize_to_image_on_load.set(false);
          self.resize_to_image_size(img.width() as i32, img.height() as i32);
        }
        self.show_toast(&format!(
          "{} | {} × {}",
          format.to_mime_type().to_uppercase(),
          img.width(),
          img.height()
        ));

        // Title
        self.window.set_title(Some(&format!("{file_name} — Pixie")));

        // Activate actions
        self.save_btn.set_sensitive(true);
        self.export_btn.set_sensitive(true);
        self
          .window
          .lookup_action("save")
          .unwrap()
          .downcast::<gio::SimpleAction>()
          .unwrap()
          .set_enabled(true);
        self
          .window
          .lookup_action("save-as")
          .unwrap()
          .downcast::<gio::SimpleAction>()
          .unwrap()
          .set_enabled(true);
        self
          .window
          .lookup_action("export")
          .unwrap()
          .downcast::<gio::SimpleAction>()
          .unwrap()
          .set_enabled(true);
        self
          .window
          .lookup_action("open-with")
          .unwrap()
          .downcast::<gio::SimpleAction>()
          .unwrap()
          .set_enabled(true);

        // Update navigation buttons
        let images = self.folder_images.borrow();
        let has_nav = images.len() > 1;
        self.nav_prev_btn.set_sensitive(has_nav);
        self.nav_next_btn.set_sensitive(has_nav);

        // Enable tools
        if self.window.content().unwrap().first_child().is_some() {
          // Quick scan to find toggles
          // We can enable buttons directly:
          self.enable_sidebar_toggles(true);
        }
      }
      WorkerOutput::OperationApplied { img } => {
        self.progress_bar.set_visible(false);
        self.canvas.set_image(img.clone());

        let doc_borrow = self.document.borrow();
        if let Some(ref doc) = *doc_borrow {
          self.undo_btn.set_sensitive(doc.history().can_undo());
          self.redo_btn.set_sensitive(doc.history().can_redo());
        }

        self.show_toast("Operation applied successfully");
      }
      WorkerOutput::Exported { path } => {
        self.progress_bar.set_visible(false);
        self.show_toast("Image converted successfully");
        if Some(&path) == self.current_file_path.borrow().as_ref() {
          if let Some(ref doc) = *self.document.borrow() {
            self.saved_history_index.set(Some(doc.history().len()));
            self.saved_metadata_policy.set(doc.metadata_policy);
          }
        }
      }
      WorkerOutput::Error(e) => {
        self.progress_bar.set_visible(false);
        self.show_error_dialog(&format!("Operation failed: {e}"));
        if self.document.borrow().is_none() {
          self.stack.set_visible_child_name("placeholder");
        }
      }
    }
  }

  fn enable_sidebar_toggles(&self, enable: bool) {
    self.crop_btn.set_sensitive(enable);
    self.resize_btn.set_sensitive(enable);
    self.metadata_btn.set_sensitive(enable);
    self.rotate_btn.set_sensitive(enable);
    self.mirror_btn.set_sensitive(enable);
    self.flip_btn.set_sensitive(enable);
  }

  fn apply_image_operation(&self, op: ImageOperation) {
    let mut doc_borrow = self.document.borrow_mut();
    if let Some(ref mut doc) = *doc_borrow {
      let current = doc.current_image().unwrap();
      match doc.apply_operation(op.clone()) {
        Ok(_) => {
          self.progress_bar.set_visible(true);
          self.progress_bar.set_fraction(0.0);
          self.worker.spawn_apply_operation(current, op);
        }
        Err(e) => {
          self.show_error_dialog(&format!("Failed to apply operation: {e}"));
        }
      }
    }
  }

  fn trigger_open_dialog(self: &Rc<Self>) {
    let file_dialog = gtk4::FileDialog::new();
    file_dialog.set_title("Open Image");

    let filter_list = gio::ListStore::new::<gtk4::FileFilter>();
    let filter = gtk4::FileFilter::new();
    filter.set_name(Some("Image Files"));
    filter.add_mime_type("image/png");
    filter.add_mime_type("image/jpeg");
    filter.add_mime_type("image/webp");
    filter.add_mime_type("image/avif");
    filter.add_mime_type("image/bmp");
    filter.add_mime_type("image/tiff");
    filter.add_mime_type("image/gif");
    filter_list.append(&filter);
    file_dialog.set_filters(Some(&filter_list));

    let self_cl = self.clone();
    file_dialog.open(
      Some(&self.window),
      None::<&gio::Cancellable>,
      move |result| {
        if let Ok(file) = result {
          if let Some(path) = file.path() {
            self_cl.open_file_with_confirmation(path);
          }
        }
      },
    );
  }

  fn trigger_save(self: &Rc<Self>) {
    // Overwriting must ask confirmation
    let path_borrow = self.current_file_path.borrow();
    if let Some(ref path) = *path_borrow {
      let dialog = adw::AlertDialog::new(
        Some("Confirm Overwrite"),
        Some(&format!(
          "Are you sure you want to overwrite {}?",
          path.file_name().unwrap_or_default().to_string_lossy()
        )),
      );

      dialog.add_response("cancel", "Cancel");
      dialog.add_response("confirm", "Overwrite");
      dialog.set_response_appearance("confirm", adw::ResponseAppearance::Destructive);
      dialog.set_close_response("cancel");

      let self_cl = self.clone();
      let path_cl = path.clone();
      dialog.choose(
        Some(&self.window),
        None::<&gio::Cancellable>,
        move |response| {
          if response == "confirm" {
            self_cl.perform_export(
              path_cl.clone(),
              ExportOptions::Png {
                compression: 6,
                preserve_transparency: true,
              },
            );
          }
        },
      );
    }
  }

  fn trigger_save_as(self: &Rc<Self>) {
    let doc_borrow = self.document.borrow();
    if let Some(ref doc) = *doc_borrow {
      let orig_format = doc.original_format();

      let options = match orig_format {
        pixie_core::ImageFormat::Png => ExportOptions::Png {
          compression: 6,
          preserve_transparency: true,
        },
        pixie_core::ImageFormat::Jpeg => ExportOptions::Jpeg {
          quality: 90,
          progressive: true,
          background_color: [255, 255, 255],
        },
        pixie_core::ImageFormat::Webp => ExportOptions::Webp {
          lossless: false,
          quality: 80.0,
        },
        pixie_core::ImageFormat::Avif => ExportOptions::Avif {
          quality: 75,
          speed: 6,
        },
        pixie_core::ImageFormat::Bmp => ExportOptions::Bmp {
          background_color: Some([255, 255, 255]),
        },
        pixie_core::ImageFormat::Tiff => ExportOptions::Tiff,
        pixie_core::ImageFormat::Gif => ExportOptions::Gif,
      };

      let file_dialog = gtk4::FileDialog::new();
      file_dialog.set_title("Save Image As…");

      if let Some(path) = self.current_file_path.borrow().clone() {
        if let Some(name) = path.file_name() {
          file_dialog.set_initial_name(Some(&name.to_string_lossy()));
        }
      }

      let filter_list = gio::ListStore::new::<gtk4::FileFilter>();
      let filter = gtk4::FileFilter::new();
      filter.set_name(Some("Images"));
      filter.add_mime_type("image/png");
      filter.add_mime_type("image/jpeg");
      filter.add_mime_type("image/webp");
      filter.add_mime_type("image/avif");
      filter.add_mime_type("image/bmp");
      filter.add_mime_type("image/tiff");
      filter.add_mime_type("image/gif");
      filter_list.append(&filter);
      file_dialog.set_filters(Some(&filter_list));

      let self_cl = self.clone();
      file_dialog.save(
        Some(&self.window),
        None::<&gio::Cancellable>,
        move |result| {
          if let Ok(file) = result {
            if let Some(path) = file.path() {
              let file_name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "Unknown".to_string());
              self_cl
                .window
                .set_title(Some(&format!("{file_name} — Pixie")));
              *self_cl.current_file_path.borrow_mut() = Some(path.clone());
              self_cl.perform_export(path, options.clone());
            }
          }
        },
      );
    }
  }

  fn trigger_export_dialog(self: &Rc<Self>) {
    let doc_borrow = self.document.borrow();
    if let Some(ref doc) = *doc_borrow {
      let has_alpha = doc.original_image().color().has_alpha();
      let orig_format = doc.original_format();
      let source_path = self.current_file_path.borrow().clone();

      let self_cl = self.clone();
      let dialog = ExportDialog::new(
        self.window.upcast_ref(),
        has_alpha,
        orig_format,
        source_path,
        move |path, options| {
          self_cl.perform_export(path, options);
        },
      );
      dialog.show();
    }
  }

  fn perform_export(&self, path: PathBuf, options: ExportOptions) {
    let doc_borrow = self.document.borrow();
    if let Some(ref doc) = *doc_borrow {
      let current = doc.current_image().unwrap();
      self.progress_bar.set_visible(true);
      self.progress_bar.set_fraction(0.0);
      let original_path = self.current_file_path.borrow().clone();
      let policy = doc.metadata_policy;
      self
        .worker
        .spawn_export(current, path, original_path, policy, options);
    }
  }

  fn trigger_undo(self: &Rc<Self>) {
    let mut doc_borrow = self.document.borrow_mut();
    if let Some(ref mut doc) = *doc_borrow {
      if doc.undo() {
        let current = doc.current_image().unwrap();
        self.canvas.set_image(current);
        self.undo_btn.set_sensitive(doc.history().can_undo());
        self.redo_btn.set_sensitive(doc.history().can_redo());
        self.show_toast("Undo applied");
      }
    }
  }

  fn trigger_redo(self: &Rc<Self>) {
    let mut doc_borrow = self.document.borrow_mut();
    if let Some(ref mut doc) = *doc_borrow {
      if doc.redo() {
        let current = doc.current_image().unwrap();
        self.canvas.set_image(current);
        self.undo_btn.set_sensitive(doc.history().can_undo());
        self.redo_btn.set_sensitive(doc.history().can_redo());
        self.show_toast("Redo applied");
      }
    }
  }

  fn show_crop_tool(self: &Rc<Self>) {
    self.canvas.set_crop_active(true);
    self.close_tool_popover();

    let self_cl = self.clone();
    let view = CropToolView::new(
      &self.canvas,
      move |op| {
        self_cl.apply_image_operation(op);
      },
      move || {},
    );

    let popover = gtk4::Popover::builder()
      .autohide(true)
      .has_arrow(true)
      .position(gtk4::PositionType::Top)
      .build();
    popover.set_child(Some(&view.container));
    popover.set_parent(&self.crop_btn);

    let active_popover = self.active_tool_popover.clone();
    popover.connect_closed(move |_| {
      *active_popover.borrow_mut() = None;
    });

    popover.popup();
    *self.active_tool_popover.borrow_mut() = Some(popover);
  }

  fn show_resize_tool(self: &Rc<Self>) {
    self.canvas.set_crop_active(false);
    self.close_tool_popover();

    let current_file_size = self
      .current_file_path
      .borrow()
      .as_ref()
      .and_then(|path| std::fs::metadata(path).ok())
      .map(|metadata| metadata.len());

    let self_cl = self.clone();
    let view = ResizeToolView::new(
      &self.canvas,
      current_file_size,
      move |op| {
        self_cl.apply_image_operation(op);
      },
      move || {},
    );

    let popover = gtk4::Popover::builder()
      .autohide(true)
      .has_arrow(true)
      .position(gtk4::PositionType::Top)
      .build();
    popover.set_child(Some(&view.container));
    popover.set_parent(&self.resize_btn);

    let active_popover = self.active_tool_popover.clone();
    popover.connect_closed(move |_| {
      *active_popover.borrow_mut() = None;
    });

    popover.popup();
    *self.active_tool_popover.borrow_mut() = Some(popover);
  }

  fn close_sidebar_if_inactive(&self) {
    // Hide sidebar when no sidebar tool is active.
    if !self.crop_btn.is_active() && !self.resize_btn.is_active() {
      self.close_tool_popover();
      self.canvas.set_crop_active(false);
      self.crop_actions_bar.set_visible(false);
    }
  }

  fn close_tool_popover(&self) {
    if let Some(popover) = self.active_tool_popover.borrow_mut().take() {
      popover.popdown();
    }
    self.sidebar.clear();
  }

  fn show_toast(&self, message: &str) {
    let toast = adw::Toast::new(message);
    self.active_toasts.borrow_mut().push(toast.clone());

    let active_toasts_cl = self.active_toasts.clone();
    let toast_cl = toast.clone();
    toast.connect_dismissed(move |_| {
      if let Ok(mut list) = active_toasts_cl.try_borrow_mut() {
        list.retain(|t| t != &toast_cl);
      }
    });

    self.toast_overlay.add_toast(toast);
  }

  fn clear_active_toasts(&self) {
    let toasts = self.active_toasts.borrow().clone();
    for toast in toasts {
      toast.dismiss();
    }
    self.active_toasts.borrow_mut().clear();
  }

  pub fn open_file_with_confirmation(self: &Rc<Self>, path: PathBuf) {
    if self.has_unsaved_changes() {
      let dialog = adw::AlertDialog::new(
        Some("Discard Changes?"),
        Some(
          "The image has unsaved changes. Do you want to discard them and open the other image?",
        ),
      );

      dialog.add_response("cancel", "Cancel");
      dialog.add_response("discard", "Discard");
      dialog.set_response_appearance("discard", adw::ResponseAppearance::Destructive);
      dialog.set_close_response("cancel");

      let self_cl = self.clone();
      let path_cl = path.clone();
      dialog.choose(
        Some(&self.window),
        None::<&gio::Cancellable>,
        move |response| {
          if response == "discard" {
            self_cl.open_file(path_cl.clone());
          }
        },
      );
    } else {
      self.open_file(path);
    }
  }

  fn navigate_to_index(self: &Rc<Self>, index: usize) {
    let images = self.folder_images.borrow();
    if let Some(path) = images.get(index) {
      let path = path.clone();
      drop(images);
      if self.has_unsaved_changes() {
        let dialog = adw::AlertDialog::new(
          Some("Discard Changes?"),
          Some("The image has unsaved changes. Do you want to discard them and switch images?"),
        );

        dialog.add_response("cancel", "Cancel");
        dialog.add_response("discard", "Discard");
        dialog.set_response_appearance("discard", adw::ResponseAppearance::Destructive);
        dialog.set_close_response("cancel");

        let self_cl = self.clone();
        let path_cl = path.clone();
        dialog.choose(
          Some(&self.window),
          None::<&gio::Cancellable>,
          move |response| {
            if response == "discard" {
              *self_cl.folder_index.borrow_mut() = index;
              self_cl.open_file(path_cl.clone());
            }
          },
        );
      } else {
        *self.folder_index.borrow_mut() = index;
        self.open_file(path);
      }
    }
  }

  fn navigate_prev(self: &Rc<Self>) {
    let index = *self.folder_index.borrow();
    let images = self.folder_images.borrow();
    let count = images.len();
    drop(images);
    if count > 0 {
      let new_index = if index == 0 { count - 1 } else { index - 1 };
      self.navigate_to_index(new_index);
    }
  }

  fn navigate_next(self: &Rc<Self>) {
    let index = *self.folder_index.borrow();
    let images = self.folder_images.borrow();
    let count = images.len();
    drop(images);
    if count > 0 {
      let new_index = (index + 1) % count;
      self.navigate_to_index(new_index);
    }
  }

  fn has_unsaved_changes(&self) -> bool {
    let doc_borrow = self.document.borrow();
    if let Some(ref doc) = *doc_borrow {
      if let Some(saved_idx) = self.saved_history_index.get() {
        doc.history().len() != saved_idx || doc.metadata_policy != self.saved_metadata_policy.get()
      } else {
        false
      }
    } else {
      false
    }
  }

  fn prompt_close_confirmation(self: &Rc<Self>) {
    let dialog = adw::AlertDialog::new(
      Some("Discard Changes?"),
      Some("The image has unsaved changes. Do you want to discard them and close the app?"),
    );

    dialog.add_response("cancel", "Cancel");
    dialog.add_response("discard", "Discard");
    dialog.set_response_appearance("discard", adw::ResponseAppearance::Destructive);
    dialog.set_close_response("cancel");

    let self_cl = self.clone();
    dialog.choose(
      Some(&self.window),
      None::<&gio::Cancellable>,
      move |response| {
        if response == "discard" {
          self_cl.force_close.set(true);
          self_cl.window.close();
        }
      },
    );
  }

  fn show_error_dialog(&self, message: &str) {
    let dialog = adw::AlertDialog::new(Some("Error"), Some(message));
    dialog.add_response("ok", "OK");
    dialog.set_close_response("ok");
    dialog.choose(Some(&self.window), None::<&gio::Cancellable>, |_| {});
  }

  pub fn resize_to_image_size(&self, img_w: i32, img_h: i32) {
    let display = gtk4::gdk::Display::default().expect("No display found");
    let monitor = self
      .window
      .surface()
      .and_then(|surface| display.monitor_at_surface(&surface))
      .or_else(|| {
        let monitors = display.monitors();
        if monitors.n_items() > 0 {
          monitors
            .item(0)
            .and_then(|item| item.downcast::<gtk4::gdk::Monitor>().ok())
        } else {
          None
        }
      });

    if let Some(m) = monitor {
      let geometry = m.geometry();
      let monitor_w = geometry.width();
      let monitor_h = geometry.height();

      let target_w = img_w.min(monitor_w);
      let target_h = img_h.min(monitor_h);

      self.window.set_default_size(target_w, target_h);
    } else {
      self.window.set_default_size(img_w, img_h);
    }
  }

  fn update_zoom_badge(&self) {
    let zoom = self.canvas.state.borrow().zoom_factor;
    if let Some(z) = zoom {
      let percentage = (z * 100.0).round() as i32;
      self.zoom_badge_btn.set_label(&format!("{}%", percentage));
      self.zoom_badge_btn.set_visible(true);
    } else {
      self.zoom_badge_btn.set_visible(false);
    }
  }
}
