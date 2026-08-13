use super::*;
use neo_core::{EvidenceVerdict, RebootRequirement, RecommendationState, RiskLevel};

fn definition(id: &str, key: &str) -> TweakDefinition {
    TweakDefinition {
        id: id.to_string(),
        title: "Fixture preference".to_string(),
        category: "fixture".to_string(),
        benefit: "Exercises read-only assessment.".to_string(),
        tradeoff: "Fixture data only.".to_string(),
        risk: RiskLevel::Low,
        recommendation: RecommendationState::Recommended,
        verdict: EvidenceVerdict::Certified,
        selected_by_default: true,
        requires_admin: false,
        reboot: RebootRequirement::None,
        target: TweakTarget {
            key: key.to_string(),
        },
        operation: TweakOperation::Set {
            value: TweakValue::U32(1),
        },
        warnings: vec![],
    }
}

fn observation(key: &str, state: ObservedState) -> TweakObservation {
    TweakObservation {
        target: TweakTarget {
            key: key.to_string(),
        },
        state,
        source: "fixture".to_string(),
    }
}

#[test]
fn catalogue_round_trip_revalidates() {
    let catalogue =
        TweakCatalogue::new(vec![definition("fixture.enabled", "fixture.target")]).unwrap();
    let json = serde_json::to_string(&catalogue).unwrap();
    let parsed: TweakCatalogue = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, catalogue);
}

#[test]
fn duplicate_ids_fail_closed() {
    let first = definition("fixture.enabled", "fixture.one");
    let second = definition("fixture.enabled", "fixture.two");
    assert!(matches!(
        TweakCatalogue::new(vec![first, second]),
        Err(StatePlanError::DuplicateId(_))
    ));
}

#[test]
fn duplicate_targets_are_case_insensitive() {
    let first = definition("fixture.one", "Fixture.Target");
    let second = definition("fixture.two", "fixture.target");
    assert!(matches!(
        TweakCatalogue::new(vec![first, second]),
        Err(StatePlanError::DuplicateTarget(_))
    ));
}

#[test]
fn high_risk_default_is_rejected() {
    let mut item = definition("fixture.high", "fixture.high");
    item.risk = RiskLevel::High;
    assert!(matches!(
        item.validate(),
        Err(StatePlanError::HighRiskPreselected(_))
    ));
}

#[test]
fn noncertified_default_is_rejected() {
    let mut item = definition("fixture.provisional", "fixture.provisional");
    item.verdict = EvidenceVerdict::Provisional;
    assert!(matches!(
        item.validate(),
        Err(StatePlanError::NonCertifiedPreselected(_))
    ));
}

#[test]
fn unsafe_default_is_rejected() {
    let mut item = definition("fixture.unknown", "fixture.unknown");
    item.recommendation = RecommendationState::Unknown;
    assert!(matches!(
        item.validate(),
        Err(StatePlanError::UnsafeRecommendationPreselected(_))
    ));
}

#[test]
fn duplicate_observations_fail_closed() {
    let first = observation("Fixture.Target", ObservedState::Absent);
    let second = observation("fixture.target", ObservedState::Absent);
    assert!(matches!(
        TweakEvidence::new(vec![first, second]),
        Err(StatePlanError::DuplicateObservation(_))
    ));
}

#[test]
fn explicit_selection_is_required() {
    let catalogue =
        TweakCatalogue::new(vec![definition("fixture.enabled", "fixture.target")]).unwrap();
    let evidence =
        TweakEvidence::new(vec![observation("fixture.target", ObservedState::Absent)]).unwrap();
    assert!(matches!(
        assess_tweaks(&catalogue, &evidence, &[], "mission"),
        Err(StatePlanError::EmptySelection)
    ));
}

#[test]
fn rejected_selection_is_blocked() {
    let mut item = definition("fixture.rejected", "fixture.target");
    item.selected_by_default = false;
    item.verdict = EvidenceVerdict::Rejected;
    let catalogue = TweakCatalogue::new(vec![item]).unwrap();
    let evidence =
        TweakEvidence::new(vec![observation("fixture.target", ObservedState::Absent)]).unwrap();
    let selected = vec!["fixture.rejected".to_string()];
    assert!(matches!(
        assess_tweaks(&catalogue, &evidence, &selected, "mission"),
        Err(StatePlanError::RejectedTweak(_))
    ));
}

#[test]
fn missing_observation_blocks_assessment() {
    let catalogue =
        TweakCatalogue::new(vec![definition("fixture.enabled", "fixture.target")]).unwrap();
    let evidence = TweakEvidence::new(vec![]).unwrap();
    let selected = vec!["fixture.enabled".to_string()];
    assert!(matches!(
        assess_tweaks(&catalogue, &evidence, &selected, "mission"),
        Err(StatePlanError::MissingObservation(_))
    ));
}

#[test]
fn unavailable_observation_blocks_assessment() {
    let catalogue =
        TweakCatalogue::new(vec![definition("fixture.enabled", "fixture.target")]).unwrap();
    let evidence = TweakEvidence::new(vec![observation(
        "fixture.target",
        ObservedState::Unavailable {
            reason: "not observed".to_string(),
        },
    )])
    .unwrap();
    let selected = vec!["fixture.enabled".to_string()];
    assert!(matches!(
        assess_tweaks(&catalogue, &evidence, &selected, "mission"),
        Err(StatePlanError::UnavailableObservation { .. })
    ));
}

#[test]
fn assessment_reports_difference_without_execution() {
    let catalogue =
        TweakCatalogue::new(vec![definition("fixture.enabled", "fixture.target")]).unwrap();
    let evidence = TweakEvidence::new(vec![observation(
        "fixture.target",
        ObservedState::Present {
            value: TweakValue::U32(0),
        },
    )])
    .unwrap();
    let selected = vec!["fixture.enabled".to_string()];
    let report = assess_tweaks(&catalogue, &evidence, &selected, "mission").unwrap();
    assert_eq!(report.items.len(), 1);
    assert!(!report.items[0].already_satisfied);
}

#[test]
fn assessment_reports_already_satisfied_state() {
    let catalogue =
        TweakCatalogue::new(vec![definition("fixture.enabled", "fixture.target")]).unwrap();
    let evidence = TweakEvidence::new(vec![observation(
        "fixture.target",
        ObservedState::Present {
            value: TweakValue::U32(1),
        },
    )])
    .unwrap();
    let selected = vec!["fixture.enabled".to_string()];
    let report = assess_tweaks(&catalogue, &evidence, &selected, "mission").unwrap();
    assert!(report.items[0].already_satisfied);
}
