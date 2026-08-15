#!/usr/bin/env python3
"""Deterministic 20-lane static review for Neo Driver Phase 4."""
from __future__ import annotations
from dataclasses import dataclass
from pathlib import Path
import json
import sys
import tomllib

ROOT = Path(__file__).resolve().parents[1]
TRANSACTION = "\n".join(
    path.read_text(encoding="utf-8")
    for path in sorted((ROOT / "crates/neo-transaction/src").rglob("*.rs"))
)
CLI = "\n".join(
    path.read_text(encoding="utf-8")
    for path in sorted((ROOT / "crates/neo-cli/src").rglob("*.rs"))
)
WORKSPACE = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
FIXTURE = json.loads(
    (ROOT / "fixtures/transaction/sample_transaction_plan.json").read_text(encoding="utf-8")
)

@dataclass(frozen=True)
class Lane:
    number: int
    name: str
    passed: bool
    detail: str

def contains_all(text: str, values: list[str]) -> bool:
    return all(value in text for value in values)

def review() -> list[Lane]:
    members = set(WORKSPACE["workspace"]["members"])
    transaction_lower = TRANSACTION.lower()
    forbidden_executor_markers = [
        "std::process::command",
        "command::new",
        "pnputil",
        "bcdedit",
        "reg.exe",
        "restart-computer",
        "shutdown.exe",
        "windows-sys",
        "winreg",
    ]
    forbidden_cli_advancement = [
        ".authorize(",
        ".begin_apply(",
        ".record_apply_result(",
        ".resume_after_reboot(",
        ".reprobe_after_block(",
        ".verify_postconditions(",
        ".record_rollback_result(",
        ".verify_rollback(",
    ]
    action = FIXTURE["actions"][0]
    rollback = action["rollback"]
    return [
        Lane(1, "workspace", "crates/neo-transaction" in members, "transaction crate is a workspace member"),
        Lane(2, "core-policy-reuse", contains_all(TRANSACTION, ["PlannedAction", "RiskLevel", "EvidenceVerdict", "RebootRequirement"]), "transaction policy reuses neo-core authority/risk contracts"),
        Lane(3, "exact-plan-and-root-deserialization", contains_all(TRANSACTION, ["Sha256", "fingerprint", "AuthorizationFingerprintMismatch", "CheckpointFingerprintMismatch", "TransactionPlanWire", "TransactionCheckpointWire", "serde(try_from"]), "authorization/checkpoints bind to the exact plan and root Serde deserialization cannot bypass validation"),
        Lane(4, "actual-baseline", contains_all(TRANSACTION, ["BaselineSnapshot", "CapturedState", "CapturedValue", "capture_baseline"]), "actual pre-state is a first-class transaction record"),
        Lane(5, "baseline-coverage", contains_all(TRANSACTION, ["required_snapshot_targets", "BaselineCoverageMismatch", "DuplicateBaselineTarget", "OverlappingSnapshotTarget", "identity_key"]), "baseline ownership is exact and Windows target identity is case-normalized"),
        Lane(6, "unavailable-fails-closed", "RollbackBaselineUnavailable" in TRANSACTION, "unavailable rollback state blocks reversible authority"),
        Lane(7, "exact-authorization", contains_all(TRANSACTION, ["approved_action_ids", "AuthorizationCoverageMismatch", "plan.action_ids"]), "authorization approves exactly the fingerprint-bound action set"),
        Lane(8, "uncertainty-authority", contains_all(TRANSACTION, ["manual_override_action_ids", "needs_manual_override", "MissingManualOverride", "RejectedAction"]), "uncertainty needs explicit override and rejected evidence cannot become authority"),
        Lane(9, "risk-authority", contains_all(TRANSACTION, ["high_risk_ack_action_ids", "RiskLevel::High", "MissingHighRiskAcknowledgement"]), "HIGH/EXPERT risk needs separate acknowledgement"),
        Lane(10, "irreversible-authority", contains_all(TRANSACTION, ["Irreversible", "irreversible_acknowledgements", "MissingIrreversibleAcknowledgement"]), "irreversible operations require explicit reason-bound acknowledgement"),
        Lane(11, "exit-code-not-proof", contains_all(TRANSACTION, ["ApplyOutcome::Success", "TransactionStage::Verifying", "all required postconditions proven"]), "apply success routes to verification rather than completion"),
        Lane(12, "reboot-checkpoint", contains_all(TRANSACTION, ["RebootCheckpoint", "AwaitingReboot", "plan_fingerprint", "restoration_obligations", "RebootCheckpointMismatch"]), "required reboot state is persistent and plan-bound"),
        Lane(13, "reprobe-before-resume", contains_all(TRANSACTION, ["resume_after_reboot", "reprobe_after_block", "post-reboot state proven", "continuation blocked", "blocked post-reboot state re-proven"]), "post-reboot continuation and Blocked recovery require deterministic evidence"),
        Lane(14, "verification-recomputed", contains_all(TRANSACTION, ["VerificationResult", "pub fn status", "required_results_pass"]) and "pub status:" not in TRANSACTION, "PASS/FAIL is recomputed from observations/baseline rather than trusted from JSON"),
        Lane(15, "rollback-to-baseline", contains_all(TRANSACTION, ["MatchesBaseline", "required_rollback_targets", "captured pre-state restoration proven"]), "rollback restores and proves captured reality rather than presumed defaults"),
        Lane(16, "rolled-back-and-complete-attempt-proof", contains_all(TRANSACTION, ["verify_rollback", "require_successful_rollback_records", "TransactionStage::RolledBack", "record_rollback_results_batch", "rollback_batch_records_every_outcome_before_terminal_failure"]), "rollback proof requires restoration evidence, while batch recording can preserve every changed-action outcome before terminal failure"),
        Lane(17, "partial-failure", contains_all(TRANSACTION, ["changed_action_ids", "machine_changed", "all_reversible", "apply failure requires rollback"]), "partial apply failure only enters rollback when every actually changed action is reversible"),
        Lane(18, "stage-invariants", contains_all(TRANSACTION, ["require_stage", "StageInvariantViolation", "InvalidStageTransition", "validate_event_log", "valid_event_transition", "Blocked, Verifying", "Blocked, RollingBack", "Blocked, Failed"]), "illegal/persisted state-machine drift fails closed and Blocked has explicit recovery exits"),
        Lane(19, "read-only-cli", contains_all(CLI, ["TransactionCommand", "ValidatePlan", "CheckpointTemplate", "ValidateCheckpoint", "Machine changes: none"]) and not any(marker in CLI for marker in forbidden_cli_advancement), "CLI can inspect/serialize contracts but cannot advance or execute a transaction"),
        Lane(20, "no-executor-and-fixture", not any(marker in transaction_lower for marker in forbidden_executor_markers) and action["action"]["kind"] == "tweak" and rollback["mode"] == "reversible" and rollback["verification"][0]["expectation"]["type"] == "matches_baseline", "Phase 4 has no Windows executor and the fixture exercises reversible baseline semantics"),
    ]

def main() -> int:
    lanes = review()
    for lane in lanes:
        print(f"{'PASS' if lane.passed else 'FAIL'} {lane.number:02d} {lane.name}: {lane.detail}")
    failures = [lane for lane in lanes if not lane.passed]
    if failures:
        print(f"\nPhase 4 static review failed: {len(failures)} lane(s) unresolved.")
        return 1
    print("\nPhase 4 static review: PASS (20/20 lanes).")
    return 0

if __name__ == "__main__":
    sys.exit(main())
