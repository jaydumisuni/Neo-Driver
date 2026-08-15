#!/usr/bin/env python3
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CRATE = ROOT / "crates" / "neo-debloat-probe"
SRC = CRATE / "src"
production = "\n".join(path.read_text(encoding="utf-8") for path in sorted(SRC.rglob("*.rs")))
workspace = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
manifest = (CRATE / "Cargo.toml").read_text(encoding="utf-8")
phase13_manifest = (ROOT / "crates" / "neo-debloat" / "Cargo.toml").read_text(encoding="utf-8")
review = (ROOT / "docs" / "PHASE14_20_LANE_REVIEW.md").read_text(encoding="utf-8")
decision = (ROOT / "docs" / "decisions" / "0014-PHASE14-DEBLOAT-WINDOWS-LIVE-INVENTORY.md").read_text(encoding="utf-8")
behavior = (CRATE / "tests" / "live_read_only.rs").read_text(encoding="utf-8")
ci = (ROOT / ".github" / "workflows" / "ci.yml").read_text(encoding="utf-8")


def has(*values: str) -> bool:
    return all(value in production for value in values)


def absent(*values: str) -> bool:
    return all(value not in production for value in values)

checks = [
    ("workspace member", '"crates/neo-debloat-probe"' in workspace),
    ("probe dependencies", "neo-debloat" in manifest and "neo-probe" in manifest),
    ("phase13 isolation preserved", "neo-probe" not in phase13_manifest and "windows" not in phase13_manifest.lower()),
    ("fixed noninteractive powershell", has('"powershell.exe"', '"-NoLogo"', '"-NoProfile"', '"-NonInteractive"', '"-Command"')),
    ("catalogue identity not command input", "Catalogue identities are never interpolated" in production and "definition.package_id" not in production.split("fn capture_script", 1)[0]),
    ("current user inventory", has("Get-AppxPackage", "PackageTypeFilter")),
    ("provisioned inventory", has("Get-AppxProvisionedPackage -Online")),
    ("no execution policy bypass", absent("ExecutionPolicy", "Bypass")),
    ("typed json inventory", has("ConvertTo-Json", "InstalledPackageRecord", "ProvisionedPackageRecord", "serde_json::from_str")),
    ("command evidence retained", has("pub command_evidence: Vec<CommandEvidence>", "vec![installed_command, provisioned_command]")),
    ("query failure unavailable", has("state remains Unavailable", "ObservedPresence::Unavailable")),
    ("malformed and identity incomplete unavailable", has("returned malformed JSON", "inventory remains Unavailable", "return None")),
    ("case insensitive matching", has("to_ascii_lowercase", "canonical(&definition.package_id)")),
    ("conservative version", has("unique_version", "versions.len() == 1")),
    ("phase13 evidence constructor reused", has("DebloatEvidence::new(observations)")),
    ("mutation engines isolated", all(name not in manifest for name in ("neo-transaction", "neo-driverstore", "neo-runtime-executor", "neo-tweak-executor"))),
    ("no appx mutation commands", absent("Remove-AppxPackage", "Remove-AppxProvisionedPackage", "Add-AppxPackage", "Add-AppxProvisionedPackage", "winget.exe")),
    ("proof binary read only", "Machine changes: none" in production and "machine_changes: false" in production),
    ("live windows behavioral proof", "live_windows_inventory_is_read_only_to_fixture_state" in behavior and "command_evidence.iter().all" in behavior and "assert_eq!(" in behavior and "live read-only inventory changed fixture state" in behavior and "CARGO_BIN_EXE_neo-debloat-live-scan" in behavior),
    ("ci and decision freeze", "Phase 14 twenty-lane static review" in ci and "Phase 14 live Windows debloat inventory proof" in ci and "**Mutation authority:** none" in review and "plugin dependency" in decision),
]

failed = []
for index, (name, ok) in enumerate(checks, 1):
    print(f"{index:02d}. {'PASS' if ok else 'FAIL'} - {name}")
    if not ok:
        failed.append(name)

if failed:
    raise SystemExit("Phase 14 static review failed: " + ", ".join(failed))

print("PHASE 14 STATIC REVIEW PASS: 20/20")
