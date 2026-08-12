#!/usr/bin/env python3
"""Fail CI when generated Cargo.lock is absent or differs from Git."""
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parents[1]
lock = root / "Cargo.lock"
if not lock.exists():
    print("Cargo.lock was not generated.")
    raise SystemExit(1)
status = subprocess.run(
    ["git", "status", "--porcelain", "--", "Cargo.lock"],
    cwd=root,
    check=True,
    capture_output=True,
    text=True,
).stdout.strip()
if status:
    print("Cargo.lock is missing or out of date; generated lockfile follows:")
    print("-----BEGIN NEO CARGO LOCK-----")
    print(lock.read_text(encoding="utf-8"), end="")
    print("-----END NEO CARGO LOCK-----")
    raise SystemExit(1)
print("Cargo.lock is committed and current.")
