use pixie_core::{decode_image, limits};

#[test]
fn test_magic_number_detection_malformed() {
  let bad_data = vec![0u8; 100];
  let res = decode_image(&bad_data);
  assert!(res.is_err());
}

#[test]
fn test_excessive_dimensions_rejection() {
  let limit_check = limits::check_dimensions(20000, 20000);
  assert!(limit_check.is_err());
}

#[test]
fn test_excessive_memory_rejection() {
  // 16384 * 16384 * 4 * 1 = 1 GB (exactly maximum limit)
  let boundary_check = limits::check_memory_allocation(16384, 16384, 4, 1);
  assert!(boundary_check.is_ok());

  // Exceeding 1 GB
  let exceed_check = limits::check_memory_allocation(16385, 16384, 4, 1);
  assert!(exceed_check.is_err());
}
