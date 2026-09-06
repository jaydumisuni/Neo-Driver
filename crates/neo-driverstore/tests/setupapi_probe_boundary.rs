const LIB_SOURCE: &str = include_str!("../src/lib.rs");
const STRICT_WINDOWS_SOURCE: &str = include_str!("../src/windows_strict.rs");

fn normalized_source(source: &str) -> String {
    source.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[test]
fn public_device_property_probe_distinguishes_absence_from_failure() {
    assert!(LIB_SOURCE.contains("pub use windows_strict::WindowsDriverHost"));
    for token in [
        "ERROR_INSUFFICIENT_BUFFER",
        "ERROR_NOT_FOUND",
        "is_missing_device_property",
        "is_insufficient_device_property_buffer",
        "device_property_wide",
        "DEVPKEY_Device_HardwareIds",
        "DEVPKEY_Device_CompatibleIds",
        "DEVPKEY_Device_DriverInfPath",
        "DEVPKEY_Device_UpperFilters",
        "DEVPKEY_Device_LowerFilters",
    ] {
        assert!(
            STRICT_WINDOWS_SOURCE.contains(token),
            "missing unified SetupAPI evidence-boundary token: {token}"
        );
    }
    assert!(!STRICT_WINDOWS_SOURCE.contains("SetupDiGetDeviceRegistryPropertyW"));
    assert!(!STRICT_WINDOWS_SOURCE.contains("ERROR_INVALID_DATA"));
    assert!(!STRICT_WINDOWS_SOURCE.contains("is_missing_registry_property"));
    assert!(!normalized_source(STRICT_WINDOWS_SOURCE)
        .contains("let _ = unsafe { SetupDiGetDevicePropertyW"));
}

#[test]
fn public_driver_evidence_requires_documented_property_types() {
    for token in [
        "DEVPROP_TYPE_STRING",
        "DEVPROP_TYPE_STRING_LIST",
        "expected_property_type",
        "device property type",
        "bytes_to_u16",
        "odd byte count",
    ] {
        assert!(
            STRICT_WINDOWS_SOURCE.contains(token),
            "missing SetupAPI property-type authority token: {token}"
        );
    }
}

#[test]
fn class_guid_comes_from_enumerated_devinfo_not_ambiguous_registry_data() {
    assert!(STRICT_WINDOWS_SOURCE.contains("class_guid_from_devinfo"));
    assert!(STRICT_WINDOWS_SOURCE.contains("data.ClassGuid"));
    assert!(!STRICT_WINDOWS_SOURCE.contains("SPDRP_CLASSGUID"));
}

#[test]
fn public_setupapi_id_dedup_is_case_insensitive_and_stable() {
    assert!(STRICT_WINDOWS_SOURCE.contains("existing.eq_ignore_ascii_case(&value)"));
}
