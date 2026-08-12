#!/usr/bin/env python3
"""One-shot validated-deserialization correction for Phase 4 root transaction types."""
from pathlib import Path


def replace_once(path: Path, old: str, new: str) -> None:
    text = path.read_text(encoding="utf-8")
    if text.count(old) != 1:
        raise SystemExit(f"expected exactly one anchor in {path}: {old[:100]!r}")
    path.write_text(text.replace(old, new, 1), encoding="utf-8")


def main() -> None:
    plan = Path("crates/neo-transaction/src/plan.rs")
    replace_once(
        plan,
        '''#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransactionPlan {
    transaction_id: String,
    revision: u32,
    mission_id: String,
    actions: Vec<TransactionAction>,
}

impl TransactionPlan {
''',
        '''#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "TransactionPlanWire")]
pub struct TransactionPlan {
    transaction_id: String,
    revision: u32,
    mission_id: String,
    actions: Vec<TransactionAction>,
}

#[derive(Debug, Deserialize)]
struct TransactionPlanWire {
    transaction_id: String,
    revision: u32,
    mission_id: String,
    actions: Vec<TransactionAction>,
}

impl TryFrom<TransactionPlanWire> for TransactionPlan {
    type Error = TransactionError;

    fn try_from(value: TransactionPlanWire) -> Result<Self, Self::Error> {
        Self::new(
            value.transaction_id,
            value.revision,
            value.mission_id,
            value.actions,
        )
    }
}

impl TransactionPlan {
''',
    )

    checkpoint = Path("crates/neo-transaction/src/checkpoint.rs")
    replace_once(
        checkpoint,
        '''#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransactionCheckpoint {
    pub(crate) plan: TransactionPlan,
    pub(crate) plan_fingerprint: String,
    pub(crate) stage: TransactionStage,
    pub(crate) baseline: Option<BaselineSnapshot>,
    pub(crate) authorization: Option<TransactionAuthorization>,
    #[serde(default)]
    pub(crate) apply_records: Vec<ApplyRecord>,
    #[serde(default)]
    pub(crate) resume_results: Vec<VerificationResult>,
    #[serde(default)]
    pub(crate) verification_results: Vec<VerificationResult>,
    #[serde(default)]
    pub(crate) rollback_records: Vec<RollbackRecord>,
    #[serde(default)]
    pub(crate) rollback_results: Vec<VerificationResult>,
    pub(crate) reboot_checkpoint: Option<RebootCheckpoint>,
    #[serde(default)]
    pub(crate) events: Vec<TransactionEvent>,
}

impl TransactionCheckpoint {
''',
        '''#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "TransactionCheckpointWire")]
pub struct TransactionCheckpoint {
    pub(crate) plan: TransactionPlan,
    pub(crate) plan_fingerprint: String,
    pub(crate) stage: TransactionStage,
    pub(crate) baseline: Option<BaselineSnapshot>,
    pub(crate) authorization: Option<TransactionAuthorization>,
    #[serde(default)]
    pub(crate) apply_records: Vec<ApplyRecord>,
    #[serde(default)]
    pub(crate) resume_results: Vec<VerificationResult>,
    #[serde(default)]
    pub(crate) verification_results: Vec<VerificationResult>,
    #[serde(default)]
    pub(crate) rollback_records: Vec<RollbackRecord>,
    #[serde(default)]
    pub(crate) rollback_results: Vec<VerificationResult>,
    pub(crate) reboot_checkpoint: Option<RebootCheckpoint>,
    #[serde(default)]
    pub(crate) events: Vec<TransactionEvent>,
}

#[derive(Debug, Deserialize)]
struct TransactionCheckpointWire {
    plan: TransactionPlan,
    plan_fingerprint: String,
    stage: TransactionStage,
    baseline: Option<BaselineSnapshot>,
    authorization: Option<TransactionAuthorization>,
    #[serde(default)]
    apply_records: Vec<ApplyRecord>,
    #[serde(default)]
    resume_results: Vec<VerificationResult>,
    #[serde(default)]
    verification_results: Vec<VerificationResult>,
    #[serde(default)]
    rollback_records: Vec<RollbackRecord>,
    #[serde(default)]
    rollback_results: Vec<VerificationResult>,
    reboot_checkpoint: Option<RebootCheckpoint>,
    #[serde(default)]
    events: Vec<TransactionEvent>,
}

impl TryFrom<TransactionCheckpointWire> for TransactionCheckpoint {
    type Error = TransactionError;

    fn try_from(value: TransactionCheckpointWire) -> Result<Self, Self::Error> {
        let checkpoint = Self {
            plan: value.plan,
            plan_fingerprint: value.plan_fingerprint,
            stage: value.stage,
            baseline: value.baseline,
            authorization: value.authorization,
            apply_records: value.apply_records,
            resume_results: value.resume_results,
            verification_results: value.verification_results,
            rollback_records: value.rollback_records,
            rollback_results: value.rollback_results,
            reboot_checkpoint: value.reboot_checkpoint,
            events: value.events,
        };
        checkpoint.validate()?;
        Ok(checkpoint)
    }
}

impl TransactionCheckpoint {
''',
    )

    tests = Path("crates/neo-transaction/src/tests.rs")
    text = tests.read_text(encoding="utf-8")
    marker = "\n#[test]\nfn rejected_action_cannot_enter_transaction() {"
    added = '''
#[test]
fn direct_serde_plan_deserialization_cannot_bypass_validation() {
    let mut value = serde_json::to_value(plan()).unwrap();
    value["revision"] = serde_json::json!(0);
    assert!(serde_json::from_value::<TransactionPlan>(value).is_err());
}

#[test]
fn direct_serde_checkpoint_deserialization_cannot_bypass_invariants() {
    let checkpoint = TransactionCheckpoint::new(plan()).unwrap();
    let mut value = serde_json::to_value(checkpoint).unwrap();
    value["plan_fingerprint"] = serde_json::Value::String("00".repeat(32));
    assert!(serde_json::from_value::<TransactionCheckpoint>(value).is_err());
}
'''
    if text.count(marker) != 1:
        raise SystemExit("tests insertion anchor mismatch")
    tests.write_text(text.replace(marker, added + marker, 1), encoding="utf-8")

    review = Path("tools/phase4_static_review.py")
    replace_once(
        review,
        'Lane(3, "exact-plan-fingerprint", contains_all(TRANSACTION, ["Sha256", "fingerprint", "AuthorizationFingerprintMismatch", "CheckpointFingerprintMismatch"]), "authorization/checkpoints bind to the exact serialized plan fingerprint"),',
        'Lane(3, "exact-plan-and-root-deserialization", contains_all(TRANSACTION, ["Sha256", "fingerprint", "AuthorizationFingerprintMismatch", "CheckpointFingerprintMismatch", "TransactionPlanWire", "TransactionCheckpointWire", "serde(try_from"]), "authorization/checkpoints bind to the exact plan and root Serde deserialization cannot bypass validation"),',
    )


if __name__ == "__main__":
    main()
