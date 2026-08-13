use neo_state_plan::{StatePlanError, TweakTarget};

#[test]
fn non_ascii_state_keys_fail_closed() {
    let target = TweakTarget {
        key: "fixture.Ä".to_string(),
    };
    assert!(matches!(
        target.validate(),
        Err(StatePlanError::InvalidTarget(_))
    ));
}
