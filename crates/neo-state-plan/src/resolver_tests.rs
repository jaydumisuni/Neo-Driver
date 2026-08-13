use super::*;
use neo_core::{EvidenceVerdict, RebootRequirement, RecommendationState, RiskLevel};

fn definition(id: &str, key: &str) -> TweakDefinition {
    TweakDefinition {
        id: id.to_string(),
        title: "Fixture preference".to_string(),
        category: "fixture".to_string(),
        benefit: "Exercises state resolution.".to_string(),
        tradeoff: "Fixture only.".to_string(),
        risk: RiskLevel::Low,
        recommendation: RecommendationState::Recommended,
        verdict: EvidenceVerdict::Certified,
        selected_by_default: false,
        requires_admin: false,
        reboot: RebootRequirement::None,
        target: TweakTarget { key: key.to_string() },
        operation: TweakOperation::Set { value: TweakValue::U32(1) },
        warnings: vec![],
    }
}

fn bindings() -> StateBindings {
    StateBindings::new(vec![StateBinding {
        target: TweakTarget { key: "fixture.target".to_string() },
        reader: ReaderId::new("fixture.reader").unwrap(),
    }])
    .unwrap()
}

#[test]
fn duplicate_bindings_fail_case_insensitively() {
    assert!(matches!(
        StateBindings::new(vec![
            StateBinding {
                target: TweakTarget { key: "Fixture.Target".to_string() },
                reader: ReaderId::new("fixture.one").unwrap(),
            },
            StateBinding {
                target: TweakTarget { key: "fixture.target".to_string() },
                reader: ReaderId::new("fixture.two").unwrap(),
            },
        ]),
        Err(StatePlanError::DuplicateBinding(_))
    ));
}

#[test]
fn invalid_reader_id_fails_closed() {
    assert!(matches!(
        ReaderId::new("Fixture Reader"),
        Err(StatePlanError::InvalidReaderId(_))
    ));
}

#[test]
fn direct_deserialization_revalidates_bindings() {
    let json = r#"{
        "bindings": [
          {"target":{"key":"Fixture.Target"},"reader":"fixture.one"},
          {"target":{"key":"fixture.target"},"reader":"fixture.two"}
        ]
    }"#;
    assert!(serde_json::from_str::<StateBindings>(json).is_err());
}

#[test]
fn missing_binding_fails_closed() {
    let catalogue = TweakCatalogue::new(vec![definition("fixture.enabled", "fixture.target")]).unwrap();
    let selected = vec!["fixture.enabled".to_string()];
    let captured = CapturedStates::new(vec![]).unwrap();
    let empty = StateBindings::new(vec![]).unwrap();
    assert!(matches!(
        resolve_selected_evidence(&catalogue, &empty, &captured, &selected),
        Err(StatePlanError::MissingBinding(_))
    ));
}

#[test]
fn missing_capture_becomes_unavailable_evidence() {
    let catalogue = TweakCatalogue::new(vec![definition("fixture.enabled", "fixture.target")]).unwrap();
    let selected = vec!["fixture.enabled".to_string()];
    let captured = CapturedStates::new(vec![]).unwrap();
    let evidence = resolve_selected_evidence(&catalogue, &bindings(), &captured, &selected).unwrap();
    assert!(matches!(
        evidence.observations[0].state,
        ObservedState::Unavailable { .. }
    ));
    assert!(matches!(
        assess_tweaks(&catalogue, &evidence, &selected, "mission"),
        Err(StatePlanError::UnavailableObservation { .. })
    ));
}

#[test]
fn captured_value_preserves_source_and_target() {
    let catalogue = TweakCatalogue::new(vec![definition("fixture.enabled", "fixture.target")]).unwrap();
    let selected = vec!["fixture.enabled".to_string()];
    let captured = CapturedStates::new(vec![CapturedState {
        reader: ReaderId::new("fixture.reader").unwrap(),
        state: ObservedState::Present { value: TweakValue::U32(0) },
        source: "fixture-source".to_string(),
    }])
    .unwrap();
    let evidence = resolve_selected_evidence(&catalogue, &bindings(), &captured, &selected).unwrap();
    assert_eq!(evidence.observations[0].target.key, "fixture.target");
    assert_eq!(evidence.observations[0].source, "fixture-source");
    assert!(matches!(
        evidence.observations[0].state,
        ObservedState::Present { value: TweakValue::U32(0) }
    ));
}
