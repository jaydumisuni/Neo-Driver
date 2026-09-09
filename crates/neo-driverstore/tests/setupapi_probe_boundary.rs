const LIB_SOURCE: &str = include_str!("../src/lib.rs");
const STRICT_WINDOWS_SOURCE: &str = include_str!("../src/windows_strict.rs");
const LEGACY_WINDOWS_SOURCE: &str = include_str!("../src/windows.rs");

fn normalized_source(source: &str) -> String {
    source.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn function_body<'a>(source: &'a str, name: &str, next_name: &str) -> &'a str {
    let start = source
        .find(&format!("fn {name}"))
        .unwrap_or_else(|| panic!("missing function {name}"));
    let end = source[start..]
        .find(&format!("fn {next_name}"))
        .map(|offset| start + offset)
        .unwrap_or(source.len());
    &source[start..end]
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

    let property_wide = normalized_source(function_body(
        STRICT_WINDOWS_SOURCE,
        "device_property_wide",
        "ensure_device_property_type",
    ));
    assert_eq!(
        property_wide.matches("SetupDiGetDevicePropertyW(").count(),
        2
    );
    assert!(property_wide.contains("let sizing = unsafe { SetupDiGetDevicePropertyW("));
    assert!(property_wide.contains("match sizing {"));
    assert!(!property_wide.contains("let _ = unsafe { SetupDiGetDevicePropertyW("));
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

#[test]
fn config_manager_problem_evidence_requires_dn_has_problem_consistency() {
    let problem = function_body(STRICT_WINDOWS_SOURCE, "problem_code", "decode_problem_code");
    let decode = function_body(
        STRICT_WINDOWS_SOURCE,
        "decode_problem_code",
        "stable_unique",
    );

    assert!(problem.contains("decode_problem_code(result, status, problem)"));
    for token in [
        "DN_HAS_PROBLEM",
        "status.0 & DN_HAS_PROBLEM.0 != 0",
        "(false, 0) => Ok(None)",
        "(true, code @ 1..=u32::MAX) => Ok(Some(code))",
        "without a nonzero problem code",
        "without DN_HAS_PROBLEM",
        "config_manager_problem_decode_requires_status_flag_and_code_consistency",
    ] {
        assert!(
            STRICT_WINDOWS_SOURCE.contains(token),
            "missing Config Manager PnP-status authority token: {token}"
        );
    }
    assert!(decode.contains("result != CR_SUCCESS"));
}

#[test]
fn exact_package_resolution_rejects_lossy_utf16_evidence() {
    let location = function_body(
        LEGACY_WINDOWS_SOURCE,
        "driver_store_location",
        "published_name_for_store_inf",
    );
    let published = function_body(
        LEGACY_WINDOWS_SOURCE,
        "published_name_for_store_inf",
        "source_catalog_path",
    );

    for body in [location, published] {
        assert!(body.contains("strict_utf16_api_string"));
        assert!(!body.contains("utf16_array(&buffer)"));
        assert!(!body.contains("from_utf16_lossy"));
    }
    assert!(LEGACY_WINDOWS_SOURCE.contains("fn strict_utf16_api_string"));
    assert!(LEGACY_WINDOWS_SOURCE.contains("String::from_utf16(&value[..end])"));
    assert!(LEGACY_WINDOWS_SOURCE.contains("exact_package_identity_utf16_is_fail_closed"));
}
