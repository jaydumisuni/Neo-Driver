use neo_core::{EvidenceVerdict, RecommendationState, RiskLevel};
use neo_debloat::{
    DebloatClass, DebloatDefinition, DebloatError, DebloatProfile, DebloatScope, RestoreMethod,
};

fn definition(class: DebloatClass) -> DebloatDefinition {
    DebloatDefinition {
        id: "appx.fixture".to_string(),
        package_id: "Contoso.Fixture".to_string(),
        title: "Fixture".to_string(),
        category: "Fixture".to_string(),
        description: "Synthetic metadata fixture.".to_string(),
        class,
        scope: DebloatScope::CurrentUser,
        risk: RiskLevel::Normal,
        recommendation: RecommendationState::OptionalComponent,
        verdict: EvidenceVerdict::Certified,
        selected_by_default: false,
        restore: RestoreMethod::Store {
            store_id: "9FIXTURE123".to_string(),
        },
        side_effects: vec![],
        preserve_in_profiles: vec![DebloatProfile::Technician],
    }
}

#[test]
fn safe_optional_may_explicitly_have_no_known_side_effect_note() {
    let mut item = definition(DebloatClass::SafeOptional);
    item.risk = RiskLevel::Low;
    assert_eq!(item.validate(), Ok(()));
}

#[test]
fn feature_dependent_requires_consequence_note() {
    assert!(matches!(
        definition(DebloatClass::FeatureDependent).validate(),
        Err(DebloatError::MissingSideEffectNotes(_))
    ));
}

#[test]
fn dependency_sensitive_requires_consequence_note() {
    assert!(matches!(
        definition(DebloatClass::DependencySensitive).validate(),
        Err(DebloatError::MissingSideEffectNotes(_))
    ));
}

#[test]
fn protected_manual_only_requires_consequence_note() {
    assert!(matches!(
        definition(DebloatClass::ProtectedManualOnly).validate(),
        Err(DebloatError::MissingSideEffectNotes(_))
    ));
}
