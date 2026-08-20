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
        disabled: None,
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

fn item(
    device: DeviceRecord,
    current_package: Option<StoredDriverPackage>,
) -> DriverRepairDeviceEvidence {
    let pnp_status = PnpStatusEvidence::from_device(&device).unwrap();
    DriverRepairDeviceEvidence {
        device,
        pnp_status,
        current_package,
    }
}

fn evidence(
    device: DeviceRecord,
    current_package: Option<StoredDriverPackage>,
) -> DriverRepairEvidence {
    DriverRepairEvidence {
        devices: vec![item(device, current_package)],
    }
}

#[test]
fn healthy_exact_binding_requires_no_action() {
    let report = assess_driver_repair_evidence(evidence(
        device("PCI\\A", None, Some("oem10.inf")),
        Some(package("oem10.inf")),
    ))
    .unwrap();
    assert_eq!(report.assessments[0].pnp_status, PnpStatusEvidence::NoProblem);
    assert_eq!(report.assessments[0].problem_code, None);
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
    assert_eq!(
        report.assessments[0].pnp_status,
        PnpStatusEvidence::Problem { code: 28 }
    );
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
fn no_problem_without_exact_store_package_fails_closed() {
    let report =
        assess_driver_repair_evidence(evidence(device("PCI\\D", None, Some("oem12.inf")), None))
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
fn no_problem_without_binding_does_not_invent_driver_selection_need() {
    let report = assess_driver_repair_evidence(evidence(device("ROOT\\NO_DRIVER", None, None), None))
        .unwrap();
    assert_eq!(
        report.assessments[0].state,
        DriverRepairState::EvidenceUnavailable
    );
    assert_eq!(
        report.assessments[0].route,
        DriverRepairRoute::ManualInvestigation
    );
    assert!(report.assessments[0]
        .detail
        .contains("will not infer that driver selection or repair is required"));
}

#[test]
fn problem_code_zero_is_rejected_as_noncanonical_phase5_evidence() {
    let device = device("PCI\\ZERO", Some(0), Some("oem13.inf"));
    let error = PnpStatusEvidence::from_device(&device).unwrap_err();
    assert!(matches!(error, DriverRepairError::InvalidEvidence(_)));
}

#[test]
fn explicit_pnp_status_must_match_device_problem_evidence() {
    let evidence = DriverRepairEvidence {
        devices: vec![DriverRepairDeviceEvidence {
            device: device("PCI\\MISMATCH", None, Some("oem13.inf")),
            pnp_status: PnpStatusEvidence::Problem { code: 28 },
            current_package: Some(package("oem13.inf")),
        }],
    };
    let error = assess_driver_repair_evidence(evidence).unwrap_err();
    assert!(matches!(error, DriverRepairError::InvalidEvidence(_)));
}

#[test]
fn disabled_device_is_recorded_without_enable_authority() {
    let mut device = device("PCI\\F", Some(22), Some("oem14.inf"));
    device.disabled = Some(true);
    let report = assess_driver_repair_evidence(evidence(device, Some(package("oem14.inf")))).unwrap();
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
fn cm_prob_disabled_is_authoritative_when_generic_disabled_field_is_unavailable() {
    let report = assess_driver_repair_evidence(evidence(
        device("PCI\\CODE22", Some(22), Some("oem14.inf")),
        Some(package("oem14.inf")),
    ))
    .unwrap();
    assert_eq!(report.assessments[0].disabled, None);
    assert_eq!(report.assessments[0].state, DriverRepairState::Disabled);
    assert_eq!(
        report.assessments[0].route,
        DriverRepairRoute::ManualInvestigation
    );
}

#[test]
fn contradictory_disabled_evidence_fails_closed() {
    let mut device = device("PCI\\CONTRADICT", Some(22), Some("oem14.inf"));
    device.disabled = Some(false);
    let error = assess_driver_repair_evidence(evidence(device, Some(package("oem14.inf"))))
        .unwrap_err();
    assert!(matches!(error, DriverRepairError::InvalidEvidence(_)));
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
    let a = item(
        device("PCI\\VEN_ABCD", None, Some("oem18.inf")),
        Some(package("oem18.inf")),
    );
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
    let a = item(
        device("PCI\\A", None, Some("oem20.inf")),
        Some(package("oem20.inf")),
    );
    let b = item(
        device("PCI\\B", Some(28), Some("oem21.inf")),
        Some(package("oem21.inf")),
    );
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
    let mut device = device("USB\\FILTERED", None, Some("oem22.inf"));
    device.upper_filters = vec!["FixtureUpper".to_string()];
    device.lower_filters = vec!["FixtureLower".to_string()];
    let report = assess_driver_repair_evidence(evidence(device, Some(package("oem22.inf")))).unwrap();
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
fn live_adapter_maps_phase5_none_to_no_problem_and_uses_only_read_authority() {
    let host = ReadOnlyHost {
        inventory: DriverInventory {
            devices: vec![device("PCI\\LIVE_HEALTHY", None, Some("oem30.inf"))],
        },
    };
    let report = crate::assessment::capture_and_assess_with_host(&host).unwrap();
    assert_eq!(report.assessments[0].pnp_status, PnpStatusEvidence::NoProblem);
    assert_eq!(report.assessments[0].state, DriverRepairState::Healthy);
    assert_eq!(report.assessments[0].route, DriverRepairRoute::NoAction);
    assert!(!report.machine_changes);
}

#[test]
fn live_adapter_problem_path_invokes_only_inventory_and_exact_package_resolution() {
    let host = ReadOnlyHost {
        inventory: DriverInventory {
            devices: vec![device("PCI\\LIVE_PROBLEM", Some(28), Some("oem31.inf"))],
        },
    };
    let report = crate::assessment::capture_and_assess_with_host(&host).unwrap();
    assert_eq!(
        report.assessments[0].route,
        DriverRepairRoute::CurrentExactDriverReinstallCandidate
    );
    assert!(!report.machine_changes);
}
