use libadwaita::prelude::*;

use crate::i18n::gettext;

pub fn show_about_dialog(parent: &gtk4::Window) {
  let dialog = libadwaita::AboutDialog
    ::builder()
    .application_name("Pixie")
    .application_icon("io.gitlab.demsking.Pixie")
    .developer_name(gettext("Modern Image Processor"))
    .version(env!("CARGO_PKG_VERSION"))
    .copyright(gettext("Copyright © Sébastien Demanou"))
    .comments(
      gettext("Crop, resize, rotate, flip, and convert your images with precision—all in a clean, intuitive interface that respects your privacy and performance.")
    )
    .license_type(gtk4::License::Gpl30)
    .website("https://gitlab.com/demsking/pixie")
    .support_url("https://demsking.gitlab.io/pixie")
    .build();
  dialog.present(Some(parent));
}
