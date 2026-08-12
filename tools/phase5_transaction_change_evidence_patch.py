#!/usr/bin/env python3
"""Phase 5 prerequisite: separate machine-change evidence from API outcome."""
from pathlib import Path


def replace_all(path: Path, old: str, new: str, expected: int | None = None) -> None:
    text = path.read_text(encoding="utf-8")
    count = text.count(old)
    if count == 0 or (expected is not None and count != expected):
        raise SystemExit(f"unexpected anchor count {count} in {path}: {old[:90]!r}")
    path.write_text(text.replace(old, new), encoding="utf-8")


def main() -> None:
    plan = Path("crates/neo-transaction/src/plan.rs")
    replace_all(
        plan,
        '''pub struct ApplyRecord {
    pub action_id: String,
    pub outcome: ApplyOutcome,
    pub detail: String,
    #[serde(default)]
    pub reboot_required: bool,
}
''',
        '''pub struct ApplyRecord {
    pub action_id: String,
    pub outcome: ApplyOutcome,
    pub detail: String,
    #[serde(default = "default_machine_changed")]
    pub machine_changed: bool,
    #[serde(default)]
    pub reboot_required: bool,
}

fn default_machine_changed() -> bool {
    true
}
''',
        expected=1,
    )

    for path_name in [
        "crates/neo-transaction/src/checkpoint.rs",
        "crates/neo-transaction/src/invariants.rs",
    ]:
        path = Path(path_name)
        replace_all(path, "successful_applied_ids", "changed_action_ids")

    invariants = Path("crates/neo-transaction/src/invariants.rs")
    replace_all(
        invariants,
        '''    pub(crate) fn changed_action_ids(&self) -> BTreeSet<String> {
        self.apply_records
            .iter()
            .filter(|record| record.outcome == ApplyOutcome::Success)
            .map(|record| record.action_id.clone())
            .collect()
    }
''',
        '''    pub(crate) fn changed_action_ids(&self) -> BTreeSet<String> {
        self.apply_records
            .iter()
            .filter(|record| record.machine_changed)
            .map(|record| record.action_id.clone())
            .collect()
    }
''',
        expected=1,
    )

    checkpoint = Path("crates/neo-transaction/src/checkpoint.rs")
    replace_all(
        checkpoint,
        '''        if record.outcome == ApplyOutcome::Failure {
            let changed = self.changed_action_ids();
''',
        '''        if record.outcome == ApplyOutcome::Failure {
            let changed = self.changed_action_ids();
''',
        expected=1,
    )

    tests = Path("crates/neo-transaction/src/tests.rs")
    text = tests.read_text(encoding="utf-8")
    needle = "detail: \"future executor reported success\".to_string(),\n            reboot_required:"
    text = text.replace(
        needle,
        "detail: \"future executor reported success\".to_string(),\n            machine_changed: true,\n            reboot_required:",
    )
    needle = "detail: \"backend discovered reboot\".to_string(),\n            reboot_required:"
    text = text.replace(
        needle,
        "detail: \"backend discovered reboot\".to_string(),\n            machine_changed: true,\n            reboot_required:",
    )
    tests.write_text(text, encoding="utf-8")

    text = tests.read_text(encoding="utf-8")
    marker = "\n#[test]\nfn runtime_apply_reboot_escalates_possible_plan() {"
    added = '''
#[test]
fn successful_no_change_does_not_create_rollback_obligation() {
    let mut checkpoint = authorized_checkpoint();
    checkpoint.begin_apply().unwrap();
    checkpoint
        .record_apply_result(ApplyRecord {
            action_id: "neo.fixture.tweak".to_string(),
            outcome: ApplyOutcome::Success,
            detail: "operation completed without changing machine state".to_string(),
            machine_changed: false,
            reboot_required: false,
        })
        .unwrap();
    checkpoint
        .verify_postconditions(vec![Observation {
            target: target(),
            value: ObservedValue::Present("0".to_string()),
        }])
        .unwrap();
    assert_eq!(checkpoint.stage(), TransactionStage::Failed);
}

#[test]
fn failed_operation_with_observed_change_routes_to_rollback() {
    let mut checkpoint = authorized_checkpoint();
    checkpoint.begin_apply().unwrap();
    checkpoint
        .record_apply_result(ApplyRecord {
            action_id: "neo.fixture.tweak".to_string(),
            outcome: ApplyOutcome::Failure,
            detail: "backend failed after state changed".to_string(),
            machine_changed: true,
            reboot_required: false,
        })
        .unwrap();
    assert_eq!(checkpoint.stage(), TransactionStage::RollingBack);
}
'''
    if text.count(marker) != 1:
        raise SystemExit("machine-change tests insertion anchor mismatch")
    tests.write_text(text.replace(marker, added + marker, 1), encoding="utf-8")


if __name__ == "__main__":
    main()
