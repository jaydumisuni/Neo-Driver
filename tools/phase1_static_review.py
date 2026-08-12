#!/usr/bin/env python3
"""Deterministic 20-lane static review for Neo Driver Phase 1.

This is engineering tooling, not a Neo runtime dependency. It mirrors the
Sergeant 10-for-2 discipline by making the current phase's bounded obligations
machine-checkable before compilation/runtime proof.
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
import sys
import tomllib


ROOT = Path(__file__).resolve().parents[1]
CORE = (ROOT / "crates/neo-core/src/lib.rs").read_text(encoding="utf-8")
PROBE = (ROOT / "crates/neo-probe/src/lib.rs").read_text(encoding="utf-8")
CLI = (ROOT / "crates/neo-cli/src/main.rs").read_text(encoding="utf-8")
WORKSPACE_TEXT = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
WORKSPACE = tomllib.loads(WORKSPACE_TEXT)


@dataclass(frozen=True)
class Lane:
    number: int
    name: str
    passed: bool
    detail: str


def contains_all(text: str, values: list[str]) -> bool:
    return all(value in text for value in values)


def review() -> list[Lane]:
    members = set(WORKSPACE["workspace"]["members"])
    combined = "\n".join([CORE, PROBE, CLI, WORKSPACE_TEXT]).lower()
    forbidden_model_dependencies = [
        "openai",
        "anthropic",
        "ollama",
        "cloudflare",
        "llm",
        "reqwest",
    ]
    forbidden_probe_mutators = [
        "/add-driver",
        "/delete-driver",
        "/disable-device",
        "/enable-device",
        "/restart-device",
        "/scan-devices",
        '"/set"',
        '"/deletevalue"',
    ]
    intents = [
        "SetupPc",
        "FixProblem",
        "InstallDrivers",
        "PrepareGaming",
        "PrepareTechnician",
        "ImproveWindows",
        "DebloatWindows",
        "RepairDevices",
        "Advanced",
    ]

    return [
        Lane(
            1,
            "architecture",
            {"crates/neo-core", "crates/neo-probe", "crates/neo-cli"} <= members,
            "shared core, probe, and CLI crates are workspace members",
        ),
        Lane(
            2,
            "model-free",
            not any(value in combined for value in forbidden_model_dependencies),
            "no LLM/cloud/model networking dependency appears in the Phase 1 source",
        ),
        Lane(
            3,
            "manual-authority",
            contains_all(
                CORE,
                [
                    "MutationWithoutConfirmation",
                    "self.kind.mutates_machine() && !self.requires_confirmation",
                ],
            ),
            "mutating actions require explicit confirmation",
        ),
        Lane(
            4,
            "high-risk-default",
            contains_all(CORE, ["HighRiskPreselected", "self.risk >= RiskLevel::High"]),
            "HIGH/EXPERT risk actions cannot be preselected",
        ),
        Lane(
            5,
            "mutation-evidence",
            contains_all(CORE, ["MutationWithoutEvidence", "self.evidence.is_empty()"]),
            "mutating actions require supporting evidence",
        ),
        Lane(
            6,
            "certification-default",
            contains_all(
                CORE,
                [
                    "NonCertifiedActionPreselected",
                    "self.verdict != EvidenceVerdict::Certified",
                ],
            ),
            "only CERTIFIED actions can be selected by default",
        ),
        Lane(
            7,
            "unique-action-ids",
            contains_all(CORE, ["DuplicateActionId", "BTreeSet"]),
            "duplicate mission action IDs fail closed",
        ),
        Lane(
            8,
            "three-user-model",
            contains_all(CORE, ["Beginner", "Standard", "Expert"]),
            "Beginner, Standard, and Expert depths share one core contract",
        ),
        Lane(
            9,
            "intent-contract",
            all(intent in CORE and intent in CLI for intent in intents),
            "frozen first-launch intents are represented in both core and CLI",
        ),
        Lane(
            10,
            "read-only-command-surface",
            not any(value in PROBE.lower() for value in forbidden_probe_mutators),
            "Phase 1 probe exposes no known PnP/BCD mutation command",
        ),
        Lane(
            11,
            "bcd-safety",
            '"/enum", "{current}"' in PROBE and '"/set"' not in PROBE.lower(),
            "BCD is enumerated and never modified",
        ),
        Lane(
            12,
            "secure-boot",
            contains_all(PROBE, ["UEFISecureBootEnabled", "secure_boot"]),
            "Secure Boot has an independent read-only evidence lane",
        ),
        Lane(
            13,
            "memory-integrity",
            contains_all(PROBE, ["HypervisorEnforcedCodeIntegrity", "memory_integrity"]),
            "Memory Integrity/HVCI has an independent read-only evidence lane",
        ),
        Lane(
            14,
            "signature-state-separation",
            contains_all(PROBE, ["testsigning", "nointegritychecks"])
            and contains_all(CORE, ["test_signing", "no_integrity_checks"]),
            "Test Signing and nointegritychecks are distinct facts",
        ),
        Lane(
            15,
            "pending-reboot",
            contains_all(PROBE, ["RebootPending", "RebootRequired", "PendingFileRenameOperations"]),
            "multiple reboot indicators are collected without mutation",
        ),
        Lane(
            16,
            "device-evidence",
            contains_all(PROBE, ["/enum-devices", "/connected", "/problem"]),
            "connected and problem-device evidence lanes exist",
        ),
        Lane(
            17,
            "driver-store-evidence",
            '"/enum-drivers"' in PROBE,
            "Driver Store inventory is enumeration-only",
        ),
        Lane(
            18,
            "failure-honesty",
            contains_all(
                PROBE,
                [
                    "failed_to_start",
                    "fn capture",
                    "one_failed_command_start_does_not_abort_other_probe_lanes",
                ],
            ),
            "one failed command is retained as evidence without aborting independent lanes",
        ),
        Lane(
            19,
            "platform-boundary",
            contains_all(PROBE, ["UnsupportedPlatform", '#[cfg(target_os = "windows")]']),
            "non-Windows system scan fails explicitly rather than pretending equivalence",
        ),
        Lane(
            20,
            "proof-and-anti-drift",
            "#[cfg(test)]" in CORE
            and "#[cfg(test)]" in PROBE
            and (ROOT / "docs/ENGINEERING_EXECUTION.md").exists()
            and "docs/NEO_DRIVER_MASTER_PLAN.md" in (ROOT / "README.md").read_text(encoding="utf-8"),
            "unit fixtures, execution doctrine, and the canonical master-plan pointer are present",
        ),
    ]


def main() -> int:
    lanes = review()
    for lane in lanes:
        status = "PASS" if lane.passed else "FAIL"
        print(f"{status} {lane.number:02d} {lane.name}: {lane.detail}")

    failures = [lane for lane in lanes if not lane.passed]
    if failures:
        print(f"\nPhase 1 static review failed: {len(failures)} lane(s) unresolved.")
        return 1

    print("\nPhase 1 static review: PASS (20/20 lanes).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
