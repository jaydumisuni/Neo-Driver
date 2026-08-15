#!/usr/bin/env python3
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
path = ROOT / "docs/IMPLEMENTATION_STATUS.md"
text = path.read_text(encoding="utf-8")

old = "Phase 12 exact implementation-head run `31899810049` passed the complete Ubuntu and Windows Phase 1–12 pipeline, including the Phase 12 authority gate, Clippy with warnings denied, full units/adversarial proof, Windows live read-only state proof, Runtime System X-Ray, and all applicable inherited fixtures. Phase 12 merged through PR #22 as `e762369e1f71a67ef51ce216af792fdd00e74ad5`. CodeRabbit was explicitly triggered but had published no review submission or inline review thread at the merge gate, so no external CodeRabbit PASS is claimed."
new = "Phase 12 exact implementation-head run `31899810049` passed the complete Ubuntu and Windows Phase 1–12 pipeline, including the Phase 12 authority gate, Clippy with warnings denied, full units/adversarial proof, Windows live read-only state proof, Runtime System X-Ray, and all applicable inherited fixtures. Phase 12 merged through PR #22 as `e762369e1f71a67ef51ce216af792fdd00e74ad5`. Canonical documentation-state run `31900134525` then passed the same complete Ubuntu and Windows Phase 1–12 pipeline on the one-file status branch. CodeRabbit was explicitly triggered but had published no review submission or inline review thread at the implementation merge gate, so no external CodeRabbit PASS is claimed."
if text.count(old) != 1:
    raise SystemExit("Phase 12 proven-baseline paragraph did not match exactly once")
text = text.replace(old, new)

old_bullet = "- Phase 12 exact implementation-head run `31899810049`: **PASS on Ubuntu and Windows** across Phase 1–12 static gates, lock integrity, rustfmt, locked workspace build, Clippy with warnings denied, full workspace units/adversarial regressions, Windows live read-only state proof, Runtime System X-Ray, and every applicable inherited fixture."
new_bullet = old_bullet + "\n- Phase 12 canonical documentation-state run `31900134525`: **PASS on Ubuntu and Windows** across the complete Phase 1–12 pipeline on the one-file `docs/IMPLEMENTATION_STATUS.md` recovery branch."
if text.count(old_bullet) != 1:
    raise SystemExit("Phase 12 implementation proof bullet did not match exactly once")
text = text.replace(old_bullet, new_bullet)

path.write_text(text, encoding="utf-8")
