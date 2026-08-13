#[test]
fn phase9_contract_has_twenty_frozen_assessment_lanes() {
    let review = include_str!("../../../docs/PHASE9_20_LANE_REVIEW.md");
    let decision =
        include_str!("../../../docs/decisions/0009-PHASE9-STATE-ASSESSMENT-FOUNDATION.md");

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
}
