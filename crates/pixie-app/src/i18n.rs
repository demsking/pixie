pub const GETTEXT_PACKAGE: &str = "pixie";

pub fn gettext(message: &str) -> String {
  glib::dgettext(Some(GETTEXT_PACKAGE), message).to_string()
}

pub fn gettext_replace(message: &str, replacements: &[(&str, String)]) -> String {
  let mut translated = gettext(message);
  for (key, value) in replacements {
    translated = translated.replace(&format!("{{{key}}}"), value);
  }
  translated
}
