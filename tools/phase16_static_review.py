#!/usr/bin/env python3
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CRATE = ROOT / "crates" / "neo-debloat-executor"
SRC = CRATE / "src"
production_files = [path for path in sorted(SRC.rglob("*.rs")) if path.name != "tests.rs"]
production = "\n".join(path.read_text(encoding="utf-8") for path in production_files)
lib = (SRC / "lib.rs").read_text(encoding="utf-8")
model = (SRC / "model.rs").read_text(encoding="utf-8")
engine = (SRC / "engine.rs").read_text(encoding="utf-8")
windows = (SRC / "windows.rs").read_text(encoding="utf-8")
tests = (SRC / "tests.rs").read_text(encoding="utf-8")
manifest = (CRATE / "Cargo.toml").read_text(encoding="utf-8")
cli_manifest = (ROOT / "crates" / "neo-cli" / "Cargo.toml").read_text(encoding="utf-8")
workspace = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
review = (ROOT / "docs" / "PHASE16_20_LANE_REVIEW.md").read_text(encoding="utf-8")
decision = (ROOT / "docs" / "decisions" / "0016-PHASE16-DEBLOAT-EXECUTOR.md").read_text(encoding="utf-8")
ci = (ROOT / ".github" / "workflows" / "ci.yml").read_text(encoding="utf-8")

phase16_static_step = """      - name: Phase 16 twenty-lane static review
        run: python -W error tools/phase16_static_review.py"""
phase16_behavior_step = """      - name: Phase 16 deterministic AppX executor proof
        run: cargo test --locked -p neo-debloat-executor"""

forbidden_native_scope = (
    "DeprovisionPackageForAllUsersAsync",
    "ProvisionPackageForAllUsersAsync",
    "RegisterPackagesByFullNameAsync",
    "RemovePackageWithOptionsAsync",
    "RemovalOptions",
    "PowerShell",
    "powershell",
    "cmd.exe",
)

checks = [
    (
        "Phase 15 prepared transaction is the only execution input authority",
        "use neo_debloat_plan::DebloatPreparedTransaction" in lib
        and "prepared: &DebloatPreparedTransaction" in lib
        and "DebloatExecutionSession::from_prepared(prepared)" in lib,
    ),
    (
        "exactly one BaselineCaptured action with fingerprint continuity is required",
        all(value in model for value in (
            "prepared.steps().len() != 1",
            "prepared.transaction().actions().len() != 1",
            "TransactionStage::BaselineCaptured",
            "prepared.checkpoint().plan_fingerprint() != prepared.transaction().fingerprint()?",
        )),
    ),
    (
        "main and dependency restore identities remain constructor-owned",
        all(value in model for value in (
            "package_full_name: String",
            "package_family_name: String",
            "dependency_full_names: Vec<String>",
            "DebloatRestoreRoute::RegisterByFullNameFromProvisioned",
            "pub fn package_full_name(&self) -> &str",
            "pub fn dependency_full_names(&self) -> &[String]",
        ))
        and "pub package_full_name:" not in model
        and "pub dependency_full_names:" not in model,
    ),
    (
        "opaque Debloat executor capability has no public constructor",
        "pub struct DebloatExecutorCapability" in model
        and "_private: ()" in model
        and "pub(crate) fn for_tests()" in model
        and "pub fn for_tests()" not in model
        and "&DebloatExecutorCapability" in lib,
    ),
    (
        "no CLI GUI plugin MCP or RPC capability issuer exists",
        "neo-debloat-executor" not in cli_manifest
        and "mod rpc" not in lib
        and "MCP_" not in production
        and "RPC_" not in production
        and "plugin" not in manifest.lower(),
    ),
    (
        "fresh captured baseline is checked before authorization",
        "ensure_baseline_unchanged(session, host)?;" in engine
        and "session.checkpoint.authorize(authorization)?;" in engine,
    ),
    (
        "Phase 4 TransactionAuthorization remains the authority contract",
        "TransactionAuthorization" in lib
        and "TransactionAuthorization" in engine
        and "checkpoint.authorize" in engine,
    ),
    (
        "same-session named mutex serializes apply",
        "THETECHGUY.NeoDriver.DebloatExecutor.v1" in windows
        and "CreateMutexW" in windows
        and "DebloatExecutionMutex::acquire()?" in lib
        and "WaitForSingleObject" in windows,
    ),
    (
        "baseline is checked a second time under apply authority before removal",
        "pub fn apply(" in lib
        and "DebloatExecutionMutex::acquire()?" in lib
        and engine.count("ensure_baseline_unchanged(session, host)?;") >= 2
        and engine.index("ensure_baseline_unchanged(session, host)?;") < engine.index("session.checkpoint.begin_apply()?;"),
    ),
    (
        "forward mutation is exact native current-user full-name removal",
        ".RemovePackageAsync(&HSTRING::from(package_full_name))" in windows
        and "step.package_full_name()" in engine
        and "remove_current_user" in engine,
    ),
    (
        "no deprovision all-users force-option or shell mutation path exists",
        all(value not in windows for value in forbidden_native_scope)
        and "DeploymentOptions::None" in windows,
    ),
    (
        "native DeploymentResult extended failure is checked",
        "ExtendedErrorCode()" in windows
        and "extended.is_err()" in windows
        and "ErrorText()" in windows
        and ".join()" in windows,
    ),
    (
        "fresh main and dependency observations follow removal",
        "let observed_after = observe_all(session, host);" in engine
        and "fn observe_all" in engine
        and "session_targets(session)" in engine
        and ".current_user" in engine
        and "eq_ignore_ascii_case(full_name)" in engine,
    ),
    (
        "unknown post-write state conservatively preserves machine-changed obligation",
        ".unwrap_or(true)" in engine
        and "any_target_changed_from_baseline" in engine
        and "ObservedValue::Unavailable" in engine,
    ),
    (
        "API outcome and machine_changed are separately recorded",
        "ApplyRecord" in engine
        and "outcome: ApplyOutcome::Success" in engine
        and "outcome: ApplyOutcome::Failure" in engine
        and "machine_changed," in engine,
    ),
    (
        "Phase 4 postcondition verification determines forward completion",
        "session.checkpoint.verify_postconditions(observations)?;" in engine
        and "TransactionStage::Complete" in engine
        and "TransactionStage::RollingBack" in engine,
    ),
    (
        "rollback is exact staged full-name registration with captured dependencies",
        ".RegisterPackageByFullNameAsync(" in windows
        and "dependency_full_names" in windows
        and "DeploymentOptions::None" in windows
        and "step.dependency_full_names()" in engine,
    ),
    (
        "rollback outcome and every captured target are freshly verified",
        "record_rollback_result(RollbackRecord" in engine
        and "session.checkpoint.verify_rollback(observations)?;" in engine
        and "session_targets(session)" in engine
        and "TransactionStage::RolledBack" in engine,
    ),
    (
        "deterministic fake-host proof covers forward drift partial failure and rollback failure",
        all(name in tests for name in (
            "phase16_successful_exact_current_user_removal_reaches_complete",
            "phase16_pre_authority_baseline_drift_fails_closed_without_mutation",
            "phase16_second_baseline_check_blocks_drift_after_authorization",
            "phase16_partial_removal_failure_restores_main_and_dependency_baselines",
            "phase16_postcondition_failure_after_dependency_change_forces_rollback",
            "phase16_api_success_without_machine_change_does_not_invent_rollback_work",
            "phase16_rollback_registration_failure_remains_failed_and_unresolved",
            "phase16_main_only_removal_keeps_dependency_and_still_verifies",
        ))
        and "impl DebloatHost for FakeHost" in tests,
    ),
    (
        "CI compiles real backend but executes only deterministic Phase 16 mutation proof",
        '"crates/neo-debloat-executor"' in workspace
        and phase16_static_step in ci
        and phase16_behavior_step in ci
        and "cargo test --locked -p neo-debloat-executor --test live" not in ci
        and "live GitHub-runner AppX mutation" in decision
        and "**Mutation authority:** opaque internal capability only; no public issuer" in review,
    ),
]

failed = []
for index, (name, ok) in enumerate(checks, 1):
    print(f"{index:02d}. {'PASS' if ok else 'FAIL'} - {name}")
    if not ok:
        print(f"::error title=Phase 16 lane {index:02d} failed::{name}")
        failed.append(name)

if failed:
    raise SystemExit("Phase 16 static review failed: " + ", ".join(failed))

print("PHASE 16 STATIC REVIEW PASS: 20/20")
