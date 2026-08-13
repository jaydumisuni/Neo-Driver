#!/usr/bin/env python3
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SRC = ROOT / "crates" / "neo-state-plan" / "src"
production = "\n".join(
    path.read_text(encoding="utf-8")
    for path in sorted(SRC.rglob("*.rs"))
    if path.name != "tests.rs"
)
workspace = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
manifest = (ROOT / "crates" / "neo-state-plan" / "Cargo.toml").read_text(encoding="utf-8")
cli = (ROOT / "crates" / "neo-cli" / "src" / "state_assess_cli.rs").read_text(encoding="utf-8")
review = (ROOT / "docs" / "PHASE9_20_LANE_REVIEW.md").read_text(encoding="utf-8")

def has(value: str) -> bool:
    return value in production

checks = [
    ("workspace member", '"crates/neo-state-plan"' in workspace),
    ("no windows dependency", "windows" not in manifest.lower()),
    ("no transaction dependency", "neo-transaction" not in manifest),
    ("no process command", "std::process::Command" not in production),
    ("typed values", has("enum TweakValue") and has("U32") and has("U64")),
    ("validated target", has("struct TweakTarget") and has("canonical_key")),
    ("catalogue serde validation", 'serde(try_from = "TweakCatalogueWire")' in production),
    ("evidence serde validation", 'serde(try_from = "TweakEvidenceWire")' in production),
    ("duplicate ids", has("DuplicateId")),
    ("duplicate targets", has("DuplicateTarget")),
    ("duplicate observations", has("DuplicateObservation")),
    ("high risk default gate", has("HighRiskPreselected")),
    ("certified default gate", has("NonCertifiedPreselected")),
    ("safe recommendation gate", has("UnsafeRecommendationPreselected")),
    ("explicit selection", has("EmptySelection")),
    ("duplicate selection", has("DuplicateSelection")),
    ("unknown selection", has("UnknownTweak")),
    ("rejected selection", has("RejectedTweak")),
    ("observation hard gates", has("MissingObservation") and has("UnavailableObservation")),
    ("read-only proof surface", "Machine changes: none" in cli and "machine change" in review.lower()),
]

failed = []
for index, (name, ok) in enumerate(checks, 1):
    print(f"{index:02d}. {'PASS' if ok else 'FAIL'} - {name}")
    if not ok:
        failed.append(name)
if failed:
    raise SystemExit("Phase 9 static review failed: " + ", ".join(failed))
print("PHASE 9 STATIC REVIEW PASS: 20/20")
