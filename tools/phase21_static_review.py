#!/usr/bin/env python3
from __future__ import annotations

from pathlib import Path
import sys
import tomllib

ROOT = Path(__file__).resolve().parents[1]
CRATE = ROOT / "crates" / "neo-repair"
SRC = CRATE / "src"
LIB = (SRC / "lib.rs").read_text(encoding="utf-8")
MODEL = (SRC / "model.rs").read_text(encoding="utf-8")
PARSE = (SRC / "parse.rs").read_text(encoding="utf-8")
HOST = (SRC / "host.rs").read_text(encoding="utf-8")
INSPECTION = (SRC / "inspection.rs").read_text(encoding="utf-8")
PLAN = (SRC / "plan.rs").read_text(encoding="utf-8")
EXECUTOR = (SRC / "executor.rs").read_text(encoding="utf-8")
STORE = (SRC / "session_store.rs").read_text(encoding="utf-8")
RPC = (SRC / "rpc.rs").read_text(encoding="utf-8")
REVIEW_TESTS = (SRC / "review_tests.rs").read_text(encoding="utf-8")
MANIFEST = (CRATE / "Cargo.toml").read_text(encoding="utf-8")
WORKSPACE_TEXT = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
WORKSPACE = tomllib.loads(WORKSPACE_TEXT)
DECISION = (ROOT / "docs" / "decisions" / "0021-PHASE21-REPAIR-WINDOWS-FEATURES.md").read_text(encoding="utf-8")
REVIEW = (ROOT / "docs" / "PHASE21_20_LANE_REVIEW.md").read_text(encoding="utf-8")
CI = (ROOT / ".github" / "workflows" / "ci.yml").read_text(encoding="utf-8")
CLI = (ROOT / "crates" / "neo-cli" / "src" / "main.rs").read_text(encoding="utf-8")
CLI_MANIFEST = (ROOT / "crates" / "neo-cli" / "Cargo.toml").read_text(encoding="utf-8")
PRODUCTION = "\n".join((LIB, MODEL, PARSE, HOST, INSPECTION, PLAN, EXECUTOR, STORE, RPC))


def all_in(text: str, values: tuple[str, ...]) -> bool:
    return all(value in text for value in values)


members = set(WORKSPACE["workspace"]["members"])
fixed_features = (
    '"NetFx3"',
    '"DirectPlay"',
    '"Microsoft-Hyper-V-All"',
    '"Microsoft-Windows-Subsystem-Linux"',
    '"VirtualMachinePlatform"',
    '"Containers-DisposableClientVM"',
)

checks = [
    (
        "master-plan-continuity",
        "Repair & Windows Features" in DECISION
        and "Windows Update service/cache reset" in DECISION
        and "networking reset/repair" in DECISION
        and "Winget repair" in DECISION,
    ),
    (
        "trusted-executable-identity",
        all_in(HOST, ("GetWindowsDirectoryW", 'join("System32")', 'join("dism.exe")', 'join("sfc.exe")'))
        and "cmd.exe" not in HOST
        and "powershell" not in HOST.lower()
        and "std::process::Command" not in HOST,
    ),
    (
        "fixed-command-surface",
        all_in(HOST, ("/CheckHealth", "/RestoreHealth", "/verifyonly", "/scannow", "/Enable-Feature", "/Disable-Feature", "/Get-FeatureInfo"))
        and '"/Remove"' not in HOST
        and '"/All"' not in HOST
        and "raw feature name" in DECISION,
    ),
    (
        "elevation-truth",
        "ELEVATION_EXIT_CODE: i32 = 740" in PARSE
        and "Elevated Windows servicing read authority is required" in DECISION
        and "elevation_failure_never_becomes_a_state_claim" in PARSE,
    ),
    (
        "bounded-command-evidence",
        "MAX_REPAIR_EVIDENCE_BYTES" in MODEL
        and "truncate_utf8" in MODEL
        and "stdout_truncated" in MODEL
        and "stderr_truncated" in MODEL,
    ),
    (
        "component-store-parsing",
        all_in(MODEL, ("ComponentStoreState", "Healthy", "Repairable", "Unrepairable", "Unavailable"))
        and "dism_health_states_are_fail_closed" in PARSE,
    ),
    (
        "system-file-parsing",
        all_in(MODEL, ("SystemFileState", "IntegrityViolations", "Unavailable"))
        and "sfc_verifyonly_states_are_distinct" in PARSE,
    ),
    (
        "feature-catalogue-identity",
        all(value in MODEL for value in fixed_features)
        and "fixed_feature_catalogue_is_unique" in MODEL
        and "feature_identity_is_closed_over_the_frozen_catalogue" in REVIEW_TESTS,
    ),
    (
        "feature-state-parsing",
        all_in(MODEL, ("EnablePending", "DisablePending", "Removed", "Unavailable"))
        and "feature_states_require_explicit_state_line" in PARSE
        and "pending_states_are_not_stable_transaction_baselines" in REVIEW_TESTS,
    ),
    (
        "read-only-inspection",
        "machine_changes: false" in INSPECTION
        and "inspection_reads_every_fixed_surface_without_execution" in INSPECTION
        and "host.execute" not in INSPECTION,
    ),
    (
        "repair-transaction-binding",
        "ActionKind::Repair" in PLAN
        and PLAN.count("RollbackPlan::Irreversible") >= 2
        and "irreversible_repair_completes_only_after_fresh_healthy_observation" in EXECUTOR,
    ),
    (
        "feature-transaction-binding",
        "ActionKind::WindowsFeature" in PLAN
        and "VerificationExpectation::MatchesBaseline" in PLAN
        and '"/Remove"' not in EXECUTOR
        and "FeatureNotReversible" in PLAN,
    ),
    (
        "reboot-write-ahead-resume",
        all_in(EXECUTOR, ("begin_apply_with_host", "execute_applying_with_host", "recover_applying_with_host", "AwaitingReboot", "AwaitingRollbackReboot"))
        and "TransactionStage::Applying" in STORE
        and "write-ahead" in DECISION.lower()
        and "NeoData/sessions" in DECISION,
    ),
    (
        "freshness-drift",
        "assert_fresh_baseline" in EXECUTOR
        and "BaselineDrift" in EXECUTOR
        and "fresh_baseline_drift_blocks_before_mutation" in EXECUTOR,
    ),
    (
        "mcp-rpc-trust-boundary",
        RPC.count("#[serde(deny_unknown_fields)]") >= 3
        and "self.policy.authorize(context" in RPC
        and "authorization_happens_before_machine_evidence_lookup" in RPC
        and "raw_requests_reject_trusted_context_injection" in RPC,
    ),
    (
        "confirmation-continuity",
        all_in(RPC, ("ConfirmationRequired", "IrreversibleAcknowledgementRequired", "PlanMismatch", "ApprovalMismatch"))
        and "approved_action_ids != vec![pending.action_id.clone()]" in RPC
        and "plan_fingerprint" in RPC,
    ),
    (
        "replay-session-safety",
        all_in(STORE, ("MAX_SESSION_VERSIONS", "record_fingerprint", "event history is not append-only", "open_dir_nofollow"))
        and all_in(RPC, ("expected_version", "VersionMismatch", "SessionNotResumable"))
        and "pending_feature_resume_is_version_bound_and_single_use" in RPC,
    ),
    (
        "cli-core-separation",
        "neo-repair" in CLI_MANIFEST
        and "RepairExecutorCapability" not in CLI
        and "RepairRpcService" not in CLI
        and "/RestoreHealth" not in CLI
        and "/scannow" not in CLI
        and "/Enable-Feature" not in CLI
        and "/Disable-Feature" not in CLI,
    ),
    (
        "regression-ci-continuity",
        "crates/neo-repair" in members
        and "Phase 21 twenty-lane static review" in CI
        and "python -W error tools/phase21_static_review.py" in CI
        and "Phase 21 Repair & Windows Features proof" in CI
        and "cargo test --locked -p neo-repair" in CI
        and "Phase 20 twenty-lane static review" in CI,
    ),
    (
        "adversarial-three-person-acceptance",
        all_in(RPC, (
            "malformed_confirmation_does_not_consume_prepared_session",
            "caller_safe_error_payload_does_not_leak_internal_session_path",
        ))
        and "interrupted_irreversible_repair_at_old_baseline_fails_closed_without_rerun" in EXECUTOR
        and "three-person" in REVIEW.lower()
        and "live destructive" in DECISION.lower(),
    ),
]

failed: list[str] = []
for index, (name, passed) in enumerate(checks, 1):
    print(f"{index:02d}. {'PASS' if passed else 'FAIL'} - {name}")
    if not passed:
        failed.append(name)

if failed:
    raise SystemExit("Phase 21 static review failed: " + ", ".join(failed))

print("PHASE 21 STATIC REVIEW PASS: 20/20")
sys.exit(0)
