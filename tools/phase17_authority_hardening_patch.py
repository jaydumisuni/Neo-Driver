from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    target = Path(path)
    text = target.read_text(encoding="utf-8")
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"expected one marker in {path}, found {count}")
    target.write_text(text.replace(old, new, 1), encoding="utf-8")


replace_once(
    "crates/neo-debloat-history/src/model.rs",
    '''        if source_plan.actions().len() != 1 {
            return Err(DebloatHistoryError::InvalidReceipt(
                "source checkpoint does not contain exactly one completed Debloat action"
                    .to_string(),
            ));
        }
''',
    '''        if source_plan.revision() != 1
            || !source_plan
                .transaction_id()
                .ends_with(":phase15-debloat-current-user")
        {
            return Err(DebloatHistoryError::InvalidReceipt(
                "source checkpoint is not the frozen Phase 15 current-user Debloat transaction shape"
                    .to_string(),
            ));
        }
        if source_plan.actions().len() != 1 {
            return Err(DebloatHistoryError::InvalidReceipt(
                "source checkpoint does not contain exactly one completed Debloat action"
                    .to_string(),
            ));
        }
''',
)

replace_once(
    "crates/neo-debloat-history/src/model.rs",
    '''        if source_action.id != self.debloat_id
            || source_action.kind != neo_core::ActionKind::Debloat
            || source_action.verdict != neo_core::EvidenceVerdict::Certified
            || !source_action.requires_confirmation
            || !source_action.rollback_available
            || source_action.selected_by_default
        {
''',
    '''        if source_action.id != self.debloat_id
            || source_action.kind != neo_core::ActionKind::Debloat
            || source_action.risk != neo_core::RiskLevel::Low
            || !matches!(
                source_action.recommendation,
                neo_core::RecommendationState::Recommended
                    | neo_core::RecommendationState::OptionalComponent
            )
            || source_action.verdict != neo_core::EvidenceVerdict::Certified
            || source_action.selected_by_default
            || !source_action.requires_confirmation
            || source_action.requires_admin
            || source_action.reboot != neo_core::RebootRequirement::None
            || !source_action.rollback_available
        {
''',
)

replace_once(
    "crates/neo-debloat-history/src/tests.rs",
    '''#[test]
fn non_complete_source_checkpoint_cannot_become_history_receipt() {
''',
    '''#[test]
fn receipt_rejects_broadened_source_authority_even_with_valid_json_shape() {
    let receipt = receipt();
    let mut value = serde_json::to_value(&receipt).expect("receipt must serialize");
    value["source_checkpoint"]["plan"]["actions"][0]["action"]["risk"] =
        serde_json::Value::String("high".to_string());
    let json = serde_json::to_string(&value).expect("tampered receipt must encode");

    let error = DebloatRemovalReceipt::from_json_str(&json)
        .expect_err("durable history must reject broadened source authority");
    assert!(matches!(error, DebloatHistoryError::InvalidReceipt(_)));
    assert!(error.to_string().contains("authority expected from Phase 15/16"));
}

#[test]
fn non_complete_source_checkpoint_cannot_become_history_receipt() {
''',
)

replace_once(
    "docs/decisions/0017-PHASE17-DEBLOAT-HISTORY-RESTORE-READINESS.md",
    '''A receipt may be created only from a Phase 16 `DebloatExecutionSession` whose checkpoint is exactly `Complete` and whose execution-plan/checkpoint transaction fingerprints still agree. Durable validation also requires that the completed source contains exactly one `Debloat` action and that action remains `Certified`, explicit-confirmation-required, reversible, and not selected by default—the authority shape frozen by the Phase 15→16 path.
''',
    '''A receipt may be created only from a Phase 16 `DebloatExecutionSession` whose checkpoint is exactly `Complete` and whose execution-plan/checkpoint transaction fingerprints still agree. Durable validation also requires the source transaction to remain revision `1` with the frozen `:phase15-debloat-current-user` identity, and exactly one `Debloat` action whose authority still matches the Phase 13→15 candidate law: `LOW` risk, `Recommended` or `OptionalComponent`, `Certified`, explicit-confirmation-required, non-admin, no-reboot, reversible, and not selected by default.
''',
)

replace_once(
    "docs/PHASE17_20_LANE_REVIEW.md",
    '''5. Receipt schema is explicitly versioned and durable JSON deserialization revalidates the complete source checkpoint plus the certified, confirmation-required, reversible, non-default source Debloat action shape.
''',
    '''5. Receipt schema is explicitly versioned and durable JSON deserialization revalidates the complete revision-1 Phase 15 source checkpoint plus the LOW-risk, allowed-recommendation, certified, confirmation-required, non-admin, no-reboot, reversible, non-default Debloat action shape.
''',
)

replace_once(
    "tools/phase17_static_review.py",
    '''        and "source_checkpoint: TransactionCheckpoint" in model
        and "source_action.verdict != neo_core::EvidenceVerdict::Certified" in model
        and "!source_action.requires_confirmation" in model
        and "!source_action.rollback_available" in model
        and "source_action.selected_by_default" in model,
''',
    '''        and "source_checkpoint: TransactionCheckpoint" in model
        and "source_plan.revision() != 1" in model
        and '.ends_with(":phase15-debloat-current-user")' in model
        and "source_action.risk != neo_core::RiskLevel::Low" in model
        and "neo_core::RecommendationState::Recommended" in model
        and "neo_core::RecommendationState::OptionalComponent" in model
        and "source_action.verdict != neo_core::EvidenceVerdict::Certified" in model
        and "source_action.selected_by_default" in model
        and "!source_action.requires_confirmation" in model
        and "source_action.requires_admin" in model
        and "source_action.reboot != neo_core::RebootRequirement::None" in model
        and "!source_action.rollback_available" in model,
''',
)

replace_once(
    "tools/phase17_static_review.py",
    '''            "receipt_fingerprint_rejects_history_tampering",
            "non_complete_source_checkpoint_cannot_become_history_receipt",
''',
    '''            "receipt_fingerprint_rejects_history_tampering",
            "receipt_rejects_broadened_source_authority_even_with_valid_json_shape",
            "non_complete_source_checkpoint_cannot_become_history_receipt",
''',
)
