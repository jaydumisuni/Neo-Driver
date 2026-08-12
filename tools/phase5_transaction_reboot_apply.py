#!/usr/bin/env python3
"""Apply the reviewed Phase 5 transaction patch while consuming repeated anchors one at a time."""
from pathlib import Path
import phase5_transaction_reboot_patch as patch


def replace_first(path: Path, old: str, new: str) -> None:
    text = path.read_text(encoding="utf-8")
    if old not in text:
        raise SystemExit(f"missing Phase 5 transaction patch anchor in {path}: {old[:100]!r}")
    path.write_text(text.replace(old, new, 1), encoding="utf-8")


patch.replace_once = replace_first
patch.main()
