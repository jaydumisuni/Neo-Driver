#!/usr/bin/env python3
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CRATE = ROOT / "crates" / "neo-debloat-plan"
SRC = CRATE / "src"
production = "\n".join(path.read_text(encoding="utf-8") for path in sorted(SRC.rglob("*.rs")))
workspace = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
manifest = (CRATE / "Cargo.toml").read_text(encoding="utf-8")
review = (ROOT / "docs" / "PHASE15_20_LANE_REVIEW.md").read_text(encoding="utf-8")
decision = (ROOT / "docs" / "decisions" / "0015-PHASE15-DEBLOAT-TRANSACTION-READINESS.md").read_text(encoding="utf-8")
behavior = (CRATE / "tests" / "live_read_only.rs").read_text(encoding="utf-8")
ci = (ROOT / ".github" / "workflows" / "ci.yml").read_text(encoding="utf-8")

phase15_static_step = """      - name: Phase 15 twenty-lane static review
        run: python -W error tools/phase15_static_review.py"""
phase15_live_step = """      - name: Phase 15 live Windows exact AppX identity proof
        if: runner.os == 'Windows'
        run: cargo test --locked -p neo-debloat-plan --test live_read_only"""
phase15_fixture_step = """      - name: Phase 15 transaction-readiness fixture proof
        run: cargo run --locked -p neo-debloat-plan --bin neo-debloat-prepare -- fixtures/debloat/phase15_catalogue.json fixtures/debloat/phase15_evidence.json fixtures/debloat/phase15_inventory.json safe-cleanup appx.contoso.phase15 phase15-fixture --json"""

checks = [
    ("workspace member", '"crates/neo-debloat-plan"' in workspace),
    ("bounded dependencies", all(name in manifest for name in ("neo-core", "neo-debloat", "neo-debloat-probe", "neo-transaction"))),
    ("native PackageManager read surface", all(value in production for value in ("PackageManager::new", "FindPackagesByUserSecurityId", "FindProvisionedPackages"))),
    ("exact package identity", all(value in production for value in ("pub name: String", "pub full_name: String", "pub family_name: String"))),
    ("direct dependency identity", all(value in production for value in ("Package.Dependencies", "dependency_full_names", "ExactPackageDependency"))),
    ("inventory validation", "inventory.validate()?" in production and "validate_unique_full_names" in production),
    ("duplicate exact identity rejected", "AmbiguousExactIdentity" in production and "duplicate {label} package full name" in production),
    ("package-name ambiguity rejected", "fn exact_one" in production and "AmbiguousExactIdentity(label.to_string())" in production),
    ("phase14 native drift rejected", "InventoryDrift" in production and "current/provisioned exact identity mismatch" in production),
    ("unsafe main package kinds blocked", "package.is_framework || package.is_resource" in production and "UnsafePackageKind" in production),
    ("single item only", "selected_ids.len() != 1" in production and "BatchNotSupported" in production),
    ("current user only", "assessed.scope != DebloatScope::CurrentUser" in production and "UnsupportedScope" in production),
    ("metadata not rollback authority", "RestoreMethod::ProvisionedImage" in production and "Store metadata is not deterministic local rollback authority" in production),
    ("main provisioned twin", "provisioned_by_name" in production and "canonical(&current.full_name) != canonical(&provisioned.full_name)" in production),
    ("dependency provisioned twins", "ensure_dependency_restore_ready" in production and "provisioned_exact" in production),
    ("debloat transaction binding", "kind: ActionKind::Debloat" in production and "requires_confirmation: true" in production and "StateTargetKind::AppxPackage" in production),
    ("captured baseline checkpoint", "checkpoint.capture_baseline(baseline_states)?" in production and "CapturedValue::Present" in production),
    (
        "constructor-owned prepared authority and fingerprint continuity",
        all(value in production for value in (
            "pub(crate) steps: Vec<DebloatPreparedStep>",
            "pub(crate) transaction: TransactionPlan",
            "pub(crate) checkpoint: TransactionCheckpoint",
            "pub fn steps(&self) -> &[DebloatPreparedStep]",
            "pub fn transaction(&self) -> &TransactionPlan",
            "pub fn checkpoint(&self) -> &TransactionCheckpoint",
            "pub fn plan_fingerprint(&self) -> &str",
            "self.checkpoint.plan_fingerprint()",
            "TransactionCheckpoint::new(transaction.clone())?",
        )),
    ),
    ("live read only behavior", "native_exact_appx_identity_scan_is_read_only_to_fixture_state" in behavior and "assert!(!inventory.machine_changes);" in behavior and "before, after," in behavior),
    (
        "negative mutation and integration boundary",
        all(value not in production for value in (".RemovePackageAsync(", ".RegisterPackageByFullNameAsync(", ".DeprovisionPackageForAllUsersAsync(", ".ProvisionPackageForAllUsersAsync(", "MCP_TWEAK", "rpc::")) and "plugin" not in manifest.lower()
        and "**Mutation authority:** none" in review
        and "plugin dependency" in decision
        and phase15_static_step in ci
        and phase15_live_step in ci
        and phase15_fixture_step in ci,
    ),
]

failed = []
for index, (name, ok) in enumerate(checks, 1):
    print(f"{index:02d}. {'PASS' if ok else 'FAIL'} - {name}")
    if not ok:
        print(f"::error title=Phase 15 lane {index:02d} failed::{name}")
        failed.append(name)

if failed:
    raise SystemExit("Phase 15 static review failed: " + ", ".join(failed))

print("PHASE 15 STATIC REVIEW PASS: 20/20")
