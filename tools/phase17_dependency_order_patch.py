from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    target = Path(path)
    text = target.read_text(encoding="utf-8")
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"expected one marker in {path}, found {count}")
    target.write_text(text.replace(old, new, 1), encoding="utf-8")


replace_once(
    "crates/neo-debloat-history/src/plan.rs",
    '''            {
                return Ok(());
            }
            return Err(DebloatHistoryError::InventoryConflict(format!(
                "exact dependency full name {} has a different current-user identity",
                dependency.full_name
            )));
''',
    '''            {
                continue;
            }
            return Err(DebloatHistoryError::InventoryConflict(format!(
                "exact dependency full name {} has a different current-user identity",
                dependency.full_name
            )));
''',
)

replace_once(
    "crates/neo-debloat-history/src/tests.rs",
    '''#[test]
fn restore_readiness_is_byte_for_byte_non_mutating() {
''',
    '''#[test]
fn side_by_side_dependency_after_exact_match_still_blocks_restore_readiness() {
    let receipt = receipt();
    let mut inventory = restore_inventory(true);
    let mut newer = dependency_identity();
    newer.full_name = "Contoso.Framework_2.0.0.0_x64__contoso".to_string();
    inventory.current_user.push(newer);
    inventory.validate().expect("inventory must remain valid");

    let error = prepare_restore_from_inventory(&receipt, &inventory, "mission-dep-order")
        .expect_err("dependency conflicts must be detected regardless of inventory order");
    assert!(matches!(error, DebloatHistoryError::InventoryConflict(_)));
}

#[test]
fn restore_readiness_is_byte_for_byte_non_mutating() {
''',
)

replace_once(
    "tools/phase17_static_review.py",
    '''        "current dependency version/name/family conflicts fail closed",
        "ensure_dependency_restore_state" in plan
        and "different_current_dependency_version_blocks_restore_readiness" in tests,
''',
    '''        "current dependency version/name/family conflicts fail closed independent of inventory order",
        "ensure_dependency_restore_state" in plan
        and "continue;" in plan
        and "different_current_dependency_version_blocks_restore_readiness" in tests
        and "side_by_side_dependency_after_exact_match_still_blocks_restore_readiness" in tests,
''',
)

replace_once(
    "tools/phase17_static_review.py",
    '''            "different_current_dependency_version_blocks_restore_readiness",
            "restore_readiness_is_byte_for_byte_non_mutating",
''',
    '''            "different_current_dependency_version_blocks_restore_readiness",
            "side_by_side_dependency_after_exact_match_still_blocks_restore_readiness",
            "restore_readiness_is_byte_for_byte_non_mutating",
''',
)

replace_once(
    "docs/PHASE17_20_LANE_REVIEW.md",
    '''14. Different current-user dependency version/name/family conflicts block preparation.
''',
    '''14. Different current-user dependency version/name/family conflicts block preparation regardless of inventory ordering; exact matches never short-circuit the remaining conflict scan.
''',
)
