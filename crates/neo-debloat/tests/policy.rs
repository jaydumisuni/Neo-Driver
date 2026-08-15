use neo_core::{EvidenceVerdict, RecommendationState, RiskLevel};
use neo_debloat::{
    assess_debloat, DebloatCatalogue, DebloatClass, DebloatDefinition, DebloatDisposition,
    DebloatEvidence, DebloatObservation, DebloatProfile, DebloatScope, ObservedPresence,
    RestoreMethod,
};

fn definition() -> DebloatDefinition {
    DebloatDefinition {
        id: "appx.fixture".to_string(),
        package_id: "Contoso.Fixture".to_string(),
        title: "Fixture".to_string(),
        category: "Fixture".to_string(),
        description: "Synthetic policy fixture.".to_string(),
        class: DebloatClass::SafeOptional,
        scope: DebloatScope::CurrentUserAndProvisioned,
        risk: RiskLevel::Low,
        recommendation: RecommendationState::OptionalComponent,
        verdict: EvidenceVerdict::Certified,
        selected_by_default: false,
        restore: RestoreMethod::Store {
            store_id: "9FIXTURE123".to_string(),
        },
        side_effects: vec![],
        preserve_in_profiles: vec![],
    }
}

fn evidence() -> DebloatEvidence {
    DebloatEvidence::new(vec![DebloatObservation {
        package_id: "Contoso.Fixture".to_string(),
        installed: ObservedPresence::Present,
        provisioned: ObservedPresence::Present,
        version: Some("1.0".to_string()),
        source: "synthetic-policy-proof".to_string(),
    }])
    .unwrap()
}

fn disposition(item: DebloatDefinition, profile: DebloatProfile) -> DebloatDisposition {
    let catalogue = DebloatCatalogue::new(vec![item]).unwrap();
    assess_debloat(
        &catalogue,
        &evidence(),
        profile,
        &["appx.fixture".to_string()],
    )
    .unwrap()
    .items[0]
        .disposition
}

#[test]
fn low_certified_optional_restorable_item_is_candidate() {
    assert_eq!(
        disposition(definition(), DebloatProfile::SafeCleanup),
        DebloatDisposition::RemovalCandidate
    );
}

#[test]
fn higher_risk_manual_selection_needs_review() {
    let mut item = definition();
    item.risk = RiskLevel::Normal;
    assert_eq!(
        disposition(item, DebloatProfile::SafeCleanup),
        DebloatDisposition::NeedsReview
    );
}

#[test]
fn provisional_manual_selection_needs_review() {
    let mut item = definition();
    item.verdict = EvidenceVerdict::Provisional;
    assert_eq!(
        disposition(item, DebloatProfile::SafeCleanup),
        DebloatDisposition::NeedsReview
    );
}

#[test]
fn unknown_recommendation_manual_selection_needs_review() {
    let mut item = definition();
    item.recommendation = RecommendationState::Unknown;
    assert_eq!(
        disposition(item, DebloatProfile::SafeCleanup),
        DebloatDisposition::NeedsReview
    );
}

#[test]
fn do_not_touch_manual_selection_is_policy_blocked() {
    let mut item = definition();
    item.recommendation = RecommendationState::DoNotTouch;
    assert_eq!(
        disposition(item, DebloatProfile::SafeCleanup),
        DebloatDisposition::BlockedPolicy
    );
}

#[test]
fn rejected_manual_selection_is_policy_blocked() {
    let mut item = definition();
    item.verdict = EvidenceVerdict::Rejected;
    assert_eq!(
        disposition(item, DebloatProfile::SafeCleanup),
        DebloatDisposition::BlockedPolicy
    );
}

#[test]
fn protected_class_beats_profile_preservation_reason() {
    let mut item = definition();
    item.class = DebloatClass::ProtectedManualOnly;
    item.risk = RiskLevel::High;
    item.recommendation = RecommendationState::DoNotTouch;
    item.restore = RestoreMethod::None;
    item.side_effects = vec!["synthetic protected consequence".to_string()];
    item.preserve_in_profiles = vec![DebloatProfile::Gaming];
    assert_eq!(
        disposition(item, DebloatProfile::Gaming),
        DebloatDisposition::BlockedProtected
    );
}
