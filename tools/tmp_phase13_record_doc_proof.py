from pathlib import Path

path = Path("docs/IMPLEMENTATION_STATUS.md")
text = path.read_text(encoding="utf-8")
old = "Canonical documentation-state proof is pending on the one-file status branch."
new = "Canonical documentation-state run `31902598020` then passed the same complete Ubuntu and Windows Phase 1–13 pipeline on the one-file status branch."
if text.count(old) != 1:
    raise SystemExit(f"pending docs-proof marker count={text.count(old)}")
text = text.replace(old, new, 1)
path.write_text(text.rstrip() + "\n", encoding="utf-8")
