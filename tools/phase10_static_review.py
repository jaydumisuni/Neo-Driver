#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import re
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
STATE = (ROOT / "crates/neo-state-plan/src/resolver.rs").read_text(encoding="utf-8")
ERRORS = (ROOT / "crates/neo-state-plan/src/error.rs").read_text(encoding="utf-8")
LIVE = (ROOT / "crates/neo-cli/src/state_readback_windows.rs").read_text(encoding="utf-8")
CLI = (ROOT / "crates/neo-cli/src/state_assess_v2.rs").read_text(encoding="utf-8")
PHASE9_CLI_PATH = ROOT / "crates/neo-cli/src/state_assess_cli.rs"
PHASE9_CLI = PHASE9_CLI_PATH.read_text(encoding="utf-8")
RESOLVER_TESTS = (ROOT / "crates/neo-state-plan/tests/resolver.rs").read_text(encoding="utf-8")
LIVE_TEST = (ROOT / "crates/neo-cli/tests/state_live_read_only.rs").read_text(encoding="utf-8")
CI = (ROOT / ".github/workflows/ci.yml").read_text(encoding="utf-8")
DECISION = (ROOT / "docs/decisions/0010-PHASE10-WINDOWS-STATE-RESOLUTION.md").read_text(encoding="utf-8")
WORKSPACE = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
LOCK = (ROOT / "Cargo.lock").read_text(encoding="utf-8")

EXPECTED_READERS = {
    "windows.os.product_name",
    "windows.os.display_version",
    "windows.os.current_build",
    "windows.os.architecture",
    "windows.security.test_signing",
    "windows.security.no_integrity_checks",
    "windows.security.secure_boot",
    "windows.security.memory_integrity",
    "windows.security.pending_reboot",
}
PHASE9_CLI_BLOB_SHA = "4a9fb689bee89b8214105ac88086c6125fc54354"


def balanced_block(text: str, anchor: str, *, start_after: str | None = None) -> str:
    anchor_pos = text.index(anchor)
    search_from = anchor_pos
    if start_after is not None:
        search_from = text.index(start_after, anchor_pos) + len(start_after) - 1
    brace = text.index("{", search_from)
    depth = 0
    for index in range(brace, len(text)):
        char = text[index]
        if char == "{":
            depth += 1
        elif char == "}":
            depth -= 1
            if depth == 0:
                return text[brace : index + 1]
    raise ValueError(f"unbalanced Rust block for {anchor!r}")


def git_blob_sha(text: str) -> str:
    data = text.encode("utf-8")
    framed = b"blob " + str(len(data)).encode("ascii") + b"\0" + data
    return hashlib.sha1(framed).hexdigest()


def test_functions(text: str) -> set[str]:
    return set(
        re.findall(
            r"(?m)^\s*#\[test\]\s*\n\s*fn\s+([A-Za-z0-9_]+)\s*\(",
            text,
        )
    )


def workflow_steps(text: str) -> dict[str, dict[str, str]]:
    lines = text.splitlines()
    steps: dict[str, dict[str, str]] = {}
    index = 0
    while index < len(lines):
        line = lines[index]
        stripped = line.strip()
        if not stripped.startswith("- name: "):
            index += 1
            continue
        indent = len(line) - len(line.lstrip())
        name = stripped[len("- name: ") :].strip()
        block: list[str] = []
        index += 1
        while index < len(lines):
            candidate = lines[index]
            candidate_stripped = candidate.strip()
            candidate_indent = len(candidate) - len(candidate.lstrip())
            if candidate_indent == indent and candidate_stripped.startswith("- name: "):
                break
            block.append(candidate)
            index += 1
        props: dict[str, str] = {}
        cursor = 0
        while cursor < len(block):
            raw = block[cursor]
            value = raw.strip()
            if value.startswith("if: "):
                props["if"] = value[len("if: ") :].strip()
            elif value.startswith("run: "):
                run_value = value[len("run: ") :].strip()
                if run_value == "|":
                    run_lines: list[str] = []
                    cursor += 1
                    while cursor < len(block):
                        next_value = block[cursor].strip()
                        if re.match(r"^(if|uses|with|env|shell):", next_value):
                            cursor -= 1
                            break
                        if next_value:
                            run_lines.append(next_value)
                        cursor += 1
                    props["run"] = "\n".join(run_lines)
                else:
                    props["run"] = run_value
            cursor += 1
        steps[name] = props
    return steps


reader_deserialize = balanced_block(STATE, "impl<'de> Deserialize<'de> for ReaderId")
state_bindings_impl = balanced_block(STATE, "impl StateBindings")
captured_impl = balanced_block(STATE, "impl CapturedStates")
resolve_body = balanced_block(STATE, "pub fn resolve_selected_evidence")
capture_body = balanced_block(LIVE, "pub fn capture_live")
reader_match = balanced_block(LIVE, "match reader.as_str()")
reader_arms = set(re.findall(r'(?m)^\s*"(windows\.[a-z0-9_.-]+)"\s*=>', reader_match))
documented_readers = set(re.findall(r"`(windows\.[a-z0-9_.-]+)`", DECISION))
resolver_tests = test_functions(RESOLVER_TESTS)
live_tests = test_functions(LIVE_TEST)
steps = workflow_steps(CI)
workspace_members = set(WORKSPACE["workspace"]["members"])

forbidden_mutation = {
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
}
production = "\n".join([STATE, ERRORS, LIVE, CLI])

required_resolver_tests = {
    "reader_id_direct_deserialization_revalidates",
    "directly_constructed_duplicate_captures_fail_before_resolution",
    "directly_constructed_blank_capture_source_fails_before_resolution",
    "duplicate_bindings_fail_case_insensitively",
    "missing_capture_is_unavailable",
    "captured_state_keeps_provenance",
}

checks = [
    ("phase9-domain-preserved", "crates/neo-state-plan" in workspace_members and "crates/neo-state-resolver" not in workspace_members),
    ("cargo-lock-domain-preserved", 'name = "neo-state-resolver"' not in LOCK),
    ("reader-deserialize-validates", "Self::new(value)" in reader_deserialize and "String::deserialize" in reader_deserialize),
    ("binding-root-validates", "pub fn validate(&self)" in state_bindings_impl and "DuplicateBinding" in state_bindings_impl and "canonical_key()?" in state_bindings_impl),
    ("captured-root-validates", "pub fn validate(&self)" in captured_impl and "DuplicateCapturedState" in captured_impl and "captured state source" in captured_impl),
    ("captured-index-validates", "self.validate()?" in balanced_block(captured_impl, "fn indexed")),
    ("resolution-uses-validated-index", "captured.indexed()?" in resolve_body and "bindings.validate()?" in resolve_body),
    ("selection-hard-gates", all(marker in resolve_body for marker in ["EmptySelection", "DuplicateSelection", "UnknownTweak", "MissingBinding"])),
    ("missing-capture-unavailable", "ObservedState::Unavailable" in resolve_body and 'reason: "state was not captured"' in resolve_body),
    ("resolver-regressions-complete", required_resolver_tests.issubset(resolver_tests)),
    ("phase9-assessment-reused", "resolve_selected_evidence" in CLI and "assess_tweaks" in CLI),
    ("system-xray-reused", "scan_current_machine()" in capture_body and "std::process" not in LIVE and "Command::new" not in LIVE),
    ("fixed-reader-catalogue-exact", reader_arms == EXPECTED_READERS),
    ("reader-catalogue-documented", EXPECTED_READERS.issubset(documented_readers)),
    ("unknown-reader-fails-closed", 'reader is not registered in the Windows readback catalogue' in reader_match and "unavailable(" in reader_match),
    ("validated-read-only-cli", "StateBindings::read_json(bindings)?" in CLI and "std::fs" not in CLI and "Machine changes: none" in CLI),
    ("phase9-cli-byte-preserved", git_blob_sha(PHASE9_CLI) == PHASE9_CLI_BLOB_SHA),
    ("no-mutation-authority", forbidden_mutation.isdisjoint(production)),
    ("windows-live-behavior-wired", "live_state_assessment_reads_proven_system_evidence_without_mutation" in live_tests and steps.get("Phase 10 live Windows state proof", {}).get("run") == "cargo test --locked -p neo-cli --test state_live_read_only" and steps.get("Phase 10 live Windows state proof", {}).get("if") == "runner.os == 'Windows'"),
    ("active-ci-proof-chain", steps.get("Phase 9 twenty-lane static review", {}).get("run") == "python -W error tools/phase9_static_review.py" and steps.get("Phase 10 twenty-lane static review", {}).get("run") == "python -W error tools/phase10_static_review.py" and steps.get("Rust unit proof", {}).get("run") == "cargo test --workspace --all-targets --locked"),
]

if len(checks) != 20 or len({name for name, _ in checks}) != 20:
    raise SystemExit("Phase 10 static review definition must contain exactly 20 unique lanes")

failed = []
for index, (name, ok) in enumerate(checks, 1):
    print(f"{index:02d}. {'PASS' if ok else 'FAIL'} - {name}")
    if not ok:
        failed.append(name)
if failed:
    raise SystemExit("Phase 10 static review failed: " + ", ".join(failed))
print("PHASE 10 STATIC REVIEW PASS: 20/20")
