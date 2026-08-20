#!/usr/bin/env python3
from pathlib import Path
import sys
import tomllib

ROOT = Path(__file__).resolve().parents[1]
CRATE = ROOT / "crates" / "neo-driver-repair"
LIB = (CRATE / "src" / "lib.rs").read_text(encoding="utf-8")
MODEL = (CRATE / "src" / "model.rs").read_text(encoding="utf-8")
ASSESS = (CRATE / "src" / "assessment.rs").read_text(encoding="utf-8")
TESTS = (CRATE / "src" / "tests.rs").read_text(encoding="utf-8")
MANIFEST = (CRATE / "Cargo.toml").read_text(encoding="utf-8")
WORKSPACE_RAW = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
WORKSPACE = tomllib.loads(WORKSPACE_RAW)
MASTER = (ROOT / "docs" / "NEO_DRIVER_MASTER_PLAN.md").read_text(encoding="utf-8")
DECISION = (ROOT / "docs" / "decisions" / "0022-PHASE22-DRIVER-PNP-REPAIR-ASSESSMENT.md").read_text(encoding="utf-8")
REVIEW = (ROOT / "docs" / "PHASE22_20_LANE_REVIEW.md").read_text(encoding="utf-8")
CLI = (ROOT / "crates" / "neo-cli" / "src" / "repair_cli.rs").read_text(encoding="utf-8")
CLI_MANIFEST = (ROOT / "crates" / "neo-cli" / "Cargo.toml").read_text(encoding="utf-8")
CI = (ROOT / ".github" / "workflows" / "ci.yml").read_text(encoding="utf-8")
FIXTURE = (ROOT / "fixtures" / "repair" / "phase22_driver_evidence.json").read_text(encoding="utf-8")

members = set(WORKSPACE["workspace"]["members"])
forbidden_assessment_calls = (
    "stage_driver(",
    "install_best_match(",
    "restore_specific_driver(",
    "remove_published_package(",
    "DiInstallDevice",
    "SetupCopyOEMInf",
    "SetupUninstallOEMInf",
)

def has_all(text, values):
    return all(value in text for value in values)

checks = [
    ("01-master-plan-continuity", has_all(MASTER, ("Driver Store/PnP repair;", "device re-enumeration;", "Windows Update reset/repair;"))),
    ("02-exact-authority-recorded", has_all(DECISION, ("5e791fd6509a818b8f6632d57e1c74ffbc258461", "neo-phase22-scope-tenfold-workspace", "four deterministic authority evidence packets"))),
    ("03-separate-crate-boundary", "crates/neo-driver-repair" in members and 'name = "neo-driver-repair"' in MANIFEST and 'neo-driverstore = { path = "../neo-driverstore" }' in MANIFEST),
    ("04-read-only-host-seam", has_all(ASSESS, ("host.inventory()?", "host.resolve_published_package(value)?"))),
    ("05-no-mutation-call-path", not any(token in ASSESS for token in forbidden_assessment_calls) and "machine_changes: false" in ASSESS),
    ("06-exact-device-identity", "to_ascii_lowercase()" in MODEL and "DriverRepairError::DuplicateDevice" in MODEL and "duplicate_instance_ids_are_case_insensitive" in TESTS),
    ("07-package-requires-binding", "DriverRepairError::PackageWithoutBinding" in MODEL and "package_without_active_binding_is_rejected" in TESTS),
    ("08-package-identity-equality", "eq_ignore_ascii_case(published)" in MODEL and "DriverRepairError::PackageMismatch" in MODEL and "mismatched_driver_store_identity_is_rejected" in TESTS),
    ("09-unknown-problem-fails-closed", "Some(0)" in ASSESS and "None => (" in ASSESS and "unknown_problem_code_never_becomes_healthy" in TESTS),
    ("10-healthy-needs-exact-package", "DriverRepairState::Healthy" in ASSESS and "evidence.current_package.is_none()" in ASSESS and "healthy_exact_binding_requires_no_action" in TESTS),
    ("11-reinstall-is-candidate-only", "CurrentExactDriverReinstallCandidate" in MODEL and "future authority phase" in ASSESS and "only_a_reinstall_candidate" in TESTS),
    ("12-selection-reuses-existing-authority", "DriverSelectionRequired" in MODEL and "existing matcher/catalogue authority" in ASSESS),
    ("13-disabled-remains-read-only", "DriverRepairState::Disabled" in ASSESS and "no enable or re-enumeration authority" in ASSESS and "disabled_device_is_recorded_without_enable_authority" in TESTS),
    ("14-filters-are-evidence-only", has_all(MODEL, ("upper_filters", "lower_filters")) and "filters_are_retained_as_evidence_not_inferred_as_fault" in TESTS),
    ("15-deterministic-order-and-digest", "evidence.devices.sort_by" in ASSESS and "source_evidence_sha256" in MODEL and "output_order_and_digest_are_independent_of_inventory_order" in TESTS),
    ("16-machine-change-false", "pub machine_changes: bool" in MODEL and "machine_changes: false" in ASSESS and "machine_changes = false" in DECISION),
    ("17-read-only-cli-surface", has_all(CLI, ("RepairCommand::Drivers", "inspect_windows_driver_repair", "DriverRepairEvidence::from_json_str", "Machine changes: none")) and "neo-driver-repair" in CLI_MANIFEST),
    ("18-adversarial-write-method-proof", has_all(TESTS, ("Phase 22 has no stage authority", "Phase 22 has no install authority", "Phase 22 has no rollback mutation authority", "Phase 22 has no Driver Store delete authority", "live_adapter_invokes_only_inventory_and_exact_package_resolution"))),
    ("19-ci-proof-binding", has_all(CI, ("Phase 22 twenty-lane static review", "Phase 22 Driver Store / PnP assessment proof", "Phase 22 live Windows driver repair source proof", "Phase 22 driver repair fixture proof")) and "phase22_driver_evidence.json" in CI),
    ("20-deferred-scope-remains-closed", has_all(DECISION, ("device re-enumeration execution", "driver staging or installation", "Driver Store package deletion", "Windows Update repair", "networking repair", "Winget repair", "AppX repair", "restore/recovery mutation")) and REVIEW.count("| 20 |") == 1),
]

failed = [name for name, ok in checks if not ok]
for name, ok in checks:
    print(f"{'PASS' if ok else 'FAIL'} {name}")
print(f"PHASE22_STATIC_REVIEW {len(checks) - len(failed)}/{len(checks)}")
if failed:
    print("FAILED: " + ", ".join(failed), file=sys.stderr)
    raise SystemExit(1)
if len(checks) != 20:
    raise SystemExit("Phase 22 review must contain exactly 20 lanes")
