use crate::{
    prepare_restore_from_inventory, receipt_from_completed_checkpoint_for_tests,
    DebloatHistoryError, DebloatRemovalReceipt, DEBLOAT_REMOVAL_RECEIPT_SCHEMA_VERSION,
};
use neo_core::{
    ActionKind, EvidenceItem, EvidenceVerdict, PlannedAction, RebootRequirement,
    RecommendationState, RiskLevel,
};
use neo_debloat_plan::{ExactAppxInventory, ExactPackageDependency, ExactPackageIdentity};
use neo_transaction::{
    ApplyOutcome, ApplyRecord, CapturedState, CapturedValue, Observation, ObservedValue,
    RollbackPlan, StateTarget, StateTargetKind, TransactionAction, TransactionAuthorization,
    TransactionCheckpoint, TransactionPlan, TransactionStage, VerificationExpectation,
    VerificationPredicate,
};

const DEBLOAT_ID: &str = "appx.contoso.phase17";
const PACKAGE_ID: &str = "Contoso.Phase17";
const MAIN_FULL: &str = "Contoso.Phase17_1.2.3.4_x64__contoso";
const MAIN_FAMILY: &str = "Contoso.Phase17_contoso";
const DEP_NAME: &str = "Contoso.Framework";
const DEP_FULL: &str = "Contoso.Framework_1.0.0.0_x64__contoso";
const DEP_FAMILY: &str = "Contoso.Framework_contoso";

#[test]
fn completed_removal_becomes_versioned_fingerprinted_durable_history() {
    let receipt = receipt();
    assert_eq!(
        receipt.schema_version(),
        DEBLOAT_REMOVAL_RECEIPT_SCHEMA_VERSION
    );
    assert!(!receipt.receipt_fingerprint().is_empty());
    assert_eq!(
        receipt.source_checkpoint().stage(),
        TransactionStage::Complete
    );

    let json = receipt.to_json_pretty().expect("receipt must serialize");
    let restored = DebloatRemovalReceipt::from_json_str(&json).expect("receipt must round-trip");
    assert_eq!(restored, receipt);
}

#[test]
fn receipt_fingerprint_rejects_history_tampering() {
    let receipt = receipt();
    let mut value = serde_json::to_value(&receipt).expect("receipt must serialize");
    value["package_id"] = serde_json::Value::String("Contoso.Tampered".to_string());
    let json = serde_json::to_string(&value).expect("tampered receipt must encode");

    let error = DebloatRemovalReceipt::from_json_str(&json)
        .expect_err("receipt fingerprint must reject changed history fields");
    assert!(matches!(error, DebloatHistoryError::InvalidReceipt(_)));
}

#[test]
fn receipt_rejects_broadened_source_authority_even_with_valid_json_shape() {
    let receipt = receipt();
    let mut value = serde_json::to_value(&receipt).expect("receipt must serialize");
    value["source_checkpoint"]["plan"]["actions"][0]["action"]["risk"] =
        serde_json::Value::String("high".to_string());
    let json = serde_json::to_string(&value).expect("tampered receipt must encode");

    let error = DebloatRemovalReceipt::from_json_str(&json)
        .expect_err("durable history must reject broadened source authority");
    assert!(matches!(
        error,
        DebloatHistoryError::InvalidReceipt(_) | DebloatHistoryError::Serialization(_)
    ));
}

#[test]
fn non_complete_source_checkpoint_cannot_become_history_receipt() {
    let checkpoint = source_checkpoint_at_baseline();
    let error = receipt_from_completed_checkpoint_for_tests(
        DEBLOAT_ID,
        PACKAGE_ID,
        main_identity(),
        vec![dependency()],
        checkpoint,
    )
    .expect_err("non-complete removal cannot be history authority");
    assert!(matches!(error, DebloatHistoryError::IncompleteRemoval(_)));
}

#[test]
fn prepares_fresh_inverse_transaction_when_exact_local_restore_is_still_ready() {
    let receipt = receipt();
    let inventory = restore_inventory(false);
    let prepared = prepare_restore_from_inventory(&receipt, &inventory, "mission-phase17-restore")
        .expect("exact local staged restore should be ready");

    assert!(!prepared.machine_changes());
    assert_eq!(
        prepared.checkpoint().stage(),
        TransactionStage::BaselineCaptured
    );
    assert_eq!(prepared.transaction().actions().len(), 1);
    let action = &prepared.transaction().actions()[0];
    assert_eq!(action.action.kind, ActionKind::Debloat);
    assert_eq!(action.action.recommendation, RecommendationState::Repair);
    assert!(!action.action.selected_by_default);
    assert!(action.action.requires_confirmation);
    assert_eq!(action.snapshot_targets.len(), 2);
    assert_eq!(action.postconditions.len(), 2);
    assert_eq!(
        prepared.receipt_fingerprint(),
        receipt.receipt_fingerprint()
    );
    let baseline = prepared
        .checkpoint()
        .baseline()
        .expect("restore baseline must be captured");
    assert_eq!(
        baseline.get(&appx_target(MAIN_FULL)),
        Some(&CapturedValue::Absent)
    );
    assert_eq!(
        baseline.get(&appx_target(DEP_FULL)),
        Some(&CapturedValue::Absent)
    );
}

#[test]
fn existing_exact_dependency_is_preserved_as_restore_time_baseline() {
    let receipt = receipt();
    let inventory = restore_inventory(true);
    let prepared = prepare_restore_from_inventory(&receipt, &inventory, "mission-phase17-dep")
        .expect("existing exact dependency should be compatible");
    let expected = CapturedValue::Present(
        serde_json::to_string(&dependency()).expect("dependency must serialize"),
    );
    assert_eq!(
        prepared
            .checkpoint()
            .baseline()
            .expect("baseline")
            .get(&appx_target(DEP_FULL)),
        Some(&expected)
    );
}

#[test]
fn already_restored_main_is_not_prepared_again() {
    let receipt = receipt();
    let mut inventory = restore_inventory(false);
    inventory.current_user.push(main_identity());
    inventory.validate().expect("inventory must remain valid");

    let error = prepare_restore_from_inventory(&receipt, &inventory, "mission-already")
        .expect_err("already restored package must not create another restore plan");
    assert!(matches!(error, DebloatHistoryError::AlreadyRestored));
}

#[test]
fn different_current_main_version_blocks_old_history_restore() {
    let receipt = receipt();
    let mut inventory = restore_inventory(false);
    let mut newer = main_identity();
    newer.full_name = "Contoso.Phase17_9.9.9.9_x64__contoso".to_string();
    inventory.current_user.push(newer);
    inventory.validate().expect("inventory must remain valid");

    let error = prepare_restore_from_inventory(&receipt, &inventory, "mission-conflict")
        .expect_err("newer current package must block deterministic old-version restore");
    assert!(matches!(error, DebloatHistoryError::InventoryConflict(_)));
}

#[test]
fn missing_exact_staged_main_blocks_restore_readiness() {
    let receipt = receipt();
    let mut inventory = restore_inventory(false);
    inventory
        .provisioned
        .retain(|package| !package.full_name.eq_ignore_ascii_case(MAIN_FULL));

    let error = prepare_restore_from_inventory(&receipt, &inventory, "mission-missing-main")
        .expect_err("missing staged main identity must block restore");
    assert!(matches!(error, DebloatHistoryError::RestoreNotReady(_)));
}

#[test]
fn staged_main_kind_flag_drift_blocks_restore_readiness() {
    let receipt = receipt();
    let mut inventory = restore_inventory(false);
    inventory.provisioned[0].is_optional = true;
    inventory.validate().expect("inventory must remain valid");

    let error = prepare_restore_from_inventory(&receipt, &inventory, "mission-kind-drift")
        .expect_err("staged package-kind drift must block deterministic restore");
    assert!(matches!(error, DebloatHistoryError::RestoreNotReady(_)));
}

#[test]
fn missing_exact_staged_dependency_blocks_restore_readiness() {
    let receipt = receipt();
    let mut inventory = restore_inventory(false);
    inventory
        .provisioned
        .retain(|package| !package.full_name.eq_ignore_ascii_case(DEP_FULL));

    let error = prepare_restore_from_inventory(&receipt, &inventory, "mission-missing-dep")
        .expect_err("missing staged dependency must block restore");
    assert!(matches!(error, DebloatHistoryError::RestoreNotReady(_)));
}

#[test]
fn different_current_dependency_version_blocks_restore_readiness() {
    let receipt = receipt();
    let mut inventory = restore_inventory(false);
    let mut newer = dependency_identity();
    newer.full_name = "Contoso.Framework_2.0.0.0_x64__contoso".to_string();
    inventory.current_user.push(newer);
    inventory.validate().expect("inventory must remain valid");

    let error = prepare_restore_from_inventory(&receipt, &inventory, "mission-dep-conflict")
        .expect_err("dependency version conflict must fail closed");
    assert!(matches!(error, DebloatHistoryError::InventoryConflict(_)));
}

#[test]
fn restore_readiness_is_byte_for_byte_non_mutating() {
    let receipt = receipt();
    let inventory = restore_inventory(false);
    let before = serde_json::to_vec(&inventory).expect("inventory must serialize");

    let prepared = prepare_restore_from_inventory(&receipt, &inventory, "mission-read-only")
        .expect("restore readiness should prepare");

    let after = serde_json::to_vec(&inventory).expect("inventory must serialize");
    assert_eq!(before, after, "Phase 17 must not mutate inventory evidence");
    assert!(!prepared.machine_changes());
}

fn receipt() -> DebloatRemovalReceipt {
    receipt_from_completed_checkpoint_for_tests(
        DEBLOAT_ID,
        PACKAGE_ID,
        main_identity(),
        vec![dependency()],
        completed_source_checkpoint(),
    )
    .expect("completed Phase 16 history must produce a receipt")
}

fn completed_source_checkpoint() -> TransactionCheckpoint {
    let mut checkpoint = source_checkpoint_at_baseline();
    let fingerprint = checkpoint
        .plan()
        .fingerprint()
        .expect("fingerprint must compute");
    checkpoint
        .authorize(TransactionAuthorization {
            plan_fingerprint: fingerprint,
            approved_action_ids: vec![DEBLOAT_ID.to_string()],
            manual_override_action_ids: Vec::new(),
            high_risk_ack_action_ids: Vec::new(),
            irreversible_acknowledgements: Vec::new(),
        })
        .expect("source authorization must succeed");
    checkpoint.begin_apply().expect("apply must begin");
    checkpoint
        .record_apply_result(ApplyRecord {
            action_id: DEBLOAT_ID.to_string(),
            outcome: ApplyOutcome::Success,
            detail: "synthetic exact removal".to_string(),
            machine_changed: true,
            reboot_required: false,
        })
        .expect("apply record must succeed");
    checkpoint
        .verify_postconditions(vec![Observation {
            target: appx_target(MAIN_FULL),
            value: ObservedValue::Absent,
        }])
        .expect("source removal must verify");
    assert_eq!(checkpoint.stage(), TransactionStage::Complete);
    checkpoint
}

fn source_checkpoint_at_baseline() -> TransactionCheckpoint {
    let main = main_identity();
    let dependency = dependency();
    let main_target = appx_target(MAIN_FULL);
    let dependency_target = appx_target(DEP_FULL);
    let action = TransactionAction {
        action: PlannedAction {
            id: DEBLOAT_ID.to_string(),
            title: "Remove Contoso Phase17 for current user".to_string(),
            kind: ActionKind::Debloat,
            risk: RiskLevel::Low,
            recommendation: RecommendationState::OptionalComponent,
            verdict: EvidenceVerdict::Certified,
            rationale: "Synthetic completed Phase 16 removal for Phase 17 history proof."
                .to_string(),
            selected_by_default: false,
            requires_confirmation: true,
            requires_admin: false,
            reboot: RebootRequirement::None,
            rollback_available: true,
            evidence: vec![EvidenceItem::new("fixture", "true", "phase17-test")],
            warnings: Vec::new(),
        },
        snapshot_targets: vec![main_target.clone(), dependency_target.clone()],
        postconditions: vec![VerificationPredicate {
            id: format!("verify:{DEBLOAT_ID}:main-absent"),
            target: main_target.clone(),
            expectation: VerificationExpectation::Absent,
            required: true,
        }],
        rollback: RollbackPlan::Reversible {
            restore_targets: vec![main_target.clone(), dependency_target.clone()],
            verification: vec![
                VerificationPredicate {
                    id: format!("rollback:{DEBLOAT_ID}:main"),
                    target: main_target.clone(),
                    expectation: VerificationExpectation::MatchesBaseline,
                    required: true,
                },
                VerificationPredicate {
                    id: format!("rollback:{DEBLOAT_ID}:dependency:0"),
                    target: dependency_target.clone(),
                    expectation: VerificationExpectation::MatchesBaseline,
                    required: true,
                },
            ],
        },
    };
    let plan = TransactionPlan::new(
        "mission-phase17-source:phase15-debloat-current-user",
        1,
        "mission-phase17-source",
        vec![action],
    )
    .expect("source plan must validate");
    let mut checkpoint = TransactionCheckpoint::new(plan).expect("checkpoint must create");
    checkpoint
        .capture_baseline(vec![
            CapturedState {
                target: main_target,
                value: CapturedValue::Present(
                    serde_json::to_string(&main).expect("main must serialize"),
                ),
            },
            CapturedState {
                target: dependency_target,
                value: CapturedValue::Present(
                    serde_json::to_string(&dependency).expect("dependency must serialize"),
                ),
            },
        ])
        .expect("source baseline must capture");
    checkpoint
}

fn restore_inventory(dependency_current: bool) -> ExactAppxInventory {
    let mut current_user = Vec::new();
    if dependency_current {
        current_user.push(dependency_identity());
    }
    ExactAppxInventory::new(
        current_user,
        vec![main_identity(), dependency_identity()],
        "phase17-test-native",
    )
    .expect("restore inventory must validate")
}

fn main_identity() -> ExactPackageIdentity {
    ExactPackageIdentity {
        name: PACKAGE_ID.to_string(),
        full_name: MAIN_FULL.to_string(),
        family_name: MAIN_FAMILY.to_string(),
        is_framework: false,
        is_resource: false,
        is_bundle: false,
        is_optional: false,
        dependencies: vec![dependency()],
    }
}

fn dependency() -> ExactPackageDependency {
    ExactPackageDependency {
        name: DEP_NAME.to_string(),
        full_name: DEP_FULL.to_string(),
        family_name: DEP_FAMILY.to_string(),
    }
}

fn dependency_identity() -> ExactPackageIdentity {
    ExactPackageIdentity {
        name: DEP_NAME.to_string(),
        full_name: DEP_FULL.to_string(),
        family_name: DEP_FAMILY.to_string(),
        is_framework: true,
        is_resource: false,
        is_bundle: false,
        is_optional: false,
        dependencies: Vec::new(),
    }
}

fn appx_target(full_name: &str) -> StateTarget {
    StateTarget {
        kind: StateTargetKind::AppxPackage,
        key: format!("current_user:{full_name}"),
    }
}
