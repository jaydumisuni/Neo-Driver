#[test]
fn phase9_contract_has_twenty_frozen_assessment_lanes() {
    let review = include_str!("../../../docs/PHASE9_20_LANE_REVIEW.md");
    let decision =
        include_str!("../../../docs/decisions/0009-PHASE9-STATE-ASSESSMENT-FOUNDATION.md");

    assert!(decision.contains("assessment only"));

    let lanes = review
        .lines()
        .filter(|line| {
            line.split_once('.')
                .map(|(prefix, _)| prefix.parse::<u8>().is_ok())
                .unwrap_or(false)
        })
        .count();
    assert_eq!(lanes, 20);
}
