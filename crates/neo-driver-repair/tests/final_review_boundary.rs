const ASSESSMENT_SOURCE: &str = include_str!("../src/assessment.rs");
const MODEL_SOURCE: &str = include_str!("../src/model.rs");
const WINDOWS_DRIVERSTORE_LIB: &str = include_str!("../../neo-driverstore/src/lib.rs");
const STRICT_WINDOWS_DRIVERSTORE_SOURCE: &str =
    include_str!("../../neo-driverstore/src/windows_strict.rs");

#[test]
fn phase5_oem_inf_law_has_one_shared_source_of_truth() {
    assert_eq!(
        MODEL_SOURCE
            .matches("fn is_phase5_oem_published_inf")
            .count(),
        1
    );
    assert!(MODEL_SOURCE.contains("pub(crate) fn is_phase5_oem_published_inf"));
    assert_eq!(
        ASSESSMENT_SOURCE
            .matches("fn is_phase5_oem_published_inf")
            .count(),
        0
    );
    assert!(ASSESSMENT_SOURCE.contains("use crate::model::is_phase5_oem_published_inf;"));
    assert!(ASSESSMENT_SOURCE.contains("Some(value) if is_phase5_oem_published_inf(value)"));
}

#[test]
fn imported_exact_package_authority_remains_oem_only() {
    assert!(MODEL_SOURCE.contains("if !is_phase5_oem_published_inf(published)"));
    assert!(MODEL_SOURCE.contains("|| !is_phase5_oem_published_inf(&package.published_inf)"));
    assert!(MODEL_SOURCE.contains("current package is not a Phase 5 OEM published INF identity"));
}

#[test]
fn imported_exact_package_authority_requires_driver_store_path_shape() {
    assert_eq!(
        MODEL_SOURCE.matches("fn is_driver_store_inf_path").count(),
        1
    );
    for token in [
        "System32",
        "DriverStore",
        "FileRepository",
        "if !is_driver_store_inf_path(&package.driver_store_inf)",
        "fully qualified Driver Store FileRepository INF path",
    ] {
        assert!(
            MODEL_SOURCE.contains(token),
            "missing imported Driver Store path authority token: {token}"
        );
    }
}

#[test]
fn live_windows_inventory_collects_real_filter_evidence_through_public_strict_host() {
    assert!(WINDOWS_DRIVERSTORE_LIB.contains("pub use windows_strict::WindowsDriverHost;"));
    for token in [
        "DEVPKEY_Device_UpperFilters",
        "DEVPKEY_Device_LowerFilters",
        "device_property_multisz(set.0, &data, &DEVPKEY_Device_UpperFilters)?",
        "device_property_multisz(set.0, &data, &DEVPKEY_Device_LowerFilters)?",
        "ERROR_NOT_FOUND",
        "ERROR_INSUFFICIENT_BUFFER",
        "DEVPROP_TYPE_STRING_LIST",
        "upper_filters,",
        "lower_filters,",
    ] {
        assert!(
            STRICT_WINDOWS_DRIVERSTORE_SOURCE.contains(token),
            "missing strict live Windows filter-evidence boundary token: {token}"
        );
    }
    assert!(!STRICT_WINDOWS_DRIVERSTORE_SOURCE.contains("SetupDiGetDeviceRegistryPropertyW"));
    assert!(!STRICT_WINDOWS_DRIVERSTORE_SOURCE.contains("ERROR_INVALID_DATA"));
    assert!(!STRICT_WINDOWS_DRIVERSTORE_SOURCE.contains("upper_filters: vec![]"));
    assert!(!STRICT_WINDOWS_DRIVERSTORE_SOURCE.contains("lower_filters: vec![]"));
}
