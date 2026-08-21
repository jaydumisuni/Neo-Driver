#!/usr/bin/env python3
from pathlib import Path
import re
import sys
import tomllib

ROOT = Path(__file__).resolve().parents[1]
CRATE = ROOT / "crates" / "neo-driver-repair"
SRC = CRATE / "src"
LIB = (SRC / "lib.rs").read_text(encoding="utf-8")
MODEL = (SRC / "model.rs").read_text(encoding="utf-8")
ASSESS = (SRC / "assessment.rs").read_text(encoding="utf-8")
TESTS = (SRC / "tests.rs").read_text(encoding="utf-8")
PRODUCTION_SOURCES = {
    path.name: path.read_text(encoding="utf-8")
    for path in sorted(SRC.glob("*.rs"))
    if path.name != "tests.rs"
}
PRODUCTION = "\n".join(PRODUCTION_SOURCES.values())
MANIFEST = (CRATE / "Cargo.toml").read_text(encoding="utf-8")
DRIVER_HOST = (ROOT / "crates" / "neo-driverstore" / "src" / "host.rs").read_text(
    encoding="utf-8"
)
WORKSPACE_RAW = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
WORKSPACE = tomllib.loads(WORKSPACE_RAW)
MASTER = (ROOT / "docs" / "NEO_DRIVER_MASTER_PLAN.md").read_text(encoding="utf-8")
DECISION = (
    ROOT / "docs" / "decisions" / "0022-PHASE22-DRIVER-PNP-REPAIR-ASSESSMENT.md"
).read_text(encoding="utf-8")
REVIEW = (ROOT / "docs" / "PHASE22_20_LANE_REVIEW.md").read_text(encoding="utf-8")
CLI = (ROOT / "crates" / "neo-cli" / "src" / "repair_cli.rs").read_text(
    encoding="utf-8"
)
CLI_MANIFEST = (ROOT / "crates" / "neo-cli" / "Cargo.toml").read_text(
    encoding="utf-8"
)
CI = (ROOT / ".github" / "workflows" / "ci.yml").read_text(encoding="utf-8")
FIXTURE = (ROOT / "fixtures" / "repair" / "phase22_driver_evidence.json").read_text(
    encoding="utf-8"
)

members = set(WORKSPACE["workspace"]["members"])
READ_ONLY_DRIVER_HOST_METHODS = {
    "windows_build",
    "inventory",
    "compatible_present_devices",
    "verify_inf_signature",
    "find_equivalent_package",
    "resolve_published_package",
}
ALLOWED_PHASE22_HOST_CALLS = {"inventory", "resolve_published_package"}
FORBIDDEN_WINDOWS_MUTATION_TOKENS = (
    "DiInstallDevice",
    "SetupCopyOEMInf",
    "SetupUninstallOEMInf",
    "UpdateDriverForPlugAndPlayDevices",
    "CM_Reenumerate_DevNode",
    "CM_Enable_DevNode",
    "CM_Disable_DevNode",
    "SetupDiCallClassInstaller",
    "DIF_PROPERTYCHANGE",
    "DIF_REGISTERDEVICE",
    "DIF_REMOVE",
    "DIF_INSTALLDEVICE",
    "pnputil",
    "devcon",
    "Command::new",
)


def has_all(text, values):
    return all(value in text for value in values)


def extract_braced(text, brace_index):
    if brace_index < 0 or brace_index >= len(text) or text[brace_index] != "{":
        return None
    depth = 0
    for index in range(brace_index, len(text)):
        char = text[index]
        if char == "{":
            depth += 1
        elif char == "}":
            depth -= 1
            if depth == 0:
                return text[brace_index + 1 : index]
    return None


def extract_block_after(text, pattern):
    match = re.search(pattern, text, re.MULTILINE)
    if not match:
        return None
    brace_index = text.find("{", match.end())
    return extract_braced(text, brace_index)


def extract_function(text, name):
    match = re.search(rf"\bfn\s+{re.escape(name)}\s*\(", text)
    if not match:
        return None
    brace_index = text.find("{", match.end())
    return extract_braced(text, brace_index)


def trait_methods(text):
    block = extract_block_after(text, r"pub\s+trait\s+DriverHost\s*")
    if block is None:
        return set()
    return set(re.findall(r"^\s*fn\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(", block, re.MULTILINE))


def host_calls(text):
    return set(re.findall(r"\bhost\s*\.\s*([A-Za-z_][A-Za-z0-9_]*)\s*\(", text))


def parse_ci_steps(text):
    steps = []
    current = None
    for raw in text.splitlines():
        name_match = re.match(r"^\s+-\s+name:\s*(.+?)\s*$", raw)
        if name_match:
            if current is not None:
                steps.append(current)
            current = {"name": name_match.group(1), "run": None, "if": None, "timeout": None}
            continue
        if current is None:
            continue
        stripped = raw.strip()
        if stripped.startswith("run: "):
            current["run"] = stripped[len("run: ") :].strip()
        elif stripped.startswith("if: "):
            current["if"] = stripped[len("if: ") :].strip()
        elif stripped.startswith("timeout-minutes: "):
            current["timeout"] = stripped[len("timeout-minutes: ") :].strip()
    if current is not None:
        steps.append(current)
    return steps


def exact_ci_step(steps, name, command, *, condition=None, timeout=None):
    matches = [step for step in steps if step["name"] == name]
    if len(matches) != 1:
        return False
    step = matches[0]
    return (
        step["run"] == command
        and step["if"] == condition
        and step["timeout"] == timeout
    )


host_method_names = trait_methods(DRIVER_HOST)
forbidden_host_mutators = host_method_names - READ_ONLY_DRIVER_HOST_METHODS
capture_body = extract_function(ASSESS, "capture_and_assess_with_host")
capture_host_calls = host_calls(capture_body or "")
production_host_calls = host_calls(PRODUCTION)
production_mutator_calls = {
    method
    for method in forbidden_host_mutators
    if re.search(rf"\.\s*{re.escape(method)}\s*\(", PRODUCTION)
}
production_windows_mutation_hits = {
    token
    for token in FORBIDDEN_WINDOWS_MUTATION_TOKENS
    if token.casefold() in PRODUCTION.casefold()
}

read_only_impl = extract_block_after(TESTS, r"impl\s+DriverHost\s+for\s+ReadOnlyHost\s*")
read_only_impl_methods = (
    set(re.findall(r"^\s*fn\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(", read_only_impl, re.MULTILINE))
    if read_only_impl is not None
    else set()
)
mutators_panic = read_only_impl is not None and all(
    (body := extract_function(read_only_impl, method)) is not None and "panic!(" in body
    for method in forbidden_host_mutators
)
live_problem_test = extract_function(
    TESTS, "live_adapter_problem_path_invokes_only_inventory_and_exact_package_resolution"
)
live_healthy_test = extract_function(
    TESTS, "live_adapter_maps_phase5_none_to_no_problem_and_uses_only_read_authority"
)
structural_read_only_adapter = (
    len(forbidden_host_mutators) == 4
    and read_only_impl_methods == host_method_names
    and mutators_panic
    and capture_host_calls == ALLOWED_PHASE22_HOST_CALLS
    and live_problem_test is not None
    and "capture_and_assess_with_host(&host)" in live_problem_test
    and "CurrentExactDriverReinstallCandidate" in live_problem_test
    and "machine_changes" in live_problem_test
    and live_healthy_test is not None
    and "capture_and_assess_with_host(&host)" in live_healthy_test
    and "PnpStatusEvidence::NoProblem" in live_healthy_test
)

ci_steps = parse_ci_steps(CI)
phase22_ci_exact = all(
    (
        exact_ci_step(
            ci_steps,
            "Phase 22 twenty-lane static review",
            "python -W error tools/phase22_static_review.py",
        ),
        exact_ci_step(
            ci_steps,
            "Phase 22 Driver Store / PnP assessment proof",
            "cargo test --locked -p neo-driver-repair",
        ),
        exact_ci_step(
            ci_steps,
            "Phase 22 live Windows driver repair source proof",
            "cargo run --locked -p neo-cli -- repair drivers --json",
            condition="runner.os == 'Windows'",
            timeout="20",
        ),
        exact_ci_step(
            ci_steps,
            "Phase 22 driver repair fixture proof",
            "cargo run --locked -p neo-cli -- repair drivers --evidence fixtures/repair/phase22_driver_evidence.json --json",
        ),
    )
)

checks = [
    (
        "01-master-plan-continuity",
        has_all(
            MASTER,
            ("Driver Store/PnP repair;", "device re-enumeration;", "Windows Update reset/repair;"),
        ),
    ),
    (
        "02-exact-authority-recorded",
        has_all(
            DECISION,
            (
                "5e791fd6509a818b8f6632d57e1c74ffbc258461",
                "neo-phase22-scope-tenfold-workspace",
                "four deterministic authority evidence packets",
            ),
        ),
    ),
    (
        "03-separate-crate-boundary",
        "crates/neo-driver-repair" in members
        and 'name = "neo-driver-repair"' in MANIFEST
        and 'neo-driverstore = { path = "../neo-driverstore" }' in MANIFEST,
    ),
    (
        "04-read-only-host-seam",
        len(forbidden_host_mutators) == 4
        and capture_host_calls == ALLOWED_PHASE22_HOST_CALLS
        and production_host_calls <= ALLOWED_PHASE22_HOST_CALLS,
    ),
    (
        "05-no-mutation-call-path",
        not production_mutator_calls
        and not production_windows_mutation_hits
        and "machine_changes: false" in ASSESS,
    ),
    (
        "06-exact-device-identity",
        "to_ascii_lowercase()" in MODEL
        and "DriverRepairError::DuplicateDevice" in MODEL
        and "duplicate_instance_ids_are_case_insensitive" in TESTS,
    ),
    (
        "07-package-requires-binding",
        "DriverRepairError::PackageWithoutBinding" in MODEL
        and "package_without_active_binding_is_rejected" in TESTS,
    ),
    (
        "08-package-identity-equality",
        "eq_ignore_ascii_case(published)" in MODEL
        and "DriverRepairError::PackageMismatch" in MODEL
        and "mismatched_driver_store_identity_is_rejected" in TESTS,
    ),
    (
        "09-phase5-pnp-semantics",
        has_all(
            MODEL,
            (
                "PnpStatusEvidence",
                "None => Ok(Self::NoProblem)",
                "Some(0)",
                "does not match the inherited Phase 5 problem-code evidence",
            ),
        )
        and has_all(
            TESTS,
            (
                "problem_code_zero_is_rejected_as_noncanonical_phase5_evidence",
                "explicit_pnp_status_must_match_device_problem_evidence",
            ),
        ),
    ),
    (
        "10-healthy-needs-no-problem-exact-package",
        "PnpStatusEvidence::NoProblem" in ASSESS
        and "DriverRepairState::Healthy" in ASSESS
        and "evidence.current_package.is_none()" in ASSESS
        and "healthy_exact_binding_requires_no_action" in TESTS,
    ),
    (
        "11-reinstall-is-candidate-only",
        "CurrentExactDriverReinstallCandidate" in MODEL
        and "future authority phase" in ASSESS
        and "only_a_reinstall_candidate" in TESTS,
    ),
    (
        "12-selection-only-for-real-problem",
        "PnpStatusEvidence::Problem { code } if !binding_present" in ASSESS
        and "PnpStatusEvidence::NoProblem if !binding_present" in ASSESS
        and "no_problem_without_binding_does_not_invent_driver_selection_need" in TESTS,
    ),
    (
        "13-disabled-code22-remains-read-only",
        "CM_PROB_DISABLED_CODE: u32 = 22" in MODEL
        and "DriverRepairState::Disabled" in ASSESS
        and not any(
            token.casefold() in PRODUCTION.casefold()
            for token in ("CM_Reenumerate_DevNode", "CM_Enable_DevNode", "CM_Disable_DevNode")
        )
        and has_all(
            TESTS,
            (
                "cm_prob_disabled_is_authoritative_when_generic_disabled_field_is_unavailable",
                "contradictory_disabled_evidence_fails_closed",
                "disabled_device_is_recorded_without_enable_authority",
            ),
        ),
    ),
    (
        "14-filters-are-evidence-only",
        has_all(MODEL, ("upper_filters", "lower_filters"))
        and "filters_are_retained_as_evidence_not_inferred_as_fault" in TESTS,
    ),
    (
        "15-deterministic-order-and-digest",
        "evidence.devices.sort_by" in ASSESS
        and "source_evidence_sha256" in MODEL
        and "output_order_and_digest_are_independent_of_inventory_order" in TESTS,
    ),
    (
        "16-machine-change-false",
        "pub machine_changes: bool" in MODEL
        and "machine_changes: false" in ASSESS
        and "machine_changes = false" in DECISION,
    ),
    (
        "17-read-only-cli-surface",
        has_all(
            CLI,
            (
                "RepairCommand::Drivers",
                "inspect_windows_driver_repair",
                "DriverRepairEvidence::from_json_str",
                "Machine changes: none",
            ),
        )
        and "neo-driver-repair" in CLI_MANIFEST
        and '"pnp_status"' in FIXTURE
        and '"state": "no_problem"' in FIXTURE,
    ),
    ("18-adversarial-write-method-proof", structural_read_only_adapter),
    ("19-ci-proof-binding", phase22_ci_exact),
    (
        "20-deferred-scope-remains-closed",
        has_all(
            DECISION,
            (
                "device re-enumeration execution",
                "device enable/disable execution",
                "driver staging or installation",
                "Driver Store package deletion",
                "Windows Update repair",
                "networking repair",
                "Winget repair",
                "AppX repair",
                "restore/recovery mutation",
            ),
        )
        and REVIEW.count("| 20 |") == 1,
    ),
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
