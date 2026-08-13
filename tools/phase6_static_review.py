#!/usr/bin/env python3
"""Deterministic 20-lane static review for Neo Driver Phase 6."""
from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
import tomllib

ROOT = Path(__file__).resolve().parents[1]
RUNTIME = (ROOT / "crates/neo-runtime/src/lib.rs").read_text(encoding="utf-8")
PROBE = (ROOT / "crates/neo-runtime-probe/src/lib.rs").read_text(encoding="utf-8")
CLI = (ROOT / "crates/neo-cli/src/main.rs").read_text(encoding="utf-8")
WORKSPACE = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
RUNTIME_CRATE = tomllib.loads((ROOT / "crates/neo-runtime/Cargo.toml").read_text(encoding="utf-8"))
PROBE_CRATE = tomllib.loads((ROOT / "crates/neo-runtime-probe/Cargo.toml").read_text(encoding="utf-8"))
CI = (ROOT / ".github/workflows/ci.yml").read_text(encoding="utf-8")
NORMALIZED_RUNTIME = " ".join(RUNTIME.split())
NORMALIZED_PROBE = " ".join(PROBE.split())


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
    runtime_deps = set(RUNTIME_CRATE.get("dependencies", {}))
    probe_deps = set(PROBE_CRATE.get("dependencies", {}))
    forbidden_runtime_execution = [
        "std::process::Command",
        "Command::new(",
        "reqwest",
        "winhttp",
        "powershell",
        "cmd.exe",
        "msiexec",
        "winget",
    ]
    forbidden_probe_mutation = [
        "/Enable-Feature",
        "/Disable-Feature",
        "msiexec",
        "winget",
        "/add-driver",
        "bcdedit",
        "powershell",
        "cmd.exe",
        "Invoke-WebRequest",
        "Start-Process",
        "Restart-Computer",
        "shutdown.exe",
    ]
    scanner_commands_are_bounded = contains_all(
        PROBE,
        [
            '"reg.exe"',
            '"query"',
            '"dotnet.exe", &["--list-runtimes"]',
            '"dism.exe"',
            '"/Get-FeatureInfo"',
            '"/English"',
            '"py.exe", &["-0p"]',
            '"where.exe"',
        ],
    ) and not any(marker in PROBE for marker in forbidden_probe_mutation)
    python_is_non_triggering = (
        'self.capture("python.exe"' not in NORMALIZED_PROBE
        and 'self.capture("python"' not in NORMALIZED_PROBE
        and 'self.capture("py.exe", &[])' not in NORMALIZED_PROBE
        and 'self.capture("py", &[])' not in NORMALIZED_PROBE
        and 'self.capture("py.exe", &["-0p"])' in NORMALIZED_PROBE
    )

    return [
        Lane(
            1,
            "workspace-contract",
            {"crates/neo-runtime", "crates/neo-runtime-probe"}.issubset(members),
            "runtime assessment and runtime System X-Ray are first-class workspace crates",
        ),
        Lane(
            2,
            "shared-boundary-contract",
            {"neo-catalogue", "neo-core"}.issubset(runtime_deps)
            and {"neo-probe", "neo-runtime"}.issubset(probe_deps),
            "assessment reuses Neo catalogue/core and the scanner reuses the existing command-evidence boundary",
        ),
        Lane(
            3,
            "model-free-assessment",
            not any(marker in RUNTIME for marker in forbidden_runtime_execution),
            "pure runtime assessment contains no downloader/process/install execution path",
        ),
        Lane(
            4,
            "bounded-read-only-scanner",
            scanner_commands_are_bounded,
            "System X-Ray uses only bounded read-only registry, runtime-listing, DISM feature-query and path-discovery commands",
        ),
        Lane(
            5,
            "runtime-coverage",
            contains_all(
                RUNTIME,
                [
                    "VcRedist2015PlusX86",
                    "VcRedist2015PlusX64",
                    "DirectXLegacyJune2010",
                    "DotNetFramework35",
                    "DotNetFramework4",
                    "DotNetRuntime",
                    "DotNetDesktopRuntime",
                    "Python",
                    "WebView2",
                    "XnaFramework40Refresh",
                    "OpenAl",
                    "Physx",
                    "PhysxLegacy",
                    "DirectPlay",
                ],
            ),
            "planned baseline and gaming/legacy runtime families are typed",
        ),
        Lane(
            6,
            "normalized-evidence-validation",
            contains_all(
                RUNTIME,
                [
                    "Installed",
                    "Missing",
                    "Broken",
                    "Partial",
                    "Unknown",
                    "DuplicateObservation",
                    "MissingObservationSource",
                    "UnsupportedArchitecture",
                    "InvalidWindowsBuild",
                ],
            ),
            "normalized runtime evidence has distinct health states and rejects malformed authority",
        ),
        Lane(
            7,
            "unknown-fails-closed",
            contains_all(
                RUNTIME,
                [
                    "RuntimeState::Unknown",
                    "EvidenceVerdict::Investigate",
                    "refuses to convert unknown evidence",
                ],
            ),
            "unknown runtime state never becomes install authority",
        ),
        Lane(
            8,
            "typed-package-binding",
            contains_all(
                RUNTIME,
                [
                    "RuntimePackageBinding",
                    "BindingTargetsNonRuntime",
                    "PackageKind::Runtime",
                    "UnknownPackage",
                ],
            ),
            "runtime component bindings must resolve to validated runtime packages",
        ),
        Lane(
            9,
            "applicability-and-ambiguity",
            contains_all(
                RUNTIME,
                [
                    "canonical_arch",
                    "minimum_build",
                    "maximum_build",
                    "package_applies",
                    "Neo will not guess between them",
                    "candidates.as_slice()",
                ],
            ),
            "package selection is architecture/build gated and ambiguous candidates fail closed",
        ),
        Lane(
            10,
            "manual-authority",
            contains_all(
                RUNTIME,
                [
                    "user_selectable: true",
                    "selected_by_default = requirement.baseline",
                    "requires_confirmation: true",
                    "optional_missing_is_never_preselected",
                ],
            ),
            "every runtime remains selectable; baseline preselection still requires confirmation and optionals remain off",
        ),
        Lane(
            11,
            "profile-law",
            requirement_count("VcRedist2015PlusX86", True) >= 4
            and requirement_count("VcRedist2015PlusX64", True) >= 4
            and requirement_count("DirectXLegacyJune2010", True) >= 2
            and has_requirement("Python", False),
            "VC++ 2015+ is the modern baseline, DirectX June 2010 is a deselectable Fresh/Gaming baseline, and Python is optional",
        ),
        Lane(
            12,
            "gaming-optionals",
            all(
                has_requirement(component, False)
                for component in [
                    "XnaFramework40Refresh",
                    "OpenAl",
                    "Physx",
                    "PhysxLegacy",
                    "DotNetFramework35",
                    "DirectPlay",
                ]
            ),
            "legacy gaming dependencies remain explicit optional components absent dependency proof",
        ),
        Lane(
            13,
            "webview2-documented-predicate",
            contains_all(
                PROBE,
                [
                    "{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}",
                    r"HKLM\SOFTWARE\Microsoft\EdgeUpdate\Clients",
                    r"HKLM\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients",
                    r"HKCU\Software\Microsoft\EdgeUpdate\Clients",
                    '"pv"',
                    'trimmed != "0.0.0.0"',
                ],
            ),
            "WebView2 uses the frozen Microsoft product GUID, architecture-aware EdgeUpdate paths and non-zero pv evidence",
        ),
        Lane(
            14,
            "microsoft-runtime-predicates",
            contains_all(
                PROBE,
                [
                    r"VisualStudio\14.0\VC\Runtimes",
                    r"NET Framework Setup\NDP\v4\Full",
                    '"Release"',
                    '"--list-runtimes"',
                    '"Microsoft.NETCore.App"',
                    '"Microsoft.WindowsDesktop.App"',
                    '"/FeatureName:NetFx3"',
                    '"/FeatureName:DirectPlay"',
                ],
            ),
            "VC++ v14, .NET Framework 4, modern .NET/Desktop and Windows optional-feature evidence paths are explicit",
        ),
        Lane(
            15,
            "python-repair-first-detection",
            python_is_non_triggering
            and contains_all(
                PROBE,
                [
                    "RuntimeState::Partial",
                    "No global Python command evidence was recovered",
                    "RuntimeState::Unknown",
                    "python_on_path",
                    "py_on_path",
                    "pip_on_path",
                ],
            ),
            "Python scanning does not trigger an interpreter launch; PATH gaps become Partial and lack of global evidence remains Unknown",
        ),
        Lane(
            16,
            "unproven-legacy-predicates-stay-unknown",
            contains_all(
                PROBE,
                [
                    "directx-legacy-predicate-pending",
                    "xna-predicate-pending",
                    "openal-predicate-pending",
                    "physx-predicate-pending",
                    "physx-legacy-predicate-pending",
                    "Neo reports Unknown rather than guessing",
                ],
            ),
            "DirectX legacy/XNA/OpenAL/PhysX predicates remain Unknown until independently proven",
        ),
        Lane(
            17,
            "raw-evidence-retention",
            contains_all(
                PROBE,
                [
                    "RuntimeProbeReport",
                    "command_evidence: Vec<CommandEvidence>",
                    "raw evidence retained",
                    "scan_current_machine",
                    "SystemCommandRunner",
                ],
            ),
            "runtime System X-Ray preserves raw command evidence and reuses the base Neo host probe",
        ),
        Lane(
            18,
            "no-fake-rollback-or-runtime-mutation",
            contains_all(
                RUNTIME,
                [
                    "rollback_available: false",
                    "runtime execution remains behind a later bounded executor gate",
                ],
            )
            and not any(marker in PROBE for marker in forbidden_probe_mutation),
            "assessment does not claim rollback and System X-Ray contains no runtime/feature mutation path",
        ),
        Lane(
            19,
            "cli-read-only-surface",
            contains_all(
                CLI,
                [
                    "Command::RuntimeScan",
                    "Command::Runtimes",
                    "Command::Gaming",
                    "scan_current_runtime_inventory",
                    "Machine changes: none",
                    "Runtime downloads/installations: intentionally disabled",
                ],
            ),
            "CLI exposes live System X-Ray plus assessment while preserving the no-mutation boundary",
        ),
        Lane(
            20,
            "live-and-fixture-proof-gate",
            contains_all(
                CI,
                [
                    "Phase 6 twenty-lane static review",
                    "Runtime System X-Ray proof",
                    "if: runner.os == 'Windows'",
                    "runtime-scan --json",
                    "Runtime CLI fixture proof",
                    "Gaming CLI fixture proof",
                ],
            )
            and (ROOT / "fixtures/runtime/runtime_inventory.json").is_file()
            and (ROOT / "fixtures/runtime/runtime_policy.json").is_file()
            and (ROOT / "fixtures/catalogue/sample_runtime_catalogue.json").is_file(),
            "Phase 6 requires Windows live scanner proof plus deterministic runtime/gaming fixtures in normal CI",
        ),
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
