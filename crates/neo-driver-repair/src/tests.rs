use super::*;
use neo_device::{DeviceRecord, DriverBinding, OpaqueDeviceId, OrderedDeviceIds};
use neo_driverstore::{
    DriverBackendResult, DriverHost, DriverInventory, DriverStoreError, StoredDriverPackage,
    VerifiedInfSignature,
};
use std::path::{Path, PathBuf};

fn device(instance: &str, problem_code: Option<u32>, published: Option<&str>) -> DeviceRecord {
    DeviceRecord {
        instance_id: OpaqueDeviceId::new(instance).unwrap(),
        description: Some(format!("Device {instance}")),
        manufacturer: Some("THETECHGUY fixture".to_string()),
        class_name: Some("Fixture".to_string()),
        class_guid: None,
        problem_code,
        disabled: Some(false),
        ids: OrderedDeviceIds::default(),
        active_driver: published.map(|value| DriverBinding {
            published_name: Some(value.to_string()),
            ..DriverBinding::default()
        }),
        upper_filters: vec![],
        lower_filters: vec![],
    }
}

fn package(published: &str) -> StoredDriverPackage {
    StoredDriverPackage {
        published_inf: published.to_string(),
        driver_store_inf: PathBuf::from(format!(
            r"C:\Windows\System32\DriverStore\FileRepository\fixture\{published}"
        )),
    }
}

fn evidence(
    device: DeviceRecord,
    current_package: Option<StoredDriverPackage>,
) -> DriverRepairEvidence {
    DriverRepairEvidence {
        devices: vec![DriverRepairDeviceEvidence {
            device,
            current_package,
        }],
    }
}

#[test]
fn healthy_exact_binding_requires_no_action() {
    let report = assess_driver_repair_evidence(evidence(
        device("PCI\\A", Some(0), Some("oem10.inf")),
        Some(package("oem10.inf")),
    ))
    .unwrap();
    assert_eq!(report.assessments[0].state, DriverRepairState::Healthy);
    assert_eq!(report.assessments[0].route, DriverRepairRoute::NoAction);
    assert!(!report.machine_changes);
}

#[test]
fn pnp_problem_with_exact_baseline_is_only_a_reinstall_candidate() {
    let report = assess_driver_repair_evidence(evidence(
        device("PCI\\B", Some(28), Some("oem11.inf")),
        Some(package("oem11.inf")),
    ))
    .unwrap();
    assert_eq!(report.assessments[0].state, DriverRepairState::PnpProblem);
    assert_eq!(
        report.assessments[0].route,
        DriverRepairRoute::CurrentExactDriverReinstallCandidate
    );
    assert!(report.assessments[0]
        .detail
        .contains("future authority phase"));
}

#[test]
fn problem_without_binding_requires_existing_selection_authority() {
    let report =
        assess_driver_repair_evidence(evidence(device("PCI\\C", Some(28), None), None)).unwrap();
    assert_eq!(
        report.assessments[0].state,
        DriverRepairState::MissingDriverBinding
    );
    assert_eq!(
        report.assessments[0].route,
        DriverRepairRoute::DriverSelectionRequired
    );
}

#[test]
fn healthy_problem_code_without_exact_store_package_fails_closed() {
    let report =
        assess_driver_repair_evidence(evidence(device("PCI\\D", Some(0), Some("oem12.inf")), None))
            .unwrap();
    assert_eq!(
        report.assessments[0].state,
        DriverRepairState::EvidenceUnavailable
    );
    assert_eq!(
        report.assessments[0].route,
        DriverRepairRoute::ManualInvestigation
    );
}

#[test]
fn unknown_problem_code_never_becomes_healthy() {
    let report = assess_driver_repair_evidence(evidence(
        device("PCI\\E", None, Some("oem13.inf")),
        Some(package("oem13.inf")),
    ))
    .unwrap();
    assert_eq!(
        report.assessments[0].state,
        DriverRepairState::EvidenceUnavailable
    );
}

#[test]
fn disabled_device_is_recorded_without_enable_authority() {
    let mut item = device("PCI\\F", Some(22), Some("oem14.inf"));
    item.disabled = Some(true);
    let report = assess_driver_repair_evidence(evidence(item, Some(package("oem14.inf")))).unwrap();
    assert_eq!(report.assessments[0].state, DriverRepairState::Disabled);
    assert_eq!(
        report.assessments[0].route,
        DriverRepairRoute::ManualInvestigation
    );
    assert!(report.assessments[0]
        .detail
        .contains("no enable or re-enumeration authority"));
}

#[test]
fn package_without_active_binding_is_rejected() {
    let error = assess_driver_repair_evidence(evidence(
        device("PCI\\G", Some(28), None),
        Some(package("oem15.inf")),
    ))
    .unwrap_err();
    assert!(matches!(error, DriverRepairError::PackageWithoutBinding(_)));
}

#[test]
fn mismatched_driver_store_identity_is_rejected() {
    let error = assess_driver_repair_evidence(evidence(
        device("PCI\\H", Some(28), Some("oem16.inf")),
        Some(package("oem17.inf")),
    ))
    .unwrap_err();
    assert!(matches!(error, DriverRepairError::PackageMismatch(_)));
}

#[test]
fn duplicate_instance_ids_are_case_insensitive() {
    let a = DriverRepairDeviceEvidence {
        device: device("PCI\\VEN_ABCD", Some(0), Some("oem18.inf")),
        current_package: Some(package("oem18.inf")),
    };
    let mut b = a.clone();
    b.device.instance_id = OpaqueDeviceId::new("pci\\ven_abcd").unwrap();
    let error = assess_driver_repair_evidence(DriverRepairEvidence {
        devices: vec![a, b],
    })
    .unwrap_err();
    assert!(matches!(error, DriverRepairError::DuplicateDevice(_)));
}

#[test]
fn output_order_and_digest_are_independent_of_inventory_order() {
    let a = DriverRepairDeviceEvidence {
        device: device("PCI\\A", Some(0), Some("oem20.inf")),
        current_package: Some(package("oem20.inf")),
    };
    let b = DriverRepairDeviceEvidence {
        device: device("PCI\\B", Some(28), Some("oem21.inf")),
        current_package: Some(package("oem21.inf")),
    };
    let left = assess_driver_repair_evidence(DriverRepairEvidence {
        devices: vec![a.clone(), b.clone()],
    })
    .unwrap();
    let right = assess_driver_repair_evidence(DriverRepairEvidence {
        devices: vec![b, a],
    })
    .unwrap();
    assert_eq!(left, right);
    assert_eq!(left.assessments[0].instance_id, "PCI\\A");
}

#[test]
fn filters_are_retained_as_evidence_not_inferred_as_fault() {
    let mut item = device("USB\\FILTERED", Some(0), Some("oem22.inf"));
    item.upper_filters = vec!["FixtureUpper".to_string()];
    item.lower_filters = vec!["FixtureLower".to_string()];
    let report = assess_driver_repair_evidence(evidence(item, Some(package("oem22.inf")))).unwrap();
    assert_eq!(report.assessments[0].state, DriverRepairState::Healthy);
    assert_eq!(report.assessments[0].route, DriverRepairRoute::NoAction);
    assert_eq!(report.assessments[0].upper_filters, vec!["FixtureUpper"]);
    assert_eq!(report.assessments[0].lower_filters, vec!["FixtureLower"]);
}

#[derive(Clone)]
struct ReadOnlyHost {
    inventory: DriverInventory,
}

impl DriverHost for ReadOnlyHost {
    fn windows_build(&self) -> Result<u32, DriverStoreError> {
        panic!("Phase 22 must not query unrelated host state")
    }

    fn inventory(&self) -> Result<DriverInventory, DriverStoreError> {
        Ok(self.inventory.clone())
    }

    fn compatible_present_devices(&self, _inf: &Path) -> Result<Vec<String>, DriverStoreError> {
        panic!("Phase 22 must not run install compatibility discovery")
    }

    fn verify_inf_signature(&self, _inf: &Path) -> Result<VerifiedInfSignature, DriverStoreError> {
        panic!("Phase 22 must not verify a proposed install package")
    }

    fn find_equivalent_package(
        &self,
        _source_inf: &Path,
        _catalogue_files: &[String],
    ) -> Result<Option<StoredDriverPackage>, DriverStoreError> {
        panic!("Phase 22 must not search proposed install packages")
    }

    fn resolve_published_package(
        &self,
        published_inf: &str,
    ) -> Result<Option<StoredDriverPackage>, DriverStoreError> {
        Ok(Some(package(published_inf)))
    }

    fn stage_driver(&self, _source_inf: &Path) -> Result<StoredDriverPackage, DriverStoreError> {
        panic!("Phase 22 has no stage authority")
    }

    fn install_best_match(
        &self,
        _instance_id: &str,
    ) -> Result<DriverBackendResult, DriverStoreError> {
        panic!("Phase 22 has no install authority")
    }

    fn restore_specific_driver(
        &self,
        _instance_id: &str,
        _published_inf: &str,
    ) -> Result<DriverBackendResult, DriverStoreError> {
        panic!("Phase 22 has no rollback mutation authority")
    }

    fn remove_published_package(&self, _published_inf: &str) -> Result<(), DriverStoreError> {
        panic!("Phase 22 has no Driver Store delete authority")
    }
}

#[test]
fn live_adapter_invokes_only_inventory_and_exact_package_resolution() {
    let host = ReadOnlyHost {
        inventory: DriverInventory {
            devices: vec![device("PCI\\LIVE", Some(28), Some("oem30.inf"))],
        },
    };
    let report = crate::assessment::capture_and_assess_with_host(&host).unwrap();
    assert_eq!(
        report.assessments[0].route,
        DriverRepairRoute::CurrentExactDriverReinstallCandidate
    );
    assert!(!report.machine_changes);
}
