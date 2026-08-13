#!/usr/bin/env python3
"""Deterministic 20-lane static review for Neo Driver Phase 8 runtime executor."""
from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
import sys
import tomllib

ROOT = Path(__file__).resolve().parents[1]
EXECUTOR_ROOT = ROOT / "crates/neo-runtime-executor"
EXECUTOR = "\n".join(
    path.read_text(encoding="utf-8")
    for path in sorted((EXECUTOR_ROOT / "src").glob("*.rs"))
    if path.name != "tests.rs"
)
LIB = (EXECUTOR_ROOT / "src/lib.rs").read_text(encoding="utf-8")
TESTS = (EXECUTOR_ROOT / "src/tests.rs").read_text(encoding="utf-8")
CATALOGUE = (ROOT / "crates/neo-catalogue/src/lib.rs").read_text(encoding="utf-8")
RUNTIME = (ROOT / "crates/neo-runtime/src/lib.rs").read_text(encoding="utf-8")
VAULT = (ROOT / "crates/neo-vault/src/store.rs").read_text(encoding="utf-8")
CLI = (ROOT / "crates/neo-cli/src/main.rs").read_text(encoding="utf-8")
WORKSPACE = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
DECISION = (ROOT / "docs/decisions/0008-PHASE8-RUNTIME-EXECUTOR.md").read_text(encoding="utf-8")
REVIEW = (ROOT / "docs/PHASE8_20_LANE_REVIEW.md").read_text(encoding="utf-8")


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
    executor_lower = EXECUTOR.lower()
    runtime_lower = RUNTIME.lower()
    cli_lower = CLI.lower()

    no_network = not any(
        marker in executor_lower
        for marker in ["reqwest", "ureq", "curl ", "wget ", "http://", "https://"]
    )
    no_shell = not any(
        marker in executor_lower
        for marker in [
            'command::new("cmd")',
            'command::new("cmd.exe")',
            'command::new("powershell")',
            'command::new("powershell.exe")',
            'command::new("pwsh")',
            'command::new("sh")',
            'command::new("bash")',
        ]
    )
    runtime_stays_read_only = not any(
        marker in runtime_lower
        for marker in ["std::process::command", "createprocess", "msiexec", "winget", "pnputil"]
    )
    public_cli_has_no_executor_apply = not any(
        marker in cli_lower
        for marker in [
            "runtimeexecutorsession",
            "windowsruntimehost",
            ".authorize(",
            ".apply(",
            "resume_after_reboot(",
            "reprobe_after_block(",
        ]
    )
    closed_library_execution_surface = (
        "pub use executor::{RuntimeExecutionSession, RuntimeExecutorCapability};" in LIB
        and "pub(crate) trait RuntimeHost" in EXECUTOR
        and "pub(crate) struct RuntimeInvocation" in EXECUTOR
        and "pub(crate) struct RuntimeProcessResult" in EXECUTOR
        and "pub(crate) struct WindowsRuntimeHost" in EXECUTOR
        and "pub use host::RuntimeHost;" not in LIB
        and "pub use windows::WindowsRuntimeHost;" not in LIB
        and "pub(crate) _private: ()" in EXECUTOR
        and "impl RuntimeExecutorCapability" not in EXECUTOR
        and "authorize_with_capability" in EXECUTOR
        and "apply_windows" in EXECUTOR
    )

    return [
        Lane(1, "separate-executor", "crates/neo-runtime-executor" in members and runtime_stays_read_only, "runtime mutation lives in a separate workspace crate while neo-runtime remains read-only"),
        Lane(2, "runtime-package-only", contains_all(CATALOGUE, ["runtime_execution", "RuntimeExecutionOnNonRuntime", "PackageKind::Runtime"]), "execution metadata is optional and valid only for runtime packages"),
        Lane(3, "phase6-certified-authority", contains_all(EXECUTOR, ["assess_runtime_profile", "EvidenceVerdict::Certified", "MissingCertifiedAction", "RecommendationNotCertified"]), "Phase 8 re-derives authority from a Certified Phase 6 recommendation"),
        Lane(4, "state-operation-binding", contains_all(EXECUTOR, ["RuntimeExecutionOperation::Install", "RuntimeState::Missing", "RuntimeExecutionOperation::Repair", "RuntimeState::Broken", "RuntimeState::Partial", "OperationStateMismatch"]) and "persisted_operation_state_tamper_is_rejected" in TESTS, "install/repair authority is bound to exact normalized baseline state and Serde validation"),
        Lane(5, "explicit-repair-contract", contains_all(EXECUTOR, ["MissingRepairArguments", "repair_args"]) and "repair_without_repair_contract_fails_closed" in TESTS, "repair requires Broken/Partial evidence plus explicit package repair arguments"),
        Lane(6, "dependency-closure-block", contains_all(EXECUTOR, ["DependencyClosureRequired", "package_dependencies", "package_conflicts"]) and "persisted_dependency_authority_tamper_is_rejected" in TESTS, "dependency/conflict edges cannot become standalone execution authority"),
        Lane(7, "security-mutation-block", contains_all(EXECUTOR, ["SecurityMutationBlocked", "changes_boot_or_security_state"]) and contains_all(DECISION, ["security/BCD", "Windows Feature mutation"]), "runtime execution cannot smuggle boot/security or Windows-feature mutation"),
        Lane(8, "derived-vault-payload", contains_all(EXECUTOR, ["runtime_pack_destination", "package_id", "package_version", "package_sha256", "verify_pack", "layout.ensure_managed"]) and "missing_runtime_prepares_exact_irreversible_install" in TESTS, "payload location is derived from Builder/portable root and exact package identity, then constrained to the managed vault"),
        Lane(9, "promoted-hash-proof", EXECUTOR.count("verify_pack") >= 2 and "Sha256Digest" in EXECUTOR, "promoted runtime payload SHA-256 is verified before preparation and again at apply preflight"),
        Lane(10, "preflight-drift", contains_all(EXECUTOR, ["validate_preflight", "windows_build", "canonical_arch", "observation_matches_baseline", "BaselineDrift", "HostDrift", "application_root.is_absolute"]) and "preflight_drift_blocks_before_applying" in TESTS and "persisted_relative_application_root_is_rejected" in TESTS, "absolute application root plus build/architecture/component baseline are re-proven before mutation"),
        Lane(11, "owned-nofollow-staging", contains_all(VAULT, ["stage_managed_file", "open_relative_file_nofollow", "create_new_file_nofollow", "validate_staging_marker"]) and contains_all(EXECUTOR, ["unique_staging_session", "stage_managed_file", "cleanup_staging"]), "runtime staging stays marker-owned and uses Phase 7 no-follow capabilities"),
        Lane(12, "direct-no-shell-execution", contains_all(EXECUTOR, ["Command::new(&invocation.payload)", "trusted_msiexec_path", 'arg("/i")', 'arg("/qn")', 'arg("/norestart")']) and no_shell and no_network, "EXE is launched directly and MSI uses trusted msiexec with no shell/network path"),
        Lane(13, "msi-operation-guard", contains_all(CATALOGUE, ["InvalidMsiRuntimeArgument", "is_msi_property_assignment", "property_value.is_empty", "RuntimeInstallerKind::Msi"]) and "msi_arguments_cannot_replace_neos_fixed_operation_switches" in CATALOGUE and "msi_bare_empty_property_is_rejected" in CATALOGUE, "MSI custom arguments cannot replace Neo's fixed install operation or smuggle a bare empty property assignment"),
        Lane(14, "session-process-serialization", contains_all(EXECUTOR, ["RUNTIME_MUTEX_NAME", "RUNTIME_MUTEX_TIMEOUT_MS", "WAIT_TIMEOUT", "Local\\\\THETECHGUY.NeoDriver.RuntimeExecutor.v1", "CreateMutexW", "WaitForSingleObject", "ReleaseMutex"]) and "INFINITE" not in EXECUTOR and "runtime-executor.lock" not in EXECUTOR and contains_all(REVIEW, ["within one Windows session", "does not claim system-wide cross-session serialization"]), "Windows runtime execution uses a bounded Local named-mutex wait for same-session cross-process serialization and makes no system-wide claim"),
        Lane(15, "locked-payload-reproof", contains_all(EXECUTOR, ["share_mode(FILE_SHARE_READ)", "FILE_FLAG_OPEN_REPARSE_POINT", "FILE_ATTRIBUTE_REPARSE_POINT", "sha256_locked_file", "child.wait()"]) , "staged payload is reparse-rejected, write/delete-locked, re-hashed and retained through process exit"),
        Lane(16, "typed-exit-reboot", contains_all(CATALOGUE, ["success_exit_codes", "reboot_exit_codes", "RuntimeRebootCodeNotSuccessful", "windows_high_bit_exit_code_bit_pattern_is_accepted"]) and contains_all(EXECUTOR, ["success_exit_codes.contains", "reboot_exit_codes.contains"]), "success/reboot exit codes preserve Windows 32-bit status bit patterns and remain explicit typed catalogue authority"),
        Lane(17, "started-means-changed", "machine_changed: process.started" in EXECUTOR and "started_failed_installer_is_conservatively_recorded_changed" in TESTS and "process_not_started_records_no_machine_change" in TESTS, "a started installer is conservatively treated as potentially machine-changing"),
        Lane(18, "reprobe-required", contains_all(EXECUTOR, ["verify_current", "verification_observation", "TransactionStage::Verifying"]) and "exit_code_zero_without_runtime_postcondition_fails" in TESTS and "transient_probe_error_leaves_verification_retryable" in TESTS, "exit code never completes the mission without deterministic re-probe and probe failures remain retryable"),
        Lane(19, "persistent-reboot", contains_all(EXECUTOR, ["resume_after_reboot", "reprobe_after_block", "ObservedValue::Unavailable"]) and "reboot_exit_uses_persistent_checkpoint_and_reprobe" in TESTS and "post_reboot_host_drift_blocks_then_fails_without_fake_rollback" in TESTS, "reboot continuation uses inherited persistent evidence and drift cannot become PASS"),
        Lane(20, "closed-public-mutation", contains_all(EXECUTOR, ["RollbackPlan::Irreversible", "TransactionAuthorization"]) and public_cli_has_no_executor_apply and closed_library_execution_surface and contains_all(CLI, ["RuntimeExecutorValidatePlan", "Execution: disabled from CLI", "Machine changes: none"]) and "irreversible_acknowledgements" in TESTS and "persisted_duplicate_package_evidence_key_is_rejected" in TESTS and "No test or green CI run is allowed to imply that a real runtime installer was executed on CI." in REVIEW, "Phase 8 exposes mutation only through a non-constructible opaque capability; raw host/invocation/Windows adapters remain crate-private and evidence keys are unique"),
    ]


def main() -> int:
    lanes = review()
    for lane in lanes:
        print(f"{'PASS' if lane.passed else 'FAIL'} {lane.number:02d} {lane.name}: {lane.detail}")
    failures = [lane for lane in lanes if not lane.passed]
    if failures:
        print(f"\nPhase 8 static review failed: {len(failures)} lane(s) unresolved.")
        return 1
    print("\nPhase 8 static review: PASS (20/20 lanes).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
