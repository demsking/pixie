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
      gettext("Pixie is a lightweight image editor designed for GNOME. It gives you straightforward tools to crop, resize, rotate, mirror, flip, clean metadata, remove backgrounds, and convert images, so your pictures are ready for sharing, saving, or publishing.")
    )
    .license_type(gtk4::License::Gpl30)
    .website("https://gitlab.com/demsking/pixie")
    .issue_url("https://gitlab.com/demsking/pixie/-/issues")
    .support_url("https://gitlab.com/demsking/pixie/-/issues/service_desk")
    .build();
  dialog.present(Some(parent));
}
