use neo_device::{DeviceRecord, DriverBinding, OpaqueDeviceId, OrderedDeviceIds};
use neo_driver_repair::{
    assess_driver_repair_evidence, DriverRepairDeviceEvidence, DriverRepairError,
    DriverRepairEvidence, PnpStatusEvidence,
};
use neo_driverstore::StoredDriverPackage;
use std::path::PathBuf;

const PHASE22_FIXTURE: &str = include_str!("../../../fixtures/repair/phase22_driver_evidence.json");

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
    assert!(error
        .to_string()
        .contains("Phase 5 OEM published INF identity"));
}

#[test]
fn imported_oem_package_outside_driver_store_cannot_claim_exact_authority() {
    let evidence = DriverRepairEvidence {
        devices: vec![DriverRepairDeviceEvidence {
            device: DeviceRecord {
                instance_id: OpaqueDeviceId::new("ROOT\\OEM").unwrap(),
                description: Some("OEM driver fixture".to_string()),
                manufacturer: Some("Neo".to_string()),
                class_name: Some("System".to_string()),
                class_guid: None,
                problem_code: None,
                disabled: None,
                ids: OrderedDeviceIds::default(),
                active_driver: Some(DriverBinding {
                    published_name: Some("oem42.inf".to_string()),
                    ..DriverBinding::default()
                }),
                upper_filters: vec![],
                lower_filters: vec![],
            },
            pnp_status: PnpStatusEvidence::NoProblem,
            current_package: Some(StoredDriverPackage {
                published_inf: "oem42.inf".to_string(),
                driver_store_inf: PathBuf::from(r"C:\Temp\neo.inf"),
            }),
        }],
    };

    let error = assess_driver_repair_evidence(evidence).unwrap_err();
    assert!(matches!(error, DriverRepairError::InvalidEvidence(_)));
    assert!(error.to_string().contains("Driver Store"));
}

#[test]
fn imported_oem_package_with_nested_prefix_before_system32_cannot_claim_exact_authority() {
    let evidence = DriverRepairEvidence {
        devices: vec![DriverRepairDeviceEvidence {
            device: DeviceRecord {
                instance_id: OpaqueDeviceId::new("ROOT\\OEM-NESTED").unwrap(),
                description: Some("OEM nested-prefix fixture".to_string()),
                manufacturer: Some("Neo".to_string()),
                class_name: Some("System".to_string()),
                class_guid: None,
                problem_code: None,
                disabled: None,
                ids: OrderedDeviceIds::default(),
                active_driver: Some(DriverBinding {
                    published_name: Some("oem43.inf".to_string()),
                    ..DriverBinding::default()
                }),
                upper_filters: vec![],
                lower_filters: vec![],
            },
            pnp_status: PnpStatusEvidence::NoProblem,
            current_package: Some(StoredDriverPackage {
                published_inf: "oem43.inf".to_string(),
                driver_store_inf: PathBuf::from(
                    r"C:\Temp\Nested\System32\DriverStore\FileRepository\neo.inf_amd64\neo.inf",
                ),
            }),
        }],
    };

    let error = assess_driver_repair_evidence(evidence).unwrap_err();
    assert!(matches!(error, DriverRepairError::InvalidEvidence(_)));
    assert!(error.to_string().contains("Driver Store"));
}

#[test]
fn imported_json_rejects_unknown_fields_at_every_authority_layer() {
    let base: serde_json::Value = serde_json::from_str(PHASE22_FIXTURE).unwrap();
    assert!(DriverRepairEvidence::from_json_str(PHASE22_FIXTURE).is_ok());

    let mut cases = Vec::new();

    let mut root = base.clone();
    root.as_object_mut().unwrap().insert(
        "untrusted_root_claim".to_string(),
        serde_json::Value::Bool(true),
    );
    cases.push(("root", root));

    let mut item = base.clone();
    item["devices"][0].as_object_mut().unwrap().insert(
        "untrusted_item_claim".to_string(),
        serde_json::Value::Bool(true),
    );
    cases.push(("device evidence", item));

    let mut device = base.clone();
    device["devices"][0]["device"]
        .as_object_mut()
        .unwrap()
        .insert(
            "untrusted_device_claim".to_string(),
            serde_json::Value::Bool(true),
        );
    cases.push(("device", device));

    let mut ids = base.clone();
    ids["devices"][0]["device"]["ids"]
        .as_object_mut()
        .unwrap()
        .insert(
            "untrusted_ids_claim".to_string(),
            serde_json::Value::Bool(true),
        );
    cases.push(("ids", ids));

    let mut binding = base.clone();
    binding["devices"][0]["device"]["active_driver"]
        .as_object_mut()
        .unwrap()
        .insert(
            "untrusted_binding_claim".to_string(),
            serde_json::Value::Bool(true),
        );
    cases.push(("active binding", binding));

    let mut status = base.clone();
    status["devices"][0]["pnp_status"]
        .as_object_mut()
        .unwrap()
        .insert(
            "untrusted_status_claim".to_string(),
            serde_json::Value::Bool(true),
        );
    cases.push(("PnP status", status));

    let mut package = base;
    package["devices"][0]["current_package"]
        .as_object_mut()
        .unwrap()
        .insert(
            "untrusted_package_claim".to_string(),
            serde_json::Value::Bool(true),
        );
    cases.push(("current package", package));

    for (layer, evidence) in cases {
        let input = serde_json::to_string(&evidence).unwrap();
        let error = match DriverRepairEvidence::from_json_str(&input) {
            Err(error) => error,
            Ok(_) => panic!("unknown field was accepted at {layer}"),
        };
        assert!(
            matches!(error, DriverRepairError::Serialization(_)),
            "unexpected rejection class at {layer}: {error}"
        );
    }
}

#[test]
fn direct_root_serde_rejects_semantically_contradictory_pnp_evidence() {
    let mut evidence: serde_json::Value = serde_json::from_str(PHASE22_FIXTURE).unwrap();
    evidence["devices"][0]["pnp_status"]["state"] =
        serde_json::Value::String("problem".to_string());
    evidence["devices"][0]["pnp_status"]["code"] = serde_json::Value::from(28u32);

    let input = serde_json::to_string(&evidence).unwrap();
    assert!(
        DriverRepairEvidence::from_json_str(&input).is_err(),
        "validated import helper must reject contradictory PnP evidence"
    );
    assert!(
        serde_json::from_str::<DriverRepairEvidence>(&input).is_err(),
        "direct root Serde must not bypass Phase 22 semantic validation"
    );
}

#[test]
fn imported_original_inf_must_match_driver_store_filename_when_supplied() {
    let mut evidence: serde_json::Value = serde_json::from_str(PHASE22_FIXTURE).unwrap();
    evidence["devices"][0]["device"]["active_driver"]["original_name"] =
        serde_json::Value::String("different-original.inf".to_string());

    let input = serde_json::to_string(&evidence).unwrap();
    assert!(
        DriverRepairEvidence::from_json_str(&input).is_err(),
        "imported original INF identity must agree with the Driver Store INF filename"
    );
}

#[test]
fn imported_active_published_inf_must_be_canonical_without_surrounding_whitespace() {
    let mut evidence: serde_json::Value = serde_json::from_str(PHASE22_FIXTURE).unwrap();
    evidence["devices"][0]["device"]["active_driver"]["published_name"] =
        serde_json::Value::String(" oem40.inf ".to_string());

    let input = serde_json::to_string(&evidence).unwrap();
    assert!(
        DriverRepairEvidence::from_json_str(&input).is_err(),
        "exact imported published INF authority must reject surrounding whitespace"
    );
}

#[test]
fn imported_original_inf_when_supplied_must_be_canonical_and_nonempty() {
    for original_name in [" oem40.inf ", "   "] {
        let mut evidence: serde_json::Value = serde_json::from_str(PHASE22_FIXTURE).unwrap();
        evidence["devices"][0]["device"]["active_driver"]["original_name"] =
            serde_json::Value::String(original_name.to_string());

        let input = serde_json::to_string(&evidence).unwrap();
        assert!(
            DriverRepairEvidence::from_json_str(&input).is_err(),
            "supplied original INF evidence must be canonical and nonempty: {original_name:?}"
        );
    }
}
