use gtk4::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;
use pixie_core::{
  CircleMaskOperation, CropOperation, ExportOptions, FlipDirection, FlipOperation, ImageOperation,
  MetadataEdits, MetadataValueEdit, PixieDocument, RotateOperation, RotationType,
  TransparentBackgroundOperation,
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
use crate::i18n::{gettext, gettext_replace};
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
  saved_metadata_edits: Rc<RefCell<MetadataEdits>>,
  history_buttons_visible: std::cell::Cell<bool>,
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
  metadata_edit_btn: gtk4::Button,
  rotate_btn: gtk4::Button,
  transparent_background_btn: gtk4::Button,
  mirror_btn: gtk4::Button,
  flip_btn: gtk4::Button,
  crop_actions_bar: gtk4::Box,
  floating_nav_bar: gtk4::Box,
  floating_tool_bar: gtk4::Box,
  undo_btn: gtk4::Button,
  redo_btn: gtk4::Button,
  save_btn: gtk4::Button,
  export_btn: gtk4::Button,
  nav_prev_btn: gtk4::Button,
  nav_next_btn: gtk4::Button,
  zoom_badge_btn: gtk4::Button,
  properties_btn: gtk4::MenuButton,
  save_dropdown_btn: gtk4::MenuButton,
  properties_folder_label: gtk4::Label,
  properties_size_label: gtk4::Label,
  properties_format_label: gtk4::Label,
  properties_aspect_ratio_label: gtk4::Label,
  properties_comments_row: gtk4::ListBoxRow,
  properties_comments_label: gtk4::Label,
  properties_artist_row: gtk4::ListBoxRow,
  properties_artist_label: gtk4::Label,
  properties_copyright_row: gtk4::ListBoxRow,
  properties_copyright_label: gtk4::Label,
  properties_software_row: gtk4::ListBoxRow,
  properties_software_label: gtk4::Label,
  properties_file_size_label: gtk4::Label,
  properties_created_label: gtk4::Label,
  properties_modified_label: gtk4::Label,
  scrolled: gtk4::ScrolledWindow,
  stack: gtk4::Stack,
}

impl PixieWindow {
  fn update_history_buttons(&self, doc: &PixieDocument) {
    let visible = self.history_buttons_visible.get();
    self.undo_btn.set_visible(visible);
    self.redo_btn.set_visible(visible);
    self.undo_btn.set_sensitive(doc.history().can_undo());
    self.redo_btn.set_sensitive(doc.history().can_redo());
  }

  fn set_history_buttons_visible(&self, visible: bool) {
    self.history_buttons_visible.set(visible);
    if let Some(ref doc) = *self.document.borrow() {
      self.update_history_buttons(doc);
    } else {
      self.undo_btn.set_visible(false);
      self.redo_btn.set_visible(false);
      self.undo_btn.set_sensitive(false);
      self.redo_btn.set_sensitive(false);
    }
  }

  fn refresh_window_title(&self) {
    let has_changes = self.has_unsaved_changes();
    self.save_dropdown_btn.set_visible(has_changes);

    let title = if let Some(path) = self.current_file_path.borrow().as_ref() {
      let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| gettext("Unknown"));
      let dirty_prefix = if has_changes { "* " } else { "" };
      format!("{dirty_prefix}{file_name} — Pixie")
    } else {
      "Pixie".to_string()
    };

    self.window.set_title(Some(&title));
  }

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
      .tooltip_text(gettext("Toggle Fullscreen"))
      .build();
    header_bar.pack_start(&fullscreen_btn);

    let update_fullscreen_btn = |btn: &gtk4::Button, is_fullscreen: bool| {
      if is_fullscreen {
        btn.set_icon_name("view-restore-symbolic");
        btn.set_tooltip_text(Some(&gettext("Exit Fullscreen")));
      } else {
        btn.set_icon_name("view-fullscreen-symbolic");
        btn.set_tooltip_text(Some(&gettext("Enter Fullscreen")));
      }
    };
    update_fullscreen_btn(&fullscreen_btn, window.is_fullscreen());

    // Left: Undo / Redo
    let undo_btn = gtk4::Button::builder()
      .icon_name("edit-undo-symbolic")
      .tooltip_text(gettext("Undo (Ctrl+Z)"))
      .sensitive(false)
      .visible(false)
      .build();
    let redo_btn = gtk4::Button::builder()
      .icon_name("edit-redo-symbolic")
      .tooltip_text(gettext("Redo (Ctrl+Shift+Z)"))
      .sensitive(false)
      .visible(false)
      .build();
    header_bar.pack_start(&undo_btn);
    header_bar.pack_start(&redo_btn);

    // File actions remain available through the app menu.
    let save_btn = gtk4::Button::builder()
      .icon_name("document-save-symbolic")
      .tooltip_text(gettext("Save (Ctrl+S)"))
      .sensitive(false)
      .build();
    let export_btn = gtk4::Button::builder()
      .icon_name("document-send-symbolic")
      .tooltip_text(gettext("Convert... (Ctrl+Alt+S)"))
      .sensitive(false)
      .build();

    // Floating tool buttons
    let crop_btn = gtk4::ToggleButton::builder()
      .icon_name("edit-cut-symbolic")
      .tooltip_text(gettext("Crop Image"))
      .sensitive(false)
      .css_classes(["floating-toolbar__button"])
      .build();

    let resize_btn = gtk4::ToggleButton::builder()
      .icon_name("zoom-fit-best-symbolic")
      .tooltip_text(gettext("Resize Image"))
      .sensitive(false)
      .css_classes(["floating-toolbar__button"])
      .build();

    let metadata_btn = gtk4::MenuButton::builder()
      .icon_name("security-high-symbolic")
      .tooltip_text(gettext("Metadata Cleaner"))
      .sensitive(false)
      .css_classes(["floating-toolbar__button"])
      .build();

    let metadata_edit_btn = gtk4::Button::builder()
      .icon_name("document-edit-symbolic")
      .tooltip_text(gettext("Edit Metadata"))
      .sensitive(false)
      .css_classes(["floating-toolbar__button"])
      .build();

    let transparent_background_btn = gtk4::Button::builder()
      .icon_name("eraser2-symbolic")
      .tooltip_text(gettext("Make Background Transparent"))
      .sensitive(false)
      .css_classes(["floating-toolbar__button"])
      .build();

    let rotate_btn = gtk4::Button::builder()
      .icon_name("object-rotate-right-symbolic")
      .tooltip_text(gettext("Rotate image to the right"))
      .sensitive(false)
      .css_classes(["floating-toolbar__button"])
      .build();

    let mirror_btn = gtk4::Button::builder()
      .icon_name("object-flip-horizontal-symbolic")
      .tooltip_text(gettext("Mirror"))
      .sensitive(false)
      .css_classes(["floating-toolbar__button"])
      .build();

    let flip_btn = gtk4::Button::builder()
      .icon_name("object-flip-vertical-symbolic")
      .tooltip_text(gettext("Flip"))
      .sensitive(false)
      .css_classes(["floating-toolbar__button"])
      .build();

    // Right: App menu button
    let app_menu = gio::Menu::new();

    // Section 1: file ops
    let file_section = gio::Menu::new();
    file_section.append(Some(&gettext("New Window")), Some("app.new-window"));
    file_section.append(Some(&gettext("Open...")), Some("win.open"));
    app_menu.append_section(None, &file_section);

    // Section 2: image ops
    let image_section = gio::Menu::new();
    image_section.append(Some(&gettext("Open With...")), Some("win.open-with"));
    image_section.append(Some(&gettext("Print...")), Some("win.print"));
    app_menu.append_section(None, &image_section);

    // Section 3: save/export ops
    let save_section = gio::Menu::new();
    save_section.append(Some(&gettext("Convert...")), Some("win.export"));
    app_menu.append_section(None, &save_section);

    // Section 4: view/fullscreen ops
    let view_section = gio::Menu::new();
    view_section.append(
      Some(&gettext("Toggle Fullscreen")),
      Some("win.toggle-fullscreen"),
    );
    app_menu.append_section(None, &view_section);

    // Section 5: help
    let help_section = gio::Menu::new();
    help_section.append(Some(&gettext("About Pixie")), Some("win.about"));
    app_menu.append_section(None, &help_section);

    let app_menu_btn = gtk4::MenuButton::builder()
      .icon_name("open-menu-symbolic")
      .tooltip_text(gettext("Main Menu"))
      .menu_model(&app_menu)
      .build();
    header_bar.pack_end(&app_menu_btn);

    // ── Image Information Popover ───────────────────────────────────────
    let properties_folder_label = gtk4::Label::builder()
      .label("")
      .halign(gtk4::Align::Start)
      .css_classes(["info-value"])
      .build();

    let properties_size_label = gtk4::Label::builder()
      .label("")
      .halign(gtk4::Align::Start)
      .css_classes(["info-value"])
      .build();

    let properties_format_label = gtk4::Label::builder()
      .label("")
      .halign(gtk4::Align::Start)
      .css_classes(["info-value"])
      .build();

    let properties_aspect_ratio_label = gtk4::Label::builder()
      .label("")
      .halign(gtk4::Align::Start)
      .css_classes(["info-value"])
      .build();

    let properties_comments_label = gtk4::Label::builder()
      .label("")
      .halign(gtk4::Align::Start)
      .css_classes(["info-value"])
      .build();

    let properties_artist_label = gtk4::Label::builder()
      .label("")
      .halign(gtk4::Align::Start)
      .css_classes(["info-value"])
      .build();

    let properties_copyright_label = gtk4::Label::builder()
      .label("")
      .halign(gtk4::Align::Start)
      .css_classes(["info-value"])
      .build();

    let properties_software_label = gtk4::Label::builder()
      .label("")
      .halign(gtk4::Align::Start)
      .css_classes(["info-value"])
      .build();

    let properties_file_size_label = gtk4::Label::builder()
      .label("")
      .halign(gtk4::Align::Start)
      .css_classes(["info-value"])
      .build();

    let properties_created_label = gtk4::Label::builder()
      .label("")
      .halign(gtk4::Align::Start)
      .css_classes(["info-value"])
      .build();

    let properties_modified_label = gtk4::Label::builder()
      .label("")
      .halign(gtk4::Align::Start)
      .css_classes(["info-value"])
      .build();

    let properties_popover = gtk4::Popover::builder()
      .autohide(true)
      .has_arrow(true)
      .position(gtk4::PositionType::Bottom)
      .build();

    let properties_popover_box = gtk4::Box::builder()
      .orientation(gtk4::Orientation::Vertical)
      .spacing(12)
      .margin_top(6)
      .margin_bottom(6)
      .margin_start(6)
      .margin_end(6)
      .width_request(260)
      .build();

    // Group 1: Folder
    let listbox1 = gtk4::ListBox::builder()
      .selection_mode(gtk4::SelectionMode::None)
      .css_classes(["boxed-list"])
      .build();

    let folder_row = gtk4::ListBoxRow::builder()
      .activatable(false)
      .selectable(false)
      .build();
    let folder_hbox = gtk4::Box::builder()
      .orientation(gtk4::Orientation::Horizontal)
      .spacing(8)
      .margin_top(8)
      .margin_bottom(8)
      .margin_start(12)
      .margin_end(12)
      .build();
    let folder_vbox = gtk4::Box::builder()
      .orientation(gtk4::Orientation::Vertical)
      .spacing(2)
      .hexpand(true)
      .build();
    let folder_title_lbl = gtk4::Label::builder()
      .label(gettext("Folder"))
      .halign(gtk4::Align::Start)
      .css_classes(["info-title"])
      .build();
    folder_vbox.append(&folder_title_lbl);
    folder_vbox.append(&properties_folder_label);

    let properties_folder_btn = gtk4::Button::builder()
      .icon_name("folder-symbolic")
      .halign(gtk4::Align::End)
      .valign(gtk4::Align::Center)
      .css_classes(["flat"])
      .tooltip_text(gettext("Open Containing Folder"))
      .build();
    folder_hbox.append(&folder_vbox);
    folder_hbox.append(&properties_folder_btn);
    folder_row.set_child(Some(&folder_hbox));
    listbox1.append(&folder_row);
    properties_popover_box.append(&listbox1);

    // Helper to create listbox row with title and value
    let create_row = |title: String, value_lbl: &gtk4::Label| -> gtk4::ListBoxRow {
      let row = gtk4::ListBoxRow::builder()
        .activatable(false)
        .selectable(false)
        .build();
      let vbox = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(2)
        .margin_top(8)
        .margin_bottom(8)
        .margin_start(12)
        .margin_end(12)
        .build();
      let title_lbl = gtk4::Label::builder()
        .label(title)
        .halign(gtk4::Align::Start)
        .css_classes(["info-title"])
        .build();
      vbox.append(&title_lbl);
      vbox.append(value_lbl);
      row.set_child(Some(&vbox));
      row
    };

    // Group 2: Image details
    let listbox2 = gtk4::ListBox::builder()
      .selection_mode(gtk4::SelectionMode::None)
      .css_classes(["boxed-list"])
      .build();
    listbox2.append(&create_row(gettext("Image Size"), &properties_size_label));
    listbox2.append(&create_row(
      gettext("Image Format"),
      &properties_format_label,
    ));
    listbox2.append(&create_row(
      gettext("Aspect Ratio"),
      &properties_aspect_ratio_label,
    ));
    listbox2.append(&create_row(
      gettext("File Size"),
      &properties_file_size_label,
    ));
    properties_popover_box.append(&listbox2);

    // Group 3: Created, Modified times
    let listbox3 = gtk4::ListBox::builder()
      .selection_mode(gtk4::SelectionMode::None)
      .css_classes(["boxed-list"])
      .build();
    listbox3.append(&create_row(
      gettext("File Created"),
      &properties_created_label,
    ));
    listbox3.append(&create_row(
      gettext("File Modified"),
      &properties_modified_label,
    ));
    properties_popover_box.append(&listbox3);

    // Group 4: Metadata
    let listbox4 = gtk4::ListBox::builder()
      .selection_mode(gtk4::SelectionMode::None)
      .css_classes(["boxed-list"])
      .build();
    let properties_comments_row = create_row(gettext("Comments"), &properties_comments_label);
    properties_comments_row.set_visible(false);
    listbox4.append(&properties_comments_row);

    let properties_artist_row = create_row(gettext("Artist"), &properties_artist_label);
    properties_artist_row.set_visible(false);
    listbox4.append(&properties_artist_row);

    let properties_copyright_row = create_row(gettext("Copyright"), &properties_copyright_label);
    properties_copyright_row.set_visible(false);
    listbox4.append(&properties_copyright_row);

    let properties_software_row = create_row(gettext("Software"), &properties_software_label);
    properties_software_row.set_visible(false);
    listbox4.append(&properties_software_row);
    properties_popover_box.append(&listbox4);

    properties_popover.set_child(Some(&properties_popover_box));

    let properties_btn = gtk4::MenuButton::builder()
      .icon_name("info-outline-symbolic")
      .tooltip_text(gettext("Image Properties"))
      .visible(false)
      .popover(&properties_popover)
      .build();
    header_bar.pack_end(&properties_btn);

    let save_dropdown_menu = gio::Menu::new();
    save_dropdown_menu.append(Some(&gettext("Save As...")), Some("win.save-as"));
    save_dropdown_menu.append(Some(&gettext("Replace Original Image")), Some("win.save"));

    let save_dropdown_btn = gtk4::MenuButton::builder()
      .icon_name("document-save-symbolic")
      .label(gettext("Save"))
      .tooltip_text(gettext("Save Changes"))
      .visible(false)
      .menu_model(&save_dropdown_menu)
      .css_classes(["save-dropdown-btn"])
      .build();
    header_bar.pack_end(&save_dropdown_btn);

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
      .label(gettext("Open Files..."))
      .halign(gtk4::Align::Center)
      .css_classes(["suggested-action", "pill"])
      .build();

    let status_page = adw::StatusPage::builder()
      .icon_name("image-x-generic-symbolic")
      .title(gettext("View and Edit Images"))
      .description(gettext("Drag and drop images here"))
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
      .tooltip_text(gettext("Previous Image"))
      .sensitive(false)
      .css_classes(["floating-toolbar__button"])
      .build();
    let nav_next_btn = gtk4::Button::builder()
      .icon_name("go-next-symbolic")
      .tooltip_text(gettext("Next Image"))
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
    tool_bar.append(&metadata_edit_btn);
    tool_bar.append(&transparent_background_btn);
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
      .label(gettext("Cancel"))
      .css_classes(["floating-toolbar__button"])
      .build();
    let crop_apply_btn = gtk4::Button::builder()
      .label(gettext("Apply"))
      .css_classes(["floating-toolbar__button"])
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
    let saved_metadata_edits = Rc::new(RefCell::new(MetadataEdits::default()));
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
      saved_metadata_edits: saved_metadata_edits.clone(),
      history_buttons_visible: std::cell::Cell::new(false),
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
      metadata_edit_btn: metadata_edit_btn.clone(),
      rotate_btn: rotate_btn.clone(),
      transparent_background_btn: transparent_background_btn.clone(),
      mirror_btn: mirror_btn.clone(),
      flip_btn: flip_btn.clone(),
      crop_actions_bar: crop_actions_bar.clone(),
      floating_nav_bar: nav_bar.clone(),
      floating_tool_bar: tool_bar.clone(),
      undo_btn: undo_btn.clone(),
      redo_btn: redo_btn.clone(),
      save_btn: save_btn.clone(),
      export_btn: export_btn.clone(),
      nav_prev_btn: nav_prev_btn.clone(),
      nav_next_btn: nav_next_btn.clone(),
      zoom_badge_btn: zoom_badge_btn.clone(),
      properties_btn: properties_btn.clone(),
      save_dropdown_btn: save_dropdown_btn.clone(),
      properties_folder_label: properties_folder_label.clone(),
      properties_size_label: properties_size_label.clone(),
      properties_format_label: properties_format_label.clone(),
      properties_aspect_ratio_label: properties_aspect_ratio_label.clone(),
      properties_comments_row: properties_comments_row.clone(),
      properties_comments_label: properties_comments_label.clone(),
      properties_artist_row: properties_artist_row.clone(),
      properties_artist_label: properties_artist_label.clone(),
      properties_copyright_row: properties_copyright_row.clone(),
      properties_copyright_label: properties_copyright_label.clone(),
      properties_software_row: properties_software_row.clone(),
      properties_software_label: properties_software_label.clone(),
      properties_file_size_label: properties_file_size_label.clone(),
      properties_created_label: properties_created_label.clone(),
      properties_modified_label: properties_modified_label.clone(),
      scrolled: scrolled.clone(),
      stack: stack.clone(),
    });

    // ── Image Information Popover Map Connection ────────────────────────
    let win_properties_map = win_struct.clone();
    properties_popover.connect_map(move |_| {
      win_properties_map.update_properties_popover(true);
    });

    let win_folder_click = win_struct.clone();
    properties_folder_btn.connect_clicked(move |_| {
      if let Some(ref path) = *win_folder_click.current_file_path.borrow() {
        if let Some(parent_path) = path.parent() {
          if let Ok(uri) = glib::filename_to_uri(parent_path, None) {
            let _ = gio::AppInfo::launch_default_for_uri(&uri, None::<&gio::AppLaunchContext>);
          }
        }
      }
    });

    // ── Metadata Cleaner Popover Setup ──────────────────────────────────
    let metadata_popover = gtk4::Popover::new();
    metadata_popover.set_autohide(false);

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
      .label(gettext("Metadata Cleaner"))
      .halign(gtk4::Align::Start)
      .css_classes(["title-4"])
      .build();
    popover_box.append(&title_label);

    let all_metadata_switch = gtk4::Switch::builder().active(false).build();
    let gps_switch = gtk4::Switch::builder().active(true).build();
    let camera_switch = gtk4::Switch::builder().active(true).build();
    let timestamp_switch = gtk4::Switch::builder().active(true).build();
    let comment_switch = gtk4::Switch::builder().active(true).build();

    let make_switch_row = |label: String, switch: &gtk4::Switch| {
      let row = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(8)
        .build();
      row.append(
        &gtk4::Label::builder()
          .label(label)
          .hexpand(true)
          .halign(gtk4::Align::Start)
          .build(),
      );
      row.append(switch);
      row
    };

    popover_box.append(&make_switch_row(
      gettext("All EXIF and related metadata"),
      &all_metadata_switch,
    ));
    popover_box.append(&make_switch_row(gettext("GPS coordinates"), &gps_switch));
    popover_box.append(&make_switch_row(
      gettext("Camera models & specs"),
      &camera_switch,
    ));
    popover_box.append(&make_switch_row(
      gettext("Timestamps & dates"),
      &timestamp_switch,
    ));
    popover_box.append(&make_switch_row(
      gettext("Comments & software"),
      &comment_switch,
    ));

    let clean_action_btn = gtk4::Button::builder()
      .label(gettext("Clean Metadata"))
      .hexpand(true)
      .css_classes(["suggested-action", "pill"])
      .build();
    popover_box.append(&clean_action_btn);

    metadata_popover.set_child(Some(&popover_box));
    win_struct.metadata_btn.set_popover(Some(&metadata_popover));

    // When the popover opens, sync switches with the document's current policy
    let gps_switch_map = gps_switch.clone();
    let camera_switch_map = camera_switch.clone();
    let timestamp_switch_map = timestamp_switch.clone();
    let comment_switch_map = comment_switch.clone();
    let all_metadata_switch_map = all_metadata_switch.clone();
    let win_map = win_struct.clone();
    metadata_popover.connect_map(move |_| {
      let doc_borrow = win_map.document.borrow();
      if let Some(ref doc) = *doc_borrow {
        all_metadata_switch_map.set_active(doc.metadata_policy.remove_all_metadata);
        gps_switch_map.set_active(doc.metadata_policy.remove_gps);
        camera_switch_map.set_active(doc.metadata_policy.remove_camera_info);
        timestamp_switch_map.set_active(doc.metadata_policy.remove_timestamps);
        comment_switch_map.set_active(doc.metadata_policy.remove_comments);

        let per_category_sensitive = !doc.metadata_policy.remove_all_metadata;
        gps_switch_map.set_sensitive(per_category_sensitive);
        camera_switch_map.set_sensitive(per_category_sensitive);
        timestamp_switch_map.set_sensitive(per_category_sensitive);
        comment_switch_map.set_sensitive(per_category_sensitive);
      }
    });

    let win_metadata_active = win_struct.clone();
    metadata_btn.connect_active_notify(move |btn| {
      if btn.is_active() {
        win_metadata_active.deactivate_sidebar_tools(false);
      }
    });

    // "Strip all" disables granular toggles, while preserving their selected state.
    let gps_switch_all = gps_switch.clone();
    let camera_switch_all = camera_switch.clone();
    let timestamp_switch_all = timestamp_switch.clone();
    let comment_switch_all = comment_switch.clone();
    all_metadata_switch.connect_active_notify(move |sw| {
      let per_category_sensitive = !sw.is_active();
      gps_switch_all.set_sensitive(per_category_sensitive);
      camera_switch_all.set_sensitive(per_category_sensitive);
      timestamp_switch_all.set_sensitive(per_category_sensitive);
      comment_switch_all.set_sensitive(per_category_sensitive);
    });

    // When clicking the clean button, update the policy and toast the user
    let win_clean = win_struct.clone();
    let all_metadata_switch_clean = all_metadata_switch.clone();
    let gps_switch_clean = gps_switch.clone();
    let camera_switch_clean = camera_switch.clone();
    let timestamp_switch_clean = timestamp_switch.clone();
    let comment_switch_clean = comment_switch.clone();
    let popover_clean = metadata_popover.clone();

    clean_action_btn.connect_clicked(move |_| {
      popover_clean.popdown();

      let mut doc_borrow = win_clean.document.borrow_mut();
      if let Some(ref mut doc) = *doc_borrow {
        let policy = pixie_core::MetadataPolicy {
          remove_all_metadata: all_metadata_switch_clean.is_active(),
          remove_gps: gps_switch_clean.is_active(),
          remove_camera_info: camera_switch_clean.is_active(),
          remove_comments: comment_switch_clean.is_active(),
          remove_timestamps: timestamp_switch_clean.is_active(),
        };
        doc.metadata_policy = policy;

        if policy.remove_all_metadata {
          win_clean.show_toast(&gettext(
            "All EXIF and related metadata will be removed on Save/Convert.",
          ));
        } else {
          let mut parts = Vec::new();
          if policy.remove_gps {
            parts.push(gettext("GPS"));
          }
          if policy.remove_camera_info {
            parts.push(gettext("Camera Info"));
          }
          if policy.remove_timestamps {
            parts.push(gettext("Timestamps"));
          }
          if policy.remove_comments {
            parts.push(gettext("Comments"));
          }

          if parts.is_empty() {
            win_clean.show_toast(&gettext("Metadata policy updated: preserve all."));
          } else {
            win_clean.show_toast(&gettext_replace(
              "Metadata marked for removal: {metadata}. Save to apply.",
              &[("metadata", parts.join(", "))],
            ));
          }
        }
      }
    });

    let win_edit = win_struct.clone();
    metadata_edit_btn.connect_clicked(move |_| {
      win_edit.deactivate_sidebar_tools(true);
      win_edit.show_metadata_editor_popover();
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
        fullscreen_btn_state.set_tooltip_text(Some(&gettext("Exit Fullscreen")));
      } else {
        fullscreen_btn_state.set_icon_name("view-fullscreen-symbolic");
        fullscreen_btn_state.set_tooltip_text(Some(&gettext("Enter Fullscreen")));
      }
    });

    // Zoom on mouse scroll
    let win_scroll = win_struct.clone();
    let scrolled_cl = scrolled.clone();
    let scroll_controller =
      gtk4::EventControllerScroll::new(gtk4::EventControllerScrollFlags::VERTICAL);
    scroll_controller.connect_scroll(move |controller, _dx, dy| {
      let _ = controller;
      if dy < 0.0 {
        win_scroll.canvas.zoom_in();
      } else if dy > 0.0 {
        win_scroll.canvas.zoom_out();
      } else {
        return glib::Propagation::Proceed;
      }

      win_scroll.update_zoom_badge();
      glib::Propagation::Stop
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
      win_mirror.deactivate_sidebar_tools(true);
      win_mirror.apply_image_operation(ImageOperation::Flip(FlipOperation::new(
        FlipDirection::Horizontal,
      )));
    });

    let win_flip = win_struct.clone();
    flip_btn.connect_clicked(move |_| {
      win_flip.deactivate_sidebar_tools(true);
      win_flip.apply_image_operation(ImageOperation::Flip(FlipOperation::new(
        FlipDirection::Vertical,
      )));
    });

    let win_rotate = win_struct.clone();
    rotate_btn.connect_clicked(move |_| {
      win_rotate.deactivate_sidebar_tools(true);
      let rotation = if win_rotate.alt_pressed.get() {
        RotationType::Ccw90
      } else {
        RotationType::Cw90
      };
      win_rotate.apply_image_operation(ImageOperation::Rotate(RotateOperation::new(rotation)));
    });

    let win_transparent_background = win_struct.clone();
    transparent_background_btn.connect_clicked(move |_| {
      win_transparent_background.deactivate_sidebar_tools(true);
      win_transparent_background.apply_image_operation(ImageOperation::TransparentBackground(
        TransparentBackgroundOperation::new(12),
      ));
    });

    // Sidebar tools switches
    let win_cl6 = win_struct.clone();
    let resize_btn_clone = resize_btn.clone();
    crop_btn.connect_toggled(move |btn| {
      if btn.is_active() {
        resize_btn_clone.set_active(false);
        win_cl6.set_floating_controls_visible(true);
        win_cl6.crop_actions_bar.set_visible(false);
        win_cl6.show_crop_tool();
      } else {
        win_cl6.canvas.set_crop_active(false);
        win_cl6.crop_actions_bar.set_visible(false);
        win_cl6.set_floating_controls_visible(true);
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
    click_close_sidebar.connect_pressed(move |_, n_press, x, y| {
      if n_press == 2
        && win_canvas_click.crop_btn.is_active()
        && win_canvas_click.canvas.state.borrow().crop_active
      {
        let state = win_canvas_click.canvas.state.borrow();
        if let Some(rect) = state.crop_rect {
          if let Some((scale, ox, oy, _, _)) =
            PixieCanvas::get_layout_scale_offset(&win_canvas_click.canvas.drawing_area, &state)
          {
            let rx = ox + f64::from(rect.x) * scale;
            let ry = oy + f64::from(rect.y) * scale;
            let rw = f64::from(rect.width) * scale;
            let rh = f64::from(rect.height) * scale;

            if x >= rx && x <= rx + rw && y >= ry && y <= ry + rh {
              drop(state);
              win_canvas_click.apply_crop();
              return;
            }
          }
        }
      }

      if win_canvas_click.active_tool_popover.borrow().is_some() {
        win_canvas_click.close_tool_popover();
        // Crop mode only starts after pressing "Start Cropping".
        if !win_canvas_click.crop_btn.is_active() {
          win_canvas_click.canvas.set_crop_active(false);
        }
      }
      win_canvas_click.close_metadata_cleaner_popover();
    });
    canvas.drawing_area.add_controller(click_close_sidebar);

    // Floating crop action buttons.
    let win_crop_cancel = win_struct.clone();
    crop_cancel_btn.connect_clicked(move |_| {
      win_crop_cancel.cancel_crop();
    });

    let win_crop_apply = win_struct.clone();
    crop_apply_btn.connect_clicked(move |_| {
      win_crop_apply.apply_crop();
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

    // Key controller to handle shortcuts & ALT key for reverse rotation
    let key_controller = gtk4::EventControllerKey::new();
    let win_key = win_struct.clone();
    key_controller.connect_key_pressed(move |_controller, key, _keycode, _state| {
      if win_key.crop_btn.is_active() && win_key.canvas.state.borrow().crop_active {
        if key == gtk4::gdk::Key::Return || key == gtk4::gdk::Key::KP_Enter {
          win_key.apply_crop();
          return glib::Propagation::Stop;
        } else if key == gtk4::gdk::Key::Escape {
          win_key.cancel_crop();
          return glib::Propagation::Stop;
        }
      }
      if key == gtk4::gdk::Key::Alt_L || key == gtk4::gdk::Key::Alt_R {
        win_key.alt_pressed.set(true);
        win_key
          .rotate_btn
          .set_icon_name("object-rotate-left-symbolic");
        win_key
          .rotate_btn
          .set_tooltip_text(Some(&gettext("Rotate image to the left")));
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
          .set_tooltip_text(Some(&gettext("Rotate image to the right")));
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
            win_cl_ow.show_toast(&gettext_replace(
              "Could not open file: {error}",
              &[("error", err.to_string())],
            ));
          }
        }
      }
    });
    window.add_action(&act_open_with);

    // print action
    let act_print = gio::SimpleAction::new("print", None);
    act_print.set_enabled(false);
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
            win_cl_print.show_toast(&gettext("Print job sent"));
          }
          Ok(gtk4::PrintOperationResult::Cancel) => {
            win_cl_print.show_toast(&gettext("Printing cancelled"));
          }
          Ok(gtk4::PrintOperationResult::InProgress) => {
            win_cl_print.show_toast(&gettext("Printing in progress"));
          }
          Ok(gtk4::PrintOperationResult::Error) => {
            win_cl_print.show_toast(&gettext("Printing failed"));
          }
          Ok(_) => {
            win_cl_print.show_toast(&gettext("Printing finished"));
          }
          Err(err) => {
            win_cl_print.show_toast(&gettext_replace(
              "Printing failed: {error}",
              &[("error", err.to_string())],
            ));
          }
        }
      } else {
        win_cl_print.show_toast(&gettext("Open an image first"));
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
    self.show_toast(&gettext_replace(
      "Opening file: {file}",
      &[(
        "file",
        path
          .file_name()
          .unwrap_or_default()
          .to_string_lossy()
          .into_owned(),
      )],
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

    *self.current_file_path.borrow_mut() = Some(path.clone());
    self.progress_bar.set_visible(true);
    self.progress_bar.set_fraction(0.0);
    self.stack.set_visible_child_name("canvas");
    self.worker.spawn_decode(path);
  }

  fn handle_worker_output(&self, task_gen: u64, output: WorkerOutput) {
    if !self.worker.is_current(task_gen) {
      return;
    }

    match output {
      WorkerOutput::Progress(fraction) => {
        self.progress_bar.set_fraction(f64::from(fraction));
      }
      WorkerOutput::Decoded { img, format } => {
        self.progress_bar.set_visible(false);
        self.stack.set_visible_child_name("canvas");

        self.canvas.set_zoom(None);
        self.update_zoom_badge();
        if let Err(err) = self.canvas.set_image(img.clone()) {
          self.show_error_dialog(&gettext_replace(
            "Failed to render image: {error}",
            &[("error", err)],
          ));
          self.stack.set_visible_child_name("placeholder");
          return;
        }

        let doc = PixieDocument::new(img.clone(), format);
        self.history_buttons_visible.set(false);
        self.update_history_buttons(&doc);
        self.saved_metadata_policy.set(doc.metadata_policy);
        *self.saved_metadata_edits.borrow_mut() = doc.metadata_edits.clone();
        *self.document.borrow_mut() = Some(doc);
        self.saved_history_index.set(Some(0));
        self.force_close.set(false);
        self.clear_active_toasts();

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

        self.properties_btn.set_visible(true);
        self.update_properties_popover(false);

        // Title
        self.refresh_window_title();

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
        self
          .window
          .lookup_action("print")
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
      WorkerOutput::OperationApplied { op, img } => {
        self.progress_bar.set_visible(false);

        if let Err(err) = self.canvas.set_image(img.clone()) {
          self.show_error_dialog(&gettext_replace(
            "Failed to render resized image: {error}",
            &[("error", err)],
          ));
          self.enable_sidebar_toggles(true);
          return;
        }

        let mut doc_borrow = self.document.borrow_mut();
        if let Some(ref mut doc) = *doc_borrow {
          doc.commit_rendered_operation(op, img.clone());
          self.history_buttons_visible.set(true);
          self.update_history_buttons(doc);
        }
        drop(doc_borrow);
        self.enable_sidebar_toggles(true);
        self.update_properties_popover(false);
        self.refresh_window_title();
        self.show_toast(&gettext("Operation applied successfully"));
      }
      WorkerOutput::Exported { path } => {
        self.progress_bar.set_visible(false);
        self.show_toast(&gettext("Image converted successfully"));
        if Some(&path) == self.current_file_path.borrow().as_ref() {
          if let Some(ref doc) = *self.document.borrow() {
            self.saved_history_index.set(Some(doc.history().len()));
            self.saved_metadata_policy.set(doc.metadata_policy);
            *self.saved_metadata_edits.borrow_mut() = doc.metadata_edits.clone();
          }
          self.set_history_buttons_visible(false);
          self.update_properties_popover(false);
          self.refresh_window_title();
        }
      }
      WorkerOutput::Error(e) => {
        self.progress_bar.set_visible(false);
        if self.document.borrow().is_some() {
          self.enable_sidebar_toggles(true);
        }
        self.show_error_dialog(&gettext_replace(
          "Operation failed: {error}",
          &[("error", e.to_string())],
        ));
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
    self.metadata_edit_btn.set_sensitive(enable);
    self.rotate_btn.set_sensitive(enable);
    self.transparent_background_btn.set_sensitive(enable);
    self.mirror_btn.set_sensitive(enable);
    self.flip_btn.set_sensitive(enable);
  }

  fn set_floating_controls_visible(&self, visible: bool) {
    self.floating_nav_bar.set_visible(visible);
    self.floating_tool_bar.set_visible(visible);
  }

  fn apply_image_operation(&self, op: ImageOperation) {
    let mut doc_borrow = self.document.borrow_mut();
    if let Some(ref mut doc) = *doc_borrow {
      match doc.current_image() {
        Ok(current) => {
          drop(doc_borrow);
          self.enable_sidebar_toggles(false);
          self.progress_bar.set_visible(true);
          self.progress_bar.set_fraction(0.0);
          self.worker.spawn_apply_operation(current, op);
        }
        Err(e) => {
          self.show_error_dialog(&gettext_replace(
            "Failed to prepare operation: {error}",
            &[("error", e.to_string())],
          ));
        }
      }
    }
  }

  fn trigger_open_dialog(self: &Rc<Self>) {
    let file_dialog = gtk4::FileDialog::new();
    file_dialog.set_title(&gettext("Open Image"));

    let filter_list = gio::ListStore::new::<gtk4::FileFilter>();
    let filter = gtk4::FileFilter::new();
    filter.set_name(Some(&gettext("Image Files")));
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
        Some(&gettext("Confirm Overwrite")),
        Some(&gettext_replace(
          "Are you sure you want to overwrite {file}?",
          &[(
            "file",
            path
              .file_name()
              .unwrap_or_default()
              .to_string_lossy()
              .into_owned(),
          )],
        )),
      );

      dialog.add_response("cancel", &gettext("Cancel"));
      dialog.add_response("confirm", &gettext("Overwrite"));
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
      file_dialog.set_title(&gettext("Save Image As..."));

      if let Some(path) = self.current_file_path.borrow().clone() {
        if let Some(name) = path.file_name() {
          file_dialog.set_initial_name(Some(&name.to_string_lossy()));
        }
      }

      let filter_list = gio::ListStore::new::<gtk4::FileFilter>();
      let filter = gtk4::FileFilter::new();
      filter.set_name(Some(&gettext("Images")));
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
              *self_cl.current_file_path.borrow_mut() = Some(path.clone());
              self_cl.refresh_window_title();
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
    let mut doc_borrow = self.document.borrow_mut();
    if let Some(ref mut doc) = *doc_borrow {
      match doc.current_image() {
        Ok(current) => {
          let policy = doc.metadata_policy;
          let metadata_edits = doc.metadata_edits.clone();
          drop(doc_borrow);

          self.progress_bar.set_visible(true);
          self.progress_bar.set_fraction(0.0);
          let original_path = self.current_file_path.borrow().clone();
          self.worker.spawn_export(
            current,
            path,
            original_path,
            policy,
            metadata_edits,
            options,
          );
        }
        Err(e) => {
          self.show_error_dialog(&gettext_replace(
            "Failed to prepare export: {error}",
            &[("error", e.to_string())],
          ));
        }
      }
    }
  }

  fn trigger_undo(self: &Rc<Self>) {
    let mut did_undo = false;
    let mut undo_error = None;
    let mut doc_borrow = self.document.borrow_mut();
    if let Some(ref mut doc) = *doc_borrow {
      if doc.undo() {
        match doc.current_image() {
          Ok(current) => {
            if let Err(e) = self.canvas.set_image(current) {
              undo_error = Some(pixie_core::CoreError::Internal(e));
            } else {
              self.history_buttons_visible.set(true);
              self.update_history_buttons(doc);
              did_undo = true;
            }
          }
          Err(e) => undo_error = Some(e),
        }
      }
    }

    drop(doc_borrow);
    if let Some(e) = undo_error {
      self.show_error_dialog(&gettext_replace(
        "Failed to undo operation: {error}",
        &[("error", e.to_string())],
      ));
      return;
    }
    if did_undo {
      self.update_properties_popover(false);
      self.refresh_window_title();
      self.show_toast(&gettext("Undo applied"));
    }
  }

  fn trigger_redo(self: &Rc<Self>) {
    let mut did_redo = false;
    let mut redo_error = None;
    let mut doc_borrow = self.document.borrow_mut();
    if let Some(ref mut doc) = *doc_borrow {
      if doc.redo() {
        match doc.current_image() {
          Ok(current) => {
            if let Err(e) = self.canvas.set_image(current) {
              redo_error = Some(pixie_core::CoreError::Internal(e));
            } else {
              self.history_buttons_visible.set(true);
              self.update_history_buttons(doc);
              did_redo = true;
            }
          }
          Err(e) => redo_error = Some(e),
        }
      }
    }

    drop(doc_borrow);
    if let Some(e) = redo_error {
      self.show_error_dialog(&gettext_replace(
        "Failed to redo operation: {error}",
        &[("error", e.to_string())],
      ));
      return;
    }
    if did_redo {
      self.update_properties_popover(false);
      self.refresh_window_title();
      self.show_toast(&gettext("Redo applied"));
    }
  }

  pub fn apply_crop(&self) {
    let state = self.canvas.state.borrow();
    if let Some(rect) = state.crop_rect {
      let op = if state.crop_mode == super::canvas::CropMode::Circle {
        ImageOperation::CircleMask(CircleMaskOperation::new(rect))
      } else {
        ImageOperation::Crop(CropOperation::new(rect))
      };
      drop(state);
      self.apply_image_operation(op);
    } else {
      drop(state);
    }
    self.set_floating_controls_visible(true);
    self.crop_actions_bar.set_visible(false);
    self.crop_btn.set_active(false);
  }

  pub fn cancel_crop(&self) {
    self.set_floating_controls_visible(true);
    self.crop_actions_bar.set_visible(false);
    self.crop_btn.set_active(false);
  }

  fn show_crop_tool(self: &Rc<Self>) {
    self.canvas.set_crop_active(false);
    self.crop_actions_bar.set_visible(false);
    self.close_tool_popover();
    self.close_metadata_cleaner_popover();

    let popover = gtk4::Popover::builder()
      .autohide(false)
      .has_arrow(true)
      .position(gtk4::PositionType::Top)
      .build();

    let self_start = self.clone();
    let popover_weak = popover.downgrade();
    let view = CropToolView::new(&self.canvas, move || {
      self_start.canvas.set_crop_active(true);
      self_start.crop_actions_bar.set_visible(true);
      self_start.set_floating_controls_visible(false);
      if let Some(popover) = popover_weak.upgrade() {
        popover.popdown();
      }
    });

    popover.set_child(Some(&view.container));
    popover.set_parent(&self.crop_btn);

    let active_popover = self.active_tool_popover.clone();
    let popover_weak = popover.downgrade();
    popover.connect_closed(move |_| {
      if let Some(closed_popover) = popover_weak.upgrade() {
        let mut active = active_popover.borrow_mut();
        if active.as_ref() == Some(&closed_popover) {
          *active = None;
        }
      }
    });

    popover.popup();
    *self.active_tool_popover.borrow_mut() = Some(popover);
  }

  fn show_resize_tool(self: &Rc<Self>) {
    self.canvas.set_crop_active(false);
    self.close_tool_popover();
    self.close_metadata_cleaner_popover();

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
        self_cl.close_tool_popover();
        self_cl.apply_image_operation(op);
      },
      move || {},
    );

    let popover = gtk4::Popover::builder()
      .autohide(false)
      .has_arrow(true)
      .position(gtk4::PositionType::Top)
      .build();
    popover.set_child(Some(&view.container));
    popover.set_parent(&self.resize_btn);

    let active_popover = self.active_tool_popover.clone();
    let popover_weak = popover.downgrade();
    popover.connect_closed(move |_| {
      if let Some(closed_popover) = popover_weak.upgrade() {
        let mut active = active_popover.borrow_mut();
        if active.as_ref() == Some(&closed_popover) {
          *active = None;
        }
      }
    });

    popover.popup();
    *self.active_tool_popover.borrow_mut() = Some(popover);
  }

  fn show_metadata_editor_popover(self: &Rc<Self>) {
    self.close_tool_popover();
    self.close_metadata_cleaner_popover();

    let popover = gtk4::Popover::builder()
      .autohide(false)
      .has_arrow(true)
      .position(gtk4::PositionType::Top)
      .build();

    let content = gtk4::Box::builder()
      .orientation(gtk4::Orientation::Vertical)
      .spacing(8)
      .margin_top(12)
      .margin_bottom(12)
      .margin_start(12)
      .margin_end(12)
      .width_request(300)
      .build();

    let title_label = gtk4::Label::builder()
      .label(gettext("Edit Metadata"))
      .halign(gtk4::Align::Start)
      .css_classes(["title-4"])
      .build();
    content.append(&title_label);

    // Helper: labelled entry row (no placeholder)
    let make_field = |label_text: String| -> (gtk4::Label, gtk4::Entry) {
      let lbl = gtk4::Label::builder()
        .label(label_text)
        .halign(gtk4::Align::Start)
        .build();
      let entry = gtk4::Entry::builder().hexpand(true).build();
      (lbl, entry)
    };

    let (desc_lbl, description_entry) = make_field(gettext("Image description"));
    let (artist_lbl, artist_entry) = make_field(gettext("Artist"));
    let (copyright_lbl, copyright_entry) = make_field(gettext("Copyright"));
    let (software_lbl, software_entry) = make_field(gettext("Software"));

    for (lbl, entry) in [
      (&desc_lbl, &description_entry),
      (&artist_lbl, &artist_entry),
      (&copyright_lbl, &copyright_entry),
      (&software_lbl, &software_entry),
    ] {
      content.append(lbl);
      content.append(entry);
    }

    // Pre-fill: pending edits take priority, else read from the original file.
    let file_fields = self
      .current_file_path
      .borrow()
      .as_deref()
      .map(pixie_core::read_image_metadata_fields)
      .unwrap_or_default();

    let fill = |entry: &gtk4::Entry, edit: &MetadataValueEdit, file_val: Option<&str>| match edit {
      MetadataValueEdit::Set(v) => entry.set_text(v),
      _ => {
        if let Some(v) = file_val {
          entry.set_text(v);
        }
      }
    };

    if let Some(ref doc) = *self.document.borrow() {
      fill(
        &description_entry,
        &doc.metadata_edits.image_description,
        file_fields.image_description.as_deref(),
      );
      fill(
        &artist_entry,
        &doc.metadata_edits.artist,
        file_fields.artist.as_deref(),
      );
      fill(
        &copyright_entry,
        &doc.metadata_edits.copyright,
        file_fields.copyright.as_deref(),
      );
      fill(
        &software_entry,
        &doc.metadata_edits.software,
        file_fields.software.as_deref(),
      );
    }

    let apply_btn = gtk4::Button::builder()
      .label(gettext("Apply"))
      .hexpand(true)
      .css_classes(["suggested-action", "pill"])
      .build();
    content.append(&apply_btn);

    let resolve = |entry: &gtk4::Entry| -> MetadataValueEdit {
      let v = entry.text().trim().to_string();
      if v.is_empty() {
        MetadataValueEdit::Keep
      } else {
        MetadataValueEdit::Set(v)
      }
    };

    let self_apply = self.clone();
    let popover_apply = popover.clone();
    let d_entry = description_entry.clone();
    let a_entry = artist_entry.clone();
    let c_entry = copyright_entry.clone();
    let s_entry = software_entry.clone();
    apply_btn.connect_clicked(move |_| {
      popover_apply.popdown();

      let mut changed = false;
      let mut doc_borrow = self_apply.document.borrow_mut();
      if let Some(ref mut doc) = *doc_borrow {
        doc.metadata_edits.image_description = resolve(&d_entry);
        doc.metadata_edits.artist = resolve(&a_entry);
        doc.metadata_edits.copyright = resolve(&c_entry);
        doc.metadata_edits.software = resolve(&s_entry);
        changed = true;
        self_apply.show_toast(&gettext("Metadata edits saved."));
      }
      drop(doc_borrow);
      if changed {
        self_apply.refresh_window_title();
      }
    });

    // Press Enter in any field triggers apply
    for entry in [
      &description_entry,
      &artist_entry,
      &copyright_entry,
      &software_entry,
    ] {
      let apply_btn_ref = apply_btn.clone();
      entry.connect_activate(move |_| {
        apply_btn_ref.emit_clicked();
      });
    }

    popover.set_child(Some(&content));
    popover.set_parent(&self.metadata_edit_btn);

    let active_popover = self.active_tool_popover.clone();
    let popover_weak = popover.downgrade();
    popover.connect_closed(move |_| {
      if let Some(closed_popover) = popover_weak.upgrade() {
        let mut active = active_popover.borrow_mut();
        if active.as_ref() == Some(&closed_popover) {
          *active = None;
        }
      }
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
    let popover = self.active_tool_popover.borrow_mut().take();
    if let Some(popover) = popover {
      popover.popdown();
    }
    self.sidebar.clear();
  }

  fn close_metadata_cleaner_popover(&self) {
    if let Some(popover) = self.metadata_btn.popover() {
      popover.popdown();
    }
  }

  fn deactivate_sidebar_tools(&self, close_metadata: bool) {
    self.crop_btn.set_active(false);
    self.resize_btn.set_active(false);
    self.canvas.set_crop_active(false);
    self.crop_actions_bar.set_visible(false);
    self.set_floating_controls_visible(true);
    self.close_tool_popover();
    if close_metadata {
      self.close_metadata_cleaner_popover();
    }
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
        Some(&gettext("Discard Changes?")),
        Some(&gettext(
          "The image has unsaved changes. Do you want to discard them and open the other image?",
        )),
      );

      dialog.add_response("cancel", &gettext("Cancel"));
      dialog.add_response("discard", &gettext("Discard"));
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
          Some(&gettext("Discard Changes?")),
          Some(&gettext(
            "The image has unsaved changes. Do you want to discard them and switch images?",
          )),
        );

        dialog.add_response("cancel", &gettext("Cancel"));
        dialog.add_response("discard", &gettext("Discard"));
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
        doc.history().len() != saved_idx
          || doc.metadata_policy != self.saved_metadata_policy.get()
          || doc.metadata_edits != *self.saved_metadata_edits.borrow()
      } else {
        false
      }
    } else {
      false
    }
  }

  fn prompt_close_confirmation(self: &Rc<Self>) {
    let dialog = adw::AlertDialog::new(
      Some(&gettext("Discard Changes?")),
      Some(&gettext(
        "The image has unsaved changes. Do you want to discard them and close the app?",
      )),
    );

    dialog.add_response("cancel", &gettext("Cancel"));
    dialog.add_response("discard", &gettext("Discard"));
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
    let dialog = adw::AlertDialog::new(Some(&gettext("Error")), Some(message));
    dialog.add_response("ok", &gettext("OK"));
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

  fn update_properties_popover(&self, force: bool) {
    if !force {
      if let Some(popover) = self.properties_btn.popover() {
        if !popover.is_mapped() {
          return;
        }
      }
    }
    let path_borrow = self.current_file_path.borrow();
    if let Some(ref path) = *path_borrow {
      let folder_name = path
        .parent()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| gettext("Unknown"));
      self.properties_folder_label.set_label(&folder_name);

      let format_str = if let Some(ref doc) = *self.document.borrow() {
        format!("{:?}", doc.original_format()).to_uppercase()
      } else {
        path
          .extension()
          .map(|e| e.to_string_lossy().to_string().to_uppercase())
          .unwrap_or_else(|| gettext("Unknown"))
      };
      self.properties_format_label.set_label(&format_str);

      let size_str = if let Some(ref img) = self.canvas.state.borrow().image {
        format!("{} × {}", img.width(), img.height())
      } else {
        gettext("Unknown")
      };
      self.properties_size_label.set_label(&size_str);

      if let Some(ref img) = self.canvas.state.borrow().image {
        let width = img.width();
        let height = img.height();

        let aspect_ratio = if height > 0 {
          format!("{:.2}:1", width as f64 / height as f64)
        } else {
          gettext("Unknown")
        };
        self.properties_aspect_ratio_label.set_label(&aspect_ratio);
      }

      if let Ok(metadata) = std::fs::metadata(path) {
        let len = metadata.len();
        let size_formatted = if len < 1024 {
          format!("{} B", len)
        } else if len < 1024 * 1024 {
          format!("{:.1} KB", len as f64 / 1024.0)
        } else {
          format!("{:.1} MB", len as f64 / (1024.0 * 1024.0))
        };
        self.properties_file_size_label.set_label(&size_formatted);

        let created_str = metadata
          .created()
          .ok()
          .map(format_system_time)
          .unwrap_or_else(|| gettext("Unknown"));
        self.properties_created_label.set_label(&created_str);

        let modified_str = metadata
          .modified()
          .ok()
          .map(format_system_time)
          .unwrap_or_else(|| gettext("Unknown"));
        self.properties_modified_label.set_label(&modified_str);
      } else {
        self
          .properties_file_size_label
          .set_label(&gettext("Unknown"));
        self.properties_created_label.set_label(&gettext("Unknown"));
        self
          .properties_modified_label
          .set_label(&gettext("Unknown"));
      }

      let metadata_fields = pixie_core::read_image_metadata_fields(path);

      let set_metadata_row = |row: &gtk4::ListBoxRow, label: &gtk4::Label, value: Option<&str>| {
        if let Some(v) = value.map(str::trim).filter(|s| !s.is_empty()) {
          label.set_label(v);
          row.set_visible(true);
        } else {
          label.set_label("");
          row.set_visible(false);
        }
      };

      set_metadata_row(
        &self.properties_comments_row,
        &self.properties_comments_label,
        metadata_fields.image_description.as_deref(),
      );
      set_metadata_row(
        &self.properties_artist_row,
        &self.properties_artist_label,
        metadata_fields.artist.as_deref(),
      );
      set_metadata_row(
        &self.properties_copyright_row,
        &self.properties_copyright_label,
        metadata_fields.copyright.as_deref(),
      );
      set_metadata_row(
        &self.properties_software_row,
        &self.properties_software_label,
        metadata_fields.software.as_deref(),
      );
    }
  }
}

fn format_system_time(system_time: std::time::SystemTime) -> String {
  if let Ok(duration) = system_time.duration_since(std::time::UNIX_EPOCH) {
    let timestamp = duration.as_secs() as i64;
    if let Ok(dt) = glib::DateTime::from_unix_local(timestamp) {
      if let Ok(formatted) = dt.format("%d/%m/%y %H:%M:%S") {
        return formatted.to_string();
      }
    }
  }
  gettext("Unknown")
}
