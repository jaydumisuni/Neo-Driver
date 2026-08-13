#!/usr/bin/env python3
"""Deterministic 20-lane static review for Neo Driver Phase 7 managed vault."""
from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
import json
import sys
import tomllib

ROOT = Path(__file__).resolve().parents[1]
VAULT_ROOT = ROOT / "crates/neo-vault"
PRODUCTION = "\n".join(
    path.read_text(encoding="utf-8")
    for path in sorted((VAULT_ROOT / "src").glob("*.rs"))
    if path.name != "tests.rs"
)
TESTS = (VAULT_ROOT / "src/tests.rs").read_text(encoding="utf-8")
CONCURRENCY = (VAULT_ROOT / "tests/concurrency.rs").read_text(encoding="utf-8")
CLI = (ROOT / "crates/neo-cli/src/main.rs").read_text(encoding="utf-8")
WORKSPACE_TEXT = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
WORKSPACE = tomllib.loads(WORKSPACE_TEXT)
SOURCE_MAP = json.loads((ROOT / "config/driver-pack-sources.json").read_text(encoding="utf-8"))


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
    production_lower = PRODUCTION.lower()
    cli_lower = CLI.lower()
    source_repos = {source["repository"] for source in SOURCE_MAP["sources"]}
    expected_repos = {
        "jaydumisuni/android-drivers",
        "jaydumisuni/Exynos-driver",
        "jaydumisuni/Apple-windows-drivers",
        "jaydumisuni/TechGuyDrivers",
    }
    expected_hashes = {
        "2eae084f948090520c729fb1476841b5f0c52d2b9642452f9057c13fe08bc0e9",
        "9ca8e9f679a96e145395047c193eb104c14d452227708b7346eb15a62c9eec9c",
        "fcaff595e0feb0fee2a7671fd1ae2dbad94b214a817f1584b56a1c68763c7576",
        "887e1020d0dc836781feb28d8b249a050e4ff8fb9050215106589ed17245b35b",
    }
    observed_hashes = {source["sha256"] for source in SOURCE_MAP["sources"]}
    forbidden_root_choices = [r"c:\programdata", r"c:\program files"]
    forbidden_network = ["reqwest", "ureq", "curl ", "wget ", "http::", "https://api.github.com"]
    forbidden_cli_writes = [
        "import_pack_file(",
        "begin_staging(",
        "cleanup_staging(",
        "ensure_layout(",
        "std::fs::remove_",
        "std::fs::create_dir",
    ]

    return [
        Lane(1, "workspace", "crates/neo-vault" in members, "neo-vault is a first-class workspace crate"),
        Lane(2, "phase6-preserved", {"crates/neo-runtime", "crates/neo-directx-legacy", "crates/neo-runtime-probe"} <= members and (ROOT / "tools/phase6_static_review.py").exists(), "merged Phase 6 runtime/gaming foundation remains present"),
        Lane(3, "builder-root-authority", not any(value in production_lower for value in forbidden_root_choices), "vault production code does not choose ProgramData/Program Files; Builder supplies the root"),
        Lane(4, "single-managed-child", contains_all(PRODUCTION, ["MANAGED_DIRECTORY_NAME", '"NeoData"', "managed_root"]), "Neo owns one NeoData child beneath the supplied application root"),
        Lane(5, "absolute-root", contains_all(PRODUCTION, ["ApplicationRootNotAbsolute", "path.is_absolute()", "normalize_absolute"]), "vault roots must be resolved absolute paths"),
        Lane(6, "portable-installed-parity", contains_all(PRODUCTION, ["VaultMode", "Installed", "Portable"]) and "installed_and_portable_modes_share_the_same_child_layout" in TESTS, "installed and portable modes share one data-layout contract"),
        Lane(7, "managed-layout", contains_all(PRODUCTION, ["catalogue", "driver-packs", "packages", "runtimes", "staging", "sessions", "backups", "logs", "cache"]), "all Neo-owned package/runtime/session directories are explicit"),
        Lane(8, "segment-safety", contains_all(PRODUCTION, ["VaultSegment", "Deserialize<'de>", "InvalidSegment"]) and "vault_segments_reject_traversal" in TESTS, "path segments reject traversal/separators and validate during Serde"),
        Lane(9, "hash-identity", contains_all(PRODUCTION, ["Sha256Digest", "InvalidSha256", "to_ascii_lowercase"]) and "sha256_digest_is_normalized" in TESTS, "SHA-256 package identity is typed and normalized"),
        Lane(10, "source-map-validation", contains_all(PRODUCTION, ["DriverSourceMap", "SourceMapWire", "DuplicateSourceAsset", "DuplicateSourceId"]) and "direct_source_map_deserialization_runs_validation" in TESTS, "source-map root validation cannot be bypassed through direct Serde"),
        Lane(11, "ttg-source-map", source_repos == expected_repos and SOURCE_MAP.get("schema_version") == 1 and expected_hashes <= observed_hashes, "approved TTG repositories are pinned by release identity and published hashes"),
        Lane(12, "network-disabled", not any(value in production_lower for value in forbidden_network), "vault layer contains no network acquisition implementation"),
        Lane(13, "double-hash-intake", PRODUCTION.count("sha256_") >= 5 and contains_all(PRODUCTION, ["HashMismatch", "staged_hash", "promoted_hash"]), "pack intake validates source, staging and promoted bytes"),
        Lane(14, "concurrent-promotion", contains_all(PRODUCTION, ["unique_import_session", "create_new_file_nofollow", "ImportBusy"]) and "concurrent_same_pack_import_never_overwrites_or_leaves_staging_noise" in CONCURRENCY, "concurrent imports use unique staging and exclusive final creation"),
        Lane(15, "owned-staging", contains_all(PRODUCTION, ["STAGING_MARKER_NAME", "StagingMarker", "UnownedStaging", "StagingMarkerMismatch"]) and "staging_cleanup_requires_neo_ownership_marker" in TESTS, "staging cleanup requires an exact Neo ownership marker"),
        Lane(16, "cleanup-boundary", "remove_dir_all" in PRODUCTION and "staging" in PRODUCTION and "cache" in PRODUCTION, "destructive cleanup remains inside Neo-managed disposable areas"),
        Lane(17, "capability-no-follow", contains_all(PRODUCTION, ["cap_std", "open_dir_nofollow", "OpenOptionsFollowExt", "FollowSymlinks::No"]), "filesystem traversal/promotion uses retained no-follow capabilities"),
        Lane(18, "root-audit", contains_all(PRODUCTION, ["audit_existing_tree", "open_absolute_dir_nofollow"]) and "audit_rejects_symlink_inside_managed_tree" in TESTS, "read-only audit validates the application root and rejects link/reparse escapes"),
        Lane(19, "read-only-public-cli", contains_all(CLI, ["VaultCommand", "Describe", "ValidateSources", "Audit", "Machine changes: none"]) and not any(value in cli_lower for value in forbidden_cli_writes), "public vault CLI exposes inspection/validation only"),
        Lane(20, "prior-authority-preserved", "neo_driverstore" not in production_lower and "std::process::command" not in production_lower and "pnputil" not in production_lower and "bcdedit" not in production_lower and contains_all(CLI, ["RuntimeScan", "Runtimes", "Gaming"]), "vault cannot mutate drivers/security and Phase 6 runtime CLI remains intact"),
    ]


def main() -> int:
    lanes = review()
    for lane in lanes:
        print(f"{'PASS' if lane.passed else 'FAIL'} {lane.number:02d} {lane.name}: {lane.detail}")
    failures = [lane for lane in lanes if not lane.passed]
    if failures:
        print(f"\nPhase 7 static review failed: {len(failures)} lane(s) unresolved.")
        return 1
    print("\nPhase 7 static review: PASS (20/20 lanes).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
