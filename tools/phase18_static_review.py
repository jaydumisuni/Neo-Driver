#!/usr/bin/env python3
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CRATE = ROOT / "crates" / "neo-debloat-restore-executor"
SRC = CRATE / "src"
lib = (SRC / "lib.rs").read_text(encoding="utf-8")
model = (SRC / "model.rs").read_text(encoding="utf-8")
engine = (SRC / "engine.rs").read_text(encoding="utf-8")
windows = (SRC / "windows.rs").read_text(encoding="utf-8")
tests = (SRC / "tests.rs").read_text(encoding="utf-8")
manifest = (CRATE / "Cargo.toml").read_text(encoding="utf-8")
history_manifest = (ROOT / "crates" / "neo-debloat-history" / "Cargo.toml").read_text(encoding="utf-8")
removal_windows = (ROOT / "crates" / "neo-debloat-executor" / "src" / "windows.rs").read_text(encoding="utf-8")
cli_manifest = (ROOT / "crates" / "neo-cli" / "Cargo.toml").read_text(encoding="utf-8")
workspace = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
review = (ROOT / "docs" / "PHASE18_20_LANE_REVIEW.md").read_text(encoding="utf-8")
decision = (ROOT / "docs" / "decisions" / "0018-PHASE18-DEBLOAT-RESTORE-EXECUTOR.md").read_text(encoding="utf-8")
ci = (ROOT / ".github" / "workflows" / "ci.yml").read_text(encoding="utf-8")
production = "\n".join((lib, model, engine, windows))

phase18_static_step = """      - name: Phase 18 twenty-lane static review
        run: python -W error tools/phase18_static_review.py"""
phase18_behavior_step = """      - name: Phase 18 deterministic AppX restore executor proof
        run: cargo test --locked -p neo-debloat-restore-executor"""
inherited_phase17_step = """      - name: Phase 17 history and restore-readiness proof
        run: cargo test --locked -p neo-debloat-history"""
inherited_gaming_step = """      - name: Gaming CLI fixture proof
        run: cargo run --locked -p neo-cli -- gaming --evidence fixtures/runtime/runtime_inventory.json --catalogue fixtures/catalogue/sample_runtime_catalogue.json --policy fixtures/runtime/runtime_policy.json"""

checks = [
    (
        "separate restore executor preserves acyclic history/removal architecture",
        '"crates/neo-debloat-restore-executor"' in workspace
        and 'neo-debloat-history = { path = "../neo-debloat-history" }' in manifest
        and 'neo-debloat-executor = { path = "../neo-debloat-executor" }' in history_manifest
        and "neo-debloat-restore-executor" not in history_manifest,
    ),
    (
        "only frozen Phase 17 inverse transaction shape is accepted",
        'ends_with(":phase17-debloat-restore-current-user")' in model
        and "prepared.transaction().revision() != 1" in model
        and "prepared.transaction().actions().len() != 1" in model
        and "TransactionStage::BaselineCaptured" in model
        and "checkpoint/transaction fingerprint continuity failed" in model,
    ),
    (
        "restore action authority remains exact low-risk confirmed reversible shape",
        "action.kind != ActionKind::Debloat" in model
        and "action.risk != RiskLevel::Low" in model
        and "action.recommendation != RecommendationState::Repair" in model
        and "action.verdict != EvidenceVerdict::Certified" in model
        and "action.selected_by_default" in model
        and "!action.requires_confirmation" in model
        and "action.requires_admin" in model
        and "action.reboot != RebootRequirement::None" in model
        and "!action.rollback_available" in model,
    ),
    (
        "receipt/main/dependency evidence remains bound into execution plan",
        '"phase17_receipt_fingerprint"' in model
        and '"restore_package_full_name"' in model
        and '"restore_dependency_count"' in model
        and "require_action_evidence" in model,
    ),
    (
        "prepared route and exact identity shape are retained",
        "validate_restore_route" in model
        and "self.main.dependencies != self.dependencies" in model
        and "package_family_name" in model
        and "dependency_full_names" in model,
    ),
    (
        "mutation requires opaque internal capability without public issuer",
        "pub struct DebloatRestoreExecutorCapability" in model
        and "_private: ()" in model
        and "pub(crate) fn for_tests()" in model
        and "&DebloatRestoreExecutorCapability" in lib
        and "neo-debloat-restore-executor" not in cli_manifest
        and "plugin" not in manifest.lower(),
    ),
    (
        "fresh baseline is checked before authorization",
        "ensure_execution_state_unchanged(session, host)?;" in engine
        and "session.checkpoint.authorize(authorization)?;" in engine,
    ),
    (
        "fresh baseline is checked again immediately before apply",
        engine.count("ensure_execution_state_unchanged(session, host)?;") >= 2
        and "session.checkpoint.begin_apply()?;" in engine,
    ),
    (
        "side-by-side current conflicts fail closed without exact-match short-circuit",
        "ensure_no_side_by_side_current_conflicts" in engine
        and "continue;" in engine
        and "phase18_side_by_side_dependency_after_exact_baseline_still_blocks_order_independently" in tests,
    ),
    (
        "exact staged main and every dependency are re-proven",
        "ensure_exact_staged_route" in engine
        and "same_main_shape(main, step.main())" in engine
        and "exact staged dependency" in engine
        and "phase18_staged_route_drift_after_authority_blocks_before_mutation" in tests,
    ),
    (
        "Windows forward restore is exact staged full-name registration only",
        "RegisterPackageByFullNameAsync" in windows
        and "package_full_name" in windows
        and "dependency_full_names" in windows
        and "AddPackage" not in windows
        and "Store" not in windows,
    ),
    (
        "native async completion and DeploymentResult are both validated",
        "status != AsyncStatus::Completed" in windows
        and "ExtendedErrorCode" in windows
        and "extended.is_err()" in windows,
    ),
    (
        "forward verification covers main and all dependency targets",
        "session.checkpoint.verify_postconditions(observations)?;" in engine
        and "session_targets(session)" in engine
        and "forward_postcondition_observations" not in engine
        and "phase18_exact_staged_restore_reaches_complete_and_registers_all_identities" in tests,
    ),
    (
        "machine-change evidence is observed and conservative on observation loss",
        "any_target_changed_from_baseline" in engine
        and ".unwrap_or(true)" in engine
        and "post_write_observation_error" in engine
        and "phase18_post_write_observation_loss_is_conservative_and_rolls_back" in tests,
    ),
    (
        "failed changed restore rolls back and compound failures retain both causes",
        "session.stage() == TransactionStage::RollingBack" in engine
        and "restore failed: {error}; rollback also failed: {rollback_error}" in engine
        and "phase18_native_failure_after_mutation_restores_fresh_phase17_baseline" in tests
        and "phase18_rollback_removal_failure_preserves_restore_and_rollback_causes" in tests,
    ),
    (
        "rollback restores Phase 17 baseline instead of historical pre-removal state",
        "apply_restore_time_baseline" in engine
        and "Some(CapturedValue::Present(_)) => {}" in engine
        and "Some(CapturedValue::Absent)" in engine
        and "phase18_failed_postcondition_preserves_existing_dependency_and_removes_restored_main" in tests,
    ),
    (
        "dependency rollback is reverse ordered and Phase 4 MatchesBaseline verified",
        "step.dependencies().iter().rev()" in engine
        and "session.checkpoint.verify_rollback(observations)?;" in engine
        and "TransactionStage::RolledBack" in engine,
    ),
    (
        "Phase 16 and Phase 18 use the same Debloat serialization mutex",
        'Local\\\\THETECHGUY.NeoDriver.DebloatExecutor.v1' in windows
        and 'Local\\\\THETECHGUY.NeoDriver.DebloatExecutor.v1' in removal_windows,
    ),
    (
        "deterministic regressions cover authority drift mutation verification recovery and no-change semantics",
        all(name in tests for name in (
            "phase18_exact_staged_restore_reaches_complete_and_registers_all_identities",
            "phase18_pre_authority_restore_time_baseline_drift_fails_without_mutation",
            "phase18_second_pre_write_check_blocks_drift_after_authorization",
            "phase18_staged_route_drift_after_authority_blocks_before_mutation",
            "phase18_side_by_side_dependency_after_exact_baseline_still_blocks_order_independently",
            "phase18_native_failure_after_mutation_restores_fresh_phase17_baseline",
            "phase18_failed_postcondition_preserves_existing_dependency_and_removes_restored_main",
            "phase18_post_write_observation_loss_is_conservative_and_rolls_back",
            "phase18_api_success_without_machine_change_does_not_invent_rollback_work",
            "phase18_rollback_removal_failure_preserves_restore_and_rollback_causes",
            "phase18_capability_is_opaque_and_not_constructible_by_external_callers",
        )),
    ),
    (
        "CI proves Phase 18 while preserving Phase 17 and inherited fixture authority with no broadened surface",
        phase18_static_step in ci
        and phase18_behavior_step in ci
        and inherited_phase17_step in ci
        and inherited_gaming_step in ci
        and "public restore/undo button" in decision
        and "persistent on-disk history-store authority" in decision
        and "MCP/RPC Debloat restore capability issuance" in decision
        and "**Mutation authority:** opaque internal capability only" in review
        and "https://" not in production.lower()
        and "http://" not in production.lower(),
    ),
]

failed = []
for index, (name, ok) in enumerate(checks, 1):
    print(f"{index:02d}. {'PASS' if ok else 'FAIL'} - {name}")
    if not ok:
        print(f"::error title=Phase 18 lane {index:02d} failed::{name}")
        failed.append(name)

if failed:
    raise SystemExit("Phase 18 static review failed: " + ", ".join(failed))

print("PHASE 18 STATIC REVIEW PASS: 20/20")
