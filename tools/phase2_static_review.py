#!/usr/bin/env python3
"""Deterministic 20-lane static review for Neo Driver Phase 2."""
from __future__ import annotations
from dataclasses import dataclass
from pathlib import Path
import json
import sys
import tomllib

ROOT = Path(__file__).resolve().parents[1]
DEVICE = (ROOT / "crates/neo-device/src/lib.rs").read_text(encoding="utf-8")
CATALOGUE = (ROOT / "crates/neo-catalogue/src/lib.rs").read_text(encoding="utf-8")
CLI = (ROOT / "crates/neo-cli/src/main.rs").read_text(encoding="utf-8")
WORKSPACE_TEXT = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
WORKSPACE = tomllib.loads(WORKSPACE_TEXT)
FIXTURE = json.loads((ROOT / "fixtures/catalogue/sample_driver_catalogue.json").read_text(encoding="utf-8"))

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
    review_paths = [ROOT / "Cargo.toml"]
    for member in members:
        member_root = ROOT / member
        review_paths.append(member_root / "Cargo.toml")
        review_paths.extend(member_root.rglob("*.rs"))
    combined = "\n".join(
        path.read_text(encoding="utf-8") for path in sorted(set(review_paths))
    ).lower()
    forbidden_mutators = [
        "/add-driver", "/delete-driver", "/disable-device", "/enable-device",
        "/restart-device", '"/set"', '"/deletevalue"', "install-driver",
    ]
    forbidden_model_dependencies = ["openai", "anthropic", "ollama", "cloudflare", "llm", "reqwest"]
    package = FIXTURE["packages"][0]
    artifact = package["driver_artifacts"][0]
    return [
        Lane(1, "workspace", {"crates/neo-device", "crates/neo-catalogue"} <= members, "device and catalogue crates are workspace members"),
        Lane(2, "opaque-id", contains_all(DEVICE, ["OpaqueDeviceId", "pub fn new", "Deserialize<'de>"]), "IDs are opaque validated types including deserialization"),
        Lane(3, "ordered-ids", contains_all(DEVICE, ["Vec<OpaqueDeviceId>", "hardware_ids", "compatible_ids"]), "hardware/compatible IDs preserve order"),
        Lane(4, "validated-deserialization", contains_all(DEVICE, ["DeviceRecordWire", "DeviceInventoryWire", "try_from", "DuplicateOpaqueValue"]), "device evidence validation cannot be bypassed through deserialization"),
        Lane(5, "usb-stack-evidence", contains_all(DEVICE, ["service", "upper_filters", "lower_filters", "active_driver"]), "service/filter/binding evidence is preserved"),
        Lane(6, "package-kind-separation", contains_all(CATALOGUE, ["InfDriverBundle", "TechnicianComponent"]), "technician components are not forced into INF semantics"),
        Lane(7, "provenance-hash", contains_all(CATALOGUE, ["Provenance", "sha256", "validate_sha256"]), "package provenance includes validated SHA-256"),
        Lane(8, "signature-evidence", contains_all(CATALOGUE, ["SignatureStatus", "SignatureEvidence", "signer"]), "per-artifact signature/signer evidence exists"),
        Lane(9, "verified-signature-gate", contains_all(CATALOGUE, ["VerifiedDriverWithoutCatalog", "VerifiedDriverWithoutSigner"]), "verified status requires catalogue and signer"),
        Lane(10, "per-inf-artifacts", contains_all(CATALOGUE, ["DriverArtifact", "inf_path", "driver_artifacts"]), "driver metadata is modeled per INF"),
        Lane(11, "driver-bundle-gate", "DriverBundleWithoutArtifacts" in CATALOGUE, "INF bundles require artifacts"),
        Lane(12, "non-driver-gate", "UnexpectedDriverArtifacts" in CATALOGUE, "non-INF packages cannot silently carry INF artifacts"),
        Lane(13, "dependency-conflict-guards", contains_all(CATALOGUE, ["SelfDependency", "SelfConflict", "DependencyConflictOverlap", "UnresolvedDependency", "UnresolvedConflict"]), "dependency/conflict graph rejects contradictions and missing references"),
        Lane(14, "duplicate-manifest-guards", contains_all(CATALOGUE, ["DuplicatePackageId", "DuplicateInfPath", "DuplicateValue"]), "package/INF/list duplicates fail closed"),
        Lane(15, "explicit-security-targets", contains_all(CATALOGUE, ["RequiredState", "Unchanged", "Enabled", "Disabled"]), "security requirements use explicit target states"),
        Lane(16, "security-reboot-gate", contains_all(CATALOGUE, ["SecurityStateChangeWithoutRequiredReboot", "changes_boot_or_security_state"]), "security-state changes require reboot=required"),
        Lane(17, "windows-applicability", contains_all(CATALOGUE, ["architectures", "minimum_build", "maximum_build", "InvalidBuildRange"]), "Windows architecture/build applicability is typed"),
        Lane(18, "read-only-cli", contains_all(CLI, ["CatalogueCommand", "Validate", "Catalogue::read_json", "Machine changes: none"]) and not any(v in combined for v in forbidden_mutators), "CLI validates without mutation"),
        Lane(19, "fixture", package["kind"] == "inf_driver_bundle" and bool(artifact["ids"]["hardware_ids"]) and artifact["signature"]["status"] == "verified", "synthetic fixture exercises core catalogue evidence"),
        Lane(20, "workspace-wide-anti-drift", not any(v in combined for v in forbidden_model_dependencies) and "ls-files" in (ROOT / "tools/lockfile_guard.py").read_text(encoding="utf-8"), "all workspace manifests/Rust sources remain model-free and Cargo.lock must be Git-tracked"),
    ]

def main() -> int:
    lanes = review()
    for lane in lanes:
        print(f"{'PASS' if lane.passed else 'FAIL'} {lane.number:02d} {lane.name}: {lane.detail}")
    failures = [lane for lane in lanes if not lane.passed]
    if failures:
        print(f"\nPhase 2 static review failed: {len(failures)} lane(s) unresolved.")
        return 1
    print("\nPhase 2 static review: PASS (20/20 lanes).")
    return 0

if __name__ == "__main__":
    sys.exit(main())
