use super::*;
use neo_core::{EvidenceVerdict, RebootRequirement, RecommendationState, RiskLevel};
use neo_state_plan::{ObservedState, TweakCatalogue, TweakDefinition, TweakOperation, TweakTarget, TweakValue};

fn definition(id: &str, key: &str) -> TweakDefinition {
    TweakDefinition {
        id: id.to_string(),
        title: "Fixture state".to_string(),
        category: "fixture".to_string(),
        benefit: "Exercises evidence resolution.".to_string(),
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
    StateBindings {
        bindings: vec![StateBinding {
            target: TweakTarget { key: "fixture.target".to_string() },
            reader: ReaderId::new("fixture.reader").unwrap(),
        }],
    }
}

#[test]
fn duplicate_bindings_fail_case_insensitively() {
    let bindings = StateBindings {
        bindings: vec![
            StateBinding {
                target: TweakTarget { key: "Fixture.Target".to_string() },
                reader: ReaderId::new("fixture.one").unwrap(),
            },
            StateBinding {
                target: TweakTarget { key: "fixture.target".to_string() },
                reader: ReaderId::new("fixture.two").unwrap(),
            },
        ],
    };
    assert!(matches!(bindings.validate(), Err(StateResolverError::DuplicateBinding(_))));
}

#[test]
fn invalid_reader_id_fails_closed() {
    assert!(matches!(ReaderId::new("Fixture Reader"), Err(StateResolverError::InvalidField(_))));
}

#[test]
fn missing_binding_fails_closed() {
    let catalogue = TweakCatalogue::new(vec![definition("fixture.enabled", "fixture.target")]).unwrap();
    let selected = vec!["fixture.enabled".to_string()];
    let captured = CapturedStates::default();
    let empty = StateBindings { bindings: vec![] };
    assert!(matches!(
        resolve_selected_evidence(&catalogue, &empty, &selected, &captured),
        Err(StateResolverError::MissingBinding(_))
    ));
}

#[test]
fn missing_capture_becomes_unavailable_evidence() {
    let catalogue = TweakCatalogue::new(vec![definition("fixture.enabled", "fixture.target")]).unwrap();
    let selected = vec!["fixture.enabled".to_string()];
    let evidence = resolve_selected_evidence(&catalogue, &bindings(), &selected, &CapturedStates::default()).unwrap();
    assert!(matches!(evidence.observations[0].state, ObservedState::Unavailable { .. }));
}

#[test]
fn captured_value_is_bound_to_selected_target() {
    let catalogue = TweakCatalogue::new(vec![definition("fixture.enabled", "fixture.target")]).unwrap();
    let selected = vec!["fixture.enabled".to_string()];
    let mut captured = CapturedStates::default();
    captured.insert(ReaderId::new("fixture.reader").unwrap(), ObservedState::Present { value: TweakValue::U32(0) });
    let evidence = resolve_selected_evidence(&catalogue, &bindings(), &selected, &captured).unwrap();
    assert_eq!(evidence.observations[0].target.key, "fixture.target");
    assert!(matches!(evidence.observations[0].state, ObservedState::Present { value: TweakValue::U32(0) }));
}
