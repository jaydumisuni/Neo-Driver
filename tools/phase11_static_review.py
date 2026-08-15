#!/usr/bin/env python3
from __future__ import annotations

import re
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
WORKSPACE = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
MANIFEST = tomllib.loads((ROOT / "crates/neo-tweak-executor/Cargo.toml").read_text(encoding="utf-8"))
MODEL = (ROOT / "crates/neo-tweak-executor/src/model.rs").read_text(encoding="utf-8")
ENGINE = (ROOT / "crates/neo-tweak-executor/src/engine.rs").read_text(encoding="utf-8")
SESSION = (ROOT / "crates/neo-tweak-executor/src/session.rs").read_text(encoding="utf-8")
WINDOWS = (ROOT / "crates/neo-tweak-executor/src/windows.rs").read_text(encoding="utf-8")
LIB = (ROOT / "crates/neo-tweak-executor/src/lib.rs").read_text(encoding="utf-8")
TESTS = (ROOT / "crates/neo-tweak-executor/src/tests.rs").read_text(encoding="utf-8")
REVIEW_TESTS = (ROOT / "crates/neo-tweak-executor/src/review_tests.rs").read_text(encoding="utf-8")
DECISION = (ROOT / "docs/decisions/0011-PHASE11-TWEAK-EXECUTOR.md").read_text(encoding="utf-8")
CLI_MANIFEST = tomllib.loads((ROOT / "crates/neo-cli/Cargo.toml").read_text(encoding="utf-8"))

EXPECTED_IDS = {
    "windows.explorer.show_file_extensions",
    "windows.explorer.show_hidden_files",
    "windows.taskbar.centered_icons",
}
EXPECTED_VALUES = {"HideFileExt", "Hidden", "TaskbarAl"}
EXPECTED_DONOR_PATH = "Software\\\\Microsoft\\\\Windows\\\\CurrentVersion\\\\Explorer\\\\Advanced"
EXPECTED_FORWARD_VALUES = {
    "SHOW_FILE_EXTENSIONS": 0,
    "SHOW_HIDDEN_FILES": 1,
    "TASKBAR_CENTERED_ICONS": 1,
}


def test_functions(text: str) -> set[str]:
    return set(re.findall(r"(?m)^\s*#\[test\]\s*\n\s*fn\s+([A-Za-z0-9_]+)\s*\(", text))


def curated_forward_values_are_exact(text: str) -> bool:
    for constant, value in EXPECTED_FORWARD_VALUES.items():
        pattern = rf"{constant}\s*=>\s*Some\(RegistryTweakSpec\s*\{{.*?desired_dword:\s*{value},"
        if re.search(pattern, text, re.S) is None:
            return False
    return "*value == spec.desired_dword" in text and "step.desired_dword != spec.desired_dword" in text


ids = set(re.findall(r'"(windows\.(?:explorer|taskbar)\.[a-z0-9_.-]+)"', MODEL))
value_names = set(re.findall(r'value_name:\s*"([A-Za-z0-9_]+)"', MODEL))
public_surface = LIB + MODEL
production = "\n".join([MODEL, ENGINE, SESSION, WINDOWS, LIB])
regressions = test_functions(TESTS + "\n" + REVIEW_TESTS)
windows_dependencies = MANIFEST.get("target", {}).get("cfg(windows)", {}).get("dependencies", {})
normal_dependencies = MANIFEST.get("dependencies", {})
cli_dependencies = CLI_MANIFEST.get("dependencies", {})

required_regressions = {
    "unsupported_tweak_fails_closed",
    "non_dword_or_out_of_range_operation_fails_closed",
    "contradictory_curated_semantics_fail_closed",
    "satisfied_selection_does_not_create_mutation_transaction",
    "prepare_captures_exact_present_baseline",
    "prepare_captures_absent_baseline",
    "baseline_drift_blocks_authority",
    "baseline_drift_after_authority_blocks_apply_before_write",
    "successful_apply_requires_fresh_verification",
    "wrong_post_write_state_rolls_back_exact_present_baseline",
    "failed_write_after_change_rolls_back_absent_baseline",
    "rollback_attempts_all_changed_tweaks_after_restore_failure",
    "multiple_curated_tweaks_complete_in_one_transaction",
    "capability_has_no_public_constructor_but_internal_tests_can_issue_it",
}

decision_markers = {
    "it does not expose a public tweak-apply CLI or GUI surface",
    "actual value captured immediately before authorization",
    "No GitHub runner Registry value is modified by Phase 11 tests",
    "require an opaque `TweakExecutorCapability` with no public constructor",
    "Local\\\\THETECHGUY.NeoDriver.TweakExecutor.v1",
    "same-session authority only",
    "exact approved forward DWORD",
}

checks = [
    ("workspace-boundary", "crates/neo-tweak-executor" in set(WORKSPACE["workspace"]["members"])),
    ("windows-dependency-is-target-only", "windows" in windows_dependencies and "windows" not in normal_dependencies),
    ("exact-three-curated-ids", ids == EXPECTED_IDS),
    ("exact-three-curated-value-names", value_names == EXPECTED_VALUES and EXPECTED_DONOR_PATH in MODEL),
    ("exact-curated-forward-value-gate", curated_forward_values_are_exact(MODEL) and "UnsupportedOperation" in MODEL),
    ("certified-evidence-gate", "definition.verdict != EvidenceVerdict::Certified" in MODEL and "NonCertifiedTweak" in MODEL),
    ("no-public-registry-spec", "pub(crate) struct RegistryTweakSpec" in MODEL and "pub struct RegistryTweakSpec" not in public_surface),
    ("opaque-capability", "pub struct TweakExecutorCapability" in MODEL and "pub(crate) fn for_tests" in MODEL and "pub fn new" not in MODEL.split("pub struct TweakExecutorCapability", 1)[1]),
    ("raw-host-private", "pub(crate) trait TweakHost" in ENGINE and "pub(crate) struct WindowsRegistryHost" in WINDOWS and "pub use windows" not in LIB),
    ("phase4-transaction-reuse", all(marker in ENGINE for marker in ["TransactionPlan::new", "TransactionCheckpoint::new", "RollbackPlan::Reversible", "MatchesBaseline"])),
    ("actual-baseline-capture", "host.read(spec)?" in ENGINE and "checkpoint.capture_baseline" in ENGINE),
    ("preapply-drift-and-same-session-serialization", SESSION.count("ensure_baseline_unchanged") >= 3 and "BaselineDrift" in SESSION and all(marker in WINDOWS for marker in ["TWEAK_MUTEX_NAME", "TWEAK_MUTEX_TIMEOUT_MS", "CreateMutexW", "WaitForSingleObject", "ReleaseMutex", "mutex_acquires_without_registry_mutation"]) and "TweakExecutionMutex::acquire()?" in LIB),
    ("api-result-separate-from-change", "machine_changed" in SESSION and "ApplyOutcome::Success" in SESSION and "ApplyOutcome::Failure" in SESSION),
    ("fresh-postwrite-verification", "verify_postconditions" in SESSION and "observe_steps" in SESSION),
    ("complete-exact-baseline-rollback", "host.restore(spec, step.baseline())" in SESSION and "record_rollback_results_batch" in SESSION and "verify_rollback" in SESSION and "rollback_attempts_all_changed_tweaks_after_restore_failure" in regressions),
    ("unsupported-registry-type-fails", all(marker in WINDOWS for marker in ["ERROR_MORE_DATA", "value_type != REG_DWORD || size != 4", "UnsupportedRegistryState"])),
    ("direct-windows-registry-no-shell", all(marker in WINDOWS for marker in ["RegOpenKeyExW", "RegQueryValueExW", "RegSetValueExW", "RegDeleteValueW"]) and all(marker not in production for marker in ["Command::new", "powershell", "reg.exe", "cmd.exe"])),
    ("no-public-cli-mutation", "neo-tweak-executor" not in cli_dependencies and "TweakExecutorCapability" not in (ROOT / "crates/neo-cli/src/main.rs").read_text(encoding="utf-8")),
    ("adversarial-regressions", required_regressions.issubset(regressions)),
    ("decision-boundary", all(marker in DECISION for marker in decision_markers)),
]

if len(checks) != 20 or len({name for name, _ in checks}) != 20:
    raise SystemExit("Phase 11 review definition must contain exactly 20 unique lanes")

failed = []
for index, (name, passed) in enumerate(checks, 1):
    print(f"{index:02d}. {'PASS' if passed else 'FAIL'} - {name}")
    if not passed:
        failed.append(name)
if failed:
    raise SystemExit("Phase 11 static review failed: " + ", ".join(failed))
print("PHASE 11 STATIC REVIEW PASS: 20/20")
