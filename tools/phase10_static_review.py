#!/usr/bin/env python3
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
STATE = (ROOT / "crates" / "neo-state-plan" / "src" / "resolver.rs").read_text(encoding="utf-8")
ERRORS = (ROOT / "crates" / "neo-state-plan" / "src" / "error.rs").read_text(encoding="utf-8")
LIVE = (ROOT / "crates" / "neo-cli" / "src" / "state_readback_windows.rs").read_text(encoding="utf-8")
CLI = (ROOT / "crates" / "neo-cli" / "src" / "state_assess_v2.rs").read_text(encoding="utf-8")
PHASE9_CLI = (ROOT / "crates" / "neo-cli" / "src" / "state_assess_cli.rs").read_text(encoding="utf-8")
LIVE_TEST = (ROOT / "crates" / "neo-cli" / "tests" / "state_live_read_only.rs").read_text(encoding="utf-8")
CI = (ROOT / ".github" / "workflows" / "ci.yml").read_text(encoding="utf-8")
WORKSPACE = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
LOCK = (ROOT / "Cargo.lock").read_text(encoding="utf-8")

production = "\n".join([STATE, ERRORS, LIVE, CLI])
forbidden_mutation = [
    "neo_transaction",
    "RuntimeExecutionSession",
    "WindowsRuntimeHost",
    "DiInstallDriver",
    "SetupCopyOEMInf",
    "RegSetValue",
    "Set-Service",
    "Enable-WindowsOptionalFeature",
    "Disable-WindowsOptionalFeature",
    "Remove-AppxPackage",
]

checks = [
    ("phase9-crate-preserved", '"crates/neo-state-plan"' in WORKSPACE and '"crates/neo-state-resolver"' not in WORKSPACE),
    ("cargo-lock-unchanged-domain", 'name = "neo-state-resolver"' not in LOCK),
    ("opaque-reader-id", "struct ReaderId" in STATE and "InvalidReaderId" in ERRORS and "as_str" in STATE),
    ("canonical-target-binding", "struct StateBinding" in STATE and "canonical_key" in STATE),
    ("binding-root-validation", 'serde(try_from = "StateBindingsWire")' in STATE and "DuplicateBinding" in ERRORS),
    ("captured-state-root-validation", 'serde(try_from = "CapturedStatesWire")' in STATE and "DuplicateCapturedState" in ERRORS),
    ("captured-provenance", "struct CapturedState" in STATE and "pub source: String" in STATE),
    ("selection-hard-gates", "EmptySelection" in STATE and "DuplicateSelection" in STATE and "UnknownTweak" in STATE),
    ("missing-binding-hard-gate", "MissingBinding" in STATE and "bindings.find" in STATE),
    ("missing-capture-unavailable", 'reason: "state was not captured"' in STATE and "ObservedState::Unavailable" in STATE),
    ("phase9-assessment-reuse", "resolve_selected_evidence" in CLI and "assess_tweaks" in CLI),
    ("proven-system-xray-reuse", "scan_current_machine" in LIVE and "CommandRunner" not in LIVE and "std::process::Command" not in LIVE),
    ("fixed-reader-catalogue", "windows.os.current_build" in LIVE and "windows.security.test_signing" in LIVE and "windows.security.memory_integrity" in LIVE),
    ("unknown-reader-unavailable", "reader is not registered" in LIVE and "ObservedState::Unavailable" in LIVE),
    ("no-configurable-command-surface", all(marker not in LIVE for marker in ["reg.exe", "sc.exe", "dism.exe", "powershell.exe", "program:", "args:"])),
    ("read-only-live-cli", "Command::Live" in CLI and "Machine changes: none" in CLI),
    ("phase9-cli-preserved", "Command::Live" not in PHASE9_CLI and "Machine changes: none" in PHASE9_CLI),
    ("no-mutation-authority", all(marker not in production for marker in forbidden_mutation)),
    ("explicit-windows-live-proof", "live_state_assessment_reads_proven_system_evidence_without_mutation" in LIVE_TEST and "Machine changes: none" in LIVE_TEST),
    ("ci-proof-chain", "phase9_static_review.py" in CI and "phase10_static_review.py" in CI and "state_live_read_only" in CI),
]

failed = []
for index, (name, ok) in enumerate(checks, 1):
    print(f"{index:02d}. {'PASS' if ok else 'FAIL'} - {name}")
    if not ok:
        failed.append(name)
if failed:
    raise SystemExit("Phase 10 static review failed: " + ", ".join(failed))
print("PHASE 10 STATIC REVIEW PASS: 20/20")
