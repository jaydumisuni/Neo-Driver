use super::*;
use neo_core::{
    ActionKind, EvidenceItem, EvidenceVerdict, PlannedAction, RebootRequirement,
    RecommendationState, RiskLevel,
};

fn target() -> StateTarget {
    StateTarget {
        kind: StateTargetKind::RegistryValue,
        key: r"HKCU\Software\NeoFixture\Enabled".to_string(),
    }
}

fn postcondition() -> VerificationPredicate {
    VerificationPredicate {
        id: "fixture.enabled".to_string(),
        target: target(),
        expectation: VerificationExpectation::Equals("1".to_string()),
        required: true,
    }
}

fn rollback_predicate() -> VerificationPredicate {
    VerificationPredicate {
        id: "fixture.restore".to_string(),
        target: target(),
        expectation: VerificationExpectation::MatchesBaseline,
        required: true,
    }
}

fn planned_action() -> PlannedAction {
    PlannedAction {
        id: "neo.fixture.tweak".to_string(),
        title: "Fixture tweak".to_string(),
        kind: ActionKind::Tweak,
        risk: RiskLevel::Normal,
        recommendation: RecommendationState::Recommended,
        verdict: EvidenceVerdict::Certified,
        rationale: "prove transaction safety".to_string(),
        selected_by_default: false,
        requires_confirmation: true,
        requires_admin: false,
        reboot: RebootRequirement::None,
        rollback_available: true,
        evidence: vec![EvidenceItem::new("fixture", "true", "unit-test")],
        warnings: vec![],
    }
}

fn transaction_action() -> TransactionAction {
    TransactionAction {
        action: planned_action(),
        snapshot_targets: vec![target()],
        postconditions: vec![postcondition()],
        rollback: RollbackPlan::Reversible {
            restore_targets: vec![target()],
            verification: vec![rollback_predicate()],
        },
    }
}

fn plan() -> TransactionPlan {
    TransactionPlan::new(
        "NEO-TX-TEST",
        1,
        "NEO-MISSION-TEST",
        vec![transaction_action()],
    )
    .unwrap()
}

fn baseline(value: CapturedValue) -> Vec<CapturedState> {
    vec![CapturedState {
        target: target(),
        value,
    }]
}

fn authorization(plan: &TransactionPlan) -> TransactionAuthorization {
    TransactionAuthorization {
        plan_fingerprint: plan.fingerprint().unwrap(),
        approved_action_ids: vec!["neo.fixture.tweak".to_string()],
        manual_override_action_ids: vec![],
        high_risk_ack_action_ids: vec![],
        irreversible_acknowledgements: vec![],
    }
}

fn authorized_checkpoint() -> TransactionCheckpoint {
    let plan = plan();
    let auth = authorization(&plan);
    let mut checkpoint = TransactionCheckpoint::new(plan).unwrap();
    checkpoint
        .capture_baseline(baseline(CapturedValue::Present("0".to_string())))
        .unwrap();
    checkpoint.authorize(auth).unwrap();
    checkpoint
}

#[test]
fn fingerprint_is_stable_and_plan_bound() {
    let first = plan();
    let same = plan();
    assert_eq!(first.fingerprint().unwrap(), same.fingerprint().unwrap());
    let mut changed_action = transaction_action();
    changed_action.postconditions[0].expectation = VerificationExpectation::Equals("2".to_string());
    let changed =
        TransactionPlan::new("NEO-TX-TEST", 1, "NEO-MISSION-TEST", vec![changed_action]).unwrap();
    assert_ne!(first.fingerprint().unwrap(), changed.fingerprint().unwrap());
}

#[test]
fn rejected_action_cannot_enter_transaction() {
    let mut action = transaction_action();
    action.action.verdict = EvidenceVerdict::Rejected;
    assert!(matches!(
        TransactionPlan::new("TX", 1, "MISSION", vec![action]),
        Err(TransactionError::RejectedAction(_))
    ));
}

#[test]
fn reversible_action_requires_exact_baseline() {
    let mut checkpoint = TransactionCheckpoint::new(plan()).unwrap();
    assert!(matches!(
        checkpoint.capture_baseline(vec![]),
        Err(TransactionError::BaselineCoverageMismatch)
    ));
}

#[test]
fn unavailable_rollback_baseline_fails_closed() {
    let mut checkpoint = TransactionCheckpoint::new(plan()).unwrap();
    assert!(matches!(
        checkpoint.capture_baseline(baseline(CapturedValue::Unavailable(
            "registry read failed".to_string()
        ))),
        Err(TransactionError::RollbackBaselineUnavailable { .. })
    ));
}

#[test]
fn authorization_is_bound_to_exact_plan_fingerprint() {
    let plan = plan();
    let mut checkpoint = TransactionCheckpoint::new(plan.clone()).unwrap();
    checkpoint
        .capture_baseline(baseline(CapturedValue::Present("0".to_string())))
        .unwrap();
    let mut auth = authorization(&plan);
    auth.plan_fingerprint = "00".repeat(32);
    assert!(matches!(
        checkpoint.authorize(auth),
        Err(TransactionError::AuthorizationFingerprintMismatch)
    ));
}

#[test]
fn authorization_requires_exact_action_coverage() {
    let plan = plan();
    let mut checkpoint = TransactionCheckpoint::new(plan.clone()).unwrap();
    checkpoint
        .capture_baseline(baseline(CapturedValue::Present("0".to_string())))
        .unwrap();
    let mut auth = authorization(&plan);
    auth.approved_action_ids.clear();
    assert!(matches!(
        checkpoint.authorize(auth),
        Err(TransactionError::AuthorizationCoverageMismatch)
    ));
}

#[test]
fn provisional_action_requires_manual_override() {
    let mut action = transaction_action();
    action.action.verdict = EvidenceVerdict::Provisional;
    let plan = TransactionPlan::new("TX", 1, "MISSION", vec![action]).unwrap();
    let mut checkpoint = TransactionCheckpoint::new(plan.clone()).unwrap();
    checkpoint
        .capture_baseline(baseline(CapturedValue::Present("0".to_string())))
        .unwrap();
    assert!(matches!(
        checkpoint.authorize(authorization(&plan)),
        Err(TransactionError::MissingManualOverride)
    ));
}

#[test]
fn high_risk_action_requires_separate_acknowledgement() {
    let mut action = transaction_action();
    action.action.risk = RiskLevel::High;
    let plan = TransactionPlan::new("TX", 1, "MISSION", vec![action]).unwrap();
    let mut checkpoint = TransactionCheckpoint::new(plan.clone()).unwrap();
    checkpoint
        .capture_baseline(baseline(CapturedValue::Present("0".to_string())))
        .unwrap();
    assert!(matches!(
        checkpoint.authorize(authorization(&plan)),
        Err(TransactionError::MissingHighRiskAcknowledgement)
    ));
}

#[test]
fn apply_success_never_completes_transaction() {
    let mut checkpoint = authorized_checkpoint();
    checkpoint.begin_apply().unwrap();
    checkpoint
        .record_apply_result(ApplyRecord {
            action_id: "neo.fixture.tweak".to_string(),
            outcome: ApplyOutcome::Success,
            detail: "future executor reported success".to_string(),
        })
        .unwrap();
    assert_eq!(checkpoint.stage(), TransactionStage::Verifying);
}

#[test]
fn verification_proof_is_required_for_completion() {
    let mut checkpoint = authorized_checkpoint();
    checkpoint.begin_apply().unwrap();
    checkpoint
        .record_apply_result(ApplyRecord {
            action_id: "neo.fixture.tweak".to_string(),
            outcome: ApplyOutcome::Success,
            detail: "future executor reported success".to_string(),
        })
        .unwrap();
    checkpoint
        .verify_postconditions(vec![Observation {
            target: target(),
            value: ObservedValue::Present("1".to_string()),
        }])
        .unwrap();
    assert_eq!(checkpoint.stage(), TransactionStage::Complete);
}

#[test]
fn failed_postcondition_routes_reversible_change_to_rollback() {
    let mut checkpoint = authorized_checkpoint();
    checkpoint.begin_apply().unwrap();
    checkpoint
        .record_apply_result(ApplyRecord {
            action_id: "neo.fixture.tweak".to_string(),
            outcome: ApplyOutcome::Success,
            detail: "future executor reported success".to_string(),
        })
        .unwrap();
    checkpoint
        .verify_postconditions(vec![Observation {
            target: target(),
            value: ObservedValue::Present("0".to_string()),
        }])
        .unwrap();
    assert_eq!(checkpoint.stage(), TransactionStage::RollingBack);
}

#[test]
fn rollback_requires_restoration_proof_before_rolled_back() {
    let mut checkpoint = authorized_checkpoint();
    checkpoint.begin_apply().unwrap();
    checkpoint
        .record_apply_result(ApplyRecord {
            action_id: "neo.fixture.tweak".to_string(),
            outcome: ApplyOutcome::Success,
            detail: "future executor reported success".to_string(),
        })
        .unwrap();
    checkpoint
        .verify_postconditions(vec![Observation {
            target: target(),
            value: ObservedValue::Present("broken".to_string()),
        }])
        .unwrap();
    checkpoint
        .record_rollback_result(RollbackRecord {
            action_id: "neo.fixture.tweak".to_string(),
            outcome: ApplyOutcome::Success,
            detail: "future executor restored captured value".to_string(),
        })
        .unwrap();
    assert_eq!(checkpoint.stage(), TransactionStage::RollingBack);
    checkpoint
        .verify_rollback(vec![Observation {
            target: target(),
            value: ObservedValue::Present("0".to_string()),
        }])
        .unwrap();
    assert_eq!(checkpoint.stage(), TransactionStage::RolledBack);
}

#[test]
fn required_reboot_must_be_proven_before_continuation() {
    let mut action = transaction_action();
    action.action.reboot = RebootRequirement::Required;
    let plan = TransactionPlan::new("TX", 1, "MISSION", vec![action]).unwrap();
    let auth = authorization(&plan);
    let mut checkpoint = TransactionCheckpoint::new(plan).unwrap();
    checkpoint
        .capture_baseline(baseline(CapturedValue::Present("0".to_string())))
        .unwrap();
    checkpoint.authorize(auth).unwrap();
    checkpoint.begin_apply().unwrap();
    checkpoint
        .record_apply_result(ApplyRecord {
            action_id: "neo.fixture.tweak".to_string(),
            outcome: ApplyOutcome::Success,
            detail: "future executor reported success".to_string(),
        })
        .unwrap();
    assert_eq!(checkpoint.stage(), TransactionStage::AwaitingReboot);
    checkpoint
        .resume_after_reboot(vec![Observation {
            target: target(),
            value: ObservedValue::Present("1".to_string()),
        }])
        .unwrap();
    assert_eq!(checkpoint.stage(), TransactionStage::Verifying);
}

#[test]
fn failed_post_reboot_probe_blocks_continuation() {
    let mut action = transaction_action();
    action.action.reboot = RebootRequirement::Required;
    let plan = TransactionPlan::new("TX", 1, "MISSION", vec![action]).unwrap();
    let auth = authorization(&plan);
    let mut checkpoint = TransactionCheckpoint::new(plan).unwrap();
    checkpoint
        .capture_baseline(baseline(CapturedValue::Present("0".to_string())))
        .unwrap();
    checkpoint.authorize(auth).unwrap();
    checkpoint.begin_apply().unwrap();
    checkpoint
        .record_apply_result(ApplyRecord {
            action_id: "neo.fixture.tweak".to_string(),
            outcome: ApplyOutcome::Success,
            detail: "future executor reported success".to_string(),
        })
        .unwrap();
    checkpoint
        .resume_after_reboot(vec![Observation {
            target: target(),
            value: ObservedValue::Present("0".to_string()),
        }])
        .unwrap();
    assert_eq!(checkpoint.stage(), TransactionStage::Blocked);
}

#[test]
fn verification_status_is_recomputed_from_observed_evidence() {
    let result = VerificationResult {
        predicate: rollback_predicate(),
        observed: ObservedValue::Present("0".to_string()),
    };
    let baseline =
        BaselineSnapshot::for_plan(&plan(), baseline(CapturedValue::Present("0".to_string())))
            .unwrap();
    assert_eq!(result.status(&baseline), VerificationStatus::Pass);
    let json = serde_json::to_string(&result).unwrap();
    assert!(!json.contains("pass"));
}

#[test]
fn persisted_checkpoint_rejects_fingerprint_tampering() {
    let checkpoint = TransactionCheckpoint::new(plan()).unwrap();
    let mut value = serde_json::to_value(checkpoint).unwrap();
    value["plan_fingerprint"] = serde_json::Value::String("00".repeat(32));
    let encoded = serde_json::to_string(&value).unwrap();
    assert!(matches!(
        TransactionCheckpoint::from_json_str(&encoded),
        Err(TransactionError::CheckpointFingerprintMismatch)
    ));
}

#[test]
fn illegal_stage_transition_is_rejected() {
    let mut checkpoint = TransactionCheckpoint::new(plan()).unwrap();
    assert!(matches!(
        checkpoint.begin_apply(),
        Err(TransactionError::InvalidStageTransition { .. })
    ));
}

#[test]
fn overlapping_snapshot_targets_across_actions_fail_closed() {
    let first = transaction_action();
    let mut second = transaction_action();
    second.action.id = "neo.fixture.tweak.second".to_string();
    second.snapshot_targets[0].key = r"hkcu\software\neofixture\enabled".to_string();
    second.postconditions[0].id = "fixture.enabled.second".to_string();
    second.rollback = RollbackPlan::Reversible {
        restore_targets: vec![target()],
        verification: vec![VerificationPredicate {
            id: "fixture.restore.second".to_string(),
            target: target(),
            expectation: VerificationExpectation::MatchesBaseline,
            required: true,
        }],
    };
    assert!(matches!(
        TransactionPlan::new("TX", 1, "MISSION", vec![first, second]),
        Err(TransactionError::OverlappingSnapshotTarget(_))
    ));
}

#[test]
fn irreversible_action_requires_reason_bound_acknowledgement() {
    let mut action = transaction_action();
    action.action.rollback_available = false;
    action.snapshot_targets.clear();
    action.rollback = RollbackPlan::Irreversible {
        reason: "fixture cannot be reversed".to_string(),
    };
    let plan = TransactionPlan::new("TX", 1, "MISSION", vec![action]).unwrap();
    let mut checkpoint = TransactionCheckpoint::new(plan.clone()).unwrap();
    checkpoint.capture_baseline(vec![]).unwrap();
    assert!(matches!(
        checkpoint.authorize(TransactionAuthorization {
            plan_fingerprint: plan.fingerprint().unwrap(),
            approved_action_ids: vec!["neo.fixture.tweak".to_string()],
            manual_override_action_ids: vec![],
            high_risk_ack_action_ids: vec![],
            irreversible_acknowledgements: vec![],
        }),
        Err(TransactionError::MissingIrreversibleAcknowledgement)
    ));
}

#[test]
fn blocked_reprobe_can_recover_to_verifying() {
    let mut action = transaction_action();
    action.action.reboot = RebootRequirement::Required;
    let plan = TransactionPlan::new("TX", 1, "MISSION", vec![action]).unwrap();
    let auth = authorization(&plan);
    let mut checkpoint = TransactionCheckpoint::new(plan).unwrap();
    checkpoint
        .capture_baseline(baseline(CapturedValue::Present("0".to_string())))
        .unwrap();
    checkpoint.authorize(auth).unwrap();
    checkpoint.begin_apply().unwrap();
    checkpoint
        .record_apply_result(ApplyRecord {
            action_id: "neo.fixture.tweak".to_string(),
            outcome: ApplyOutcome::Success,
            detail: "future executor reported success".to_string(),
        })
        .unwrap();
    checkpoint
        .resume_after_reboot(vec![Observation {
            target: target(),
            value: ObservedValue::Present("0".to_string()),
        }])
        .unwrap();
    assert_eq!(checkpoint.stage(), TransactionStage::Blocked);
    checkpoint
        .reprobe_after_block(vec![Observation {
            target: target(),
            value: ObservedValue::Present("1".to_string()),
        }])
        .unwrap();
    assert_eq!(checkpoint.stage(), TransactionStage::Verifying);
}

#[test]
fn blocked_reprobe_routes_reversible_change_to_rollback() {
    let mut action = transaction_action();
    action.action.reboot = RebootRequirement::Required;
    let plan = TransactionPlan::new("TX", 1, "MISSION", vec![action]).unwrap();
    let auth = authorization(&plan);
    let mut checkpoint = TransactionCheckpoint::new(plan).unwrap();
    checkpoint
        .capture_baseline(baseline(CapturedValue::Present("0".to_string())))
        .unwrap();
    checkpoint.authorize(auth).unwrap();
    checkpoint.begin_apply().unwrap();
    checkpoint
        .record_apply_result(ApplyRecord {
            action_id: "neo.fixture.tweak".to_string(),
            outcome: ApplyOutcome::Success,
            detail: "future executor reported success".to_string(),
        })
        .unwrap();
    checkpoint
        .resume_after_reboot(vec![Observation {
            target: target(),
            value: ObservedValue::Present("0".to_string()),
        }])
        .unwrap();
    checkpoint
        .reprobe_after_block(vec![Observation {
            target: target(),
            value: ObservedValue::Present("0".to_string()),
        }])
        .unwrap();
    assert_eq!(checkpoint.stage(), TransactionStage::RollingBack);
}

#[test]
fn persisted_reboot_checkpoint_tampering_is_rejected_after_resume() {
    let mut action = transaction_action();
    action.action.reboot = RebootRequirement::Required;
    let plan = TransactionPlan::new("TX", 1, "MISSION", vec![action]).unwrap();
    let auth = authorization(&plan);
    let mut checkpoint = TransactionCheckpoint::new(plan).unwrap();
    checkpoint
        .capture_baseline(baseline(CapturedValue::Present("0".to_string())))
        .unwrap();
    checkpoint.authorize(auth).unwrap();
    checkpoint.begin_apply().unwrap();
    checkpoint
        .record_apply_result(ApplyRecord {
            action_id: "neo.fixture.tweak".to_string(),
            outcome: ApplyOutcome::Success,
            detail: "future executor reported success".to_string(),
        })
        .unwrap();
    let mut value = serde_json::to_value(checkpoint).unwrap();
    value["reboot_checkpoint"]["expected_post_reboot"][0]["id"] =
        serde_json::Value::String("tampered".to_string());
    value["stage"] = serde_json::Value::String("blocked".to_string());
    value["resume_results"] = serde_json::json!([{
        "predicate": value["reboot_checkpoint"]["expected_post_reboot"][0].clone(),
        "observed": {"state":"present","value":"0"}
    }]);
    value["events"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({
            "sequence": 7,
            "stage":"blocked",
            "message":"tampered resume"
        }));
    let encoded = serde_json::to_string(&value).unwrap();
    assert!(matches!(
        TransactionCheckpoint::from_json_str(&encoded),
        Err(TransactionError::RebootCheckpointMismatch)
    ));
}

#[test]
fn persisted_event_history_cannot_skip_authority_stage() {
    let mut checkpoint = authorized_checkpoint();
    checkpoint.begin_apply().unwrap();
    let mut value = serde_json::to_value(checkpoint).unwrap();
    value["events"][2]["stage"] = serde_json::Value::String("applying".to_string());
    let encoded = serde_json::to_string(&value).unwrap();
    assert!(matches!(
        TransactionCheckpoint::from_json_str(&encoded),
        Err(TransactionError::InvalidEventLog)
    ));
}
