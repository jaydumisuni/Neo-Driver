const ASSESSMENT_SOURCE: &str = include_str!("../src/assessment.rs");
const MODEL_SOURCE: &str = include_str!("../src/model.rs");
const WINDOWS_DRIVERSTORE_SOURCE: &str = include_str!("../../neo-driverstore/src/windows.rs");

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
fn live_windows_inventory_collects_real_filter_evidence() {
    for token in [
        "SPDRP_UPPERFILTERS",
        "SPDRP_LOWERFILTERS",
        "let upper_filters = registry_multisz(set.0, &data, SPDRP_UPPERFILTERS)?;",
        "let lower_filters = registry_multisz(set.0, &data, SPDRP_LOWERFILTERS)?;",
        "upper_filters,",
        "lower_filters,",
    ] {
        assert!(
            WINDOWS_DRIVERSTORE_SOURCE.contains(token),
            "missing live Windows filter-evidence boundary token: {token}"
        );
    }
    assert!(!WINDOWS_DRIVERSTORE_SOURCE.contains("upper_filters: vec![]"));
    assert!(!WINDOWS_DRIVERSTORE_SOURCE.contains("lower_filters: vec![]"));
}
