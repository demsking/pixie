use little_exif::exif_tag::ExifTag;

#[test]
fn test_debug_str() {
    let tag = ExifTag::ImageDescription("test".to_string());
    let debug_str = format!("{:?}", tag);
    assert!(debug_str.starts_with("ImageDescription"));
}
