#!/usr/bin/env python3
from pathlib import Path

path = Path(__file__).resolve().parents[1] / "crates/neo-tweak-executor/src/rpc.rs"
text = path.read_text(encoding="utf-8")
old = '''#[cfg(any(windows, test))]
use crate::{
    engine::{prepare_with_host, TweakHost},
    session::{apply_with_host, authorize_with_host},
};'''
new = '''#[cfg(test)]
use crate::{
    engine::{prepare_with_host, TweakHost},
    session::{apply_with_host, authorize_with_host},
};'''
if text.count(old) != 1:
    raise SystemExit("Windows Clippy import target did not match exactly once")
path.write_text(text.replace(old, new), encoding="utf-8")
