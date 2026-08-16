#!/usr/bin/env python3
from __future__ import annotations

from pathlib import Path
import sys
import tomllib

ROOT = Path(__file__).resolve().parents[1]
CRATE = ROOT / "crates" / "neo-debloat-history-store"
SRC = CRATE / "src"
LIB = (SRC / "lib.rs").read_text(encoding="utf-8")
MODEL = (SRC / "model.rs").read_text(encoding="utf-8")
STORE = (SRC / "store.rs").read_text(encoding="utf-8")
TESTS = (SRC / "tests.rs").read_text(encoding="utf-8")
MANIFEST = (CRATE / "Cargo.toml").read_text(encoding="utf-8")
WORKSPACE_TEXT = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
WORKSPACE = tomllib.loads(WORKSPACE_TEXT)
VAULT_LAYOUT = (ROOT / "crates" / "neo-vault" / "src" / "layout.rs").read_text(encoding="utf-8")
HISTORY_MANIFEST = (ROOT / "crates" / "neo-debloat-history" / "Cargo.toml").read_text(encoding="utf-8")
EXECUTOR_MANIFEST = (ROOT / "crates" / "neo-debloat-executor" / "Cargo.toml").read_text(encoding="utf-8")
RESTORE_MANIFEST = (ROOT / "crates" / "neo-debloat-restore-executor" / "Cargo.toml").read_text(encoding="utf-8")
DECISION = (ROOT / "docs" / "decisions" / "0019-PHASE19-DEBLOAT-HISTORY-STORE.md").read_text(encoding="utf-8")
REVIEW = (ROOT / "docs" / "PHASE19_20_LANE_REVIEW.md").read_text(encoding="utf-8")
CI = (ROOT / ".github" / "workflows" / "ci.yml").read_text(encoding="utf-8")
PRODUCTION = "\n".join((LIB, MODEL, STORE))


def has_all(text: str, values: tuple[str, ...]) -> bool:
    return all(value in text for value in values)


members = set(WORKSPACE["workspace"]["members"])
forbidden_write_surfaces = (
    "pub fn save_receipt",
    "pub fn import_receipt",
    "pub fn delete",
    "pub fn remove_record",
    "pub fn overwrite",
)
forbidden_appx_mutation = (
    "RegisterPackageByFullNameAsync",
    "RemovePackageAsync",
    "PackageManager::new",
    "AddPackage",
)

checks = [
    (
        "authority-continuity",
        'pub fn record_completed_execution' in STORE
        and 'receipt_from_completed_execution(session)?' in STORE
        and 'record_validated_receipt_for_tests' in STORE
        and '#[cfg(test)]' in STORE,
    ),
    (
        "single-managed-root",
        'history: managed_root.join("history")' in VAULT_LAYOUT
        and 'pub fn history(&self)' in VAULT_LAYOUT
        and 'self.layout.history().to_path_buf()' in STORE
        and "ProgramData" not in PRODUCTION
        and "Program Files" not in PRODUCTION,
    ),
    (
        "typed-record-identity",
        has_all(MODEL, ('value.len() != 64', 'is_ascii_hexdigit()', 'to_ascii_lowercase()', 'receipt.receipt_fingerprint()'))
        and 'record_ids_reject_non_fingerprint_and_traversal_like_input' in TESTS,
    ),
    (
        "no-arbitrary-receipt-import",
        not any(surface in PRODUCTION for surface in forbidden_write_surfaces)
        and 'record_validated_receipt(' not in LIB
        and 'caller-supplied receipt JSON' in LIB,
    ),
    (
        "no-follow-traversal",
        has_all(STORE, ('open_dir_nofollow', 'FollowSymlinks::No', 'open_absolute_dir_nofollow', 'Component::ParentDir')),
    ),
    (
        "bounded-record-envelope",
        has_all(MODEL, ('DEBLOAT_HISTORY_STORE_SCHEMA_VERSION', 'MAX_HISTORY_RECORD_BYTES', 'StoredReceiptEnvelope'))
        and 'metadata.len() > MAX_HISTORY_RECORD_BYTES' in STORE,
    ),
    (
        "identity-continuity-on-read",
        has_all(MODEL, ('expected_record_id', 'receipt_record_id', 'directory, envelope, and receipt record identities differ'))
        and 'oversized_and_identity_mismatched_records_fail_before_selection' in TESTS,
    ),
    (
        "append-only-retention",
        not any(surface in PRODUCTION for surface in forbidden_write_surfaces)
        and 'HistoryRecordDisposition::AlreadyPresent' in STORE,
    ),
    (
        "marker-owned-nested-promotion",
        has_all(STORE, ('STAGED_RECORD_DIRECTORY_NAME', 'write_staging_marker', 'create_dir(STAGED_RECORD_DIRECTORY_NAME)', 'staging_dir.rename(', 'STAGED_RECORD_DIRECTORY_NAME'))
        and 'marker-owned unique staging session' in REVIEW,
    ),
    (
        "concurrent-idempotence",
        'concurrent_identical_writers_converge_on_one_valid_record' in TESTS
        and 'HistoryRecordDisposition::Recorded' in TESTS
        and 'HistoryRecordDisposition::AlreadyPresent' in TESTS,
    ),
    (
        "content-drift-fail-closed",
        'tampered_final_record_fails_closed_and_is_never_repaired_by_recording_again' in TESTS
        and 'RecordConflict' in STORE,
    ),
    (
        "crash-staging-isolation",
        has_all(STORE, ('audit_staging', 'validate_staging_marker', 'STAGED_RECORD_DIRECTORY_NAME'))
        and 'staging is never enumerated or selected as completed history' in DECISION,
    ),
    (
        "trusted-selection",
        has_all(STORE, ('pub fn load(', 'prepare_restore_from_inventory_by_id', 'prepare_windows_restore_by_id'))
        and 'caller-supplied filesystem paths' in DECISION,
    ),
    (
        "fresh-restore-readiness",
        'prepare_restore_from_inventory(' in STORE
        and 'trusted_selection_by_id_preserves_phase17_fresh_restore_readiness' in TESTS,
    ),
    (
        "phase18-capability-preserved",
        'neo-debloat-restore-executor' not in MANIFEST
        and 'DebloatRestoreExecutorCapability' not in PRODUCTION
        and 'does **not** issue `DebloatRestoreExecutorCapability`' in DECISION,
    ),
    (
        "bounded-acyclic-dependencies",
        'crates/neo-debloat-history-store' in members
        and 'neo-debloat-executor = { path = "../neo-debloat-executor" }' in MANIFEST
        and 'neo-debloat-history = { path = "../neo-debloat-history" }' in MANIFEST
        and 'neo-vault = { path = "../neo-vault" }' in MANIFEST
        and 'neo-debloat-history-store' not in HISTORY_MANIFEST
        and 'neo-debloat-history-store' not in EXECUTOR_MANIFEST
        and 'neo-debloat-history-store' not in RESTORE_MANIFEST,
    ),
    (
        "installed-portable-parity",
        'installed_and_portable_modes_share_canonical_history_child' in TESTS
        and 'VaultMode::Installed' in TESTS
        and 'VaultMode::Portable' in TESTS,
    ),
    (
        "non-appx-mutation",
        not any(value in PRODUCTION for value in forbidden_appx_mutation)
        and 'std::process::Command' not in PRODUCTION
        and 'AppX mutation authority:** none' in REVIEW,
    ),
    (
        "regression-and-ci-proof",
        'Phase 19 twenty-lane static review' in CI
        and 'python -W error tools/phase19_static_review.py' in CI
        and 'Phase 19 trusted Debloat history store proof' in CI
        and 'cargo test --locked -p neo-debloat-history-store' in CI
        and 'Phase 18 twenty-lane static review' in CI
        and 'Phase 18 deterministic AppX restore executor proof' in CI,
    ),
    (
        "adversarial-trust-boundary",
        all(
            name in TESTS
            for name in (
                'record_ids_reject_non_fingerprint_and_traversal_like_input',
                'concurrent_identical_writers_converge_on_one_valid_record',
                'tampered_final_record_fails_closed_and_is_never_repaired_by_recording_again',
                'oversized_and_identity_mismatched_records_fail_before_selection',
                'unexpected_final_or_staging_entries_fail_audit_closed',
                'trusted_selection_by_id_preserves_phase17_fresh_restore_readiness',
                'receipt_symlink_substitution_is_rejected',
            )
        )
        and 'same OS principal' in DECISION,
    ),
]

failed: list[str] = []
for index, (name, passed) in enumerate(checks, 1):
    print(f"{index:02d}. {'PASS' if passed else 'FAIL'} - {name}")
    if not passed:
        failed.append(name)

if failed:
    raise SystemExit("Phase 19 static review failed: " + ", ".join(failed))

print("PHASE 19 STATIC REVIEW PASS: 20/20")
sys.exit(0)
