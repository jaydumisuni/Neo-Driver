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
WORKSPACE = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
DECISION = (
    ROOT / "docs" / "decisions" / "0021-PHASE21-REPAIR-WINDOWS-FEATURES.md"
).read_text(encoding="utf-8")
REVIEW = (ROOT / "docs" / "PHASE21_20_LANE_REVIEW.md").read_text(encoding="utf-8")
CI = (ROOT / ".github" / "workflows" / "ci.yml").read_text(encoding="utf-8")
CLI_MAIN = (ROOT / "crates" / "neo-cli" / "src" / "main.rs").read_text(encoding="utf-8")
REPAIR_CLI = (ROOT / "crates" / "neo-cli" / "src" / "repair_cli.rs").read_text(
    encoding="utf-8"
)
CLI_MANIFEST = (ROOT / "crates" / "neo-cli" / "Cargo.toml").read_text(encoding="utf-8")


def has_all(text: str, values: tuple[str, ...]) -> bool:
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
        and all(
            value in DECISION
            for value in (
                "Windows Update service/cache reset",
                "networking reset/repair",
                "Winget repair",
                "AppX repair",
                "PnP repair",
            )
        )
        and "crates/neo-repair" in members,
    ),
    (
        "platform-and-trusted-executable-boundary",
        LIB.count("#[cfg(any(windows, test))]") >= 8
        and "[target.'cfg(windows)'.dependencies]" in MANIFEST
        and "windows.workspace = true" in MANIFEST
        and has_all(HOST, ("GetWindowsDirectoryW", 'join("System32")', 'join("dism.exe")', 'join("sfc.exe")'))
        and "cmd.exe" not in HOST
        and "powershell" not in HOST.lower(),
    ),
    (
        "fixed-command-surface",
        has_all(
            HOST,
            (
                '"/CheckHealth"',
                '"/RestoreHealth"',
                '"/verifyonly"',
                '"/scannow"',
                '"/Get-FeatureInfo"',
                '"/Enable-Feature"',
                '"/Disable-Feature"',
            ),
        )
        and '"/Remove"' not in HOST
        and "std::process::Command" not in HOST,
    ),
    (
        "elevation-truth",
        "ELEVATION_EXIT_CODE: i32 = 740" in PARSE
        and "elevation_failure_never_becomes_a_state_claim" in PARSE
        and "nul_separated_sfc_admin_failure_is_elevation_required" in PARSE,
    ),
    (
        "bounded-command-evidence",
        has_all(
            MODEL,
            (
                "MAX_REPAIR_EVIDENCE_BYTES",
                "truncate_utf8",
                "stdout_truncated",
                "stderr_truncated",
                "start_error",
                "exit_code",
            ),
        )
        and "command_evidence_is_bounded_at_utf8_boundary" in MODEL,
    ),
    (
        "component-store-parsing",
        has_all(MODEL, ("ComponentStoreState", "Healthy", "Repairable", "Unrepairable", "Unavailable"))
        and "dism_health_states_are_fail_closed" in PARSE,
    ),
    (
        "system-file-parsing",
        has_all(MODEL, ("SystemFileState", "IntegrityViolations", "Unavailable"))
        and "sfc_verifyonly_states_are_distinct" in PARSE,
    ),
    (
        "closed-feature-catalogue",
        all(value in MODEL for value in fixed_features)
        and "fixed_feature_catalogue_is_unique" in MODEL
        and "feature_identity_is_closed_over_the_frozen_catalogue" in REVIEW_TESTS
        and has_all(MODEL, ("EnablePending", "DisablePending", "Removed", "Unavailable"))
        and "feature_states_require_explicit_state_line" in PARSE,
    ),
    (
        "read-only-probe-separation",
        has_all(
            INSPECTION,
            (
                "inspect_repair_health_with_host",
                "inspect_features_with_host",
                "health_inspection_does_not_probe_optional_features",
                "feature_inspection_does_not_probe_component_store_or_sfc",
            ),
        )
        and "host.execute" not in INSPECTION
        and "inspect_windows_repair_health" in REPAIR_CLI
        and "inspect_windows_features" in REPAIR_CLI,
    ),
    (
        "repair-transaction-truth",
        "ActionKind::Repair" in PLAN
        and PLAN.count("RollbackPlan::Irreversible") >= 2
        and "irreversible_repair_completes_only_after_fresh_healthy_observation" in EXECUTOR
        and "interrupted_irreversible_repair_at_old_baseline_fails_closed_without_rerun" in EXECUTOR,
    ),
    (
        "feature-rollback-truth",
        "ActionKind::WindowsFeature" in PLAN
        and "VerificationExpectation::MatchesBaseline" in PLAN
        and "FeatureNotReversible" in PLAN
        and '"/Remove"' not in EXECUTOR
        and "pending_states_are_not_stable_transaction_baselines" in REVIEW_TESTS,
    ),
    (
        "freshness-drift-proof",
        "assert_fresh_baseline" in EXECUTOR
        and "BaselineDrift" in EXECUTOR
        and "fresh_baseline_drift_blocks_before_mutation" in EXECUTOR
        and "verify_current_with_observation" in EXECUTOR,
    ),
    (
        "servicing-reboot-semantics",
        "matches!(self.exit_code, Some(0) | Some(3010))" in MODEL
        and "servicing_reboot_exit_is_successful" in MODEL
        and "execution.exit_code == Some(3010)" in EXECUTOR
        and "servicing_3010_success_requires_reboot_even_when_feature_state_is_stable" in EXECUTOR,
    ),
    (
        "write-ahead-mutation-state",
        has_all(
            EXECUTOR,
            (
                "begin_apply_with_host",
                "execute_applying_with_host",
                "recover_applying_with_host",
                "TransactionStage::Applying",
            ),
        )
        and "TransactionStage::Applying" in STORE
        and "write-ahead" in DECISION.lower(),
    ),
    (
        "durable-session-ownership",
        has_all(
            STORE,
            (
                "NeoData/sessions",
                "open_dir_nofollow",
                "MAX_SESSION_VERSIONS",
                "#[serde(deny_unknown_fields)]",
                "newly_created",
            ),
        )
        and "persisted_owner_rejects_unknown_fields" in STORE
        and "existing_session_directory_is_never_reported_as_newly_created" in STORE
        and "#[serde(deny_unknown_fields)]" in PLAN
        and "#[serde(deny_unknown_fields)]" in EXECUTOR,
    ),
    (
        "rpc-trust-confirmation-continuity",
        RPC.count("#[serde(deny_unknown_fields)]") >= 3
        and "self.policy.authorize(context" in RPC
        and "authorization_happens_before_machine_evidence_lookup" in RPC
        and "raw_requests_reject_trusted_context_injection" in RPC
        and has_all(
            RPC,
            (
                "ConfirmationRequired",
                "IrreversibleAcknowledgementRequired",
                "PlanMismatch",
                "ApprovalMismatch",
            ),
        ),
    ),
    (
        "replay-resume-continuity",
        has_all(STORE, ("record_fingerprint", "event history is not append-only"))
        and has_all(RPC, ("expected_version", "VersionMismatch", "SessionNotResumable"))
        and "pending_feature_resume_is_version_bound_and_single_use" in RPC,
    ),
    (
        "opaque-capability-and-cli-separation",
        "pub(crate) fn for_rpc() -> Self" in EXECUTOR
        and "pub fn for_rpc()" not in EXECUTOR
        and "neo-repair" in CLI_MANIFEST
        and "RepairExecutorCapability" not in CLI_MAIN
        and "RepairRpcService" not in CLI_MAIN
        and "RepairExecutorCapability" not in REPAIR_CLI
        and "RepairRpcService" not in REPAIR_CLI
        and all(value not in REPAIR_CLI for value in ("/RestoreHealth", "/scannow", "/Enable-Feature", "/Disable-Feature")),
    ),
    (
        "regression-and-ci-continuity",
        "Phase 21 twenty-lane static review" in CI
        and "python -W error tools/phase21_static_review.py" in CI
        and "Phase 21 Repair & Windows Features proof" in CI
        and "cargo test --locked -p neo-repair" in CI
        and "Phase 21 read-only Windows repair source proof" in CI
        and "Phase 21 read-only Windows feature source proof" in CI
        and "Phase 20 twenty-lane static review" in CI,
    ),
    (
        "adversarial-source-first-acceptance",
        all(
            value in (MODEL + PARSE + EXECUTOR + STORE + RPC + INSPECTION)
            for value in (
                "nul_separated_sfc_admin_failure_is_elevation_required",
                "servicing_3010_success_requires_reboot_even_when_feature_state_is_stable",
                "persisted_owner_rejects_unknown_fields",
                "existing_session_directory_is_never_reported_as_newly_created",
                "malformed_confirmation_does_not_consume_prepared_session",
                "caller_safe_error_payload_does_not_leak_internal_session_path",
                "feature_inspection_does_not_probe_component_store_or_sfc",
            )
        )
        and "source-first" in REVIEW.lower()
        and "live destructive" in REVIEW.lower(),
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
