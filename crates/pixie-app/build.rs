fn main() {
  glib_build_tools::compile_resources(
    &["resources", "../../data"],
    "resources/resources.gresource.xml",
    "pixie.gresource",
  );
}
