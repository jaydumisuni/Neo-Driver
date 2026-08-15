use crate::*;
use neo_core::{
    ActionKind, EvidenceItem, EvidenceVerdict, PlannedAction, RebootRequirement,
    RecommendationState, RiskLevel,
};

fn state_target(name: &str) -> StateTarget {
    StateTarget {
        kind: StateTargetKind::RegistryValue,
        key: format!(r"HKCU\Software\NeoFixture\{name}"),
    }
}

fn transaction_action(id: &str, name: &str) -> TransactionAction {
    let target = state_target(name);
    TransactionAction {
        action: PlannedAction {
            id: id.to_string(),
            title: format!("Fixture {name}"),
            kind: ActionKind::Tweak,
            risk: RiskLevel::Normal,
            recommendation: RecommendationState::Recommended,
            verdict: EvidenceVerdict::Certified,
            rationale: "prove complete rollback evidence".to_string(),
            selected_by_default: false,
            requires_confirmation: true,
            requires_admin: false,
            reboot: RebootRequirement::None,
            rollback_available: true,
            evidence: vec![EvidenceItem::new("fixture", name, "unit-test")],
            warnings: vec![],
        },
        snapshot_targets: vec![target.clone()],
        postconditions: vec![VerificationPredicate {
            id: format!("{id}.desired"),
            target: target.clone(),
            expectation: VerificationExpectation::Equals("1".to_string()),
            required: true,
        }],
        rollback: RollbackPlan::Reversible {
            restore_targets: vec![target.clone()],
            verification: vec![VerificationPredicate {
                id: format!("{id}.restore"),
                target,
                expectation: VerificationExpectation::MatchesBaseline,
                required: true,
            }],
        },
    }
}

fn rolling_back_checkpoint() -> TransactionCheckpoint {
    let first_id = "neo.fixture.first";
    let second_id = "neo.fixture.second";
    let plan = TransactionPlan::new(
        "NEO-TX-BATCH-ROLLBACK",
        1,
        "NEO-MISSION-BATCH-ROLLBACK",
        vec![
            transaction_action(first_id, "First"),
            transaction_action(second_id, "Second"),
        ],
    )
    .unwrap();
    let authorization = TransactionAuthorization {
        plan_fingerprint: plan.fingerprint().unwrap(),
        approved_action_ids: vec![first_id.to_string(), second_id.to_string()],
        manual_override_action_ids: vec![],
        high_risk_ack_action_ids: vec![],
        irreversible_acknowledgements: vec![],
    };
    let mut checkpoint = TransactionCheckpoint::new(plan).unwrap();
    checkpoint
        .capture_baseline(vec![
            CapturedState {
                target: state_target("First"),
                value: CapturedValue::Present("0".to_string()),
            },
            CapturedState {
                target: state_target("Second"),
                value: CapturedValue::Present("0".to_string()),
            },
        ])
        .unwrap();
    checkpoint.authorize(authorization).unwrap();
    checkpoint.begin_apply().unwrap();
    for id in [first_id, second_id] {
        checkpoint
            .record_apply_result(ApplyRecord {
                action_id: id.to_string(),
                outcome: ApplyOutcome::Success,
                detail: "fixture write succeeded".to_string(),
                machine_changed: true,
                reboot_required: false,
            })
            .unwrap();
    }
    checkpoint
        .verify_postconditions(vec![
            Observation {
                target: state_target("First"),
                value: ObservedValue::Present("broken".to_string()),
            },
            Observation {
                target: state_target("Second"),
                value: ObservedValue::Present("broken".to_string()),
            },
        ])
        .unwrap();
    assert_eq!(checkpoint.stage(), TransactionStage::RollingBack);
    checkpoint
}

#[test]
fn rollback_batch_requires_complete_changed_action_coverage() {
    let mut checkpoint = rolling_back_checkpoint();
    assert!(matches!(
        checkpoint.record_rollback_results_batch(vec![RollbackRecord {
            action_id: "neo.fixture.first".to_string(),
            outcome: ApplyOutcome::Success,
            detail: "first restored".to_string(),
            reboot_required: false,
        }]),
        Err(TransactionError::IncompleteRollbackProof)
    ));
    assert_eq!(checkpoint.stage(), TransactionStage::RollingBack);
}

#[test]
fn rollback_batch_records_every_outcome_before_terminal_failure() {
    let mut checkpoint = rolling_back_checkpoint();
    checkpoint
        .record_rollback_results_batch(vec![
            RollbackRecord {
                action_id: "neo.fixture.first".to_string(),
                outcome: ApplyOutcome::Failure,
                detail: "first restore failed".to_string(),
                reboot_required: false,
            },
            RollbackRecord {
                action_id: "neo.fixture.second".to_string(),
                outcome: ApplyOutcome::Success,
                detail: "second restore succeeded".to_string(),
                reboot_required: false,
            },
        ])
        .unwrap();
    assert_eq!(checkpoint.stage(), TransactionStage::Failed);
    let encoded = serde_json::to_value(&checkpoint).unwrap();
    assert_eq!(encoded["rollback_records"].as_array().unwrap().len(), 2);
}
