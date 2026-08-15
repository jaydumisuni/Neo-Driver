#!/usr/bin/env python3
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CRATE = ROOT / "crates" / "neo-debloat"
SRC = CRATE / "src"
production = "\n".join(
    path.read_text(encoding="utf-8")
    for path in sorted(SRC.rglob("*.rs"))
)
workspace = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
manifest = (CRATE / "Cargo.toml").read_text(encoding="utf-8")
review = (ROOT / "docs" / "PHASE13_20_LANE_REVIEW.md").read_text(encoding="utf-8")
decision = (ROOT / "docs" / "decisions" / "0013-PHASE13-DEBLOAT-ASSESSMENT.md").read_text(encoding="utf-8")
behavior = (CRATE / "tests" / "read_only.rs").read_text(encoding="utf-8")
ci = (ROOT / ".github" / "workflows" / "ci.yml").read_text(encoding="utf-8")


def has(*values: str) -> bool:
    return all(value in production for value in values)


def absent(*values: str) -> bool:
    return all(value not in production for value in values)


checks = [
    ("workspace member", '"crates/neo-debloat"' in workspace),
    ("platform neutral", "windows" not in manifest.lower()),
    ("transaction isolated", "neo-transaction" not in manifest),
    (
        "executor isolated",
        all(name not in manifest for name in ("neo-tweak-executor", "neo-runtime-executor", "neo-driverstore")),
    ),
    (
        "no production command execution",
        absent("std::process::Command", "powershell", "cmd.exe", "Remove-Appx", "winget", "dism.exe"),
    ),
    (
        "four typed debloat classes",
        has("enum DebloatClass", "SafeOptional", "FeatureDependent", "DependencySensitive", "ProtectedManualOnly"),
    ),
    (
        "installed and provisioned evidence",
        has("pub installed: ObservedPresence", "pub provisioned: ObservedPresence"),
    ),
    (
        "typed restore routes",
        has("enum RestoreMethod", "Store { store_id", "ProvisionedImage", "Vendor { source", "None"),
    ),
    (
        "catalogue serde validation",
        has("struct DebloatCatalogueWire", "impl<'de> Deserialize<'de> for DebloatCatalogue", "Self::try_from(wire)"),
    ),
    (
        "evidence serde validation",
        has("struct DebloatEvidenceWire", "impl<'de> Deserialize<'de> for DebloatEvidence", "DuplicateObservation"),
    ),
    (
        "identity uniqueness",
        has("DuplicateId", "DuplicatePackageId", "canonical_package_id"),
    ),
    (
        "default class and risk gates",
        has("UnsafeDefaultClass", "UnsafeDefaultRisk", "DebloatClass::SafeOptional", "RiskLevel::Low"),
    ),
    (
        "default evidence recommendation gate",
        has("NonCertifiedDefault", "UnsafeRecommendationDefault", "recommendation_allows_removal"),
    ),
    (
        "default restore gate",
        has("DefaultWithoutRestore", "self.restore.available()"),
    ),
    (
        "profile preservation and custom defaults",
        has("preserve_in_profiles", "DebloatProfile::Custom", "BlockedByProfile"),
    ),
    (
        "explicit selection gates",
        has("EmptySelection", "DuplicateSelection", "UnknownSelection"),
    ),
    (
        "observation completeness gates",
        has("MissingObservation", "UnavailableObservation", "fully_available"),
    ),
    (
        "candidate policy",
        has("RemovalCandidate", "candidate_policy_allows", "RiskLevel::Low", "EvidenceVerdict::Certified"),
    ),
    (
        "protected and policy blocking",
        has("BlockedProtected", "BlockedPolicy", "EvidenceVerdict::Rejected", "RecommendationState::DoNotTouch"),
    ),
    (
        "behavioral read-only proof wired",
        "Machine changes: none" in production
        and "machine_changes: false" in production
        and "directory_snapshot" in behavior
        and "assert_eq!(before, after" in behavior
        and "Phase 13 twenty-lane static review" in ci
        and "Phase 13 behavioral read-only proof" in ci
        and "**Mutation authority:** none" in review
        and "synthetic `Contoso.*`" in decision,
    ),
]

failed = []
for index, (name, ok) in enumerate(checks, 1):
    print(f"{index:02d}. {'PASS' if ok else 'FAIL'} - {name}")
    if not ok:
        failed.append(name)

if failed:
    raise SystemExit("Phase 13 static review failed: " + ", ".join(failed))

print("PHASE 13 STATIC REVIEW PASS: 20/20")
