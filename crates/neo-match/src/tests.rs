use super::*;
use neo_catalogue::{
    PackageManifest, Provenance, RebootRequirement, RedistributionPolicy,
    SecurityRequirements, SignatureEvidence, WindowsApplicability,
};
use neo_device::{OpaqueDeviceId, OrderedDeviceIds};

fn id(value: &str) -> OpaqueDeviceId {
    OpaqueDeviceId::new(value).unwrap()
}

fn device() -> DeviceRecord {
    DeviceRecord {
        instance_id: id(r"USB\VID_1234&PID_5678\ABC"),
        description: Some("Fixture Device".to_string()),
        manufacturer: Some("Fixture Vendor".to_string()),
        class_name: None,
        class_guid: None,
        problem_code: None,
        disabled: Some(false),
        ids: OrderedDeviceIds {
            hardware_ids: vec![
                id(r"USB\VID_1234&PID_5678&REV_0001"),
                id(r"USB\VID_1234&PID_5678"),
            ],
            compatible_ids: vec![id(r"USB\Class_FF")],
        },
        active_driver: None,
        upper_filters: vec![],
        lower_filters: vec![],
    }
}

fn artifact(
    inf_path: &str,
    hardware_id: OpaqueDeviceId,
    compatible_ids: Vec<OpaqueDeviceId>,
    date: &str,
    version: &str,
    signature: SignatureStatus,
) -> DriverArtifact {
    DriverArtifact {
        inf_path: inf_path.to_string(),
        models: vec![InfModelEntry {
            hardware_id: Some(hardware_id),
            compatible_ids,
        }],
        catalog_files: if signature == SignatureStatus::Verified {
            vec!["fixture.cat".to_string()]
        } else {
            vec![]
        },
        provider: Some("Fixture Vendor".to_string()),
        driver_version: Some(version.to_string()),
        driver_date: Some(date.to_string()),
        signature: SignatureEvidence {
            status: signature,
            signer: if signature == SignatureStatus::Verified {
                Some("Fixture Signer".to_string())
            } else {
                None
            },
            verification_note: None,
        },
    }
}

fn package(package_id: &str, artifact: DriverArtifact, architectures: Vec<&str>) -> PackageManifest {
    PackageManifest {
        package_id: package_id.to_string(),
        name: package_id.to_string(),
        vendor: "Fixture Vendor".to_string(),
        version: "1.0".to_string(),
        kind: PackageKind::InfDriverBundle,
        provenance: Provenance {
            source_name: "fixture".to_string(),
            source_url: None,
            sha256: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                .to_string(),
            redistribution: RedistributionPolicy::Unknown,
        },
        windows: WindowsApplicability {
            architectures: architectures.into_iter().map(str::to_string).collect(),
            minimum_build: Some(19041),
            maximum_build: None,
        },
        driver_artifacts: vec![artifact],
        dependencies: vec![],
        conflicts: vec![],
        security: SecurityRequirements::default(),
        reboot: RebootRequirement::None,
    }
}

fn context() -> MatchContext {
    MatchContext {
        architecture: "x64".to_string(),
        windows_build: 26100,
    }
}

#[test]
fn identifier_score_matches_microsoft_classes() {
    assert_eq!(
        identifier_score(IdentifierMatchType::DeviceHardwareToInfHardware, 2, 0),
        Some(0x0002)
    );
    assert_eq!(
        identifier_score(IdentifierMatchType::DeviceHardwareToInfCompatible, 2, 7),
        Some(0x1002)
    );
    assert_eq!(
        identifier_score(IdentifierMatchType::DeviceCompatibleToInfHardware, 3, 0),
        Some(0x2003)
    );
    assert_eq!(
        identifier_score(IdentifierMatchType::DeviceCompatibleToInfCompatible, 3, 2),
        Some(0x3203)
    );
}

#[test]
fn identifier_score_refuses_values_outside_documented_range() {
    assert_eq!(
        identifier_score(
            IdentifierMatchType::DeviceCompatibleToInfCompatible,
            0,
            16,
        ),
        None
    );
    assert_eq!(
        identifier_score(IdentifierMatchType::DeviceHardwareToInfHardware, 0x1000, 0),
        None
    );
}

#[test]
fn more_specific_hardware_id_position_wins() {
    let device = device();
    let first = artifact(
        "first.inf",
        id(r"USB\VID_1234&PID_5678&REV_0001"),
        vec![],
        "2025-01-01",
        "1.0.0.0",
        SignatureStatus::Verified,
    );
    let second = artifact(
        "second.inf",
        id(r"USB\VID_1234&PID_5678"),
        vec![],
        "2026-01-01",
        "9.0.0.0",
        SignatureStatus::Verified,
    );
    let catalogue = Catalogue {
        packages: vec![
            package("neo.first", first, vec!["x64"]),
            package("neo.second", second, vec!["x64"]),
        ],
    };
    let report = match_device(&device, &catalogue, &context()).unwrap();
    assert_eq!(report.candidates[0].package_id, "neo.first");
    assert_eq!(report.candidates[0].identifier.as_ref().unwrap().identifier_score, 0);
    assert_eq!(report.candidates[1].identifier.as_ref().unwrap().identifier_score, 1);
}

#[test]
fn inf_compatible_position_resets_for_each_model_entry() {
    let mut candidate = artifact(
        "multi-model.inf",
        id(r"UNUSED\MODEL_A"),
        (0..10)
            .map(|index| id(&format!(r"USB\Other_{index:02X}")))
            .collect(),
        "2026-01-01",
        "1.0.0.0",
        SignatureStatus::Verified,
    );
    candidate.models.push(InfModelEntry {
        hardware_id: Some(id(r"UNUSED\MODEL_B")),
        compatible_ids: vec![id(r"USB\Class_FF")],
    });
    let catalogue = Catalogue {
        packages: vec![package("neo.multi-model", candidate, vec!["x64"])],
    };
    let report = match_device(&device(), &catalogue, &context()).unwrap();
    let evidence = report.candidates[0].identifier.as_ref().unwrap();
    assert_eq!(
        evidence.match_type,
        IdentifierMatchType::DeviceCompatibleToInfCompatible
    );
    assert_eq!(evidence.model_position, 1);
    assert_eq!(evidence.inf_position, 0);
    assert_eq!(evidence.identifier_score, 0x3000);
}

#[test]
fn newer_generic_does_not_beat_exact_hardware_match() {
    let device = device();
    let exact = artifact(
        "exact.inf",
        id(r"USB\VID_1234&PID_5678&REV_0001"),
        vec![],
        "2025-01-01",
        "1.0.0.0",
        SignatureStatus::Verified,
    );
    let generic = artifact(
        "generic.inf",
        id(r"USB\Class_FF"),
        vec![],
        "2026-12-31",
        "99.0.0.0",
        SignatureStatus::Verified,
    );
    let catalogue = Catalogue {
        packages: vec![
            package("neo.generic", generic, vec!["x64"]),
            package("neo.exact", exact, vec!["x64"]),
        ],
    };
    let report = match_device(&device, &catalogue, &context()).unwrap();
    assert_eq!(report.candidates[0].package_id, "neo.exact");
}

#[test]
fn date_then_version_break_equal_identifier_ties() {
    let device = device();
    let old = artifact(
        "old.inf",
        id(r"USB\VID_1234&PID_5678&REV_0001"),
        vec![],
        "01/01/2025",
        "9.9.9.9",
        SignatureStatus::Verified,
    );
    let new = artifact(
        "new.inf",
        id(r"USB\VID_1234&PID_5678&REV_0001"),
        vec![],
        "2026-01-01",
        "1.0.0.0",
        SignatureStatus::Verified,
    );
    let catalogue = Catalogue {
        packages: vec![
            package("neo.old", old, vec!["x64"]),
            package("neo.new", new, vec!["x64"]),
        ],
    };
    let report = match_device(&device, &catalogue, &context()).unwrap();
    assert_eq!(report.candidates[0].package_id, "neo.new");
}

#[test]
fn architecture_mismatch_rejects_candidate() {
    let candidate = artifact(
        "arm.inf",
        id(r"USB\VID_1234&PID_5678&REV_0001"),
        vec![],
        "2026-01-01",
        "1.0.0.0",
        SignatureStatus::Verified,
    );
    let catalogue = Catalogue {
        packages: vec![package("neo.arm", candidate, vec!["arm64"])],
    };
    let report = match_device(&device(), &catalogue, &context()).unwrap();
    assert_eq!(report.candidates[0].verdict, EvidenceVerdict::Rejected);
    assert!(report.candidates[0]
        .rejection_reasons
        .contains(&RejectionReason::ArchitectureMismatch));
}

#[test]
fn unknown_signature_never_becomes_certified() {
    let candidate = artifact(
        "unknown.inf",
        id(r"USB\VID_1234&PID_5678&REV_0001"),
        vec![],
        "2026-01-01",
        "1.0.0.0",
        SignatureStatus::Unknown,
    );
    let catalogue = Catalogue {
        packages: vec![package("neo.unknown", candidate, vec!["x64"])],
    };
    let report = match_device(&device(), &catalogue, &context()).unwrap();
    assert_eq!(report.candidates[0].verdict, EvidenceVerdict::Investigate);
}

#[test]
fn missing_architecture_metadata_fails_closed() {
    let candidate = artifact(
        "missing-arch.inf",
        id(r"USB\VID_1234&PID_5678&REV_0001"),
        vec![],
        "2026-01-01",
        "1.0.0.0",
        SignatureStatus::Verified,
    );
    let catalogue = Catalogue {
        packages: vec![package("neo.noarch", candidate, vec![])],
    };
    let report = match_device(&device(), &catalogue, &context()).unwrap();
    assert!(report.candidates[0]
        .rejection_reasons
        .contains(&RejectionReason::ArchitectureMetadataMissing));
}

#[test]
fn missing_tie_break_metadata_keeps_best_ambiguous() {
    let device = device();
    let known = artifact(
        "known.inf",
        id(r"USB\VID_1234&PID_5678&REV_0001"),
        vec![],
        "2026-01-01",
        "1.0.0.0",
        SignatureStatus::Verified,
    );
    let unknown = artifact(
        "unknown-date.inf",
        id(r"USB\VID_1234&PID_5678&REV_0001"),
        vec![],
        "not-a-date",
        "1.0.0.0",
        SignatureStatus::Verified,
    );
    let catalogue = Catalogue {
        packages: vec![
            package("neo.known", known, vec!["x64"]),
            package("neo.unknown-date", unknown, vec!["x64"]),
        ],
    };
    let report = match_device(&device, &catalogue, &context()).unwrap();
    assert!(report.best_candidate.is_none());
}

#[test]
fn unknown_date_blocks_version_from_manufacturing_a_winner() {
    let device = device();
    let known = artifact(
        "known.inf",
        id(r"USB\VID_1234&PID_5678&REV_0001"),
        vec![],
        "2026-01-01",
        "1.0.0.0",
        SignatureStatus::Verified,
    );
    let unknown = artifact(
        "unknown.inf",
        id(r"USB\VID_1234&PID_5678&REV_0001"),
        vec![],
        "not-a-date",
        "99.0.0.0",
        SignatureStatus::Verified,
    );
    let catalogue = Catalogue {
        packages: vec![
            package("neo.known", known, vec!["x64"]),
            package("neo.unknown", unknown, vec!["x64"]),
        ],
    };
    let report = match_device(&device, &catalogue, &context()).unwrap();
    assert!(report.best_candidate.is_none());
}

#[test]
fn all_rejected_candidates_do_not_claim_complete_ranking() {
    let candidate = artifact(
        "wrong-arch.inf",
        id(r"USB\VID_1234&PID_5678&REV_0001"),
        vec![],
        "2026-01-01",
        "1.0.0.0",
        SignatureStatus::Verified,
    );
    let catalogue = Catalogue {
        packages: vec![package("neo.wrong-arch", candidate, vec!["arm64"])],
    };
    let report = match_device(&device(), &catalogue, &context()).unwrap();
    assert!(!report.ranking_complete);
    assert!(report.best_candidate.is_none());
}

#[test]
fn version_breaks_tie_when_identifier_and_date_are_equal() {
    let device = device();
    let lower = artifact(
        "lower.inf",
        id(r"USB\VID_1234&PID_5678&REV_0001"),
        vec![],
        "2026-01-01",
        "1.0.0.0",
        SignatureStatus::Verified,
    );
    let higher = artifact(
        "higher.inf",
        id(r"USB\VID_1234&PID_5678&REV_0001"),
        vec![],
        "2026-01-01",
        "2.0.0.0",
        SignatureStatus::Verified,
    );
    let catalogue = Catalogue {
        packages: vec![
            package("neo.lower", lower, vec!["x64"]),
            package("neo.higher", higher, vec!["x64"]),
        ],
    };
    let report = match_device(&device, &catalogue, &context()).unwrap();
    assert_eq!(report.candidates[0].package_id, "neo.higher");
}

#[test]
fn out_of_range_identifier_position_fails_closed() {
    let mut compatible_ids: Vec<OpaqueDeviceId> = (0..16)
        .map(|index| id(&format!(r"USB\Class_{index:02X}")))
        .collect();
    compatible_ids.push(id(r"USB\Class_FF"));
    let candidate = artifact(
        "range.inf",
        id(r"UNUSED\PRIMARY"),
        compatible_ids,
        "2026-01-01",
        "1.0.0.0",
        SignatureStatus::Verified,
    );
    let catalogue = Catalogue {
        packages: vec![package("neo.range", candidate, vec!["x64"])],
    };
    let report = match_device(&device(), &catalogue, &context()).unwrap();
    assert!(report.candidates[0]
        .rejection_reasons
        .contains(&RejectionReason::IdentifierScoreOutOfRange));
    assert!(report.candidates[0].identifier.is_none());
}

#[test]
fn compatible_only_inf_entry_matches_as_inf_compatible() {
    let mut candidate = artifact(
        "compatible-only.inf",
        id(r"UNUSED\PRIMARY"),
        vec![id(r"USB\VID_1234&PID_5678&REV_0001")],
        "2026-01-01",
        "1.0.0.0",
        SignatureStatus::Verified,
    );
    candidate.models[0].hardware_id = None;
    let catalogue = Catalogue {
        packages: vec![package("neo.compatible-only", candidate, vec!["x64"])],
    };
    let report = match_device(&device(), &catalogue, &context()).unwrap();
    let evidence = report.candidates[0].identifier.as_ref().unwrap();
    assert_eq!(
        evidence.match_type,
        IdentifierMatchType::DeviceHardwareToInfCompatible
    );
    assert_eq!(evidence.identifier_score, 0x1000);
}

#[test]
fn invalid_non_leap_february_date_does_not_break_ties() {
    let device = device();
    let first = artifact(
        "first.inf",
        id(r"USB\VID_1234&PID_5678&REV_0001"),
        vec![],
        "2025-02-29",
        "9.0.0.0",
        SignatureStatus::Verified,
    );
    let second = artifact(
        "second.inf",
        id(r"USB\VID_1234&PID_5678&REV_0001"),
        vec![],
        "2025-02-28",
        "1.0.0.0",
        SignatureStatus::Verified,
    );
    let catalogue = Catalogue {
        packages: vec![
            package("neo.first", first, vec!["x64"]),
            package("neo.second", second, vec!["x64"]),
        ],
    };
    let report = match_device(&device, &catalogue, &context()).unwrap();
    assert!(report.best_candidate.is_none());
}
