use crate::engine::{prepare_with_host, TweakHost};
use crate::model::{RegistrySnapshot, RegistryTweakSpec};
use crate::session::{apply_with_host, authorize_with_host};
use crate::{TweakExecutionError, TweakExecutionSession, SHOW_FILE_EXTENSIONS, SHOW_HIDDEN_FILES};
use neo_core::{EvidenceVerdict, RebootRequirement, RecommendationState, RiskLevel};
use neo_state_plan::{TweakCatalogue, TweakDefinition, TweakOperation, TweakTarget, TweakValue};
use neo_transaction::{TransactionAuthorization, TransactionStage};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
struct ReviewHost {
    values: BTreeMap<String, RegistrySnapshot>,
    corrupt_write: bool,
    restore_fail_ids: BTreeSet<String>,
    restore_attempts: Vec<String>,
}

impl ReviewHost {
    fn value(&self, id: &str) -> RegistrySnapshot {
        self.values
            .get(id)
            .copied()
            .unwrap_or(RegistrySnapshot::Absent)
    }
}

impl TweakHost for ReviewHost {
    fn read(&self, spec: RegistryTweakSpec) -> Result<RegistrySnapshot, TweakExecutionError> {
        Ok(self.value(spec.id))
    }

    fn write_dword(
        &mut self,
        spec: RegistryTweakSpec,
        value: u32,
    ) -> Result<(), TweakExecutionError> {
        self.values.insert(
            spec.id.to_string(),
            RegistrySnapshot::Dword(if self.corrupt_write { 99 } else { value }),
        );
        Ok(())
    }

    fn restore(
        &mut self,
        spec: RegistryTweakSpec,
        baseline: RegistrySnapshot,
    ) -> Result<(), TweakExecutionError> {
        self.restore_attempts.push(spec.id.to_string());
        if self.restore_fail_ids.contains(spec.id) {
            return Err(TweakExecutionError::Registry(format!(
                "synthetic restore failure for {}",
                spec.id
            )));
        }
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
        benefit: "review proof".to_string(),
        tradeoff: "changes one current-user preference".to_string(),
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

fn authorization(session: &TweakExecutionSession) -> TransactionAuthorization {
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
fn contradictory_curated_semantics_fail_closed() {
    for (id, contradictory) in [
        (SHOW_FILE_EXTENSIONS, 1),
        (SHOW_HIDDEN_FILES, 0),
        (crate::TASKBAR_CENTERED_ICONS, 0),
    ] {
        let catalogue = TweakCatalogue::new(vec![definition(id, contradictory)]).unwrap();
        let host = ReviewHost::default();
        assert!(matches!(
            prepare_with_host(&catalogue, &[id.to_string()], "review-semantic", &host),
            Err(TweakExecutionError::UnsupportedOperation(_))
        ));
    }
}

#[test]
fn rollback_attempts_all_changed_tweaks_after_restore_failure() {
    let mut host = ReviewHost {
        corrupt_write: true,
        ..Default::default()
    };
    host.values
        .insert(SHOW_FILE_EXTENSIONS.to_string(), RegistrySnapshot::Dword(1));
    host.values
        .insert(SHOW_HIDDEN_FILES.to_string(), RegistrySnapshot::Dword(0));
    host.restore_fail_ids
        .insert(SHOW_FILE_EXTENSIONS.to_string());

    let catalogue = TweakCatalogue::new(vec![
        definition(SHOW_FILE_EXTENSIONS, 0),
        definition(SHOW_HIDDEN_FILES, 1),
    ])
    .unwrap();
    let selected = vec![
        SHOW_FILE_EXTENSIONS.to_string(),
        SHOW_HIDDEN_FILES.to_string(),
    ];
    let mut session = prepare_with_host(&catalogue, &selected, "review-rollback", &host).unwrap();
    let auth = authorization(&session);
    authorize_with_host(&mut session, auth, &host).unwrap();

    assert!(apply_with_host(&mut session, &mut host).is_err());
    assert_eq!(session.stage(), TransactionStage::Failed);
    assert_eq!(
        host.restore_attempts,
        vec![
            SHOW_FILE_EXTENSIONS.to_string(),
            SHOW_HIDDEN_FILES.to_string()
        ]
    );
    assert_eq!(host.value(SHOW_HIDDEN_FILES), RegistrySnapshot::Dword(0));
    assert_eq!(
        host.value(SHOW_FILE_EXTENSIONS),
        RegistrySnapshot::Dword(99)
    );

    let encoded = serde_json::to_value(session.checkpoint()).unwrap();
    assert_eq!(encoded["rollback_records"].as_array().unwrap().len(), 2);
}
