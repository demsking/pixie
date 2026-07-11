use gtk4::prelude::*;
use libadwaita as adw;

use crate::ui::PixieWindow;

pub struct PixieApplication {
  pub app: adw::Application,
}

impl PixieApplication {
  pub fn new() -> Self {
    // Register GResources compiled by build.rs
    gio::resources_register_include!("pixie.gresource").expect("Failed to register GResources");

    let app = adw::Application::builder()
      .application_id("io.gitlab.demsking.Pixie")
      .flags(gio::ApplicationFlags::HANDLES_OPEN)
      .build();

    let app_instance = Self { app };
    app_instance.setup_signals();
    app_instance
  }

  fn setup_signals(&self) {
    self.app.connect_startup(|_| {
      load_css();
      register_icon_theme_path();
    });

    let app = self.app.clone();
    let new_window_action = gio::SimpleAction::new("new-window", None);
    let app_for_action = app.clone();
    new_window_action.connect_activate(move |_, _| {
      let _ = PixieWindow::new(&app_for_action);
    });
    app.add_action(&new_window_action);
    app.set_accels_for_action("app.new-window", &["<Primary>n"]);

    self.app.connect_activate(move |app| {
      let _win = PixieWindow::new(app);
    });

    self.app.connect_open(move |app, files, _hint| {
      for file in files {
        if let Some(path) = file.path() {
          let win = PixieWindow::new(app);
          win.resize_to_image_on_load.set(true);
          win.open_file(path);
        }
      }
    });
  }

  pub fn run(&self) -> glib::ExitCode {
    self.app.run()
  }
}

fn load_css() {
  let provider = gtk4::CssProvider::new();
  provider.load_from_resource("/io/gitlab/demsking/Pixie/style.css");
  if let Some(display) = gtk4::gdk::Display::default() {
    gtk4::style_context_add_provider_for_display(
      &display,
      &provider,
      gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
  }
}

fn register_icon_theme_path() {
  if let Some(display) = gtk4::gdk::Display::default() {
    let theme = gtk4::IconTheme::for_display(&display);
    theme.add_resource_path("/io/gitlab/demsking/Pixie/icons");
  }
}
