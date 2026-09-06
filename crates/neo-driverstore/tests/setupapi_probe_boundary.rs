const WINDOWS_SOURCE: &str = include_str!("../src/windows.rs");

#[test]
fn registry_property_probe_distinguishes_absence_from_failure() {
    assert!(WINDOWS_SOURCE.contains("ERROR_INSUFFICIENT_BUFFER"));
    assert!(WINDOWS_SOURCE.contains("ERROR_INVALID_DATA"));
    assert!(WINDOWS_SOURCE.contains("is_missing_registry_property"));
    assert!(WINDOWS_SOURCE.contains("is_insufficient_registry_buffer"));
    assert!(!WINDOWS_SOURCE.contains(
        "let _ = unsafe {\n        SetupDiGetDeviceRegistryPropertyW"
    ));
}
