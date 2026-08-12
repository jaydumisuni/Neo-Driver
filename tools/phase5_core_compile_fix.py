#!/usr/bin/env python3
from pathlib import Path


def replace_once(path: Path, old: str, new: str) -> None:
    text = path.read_text(encoding="utf-8")
    if text.count(old) != 1:
        raise SystemExit(f"anchor mismatch in {path}: {old[:80]!r}")
    path.write_text(text.replace(old, new, 1), encoding="utf-8")


executor = Path("crates/neo-driverstore/src/executor.rs")
replace_once(
    executor,
    """use neo_transaction::{\n    ApplyOutcome, ApplyRecord, BaselineSnapshot, Observation, ObservedValue, RollbackRecord,\n    TransactionAuthorization, TransactionCheckpoint, TransactionStage,\n};\nuse serde::{Deserialize, Serialize};\nuse std::collections::BTreeSet;\n""",
    """use neo_transaction::{\n    ApplyOutcome, ApplyRecord, Observation, ObservedValue, RollbackRecord,\n    TransactionAuthorization, TransactionCheckpoint, TransactionStage,\n};\nuse serde::{Deserialize, Serialize};\n""",
)
replace_once(
    executor,
    """\n#[allow(dead_code)]\nfn _baseline_is_exact(_baseline: &BaselineSnapshot) {}\n""",
    "\n",
)

tests = Path("crates/neo-driverstore/src/tests.rs")
replace_once(
    tests,
    """        if state.install_changes {\n            for device in &mut state.inventory.devices {\n                if state\n                    .compatible\n                    .iter()\n                    .any(|id| id.eq_ignore_ascii_case(device.instance_id.as_str()))\n                {\n""",
    """        if state.install_changes {\n            let compatible = state.compatible.clone();\n            let target_problem_code = state.target_problem_code;\n            for device in &mut state.inventory.devices {\n                if compatible\n                    .iter()\n                    .any(|id| id.eq_ignore_ascii_case(device.instance_id.as_str()))\n                {\n""",
)
replace_once(
    tests,
    """                    device.problem_code = state.target_problem_code;\n""",
    """                    device.problem_code = target_problem_code;\n""",
)
