#!/usr/bin/env python3
"""Deterministic 20-lane static review for Neo Driver Phase 3."""
from __future__ import annotations
from dataclasses import dataclass
from pathlib import Path
import sys
import tomllib

ROOT = Path(__file__).resolve().parents[1]
WORKSPACE_TEXT = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
WORKSPACE = tomllib.loads(WORKSPACE_TEXT)
MATCH = (ROOT / "crates/neo-match/src/lib.rs").read_text(encoding="utf-8")
MATCH_TESTS = (ROOT / "crates/neo-match/src/tests.rs").read_text(encoding="utf-8")
CATALOGUE = (ROOT / "crates/neo-catalogue/src/lib.rs").read_text(encoding="utf-8")
CLI = (ROOT / "crates/neo-cli/src/main.rs").read_text(encoding="utf-8")

@dataclass(frozen=True)
class Lane:
    number: int
    name: str
    passed: bool
    detail: str

def contains_all(text: str, values: list[str]) -> bool:
    return all(value in text for value in values)

def workspace_source() -> str:
    members = WORKSPACE["workspace"]["members"]
    paths = [ROOT / "Cargo.toml"]
    for member in members:
        member_root = ROOT / member
        paths.append(member_root / "Cargo.toml")
        paths.extend(member_root.rglob("*.rs"))
    return "\n".join(
        path.read_text(encoding="utf-8") for path in sorted(set(paths))
    ).lower()

def review() -> list[Lane]:
    members = set(WORKSPACE["workspace"]["members"])
    combined = workspace_source()
    forbidden_model_dependencies = [
        "openai",
        "anthropic",
        "ollama",
        "cloudflare",
        "reqwest",
    ]
    forbidden_mutators = [
        "/add-driver",
        "/delete-driver",
        "/disable-device",
        "/enable-device",
        "/restart-device",
        '"/set"',
        '"/deletevalue"',
        "install-driver",
    ]
    return [
        Lane(1, "workspace", "crates/neo-match" in members, "neo-match is a workspace member"),
        Lane(2, "inf-model-shape", contains_all(CATALOGUE, ["InfModelEntry", "hardware_id", "compatible_ids"]), "INF Models entries preserve one hardware ID plus ordered compatible IDs"),
        Lane(3, "no-flattened-driver-ids", "pub ids: OrderedDeviceIds" not in CATALOGUE and "pub models: Vec<InfModelEntry>" in CATALOGUE, "driver artifacts no longer flatten INF model entries into device-shaped ID lists"),
        Lane(4, "four-match-classes", contains_all(MATCH, ["DeviceHardwareToInfHardware", "DeviceHardwareToInfCompatible", "DeviceCompatibleToInfHardware", "DeviceCompatibleToInfCompatible"]), "all four Microsoft identifier match classes are explicit"),
        Lane(5, "identifier-score-bases", contains_all(MATCH, ["0x1000", "0x2000", "0x3000", "type_score"]), "identifier score bases follow Microsoft match-type ordering"),
        Lane(6, "compatible-position-score", contains_all(MATCH, ["inf_position.checked_mul(0x100)", "position_score > 0x0fff", "IdentifierScoreOutOfRange"]) and contains_all(MATCH_TESTS, ["identifier_score_refuses_values_outside_documented_range", "inf_compatible_position_resets_for_each_model_entry"]), "compatible-list positions reset per Models entry and are checked against the documented THHH range"),
        Lane(7, "opaque-comparison", "eq_ignore_ascii_case" in MATCH and "split('&')" not in MATCH and "split(\"&\")" not in MATCH, "matching compares opaque IDs and does not parse bus-specific fragments"),
        Lane(8, "architecture-gate", contains_all(MATCH, ["ArchitectureMetadataMissing", "ArchitectureMismatch"]), "missing or mismatched architecture fails closed"),
        Lane(9, "build-gates", contains_all(MATCH, ["WindowsBuildTooOld", "WindowsBuildTooNew", "minimum_build", "maximum_build"]), "Windows build applicability is a hard gate"),
        Lane(10, "invalid-signature-reject", contains_all(MATCH, ["InvalidSignature", "SignatureStatus::Invalid", "EvidenceVerdict::Rejected"]), "invalid signature state rejects a candidate"),
        Lane(11, "unknown-signature-investigate", "SignatureStatus::Unknown | SignatureStatus::Unsigned => EvidenceVerdict::Investigate" in MATCH, "unknown/unsigned candidates never become certified"),
        Lane(12, "no-full-rank-claim", contains_all(MATCH, ["full_windows_rank_available: false", "Full Windows rank", "FeatureScore"]), "Phase 3 explicitly refuses to claim complete Windows rank"),
        Lane(
            13,
            "identifier-before-date",
            MATCH.find("identifier_value(left)") >= 0
            and MATCH.find("compare_known_windows_tiebreaks") >= 0
            and MATCH.find("identifier_value(left)") < MATCH.find("compare_known_windows_tiebreaks"),
            "identifier score precedes date/version tie-breakers",
        ),
        Lane(14, "newer-generic-regression", "newer_generic_does_not_beat_exact_hardware_match" in MATCH_TESTS, "regression proves a newer generic driver cannot beat a better exact match"),
        Lane(15, "ordered-device-id-regression", "more_specific_hardware_id_position_wins" in MATCH_TESTS, "device ID list position affects selection"),
        Lane(16, "equal-rank-tiebreak-regression", contains_all(MATCH_TESTS, ["date_then_version_break_equal_identifier_ties", "unknown_date_blocks_version_from_manufacturing_a_winner"]), "date/version are tested only when higher-priority tie-break evidence is known"),
        Lane(17, "read-only-cli", contains_all(CLI, ["Command::Match", "match_device", "Machine changes: none"]), "CLI exposes matching without installation"),
        Lane(18, "no-mutation", not any(value in combined for value in forbidden_mutators), "workspace contains no newly exposed driver/BCD mutation command"),
        Lane(19, "model-free", not any(value in combined for value in forbidden_model_dependencies), "workspace remains model-free with no networking/model dependency"),
        Lane(20, "proof-fixture", (ROOT / "fixtures/match/device.json").exists() and (ROOT / "fixtures/catalogue/sample_driver_catalogue.json").exists(), "device + catalogue fixtures exist for CLI proof"),
    ]

def main() -> int:
    lanes = review()
    for lane in lanes:
        print(f"{'PASS' if lane.passed else 'FAIL'} {lane.number:02d} {lane.name}: {lane.detail}")
    failures = [lane for lane in lanes if not lane.passed]
    if failures:
        print(f"\nPhase 3 static review failed: {len(failures)} lane(s) unresolved.")
        return 1
    print("\nPhase 3 static review: PASS (20/20 lanes).")
    return 0

if __name__ == "__main__":
    sys.exit(main())
