use super::*;
use crate::engine::{prepare_with_host, TweakHost};
use crate::model::spec_for_id;
use crate::session::{apply_with_host, authorize_with_host};
use neo_core::{EvidenceVerdict, RebootRequirement, RecommendationState, RiskLevel};
use neo_state_plan::{TweakCatalogue, TweakDefinition, TweakOperation, TweakTarget, TweakValue};
use neo_transaction::{TransactionAuthorization, TransactionStage};
use std::collections::BTreeMap;

#[derive(Default)]
struct FakeHost {
    values: BTreeMap<String, RegistrySnapshot>,
    corrupt_write: bool,
    fail_after_change: bool,
}

impl FakeHost {
    fn with(id: &str, value: RegistrySnapshot) -> Self {
        let mut host = Self::default();
        host.values.insert(id.to_string(), value);
        host
    }

    fn value(&self, id: &str) -> RegistrySnapshot {
        self.values
            .get(id)
            .copied()
            .unwrap_or(RegistrySnapshot::Absent)
    }
}

impl TweakHost for FakeHost {
    fn read(
        &self,
        spec: crate::model::RegistryTweakSpec,
    ) -> Result<RegistrySnapshot, TweakExecutionError> {
        Ok(self.value(spec.id))
    }

    fn write_dword(
        &mut self,
        spec: crate::model::RegistryTweakSpec,
        value: u32,
    ) -> Result<(), TweakExecutionError> {
        let stored = if self.corrupt_write { 1 - value } else { value };
        self.values
            .insert(spec.id.to_string(), RegistrySnapshot::Dword(stored));
        if self.fail_after_change {
            return Err(TweakExecutionError::Registry(
                "synthetic write failure after change".to_string(),
            ));
        }
        Ok(())
    }

    fn restore(
        &mut self,
        spec: crate::model::RegistryTweakSpec,
        baseline: RegistrySnapshot,
    ) -> Result<(), TweakExecutionError> {
        match baseline {
            RegistrySnapshot::Absent => {
                self.values.remove(spec.id);
            }
            RegistrySnapshot::Dword(_) => {
                self.values.insert(spec.id.to_string(), baseline);
            }
        }
        Ok(())
    }
}

fn definition(id: &str, desired: u32) -> TweakDefinition {
    TweakDefinition {
        id: id.to_string(),
        title: id.to_string(),
        category: "customize_preferences".to_string(),
        benefit: "Exercise the curated Phase 11 preference.".to_string(),
        tradeoff: "Changes one current-user Explorer preference.".to_string(),
        risk: RiskLevel::Low,
        recommendation: RecommendationState::Recommended,
        verdict: EvidenceVerdict::Certified,
        selected_by_default: false,
        requires_admin: false,
        reboot: RebootRequirement::None,
        target: TweakTarget {
            key: id.to_string(),
        },
        operation: TweakOperation::Set {
            value: TweakValue::U32(desired),
        },
        warnings: vec![],
    }
}

fn catalogue(definitions: Vec<TweakDefinition>) -> TweakCatalogue {
    TweakCatalogue::new(definitions).unwrap()
}

fn auth(session: &TweakExecutionSession) -> TransactionAuthorization {
    TransactionAuthorization {
        plan_fingerprint: session.plan().transaction().fingerprint().unwrap(),
        approved_action_ids: session
            .plan()
            .steps()
            .iter()
            .map(|step| step.tweak_id().to_string())
            .collect(),
        manual_override_action_ids: vec![],
        high_risk_ack_action_ids: vec![],
        irreversible_acknowledgements: vec![],
    }
}

#[test]
fn curated_catalogue_is_exactly_three_tweaks() {
    assert_eq!(
        curated_tweak_ids(),
        [SHOW_FILE_EXTENSIONS, SHOW_HIDDEN_FILES, TASKBAR_CENTERED_ICONS]
    );
    for id in curated_tweak_ids() {
        assert!(spec_for_id(id).is_some());
    }
}

#[test]
fn unsupported_tweak_fails_closed() {
    let definition = definition("windows.other.unapproved", 1);
    let host = FakeHost::default();
    assert!(matches!(
        prepare_with_host(
            &catalogue(vec![definition]),
            &["windows.other.unapproved".to_string()],
            "mission",
            &host,
        ),
        Err(TweakExecutionError::UnsupportedTweak(_))
    ));
}

#[test]
fn non_dword_or_out_of_range_operation_fails_closed() {
    let mut item = definition(SHOW_FILE_EXTENSIONS, 0);
    item.operation = TweakOperation::Set {
        value: TweakValue::Text("0".to_string()),
    };
    let host = FakeHost::with(SHOW_FILE_EXTENSIONS, RegistrySnapshot::Dword(1));
    assert!(matches!(
        prepare_with_host(
            &catalogue(vec![item]),
            &[SHOW_FILE_EXTENSIONS.to_string()],
            "mission",
            &host,
        ),
        Err(TweakExecutionError::UnsupportedOperation(_))
    ));

    let host = FakeHost::with(SHOW_FILE_EXTENSIONS, RegistrySnapshot::Dword(1));
    assert!(matches!(
        prepare_with_host(
            &catalogue(vec![definition(SHOW_FILE_EXTENSIONS, 2)]),
            &[SHOW_FILE_EXTENSIONS.to_string()],
            "mission",
            &host,
        ),
        Err(TweakExecutionError::UnsupportedOperation(_))
    ));
}

#[test]
fn satisfied_selection_does_not_create_mutation_transaction() {
    let host = FakeHost::with(SHOW_FILE_EXTENSIONS, RegistrySnapshot::Dword(0));
    assert!(matches!(
        prepare_with_host(
            &catalogue(vec![definition(SHOW_FILE_EXTENSIONS, 0)]),
            &[SHOW_FILE_EXTENSIONS.to_string()],
            "mission",
            &host,
        ),
        Err(TweakExecutionError::NothingToChange)
    ));
}

#[test]
fn prepare_captures_exact_present_baseline() {
    let host = FakeHost::with(SHOW_FILE_EXTENSIONS, RegistrySnapshot::Dword(1));
    let session = prepare_with_host(
        &catalogue(vec![definition(SHOW_FILE_EXTENSIONS, 0)]),
        &[SHOW_FILE_EXTENSIONS.to_string()],
        "mission",
        &host,
    )
    .unwrap();
    assert_eq!(session.stage(), TransactionStage::BaselineCaptured);
    assert_eq!(session.plan().steps()[0].baseline(), RegistrySnapshot::Dword(1));
}

#[test]
fn prepare_captures_absent_baseline() {
    let host = FakeHost::default();
    let session = prepare_with_host(
        &catalogue(vec![definition(SHOW_HIDDEN_FILES, 1)]),
        &[SHOW_HIDDEN_FILES.to_string()],
        "mission",
        &host,
    )
    .unwrap();
    assert_eq!(session.plan().steps()[0].baseline(), RegistrySnapshot::Absent);
}

#[test]
fn baseline_drift_blocks_authority() {
    let mut host = FakeHost::with(SHOW_FILE_EXTENSIONS, RegistrySnapshot::Dword(1));
    let mut session = prepare_with_host(
        &catalogue(vec![definition(SHOW_FILE_EXTENSIONS, 0)]),
        &[SHOW_FILE_EXTENSIONS.to_string()],
        "mission",
        &host,
    )
    .unwrap();
    host.values.insert(
        SHOW_FILE_EXTENSIONS.to_string(),
        RegistrySnapshot::Dword(0),
    );
    assert!(matches!(
        authorize_with_host(&mut session, auth(&session), &host),
        Err(TweakExecutionError::BaselineDrift(_))
    ));
    assert_eq!(session.stage(), TransactionStage::BaselineCaptured);
}

#[test]
fn baseline_drift_after_authority_blocks_apply_before_write() {
    let mut host = FakeHost::with(SHOW_FILE_EXTENSIONS, RegistrySnapshot::Dword(1));
    let mut session = prepare_with_host(
        &catalogue(vec![definition(SHOW_FILE_EXTENSIONS, 0)]),
        &[SHOW_FILE_EXTENSIONS.to_string()],
        "mission",
        &host,
    )
    .unwrap();
    let authorization = auth(&session);
    authorize_with_host(&mut session, authorization, &host).unwrap();
    host.values.insert(
        SHOW_FILE_EXTENSIONS.to_string(),
        RegistrySnapshot::Dword(0),
    );
    assert!(matches!(
        apply_with_host(&mut session, &mut host),
        Err(TweakExecutionError::BaselineDrift(_))
    ));
    assert_eq!(session.stage(), TransactionStage::Authorized);
}

#[test]
fn successful_apply_requires_fresh_verification() {
    let mut host = FakeHost::with(SHOW_FILE_EXTENSIONS, RegistrySnapshot::Dword(1));
    let mut session = prepare_with_host(
        &catalogue(vec![definition(SHOW_FILE_EXTENSIONS, 0)]),
        &[SHOW_FILE_EXTENSIONS.to_string()],
        "mission",
        &host,
    )
    .unwrap();
    let authorization = auth(&session);
    authorize_with_host(&mut session, authorization, &host).unwrap();
    apply_with_host(&mut session, &mut host).unwrap();
    assert_eq!(session.stage(), TransactionStage::Complete);
    assert_eq!(host.value(SHOW_FILE_EXTENSIONS), RegistrySnapshot::Dword(0));
}

#[test]
fn wrong_post_write_state_rolls_back_exact_present_baseline() {
    let mut host = FakeHost::with(SHOW_FILE_EXTENSIONS, RegistrySnapshot::Dword(1));
    host.corrupt_write = true;
    let mut session = prepare_with_host(
        &catalogue(vec![definition(SHOW_FILE_EXTENSIONS, 0)]),
        &[SHOW_FILE_EXTENSIONS.to_string()],
        "mission",
        &host,
    )
    .unwrap();
    let authorization = auth(&session);
    authorize_with_host(&mut session, authorization, &host).unwrap();
    apply_with_host(&mut session, &mut host).unwrap();
    assert_eq!(session.stage(), TransactionStage::RolledBack);
    assert_eq!(host.value(SHOW_FILE_EXTENSIONS), RegistrySnapshot::Dword(1));
}

#[test]
fn failed_write_after_change_rolls_back_absent_baseline() {
    let mut host = FakeHost::default();
    host.fail_after_change = true;
    let mut session = prepare_with_host(
        &catalogue(vec![definition(SHOW_HIDDEN_FILES, 1)]),
        &[SHOW_HIDDEN_FILES.to_string()],
        "mission",
        &host,
    )
    .unwrap();
    let authorization = auth(&session);
    authorize_with_host(&mut session, authorization, &host).unwrap();
    assert!(apply_with_host(&mut session, &mut host).is_err());
    assert_eq!(session.stage(), TransactionStage::RolledBack);
    assert_eq!(host.value(SHOW_HIDDEN_FILES), RegistrySnapshot::Absent);
}

#[test]
fn multiple_curated_tweaks_complete_in_one_transaction() {
    let mut host = FakeHost::default();
    host.values.insert(
        SHOW_FILE_EXTENSIONS.to_string(),
        RegistrySnapshot::Dword(1),
    );
    host.values.insert(
        SHOW_HIDDEN_FILES.to_string(),
        RegistrySnapshot::Dword(0),
    );
    let catalogue = catalogue(vec![
        definition(SHOW_FILE_EXTENSIONS, 0),
        definition(SHOW_HIDDEN_FILES, 1),
    ]);
    let selected = vec![
        SHOW_FILE_EXTENSIONS.to_string(),
        SHOW_HIDDEN_FILES.to_string(),
    ];
    let mut session = prepare_with_host(&catalogue, &selected, "mission", &host).unwrap();
    let authorization = auth(&session);
    authorize_with_host(&mut session, authorization, &host).unwrap();
    apply_with_host(&mut session, &mut host).unwrap();
    assert_eq!(session.stage(), TransactionStage::Complete);
    assert_eq!(host.value(SHOW_FILE_EXTENSIONS), RegistrySnapshot::Dword(0));
    assert_eq!(host.value(SHOW_HIDDEN_FILES), RegistrySnapshot::Dword(1));
}

#[test]
fn capability_has_no_public_constructor_but_internal_tests_can_issue_it() {
    let _ = TweakExecutorCapability::for_tests();
}
