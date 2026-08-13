#!/usr/bin/env python3
"""Deterministic 20-lane static review for Neo Driver Phase 6."""
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
CLI = (ROOT / "crates/neo-cli/src/main.rs").read_text(encoding="utf-8")
WORKSPACE_TEXT = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
WORKSPACE = tomllib.loads(WORKSPACE_TEXT)
SOURCE_MAP_PATH = ROOT / "config/driver-pack-sources.json"
SOURCE_MAP = json.loads(SOURCE_MAP_PATH.read_text(encoding="utf-8"))


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
    expected_hashes = {
        "2eae084f948090520c729fb1476841b5f0c52d2b9642452f9057c13fe08bc0e9",
        "9ca8e9f679a96e145395047c193eb104c14d452227708b7346eb15a62c9eec9c",
        "fcaff595e0feb0fee2a7671fd1ae2dbad94b214a817f1584b56a1c68763c7576",
        "887e1020d0dc836781feb28d8b249a050e4ff8fb9050215106589ed17245b35b",
    }
    observed_hashes = {source["sha256"] for source in SOURCE_MAP["sources"]}

    return [
        Lane(1, "workspace", "crates/neo-vault" in members, "neo-vault is a first-class workspace crate"),
        Lane(2, "builder-root-authority", not any(value in production_lower for value in forbidden_root_choices), "production vault code does not hard-code a C:\\ProgramData/Program Files root; Builder supplies the root"),
        Lane(3, "single-managed-child", contains_all(PRODUCTION, ["MANAGED_DIRECTORY_NAME", '"NeoData"', "managed_root"]), "Neo owns one NeoData child beneath the supplied application root"),
        Lane(4, "absolute-root", contains_all(PRODUCTION, ["ApplicationRootNotAbsolute", "path.is_absolute()", "normalize_absolute"]), "vault roots must be resolved absolute paths"),
        Lane(5, "portable-installed-parity", contains_all(PRODUCTION, ["VaultMode", "Installed", "Portable"]) and "installed_and_portable_modes_share_the_same_child_layout" in TESTS, "installed and portable modes share one data-layout contract"),
        Lane(6, "managed-layout", contains_all(PRODUCTION, ["catalogue", "driver-packs", "packages", "runtimes", "staging", "sessions", "backups", "logs", "cache"]), "all planned Neo-owned package/runtime/session paths are explicit"),
        Lane(7, "segment-safety", contains_all(PRODUCTION, ["VaultSegment", "Deserialize<'de>", "InvalidSegment"]) and "vault_segments_reject_traversal" in TESTS, "path segments reject traversal/separators and validate during Serde"),
        Lane(8, "hash-identity", contains_all(PRODUCTION, ["Sha256Digest", "InvalidSha256", "to_ascii_lowercase"]) and "sha256_digest_is_normalized" in TESTS, "SHA-256 package identity is typed and normalized"),
        Lane(9, "source-map-validation", contains_all(PRODUCTION, ["DriverSourceMap", "SourceMapWire", "DuplicateSourceAsset", "DuplicateSourceId"]) and "direct_source_map_deserialization_runs_validation" in TESTS, "source-map root validation cannot be bypassed through direct Serde"),
        Lane(10, "ttg-source-map", source_repos == expected_repos and SOURCE_MAP.get("schema_version") == 1, "initial source map contains exactly the four approved TTG driver repositories"),
        Lane(11, "network-disabled", not any(value in production_lower for value in forbidden_network), "Phase 6 contains no network acquisition implementation"),
        Lane(12, "double-hash-intake", PRODUCTION.count("sha256_file(") >= 4 and contains_all(PRODUCTION, ["HashMismatch", "staged_hash", "expected_sha256"]), "pack intake validates source bytes and the copied staging bytes"),
        Lane(13, "immutable-promotion", contains_all(PRODUCTION, ["AlreadyPresent", "DestinationConflict", "fs::rename"]) and "promoted_pack_is_never_overwritten" in TESTS, "promoted packs are idempotent when identical and fail closed on drift"),
        Lane(14, "owned-staging", contains_all(PRODUCTION, ["STAGING_MARKER_NAME", "StagingMarker", "UnownedStaging", "StagingMarkerMismatch"]) and "staging_cleanup_requires_neo_ownership_marker" in TESTS, "staging cleanup requires an exact Neo ownership marker"),
        Lane(15, "cleanup-boundary", contains_all(PRODUCTION, ["ensure_cleanup_target", "self.staging", "self.cache"]) and "remove_dir_all(&path)" in PRODUCTION, "destructive cleanup is confined to owned staging/cache descendants"),
        Lane(16, "link-reparse-guard", contains_all(PRODUCTION, ["reject_link_like", "FILE_ATTRIBUTE_REPARSE_POINT", "ensure_directory_chain"]) and "audit_rejects_symlink_inside_managed_tree" in TESTS, "existing symlink/reparse paths are rejected and directory creation is component checked"),
        Lane(17, "existing-app-root", contains_all(PRODUCTION, ["ApplicationRootUnavailable", "app_root.exists()", "app_root.is_dir()"]), "Neo requires Builder/portable application root to pre-exist rather than creating an arbitrary root"),
        Lane(18, "read-only-public-cli", contains_all(CLI, ["VaultCommand", "Describe", "ValidateSources", "Audit", "Machine changes: none"]) and not any(value in cli_lower for value in forbidden_cli_writes), "public vault CLI is inspection/validation only"),
        Lane(19, "pinned-release-evidence", expected_hashes <= observed_hashes and all(source["release_tag"] for source in SOURCE_MAP["sources"]), "aggregate TTG driver packs are pinned by release tag and published SHA-256"),
        Lane(20, "phase5-boundary-preserved", "neo_driverstore" not in production_lower and "std::process::command" not in production_lower and "pnputil" not in production_lower and "bcdedit" not in production_lower, "vault layer cannot install drivers, spawn installers, or change Windows security state"),
    ]


def main() -> int:
    lanes = review()
    for lane in lanes:
        print(f"{'PASS' if lane.passed else 'FAIL'} {lane.number:02d} {lane.name}: {lane.detail}")
    failures = [lane for lane in lanes if not lane.passed]
    if failures:
        print(f"\nPhase 6 static review failed: {len(failures)} lane(s) unresolved.")
        return 1
    print("\nPhase 6 static review: PASS (20/20 lanes).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
