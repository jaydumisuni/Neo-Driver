const LIB_SOURCE: &str = include_str!("../src/lib.rs");
const STRICT_WINDOWS_SOURCE: &str = include_str!("../src/windows_strict.rs");

#[test]
fn public_registry_property_probe_distinguishes_absence_from_failure() {
    assert!(LIB_SOURCE.contains("pub use windows_strict::WindowsDriverHost"));
    assert!(STRICT_WINDOWS_SOURCE.contains("ERROR_INSUFFICIENT_BUFFER"));
    assert!(STRICT_WINDOWS_SOURCE.contains("ERROR_INVALID_DATA"));
    assert!(STRICT_WINDOWS_SOURCE.contains("ERROR_NOT_FOUND"));
    assert!(STRICT_WINDOWS_SOURCE.contains("is_missing_registry_property"));
    assert!(STRICT_WINDOWS_SOURCE.contains("is_missing_device_property"));
    assert!(STRICT_WINDOWS_SOURCE.contains("is_insufficient_registry_buffer"));
    assert!(!STRICT_WINDOWS_SOURCE.contains(
        "let _ = unsafe {\n        SetupDiGetDeviceRegistryPropertyW"
    ));
    assert!(!STRICT_WINDOWS_SOURCE.contains(
        "let _ = unsafe {\n        SetupDiGetDevicePropertyW"
    ));
}
