const MODEL_SOURCE: &str = include_str!("../src/model.rs");
const WINDOWS_DRIVERSTORE_SOURCE: &str = include_str!("../../neo-driverstore/src/windows.rs");

#[test]
fn imported_exact_package_authority_remains_oem_only() {
    assert!(MODEL_SOURCE.contains("if !is_phase5_oem_published_inf(published)"));
    assert!(MODEL_SOURCE.contains("|| !is_phase5_oem_published_inf(&package.published_inf)"));
    assert!(MODEL_SOURCE.contains("current package is not a Phase 5 OEM published INF identity"));
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
