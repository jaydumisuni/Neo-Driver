#[test]
fn phase9_contract_has_twenty_frozen_assessment_lanes() {
    let review = include_str!("../../../docs/PHASE9_20_LANE_REVIEW.md");
    let decision =
        include_str!("../../../docs/decisions/0009-PHASE9-STATE-ASSESSMENT-FOUNDATION.md");
    let state_io = concat!(
        include_str!("../src/model/catalogue.rs"),
        include_str!("../src/model/evidence.rs"),
    );
    let proof_cli = include_str!("../../neo-cli/src/state_assess_cli.rs");

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

    assert_eq!(state_io.matches("std::fs::").count(), 2);
    assert_eq!(state_io.matches("std::fs::read_to_string").count(), 2);
    assert!(!proof_cli.contains("std::fs::"));
}
