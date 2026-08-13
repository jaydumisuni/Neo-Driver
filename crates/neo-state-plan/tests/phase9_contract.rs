#[test]
fn phase9_contract_has_twenty_frozen_assessment_lanes() {
    let review = include_str!("../../../docs/PHASE9_20_LANE_REVIEW.md");
    let decision =
        include_str!("../../../docs/decisions/0009-PHASE9-STATE-ASSESSMENT-FOUNDATION.md");
    let production = concat!(
        include_str!("../src/assessment.rs"),
        include_str!("../src/error.rs"),
        include_str!("../src/lib.rs"),
        include_str!("../src/model.rs"),
        include_str!("../src/model/catalogue.rs"),
        include_str!("../src/model/definition.rs"),
        include_str!("../src/model/evidence.rs"),
        include_str!("../src/model/value.rs"),
    );
    let proof_cli = include_str!("../../neo-cli/src/state_assess_cli.rs");
    let behavior_proof = include_str!("../../neo-cli/tests/state_assess_read_only.rs");

    assert!(decision.contains("assessment only"));

    let lanes: Vec<u8> = review
        .lines()
        .filter_map(|line| {
            line.split_once('.')
                .and_then(|(prefix, _)| prefix.parse::<u8>().ok())
        })
        .collect();
    let expected: Vec<u8> = (1..=20).collect();
    assert_eq!(lanes, expected);

    assert_eq!(production.matches("std::fs::").count(), 2);
    assert_eq!(production.matches("std::fs::read_to_string").count(), 2);
    assert!(!production.contains(concat!("std::pro", "cess")));
    assert!(!proof_cli.contains(concat!("std::f", "s::")));
    assert!(!proof_cli.contains(concat!("std::pro", "cess")));

    assert!(behavior_proof.contains("CARGO_BIN_EXE_neo-state-assess"));
    assert!(behavior_proof.contains("snapshot_tree"));
    assert!(behavior_proof.contains("before, after"));
    assert!(
        behavior_proof.contains("state_assess_subcommands_leave_isolated_fixture_tree_unchanged")
    );
}
