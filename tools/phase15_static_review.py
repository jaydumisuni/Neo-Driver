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

mutator_markers = (
    ".RemovePackageAsync(",
    ".RegisterPackageByFullNameAsync(",
    ".RegisterPackagesByFullNameAsync(",
    ".DeprovisionPackageForAllUsersAsync(",
    ".ProvisionPackageForAllUsersAsync(",
    ".AddPackageAsync(",
    ".StagePackageAsync(",
)

checks = [
    (
        "Phase 13 removal-candidate law remains authoritative",
        all(value in production for value in (
            "assess_debloat",
            "DebloatDisposition::RemovalCandidate",
            "NotRemovalCandidate",
        )),
    ),
    (
        "Phase 14 presence evidence is composed",
        "neo_debloat_probe::scan_current_debloat_evidence" in production
        and "&phase14.evidence" in production,
    ),
    (
        "native PackageManager inventory is read-only",
        '"crates/neo-debloat-plan"' in workspace
        and all(name in manifest for name in ("neo-debloat", "neo-debloat-probe", "neo-transaction"))
        and all(value in production for value in (
            "PackageManager::new",
            "FindPackagesByUserSecurityId",
            "FindProvisionedPackages",
        ))
        and all(value not in production for value in mutator_markers),
    ),
    (
        "exact package Name FullName FamilyName captured",
        all(value in production for value in (
            "pub name: String",
            "pub full_name: String",
            "pub family_name: String",
            ".Name()",
            ".FullName()",
            ".FamilyName()",
        )),
    ),
    (
        "direct dependency identities captured",
        all(value in production for value in (
            ".Dependencies()",
            "ExactPackageDependency",
            "dependency_full_names",
        )),
    ),
    (
        "exact inventory validates non-empty identities",
        "inventory.validate()?" in production
        and "require_text(\"package name\"" in production
        and "require_text(\"package full name\"" in production
        and "require_text(\"package family name\"" in production,
    ),
    (
        "duplicate exact full names fail closed",
        "validate_unique_full_names" in production
        and "duplicate {label} package full name" in production
        and "AmbiguousExactIdentity" in production,
    ),
    (
        "package-name ambiguity fails closed",
        "fn exact_one" in production
        and "AmbiguousExactIdentity(label.to_string())" in production,
    ),
    (
        "Phase 14 versus native drift fails closed",
        "assessed.installed != ObservedPresence::Present" in production
        and "current/provisioned exact identity mismatch" in production
        and "InventoryDrift" in production,
    ),
    (
        "framework and resource main candidates are blocked",
        "package.is_framework || package.is_resource" in production
        and "UnsafePackageKind" in production,
    ),
    (
        "exactly one selected item is allowed",
        "selected_ids.len() != 1" in production
        and "BatchNotSupported" in production,
    ),
    (
        "only current-user scope is allowed",
        "assessed.scope != DebloatScope::CurrentUser" in production
        and "UnsupportedScope" in production,
    ),
    (
        "Store and vendor metadata are not rollback authority",
        "RestoreMethod::ProvisionedImage" in production
        and "Store metadata is not deterministic local rollback authority" in production
        and "RestoreNotReady" in production,
    ),
    (
        "main package requires exact provisioned twin",
        "provisioned_by_name" in production
        and "canonical(&current.full_name) != canonical(&provisioned.full_name)" in production
        and "canonical(&current.family_name) != canonical(&provisioned.family_name)" in production,
    ),
    (
        "every direct dependency requires exact provisioned twin",
        "ensure_dependency_restore_ready" in production
        and "provisioned_exact" in production
        and "dependency {} is not present as the exact provisioned staged identity" in production,
    ),
    (
        "Debloat transaction uses exact targets and confirmation",
        "kind: ActionKind::Debloat" in production
        and "requires_confirmation: true" in production
        and "StateTargetKind::AppxPackage" in production
        and "VerificationExpectation::Absent" in production,
    ),
    (
        "baseline checkpoint contains main and dependency identity state",
        "checkpoint.capture_baseline(baseline_states)?" in production
        and "CapturedValue::Present" in production
        and "TransactionStage::BaselineCaptured" in production,
    ),
    (
        "prepared state is constructor-owned and plan fingerprint remains continuous",
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
    (
        "Windows live inventory proof is non-empty and behaviorally read-only",
        "native_exact_appx_identity_scan_is_read_only_to_fixture_state" in behavior
        and "!inventory.current_user.is_empty()" in behavior
        and "!inventory.provisioned.is_empty()" in behavior
        and "assert!(!inventory.machine_changes);" in behavior
        and "before, after," in behavior
        and phase15_live_step in ci,
    ),
    (
        "no mutation public-write plugin or MCP/RPC capability exists",
        all(value not in production for value in mutator_markers)
        and "MCP_TWEAK" not in production
        and "rpc::" not in production
        and "plugin" not in manifest.lower()
        and "**Mutation authority:** none" in review
        and "plugin dependency" in decision
        and "MCP/RPC debloat capability issuance or execution" in decision
        and phase15_static_step in ci
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
