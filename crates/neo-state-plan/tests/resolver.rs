use neo_core::{EvidenceVerdict, RebootRequirement, RecommendationState, RiskLevel};
use neo_state_plan::{
    assess_tweaks, resolve_selected_evidence, CapturedState, CapturedStates, ObservedState,
    ReaderId, StateBinding, StateBindings, StatePlanError, TweakCatalogue, TweakDefinition,
    TweakOperation, TweakTarget, TweakValue,
};

fn definition() -> TweakDefinition {
    TweakDefinition {
        id: "fixture.enabled".to_string(),
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
        target: TweakTarget {
            key: "fixture.target".to_string(),
        },
        operation: TweakOperation::Set {
            value: TweakValue::U32(1),
        },
        warnings: vec![],
    }
}

fn catalogue() -> TweakCatalogue {
    TweakCatalogue::new(vec![definition()]).unwrap()
}

fn bindings() -> StateBindings {
    StateBindings::new(vec![StateBinding {
        target: TweakTarget {
            key: "fixture.target".to_string(),
        },
        reader: ReaderId::new("fixture.reader").unwrap(),
    }])
    .unwrap()
}

#[test]
fn reader_id_direct_deserialization_revalidates() {
    assert!(serde_json::from_str::<ReaderId>("\"fixture.reader\"").is_ok());
    assert!(serde_json::from_str::<ReaderId>("\"Fixture.Reader\"").is_err());
    assert!(serde_json::from_str::<ReaderId>("\"fixture reader\"").is_err());
}

#[test]
fn duplicate_bindings_fail_case_insensitively() {
    assert!(matches!(
        StateBindings::new(vec![
            StateBinding {
                target: TweakTarget {
                    key: "Fixture.Target".to_string()
                },
                reader: ReaderId::new("fixture.one").unwrap()
            },
            StateBinding {
                target: TweakTarget {
                    key: "fixture.target".to_string()
                },
                reader: ReaderId::new("fixture.two").unwrap()
            },
        ]),
        Err(StatePlanError::DuplicateBinding(_))
    ));
}

#[test]
fn missing_capture_is_unavailable() {
    let selected = vec!["fixture.enabled".to_string()];
    let evidence = resolve_selected_evidence(
        &catalogue(),
        &bindings(),
        &CapturedStates::new(vec![]).unwrap(),
        &selected,
    )
    .unwrap();
    assert!(matches!(
        evidence.observations[0].state,
        ObservedState::Unavailable { .. }
    ));
    assert!(matches!(
        assess_tweaks(&catalogue(), &evidence, &selected, "mission"),
        Err(StatePlanError::UnavailableObservation { .. })
    ));
}

#[test]
fn captured_state_keeps_provenance() {
    let selected = vec!["fixture.enabled".to_string()];
    let captured = CapturedStates::new(vec![CapturedState {
        reader: ReaderId::new("fixture.reader").unwrap(),
        state: ObservedState::Present {
            value: TweakValue::U32(0),
        },
        source: "fixture-source".to_string(),
    }])
    .unwrap();
    let evidence =
        resolve_selected_evidence(&catalogue(), &bindings(), &captured, &selected).unwrap();
    assert_eq!(evidence.observations[0].target.key, "fixture.target");
    assert_eq!(evidence.observations[0].source, "fixture-source");
}
