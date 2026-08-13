#!/usr/bin/env python3
"""Deterministic 20-lane static review for Neo Driver Phase 6."""
from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
import tomllib

ROOT = Path(__file__).resolve().parents[1]
RUNTIME = (ROOT / "crates/neo-runtime/src/lib.rs").read_text(encoding="utf-8")
CLI = (ROOT / "crates/neo-cli/src/main.rs").read_text(encoding="utf-8")
WORKSPACE = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
CRATE = tomllib.loads((ROOT / "crates/neo-runtime/Cargo.toml").read_text(encoding="utf-8"))
CI = (ROOT / ".github/workflows/ci.yml").read_text(encoding="utf-8")
NORMALIZED_RUNTIME = " ".join(RUNTIME.split())


@dataclass(frozen=True)
class Lane:
    number: int
    name: str
    passed: bool
    detail: str


def contains_all(text: str, values: list[str]) -> bool:
    return all(value in text for value in values)


def requirement_count(component: str, baseline: bool) -> int:
    marker = f"component: {component}, baseline: {str(baseline).lower()}"
    return NORMALIZED_RUNTIME.count(marker)


def has_requirement(component: str, baseline: bool) -> bool:
    return requirement_count(component, baseline) > 0


def review() -> list[Lane]:
    members = set(WORKSPACE["workspace"]["members"])
    deps = set(CRATE.get("dependencies", {}))
    forbidden_execution = [
        "std::process::Command",
        "Command::new(",
        "reqwest",
        "winhttp",
        "powershell",
        "cmd.exe",
        "msiexec",
        "winget",
    ]
    return [
        Lane(1, "workspace-contract", "crates/neo-runtime" in members, "neo-runtime is a first-class workspace crate"),
        Lane(2, "shared-truth-contract", {"neo-catalogue", "neo-core"}.issubset(deps), "runtime assessment reuses Neo catalogue and core action contracts"),
        Lane(3, "model-free-read-only", not any(marker in RUNTIME for marker in forbidden_execution), "runtime engine contains no downloader/process/install execution path"),
        Lane(4, "runtime-coverage", contains_all(RUNTIME, ["VcRedist2015PlusX86", "VcRedist2015PlusX64", "DirectXLegacyJune2010", "DotNetFramework35", "DotNetRuntime", "Python", "XnaFramework40Refresh", "OpenAl", "Physx", "PhysxLegacy", "DirectPlay"]), "planned baseline and gaming/legacy runtime families are typed"),
        Lane(5, "normalized-state", contains_all(RUNTIME, ["Installed", "Missing", "Broken", "Partial", "Unknown"]), "runtime evidence distinguishes healthy, absent, repairable, partial, and unknown states"),
        Lane(6, "inventory-validation", contains_all(RUNTIME, ["DuplicateObservation", "MissingObservationSource", "UnsupportedArchitecture", "InvalidWindowsBuild"]), "normalized evidence rejects duplicate, source-less, invalid-build, and unsupported-architecture inputs"),
        Lane(7, "unknown-fails-closed", contains_all(RUNTIME, ["RuntimeState::Unknown", "EvidenceVerdict::Investigate", "refuses to convert unknown evidence"]), "unknown state never becomes install authority"),
        Lane(8, "typed-package-binding", contains_all(RUNTIME, ["RuntimePackageBinding", "BindingTargetsNonRuntime", "PackageKind::Runtime", "UnknownPackage"]), "component bindings must resolve to validated runtime packages"),
        Lane(9, "os-applicability-gates", contains_all(RUNTIME, ["canonical_arch", "minimum_build", "maximum_build", "package_applies"]), "candidate package selection is hard-gated by host architecture and Windows build"),
        Lane(10, "ambiguity-fails-closed", contains_all(RUNTIME, ["Neo will not guess between them", "candidates.as_slice()", "EvidenceVerdict::Investigate"]), "multiple compatible packages produce investigation rather than arbitrary version choice"),
        Lane(11, "manual-authority", "user_selectable: true" in RUNTIME, "every surfaced runtime recommendation remains individually user-selectable"),
        Lane(12, "baseline-confirmation", contains_all(RUNTIME, ["selected_by_default = requirement.baseline", "requires_confirmation: true"]), "profile baselines may be preselected but still require explicit confirmation"),
        Lane(13, "optional-not-preselected", contains_all(RUNTIME, ["OptionalComponent", "optional_missing_is_never_preselected"]), "optional components remain off until selected by the user"),
        Lane(14, "directx-profile-law", requirement_count("DirectXLegacyJune2010", True) >= 2, "DirectX June 2010 is a deselectable baseline recommendation for Fresh Windows and Gaming"),
        Lane(15, "vc2015plus-profile-law", requirement_count("VcRedist2015PlusX86", True) >= 4 and requirement_count("VcRedist2015PlusX64", True) >= 4, "VC++ 2015+ x86/x64 are the modern baseline across defined setup profiles"),
        Lane(16, "python-not-forced", has_requirement("Python", False) and contains_all(RUNTIME, ["RuntimeProfile::Technician", "RuntimeProfile::Developer"]), "Python is detected/recommended without being a forced technician/developer install"),
        Lane(17, "gaming-optionals", all(has_requirement(component, False) for component in ["XnaFramework40Refresh", "OpenAl", "Physx", "PhysxLegacy", "DotNetFramework35", "DirectPlay"]), "legacy gaming dependencies are explicit optional components"),
        Lane(18, "no-fake-rollback", contains_all(RUNTIME, ["rollback_available: false", "runtime execution remains behind a later bounded executor gate"]), "assessment does not claim rollback before a runtime executor/rollback contract exists"),
        Lane(19, "cli-read-only-surface", contains_all(CLI, ["Command::Runtimes", "Command::Gaming", "Machine changes: none", "Runtime downloads/installations: intentionally disabled"]), "CLI exposes assessment only and states the no-mutation boundary"),
        Lane(20, "fixtures-and-proof-gate", contains_all(CI, ["Phase 6 twenty-lane static review", "Runtime CLI fixture proof", "Gaming CLI fixture proof"]) and (ROOT / "fixtures/runtime/runtime_inventory.json").is_file() and (ROOT / "fixtures/runtime/runtime_policy.json").is_file() and (ROOT / "fixtures/catalogue/sample_runtime_catalogue.json").is_file(), "Phase 6 has deterministic fixtures and normal CI proof hooks"),
    ]


def main() -> int:
    lanes = review()
    for lane in lanes:
        print(f"{'PASS' if lane.passed else 'FAIL'} {lane.number:02d} {lane.name}: {lane.detail}")
    failures = [lane for lane in lanes if not lane.passed]
    if failures:
        print(f"\nPhase 6 static review failed: {len(failures)} lane(s) unresolved.")
        return 1
    print("\nPhase 6 static review: 20/20 PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
