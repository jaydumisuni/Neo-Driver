#!/usr/bin/env python3
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CRATE = ROOT / "crates" / "neo-debloat-history"
SRC = CRATE / "src"
lib = (SRC / "lib.rs").read_text(encoding="utf-8")
model = (SRC / "model.rs").read_text(encoding="utf-8")
plan = (SRC / "plan.rs").read_text(encoding="utf-8")
tests = (SRC / "tests.rs").read_text(encoding="utf-8")
manifest = (CRATE / "Cargo.toml").read_text(encoding="utf-8")
cli_manifest = (ROOT / "crates" / "neo-cli" / "Cargo.toml").read_text(encoding="utf-8")
workspace = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
review = (ROOT / "docs" / "PHASE17_20_LANE_REVIEW.md").read_text(encoding="utf-8")
decision = (ROOT / "docs" / "decisions" / "0017-PHASE17-DEBLOAT-HISTORY-RESTORE-READINESS.md").read_text(encoding="utf-8")
ci = (ROOT / ".github" / "workflows" / "ci.yml").read_text(encoding="utf-8")
production = "\n".join((lib, model, plan))

phase17_static_step = """      - name: Phase 17 twenty-lane static review
        run: python -W error tools/phase17_static_review.py"""
phase17_behavior_step = """      - name: Phase 17 history and restore-readiness proof
        run: cargo test --locked -p neo-debloat-history"""
inherited_runtime_step = """      - name: Runtime CLI fixture proof
        run: cargo run --locked -p neo-cli -- runtimes --evidence fixtures/runtime/runtime_inventory.json --catalogue fixtures/catalogue/sample_runtime_catalogue.json --policy fixtures/runtime/runtime_policy.json --profile fresh-windows"""
inherited_gaming_step = """      - name: Gaming CLI fixture proof
        run: cargo run --locked -p neo-cli -- gaming --evidence fixtures/runtime/runtime_inventory.json --catalogue fixtures/catalogue/sample_runtime_catalogue.json --policy fixtures/runtime/runtime_policy.json"""

checks = [
    (
        "receipt source must be completed Phase 16 execution",
        "session.stage() != TransactionStage::Complete" in lib
        and "receipt_from_completed_execution" in lib,
    ),
    (
        "execution and checkpoint fingerprint continuity is required",
        "session.checkpoint().plan_fingerprint() != transaction.fingerprint()?" in lib
        and "session.checkpoint().plan().fingerprint()? != transaction.fingerprint()?" in lib,
    ),
    (
        "receipt identities come from captured baseline",
        "session.checkpoint().baseline()" in lib
        and "CapturedValue::Present" in lib
        and "serde_json::from_str(main_json)" in lib
        and "ExactPackageDependency = serde_json::from_str(json)?" in lib,
    ),
    (
        "restore route is bound to main and ordered dependencies",
        "restore route does not match captured main/dependency identities" in model
        and "zip(&self.dependencies)" in model,
    ),
    (
        "receipt schema is versioned and durable deserialization revalidates",
        "DEBLOAT_REMOVAL_RECEIPT_SCHEMA_VERSION: u32 = 1" in model
        and "impl<'de> Deserialize<'de> for DebloatRemovalReceipt" in model
        and "receipt.validate()?" in model
        and "source_checkpoint: TransactionCheckpoint" in model
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
    ),
    (
        "receipt id is deterministic and source-transaction-bound",
        'phase17-removal-receipt' in model
        and "receipt id does not bind to source transaction id" in model,
    ),
    (
        "receipt SHA-256 fingerprint is recomputed and checked",
        "Sha256::digest" in model
        and "ReceiptFingerprintMaterial" in model
        and "receipt fingerprint mismatch" in model,
    ),
    (
        "fingerprint is explicitly not a signature/authentication mechanism",
        "cryptographic signature" in decision
        and "caller-authentication mechanism" in decision
        and "trusted storage provenance" in decision,
    ),
    (
        "fresh exact AppX inventory is mandatory before restore preparation",
        "inventory.validate()?;" in plan
        and "scan_windows_exact_appx_inventory()?" in lib
        and "prepare_restore_from_inventory" in lib,
    ),
    (
        "already-restored exact main returns dedicated disposition",
        "DebloatHistoryError::AlreadyRestored" in plan
        and "already_restored_main_is_not_prepared_again" in tests,
    ),
    (
        "different current main version or family blocks old receipt restore",
        "a different current-user version/identity" in plan
        and "different_current_main_version_blocks_old_history_restore" in tests,
    ),
    (
        "exact staged main must remain present with receipt dependency shape",
        "ensure_provisioned_restore_route" in plan
        and "same_main_restore_shape(main, receipt.main())" in plan
        and "left.is_framework == right.is_framework" in plan
        and "left.is_resource == right.is_resource" in plan
        and "left.is_bundle == right.is_bundle" in plan
        and "left.is_optional == right.is_optional" in plan
        and "missing_exact_staged_main_blocks_restore_readiness" in tests
        and "staged_main_kind_flag_drift_blocks_restore_readiness" in tests,
    ),
    (
        "every original dependency must remain exactly staged",
        "exact staged dependency" in plan
        and "missing_exact_staged_dependency_blocks_restore_readiness" in tests,
    ),
    (
        "current dependency version/name/family conflicts fail closed",
        "ensure_dependency_restore_state" in plan
        and "different_current_dependency_version_blocks_restore_readiness" in tests,
    ),
    (
        "fresh restore baseline captures main absent and dependency present-or-absent",
        "value: CapturedValue::Absent" in plan
        and "current_dependency_baseline" in plan
        and "CapturedValue::Present" in plan
        and "existing_exact_dependency_is_preserved_as_restore_time_baseline" in tests,
    ),
    (
        "inverse forward postconditions require exact original main and dependencies",
        "VerificationExpectation::Equals(serde_json::to_string(receipt.main())?)" in plan
        and "VerificationExpectation::Equals(serde_json::to_string(dependency)?)" in plan,
    ),
    (
        "inverse rollback matches every restore-time baseline target",
        "VerificationExpectation::MatchesBaseline" in plan
        and "restore_targets: snapshot_targets" in plan
        and "rollback_verification" in plan,
    ),
    (
        "restore is explicit never-preselected and read-only prepared",
        "selected_by_default: false" in plan
        and "requires_confirmation: true" in plan
        and "risk: source_action.risk" in plan
        and "RecommendationState::Repair" in plan
        and "machine_changes: false" in model,
    ),
    (
        "deterministic regressions cover durable history conflicts and non-mutation",
        all(name in tests for name in (
            "completed_removal_becomes_versioned_fingerprinted_durable_history",
            "receipt_fingerprint_rejects_history_tampering",
            "receipt_rejects_broadened_source_authority_even_with_valid_json_shape",
            "non_complete_source_checkpoint_cannot_become_history_receipt",
            "prepares_fresh_inverse_transaction_when_exact_local_restore_is_still_ready",
            "existing_exact_dependency_is_preserved_as_restore_time_baseline",
            "already_restored_main_is_not_prepared_again",
            "different_current_main_version_blocks_old_history_restore",
            "missing_exact_staged_main_blocks_restore_readiness",
            "staged_main_kind_flag_drift_blocks_restore_readiness",
            "missing_exact_staged_dependency_blocks_restore_readiness",
            "different_current_dependency_version_blocks_restore_readiness",
            "restore_readiness_is_byte_for_byte_non_mutating",
        )),
    ),
    (
        "CI proves Phase 17 without restore mutation and preserves inherited runtime fixture authority",
        '"crates/neo-debloat-history"' in workspace
        and phase17_static_step in ci
        and phase17_behavior_step in ci
        and inherited_runtime_step in ci
        and inherited_gaming_step in ci
        and "neo-debloat-history" not in cli_manifest
        and "RemovePackageAsync" not in production
        and "RegisterPackageByFullNameAsync" not in production
        and "DebloatExecutorCapability" not in production
        and ".apply(" not in production
        and "plugin" not in manifest.lower()
        and "**Mutation authority:** none" in review,
    ),
]

failed = []
for index, (name, ok) in enumerate(checks, 1):
    print(f"{index:02d}. {'PASS' if ok else 'FAIL'} - {name}")
    if not ok:
        print(f"::error title=Phase 17 lane {index:02d} failed::{name}")
        failed.append(name)

if failed:
    raise SystemExit("Phase 17 static review failed: " + ", ".join(failed))

print("PHASE 17 STATIC REVIEW PASS: 20/20")
