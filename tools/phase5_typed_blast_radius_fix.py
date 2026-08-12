#!/usr/bin/env python3
from pathlib import Path

path = Path("crates/neo-driverstore/src/executor.rs")
text = path.read_text(encoding="utf-8")
old = '''            operational_error.get_or_insert_with(|| {
                format!("unexpected binding change outside authority: {instance_id}")
            });
'''
new = '''            operational_error.get_or_insert_with(|| {
                DriverStoreError::UnexpectedBindingChange(instance_id).to_string()
            });
'''
if text.count(old) != 1:
    raise SystemExit("blast-radius typed-error anchor mismatch")
path.write_text(text.replace(old, new, 1), encoding="utf-8")
