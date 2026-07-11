#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod application;
mod errors;
mod logging;
mod settings;
mod ui;
mod workers;

use application::PixieApplication;

fn main() -> glib::ExitCode {
  logging::init_logging();
  let app = PixieApplication::new();
  app.run()
}
