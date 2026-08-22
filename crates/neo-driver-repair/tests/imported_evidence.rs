use neo_device::{DeviceRecord, DriverBinding, OpaqueDeviceId, OrderedDeviceIds};
use neo_driver_repair::{
    assess_driver_repair_evidence, DriverRepairDeviceEvidence, DriverRepairError,
    DriverRepairEvidence, PnpStatusEvidence,
};
use neo_driverstore::StoredDriverPackage;
use std::path::PathBuf;

#[test]
fn imported_inbox_inf_cannot_claim_exact_package_authority() {
    let evidence = DriverRepairEvidence {
        devices: vec![DriverRepairDeviceEvidence {
            device: DeviceRecord {
                instance_id: OpaqueDeviceId::new("ROOT\\INBOX").unwrap(),
                description: Some("Inbox driver fixture".to_string()),
                manufacturer: Some("Microsoft".to_string()),
                class_name: Some("System".to_string()),
                class_guid: None,
                problem_code: None,
                disabled: None,
                ids: OrderedDeviceIds::default(),
                active_driver: Some(DriverBinding {
                    published_name: Some("machine.inf".to_string()),
                    ..DriverBinding::default()
                }),
                upper_filters: vec![],
                lower_filters: vec![],
            },
            pnp_status: PnpStatusEvidence::NoProblem,
            current_package: Some(StoredDriverPackage {
                published_inf: "machine.inf".to_string(),
                driver_store_inf: PathBuf::from(
                    r"C:\Windows\System32\DriverStore\FileRepository\machine.inf_amd64\machine.inf",
                ),
            }),
        }],
    };

    let error = assess_driver_repair_evidence(evidence).unwrap_err();
    assert!(matches!(error, DriverRepairError::InvalidEvidence(_)));
    assert!(error.to_string().contains("Phase 5 OEM published INF identity"));
}
