use crate::model::{
    fixed_recommendation, spec_for_id, validate_definition, RegistrySnapshot, RegistryTweakSpec,
    TweakExecutionPlan, TweakExecutionSession, TweakExecutionStep,
};
use crate::TweakExecutionError;
use neo_core::{ActionKind, EvidenceItem, EvidenceVerdict, PlannedAction, RebootRequirement};
use neo_state_plan::{
    assess_tweaks, ObservedState, TweakCatalogue, TweakEvidence, TweakObservation, TweakValue,
};
use neo_transaction::{
    CapturedState, CapturedValue, RollbackPlan, StateTarget, StateTargetKind, TransactionAction,
    TransactionCheckpoint, TransactionPlan, VerificationExpectation, VerificationPredicate,
};
use std::collections::BTreeMap;

pub(crate) trait TweakHost {
    fn read(&self, spec: RegistryTweakSpec) -> Result<RegistrySnapshot, TweakExecutionError>;
    fn write_dword(
        &mut self,
        spec: RegistryTweakSpec,
        value: u32,
    ) -> Result<(), TweakExecutionError>;
    fn restore(
        &mut self,
        spec: RegistryTweakSpec,
        baseline: RegistrySnapshot,
    ) -> Result<(), TweakExecutionError>;
}

pub(crate) fn prepare_with_host<H: TweakHost>(
    catalogue: &TweakCatalogue,
    selected_ids: &[String],
    mission_id: impl Into<String>,
    host: &H,
) -> Result<TweakExecutionSession, TweakExecutionError> {
    catalogue.validate()?;
    let mission_id = mission_id.into();
    if mission_id.trim().is_empty() {
        return Err(TweakExecutionError::Registry(
            "mission id must not be empty".to_string(),
        ));
    }

    let mut observations = Vec::with_capacity(selected_ids.len());
    let mut snapshots = BTreeMap::new();
    let mut desired = BTreeMap::new();
    for id in selected_ids {
        let definition = catalogue
            .get(id)
            .ok_or_else(|| neo_state_plan::StatePlanError::UnknownTweak(id.clone()))?;
        let (spec, desired_dword) = validate_definition(definition)?;
        let snapshot = host.read(spec)?;
        observations.push(TweakObservation {
            target: definition.target.clone(),
            state: snapshot_to_observed(snapshot),
            source: format!("phase11-registry:{}", spec.value_name),
        });
        snapshots.insert(id.clone(), snapshot);
        desired.insert(id.clone(), desired_dword);
    }

    let evidence = TweakEvidence::new(observations)?;
    let assessment = assess_tweaks(catalogue, &evidence, selected_ids, mission_id.clone())?;

    let mut steps = Vec::new();
    let mut actions = Vec::new();
    let mut baseline_states = Vec::new();
    for item in &assessment.items {
        if item.already_satisfied {
            continue;
        }
        let definition = catalogue
            .get(&item.id)
            .ok_or_else(|| neo_state_plan::StatePlanError::UnknownTweak(item.id.clone()))?;
        let (spec, desired_dword) = validate_definition(definition)?;
        let baseline = *snapshots
            .get(&item.id)
            .ok_or_else(|| TweakExecutionError::BaselineDrift(item.id.clone()))?;
        if desired.get(&item.id).copied() != Some(desired_dword) {
            return Err(TweakExecutionError::BaselineDrift(item.id.clone()));
        }
        let target = registry_state_target(spec);
        let desired_encoded = RegistrySnapshot::Dword(desired_dword).encoded()?;
        let baseline_value = match baseline {
            RegistrySnapshot::Absent => CapturedValue::Absent,
            RegistrySnapshot::Dword(_) => CapturedValue::Present(baseline.encoded()?),
        };
        baseline_states.push(CapturedState {
            target: target.clone(),
            value: baseline_value,
        });
        actions.push(TransactionAction {
            action: PlannedAction {
                id: item.id.clone(),
                title: spec.title.to_string(),
                kind: ActionKind::Tweak,
                risk: spec.risk,
                recommendation: fixed_recommendation(),
                verdict: EvidenceVerdict::Certified,
                rationale:
                    "Apply a curated current-user Windows preference after exact pre-state capture."
                        .to_string(),
                selected_by_default: false,
                requires_confirmation: true,
                requires_admin: false,
                reboot: RebootRequirement::None,
                rollback_available: true,
                evidence: vec![
                    EvidenceItem::new(
                        "tweak_id",
                        item.id.clone(),
                        "neo-tweak-executor curated binding",
                    ),
                    EvidenceItem::new("baseline", baseline.encoded()?, "live HKCU registry read"),
                    EvidenceItem::new(
                        "desired_dword",
                        desired_dword.to_string(),
                        "certified tweak definition",
                    ),
                ],
                warnings: vec![
                    "Phase 11 does not restart Explorer; UI refresh may occur later.".to_string(),
                ],
            },
            snapshot_targets: vec![target.clone()],
            postconditions: vec![VerificationPredicate {
                id: format!("verify:{}", item.id),
                target: target.clone(),
                expectation: VerificationExpectation::Equals(desired_encoded),
                required: true,
            }],
            rollback: RollbackPlan::Reversible {
                restore_targets: vec![target.clone()],
                verification: vec![VerificationPredicate {
                    id: format!("rollback:{}", item.id),
                    target,
                    expectation: VerificationExpectation::MatchesBaseline,
                    required: true,
                }],
            },
        });
        steps.push(TweakExecutionStep::new(
            item.id.clone(),
            desired_dword,
            baseline,
        ));
    }

    if actions.is_empty() {
        return Err(TweakExecutionError::NothingToChange);
    }
    let transaction = TransactionPlan::new(
        format!("{mission_id}:phase11-tweaks"),
        1,
        mission_id,
        actions,
    )?;
    let mut checkpoint = TransactionCheckpoint::new(transaction.clone())?;
    checkpoint.capture_baseline(baseline_states)?;
    let plan = TweakExecutionPlan::new(assessment, steps, transaction)?;
    TweakExecutionSession::new(plan, checkpoint)
}

pub(crate) fn registry_state_target(spec: RegistryTweakSpec) -> StateTarget {
    StateTarget {
        kind: StateTargetKind::RegistryValue,
        key: spec.state_target_key(),
    }
}

pub(crate) fn snapshot_to_observed(snapshot: RegistrySnapshot) -> ObservedState {
    match snapshot {
        RegistrySnapshot::Absent => ObservedState::Absent,
        RegistrySnapshot::Dword(value) => ObservedState::Present {
            value: TweakValue::U32(value),
        },
    }
}

pub(crate) fn baseline_snapshot(
    session: &TweakExecutionSession,
    tweak_id: &str,
) -> Result<RegistrySnapshot, TweakExecutionError> {
    let step = session
        .plan
        .steps()
        .iter()
        .find(|step| step.tweak_id() == tweak_id)
        .ok_or_else(|| TweakExecutionError::UnsupportedTweak(tweak_id.to_string()))?;
    Ok(step.baseline())
}

pub(crate) fn spec_for_step(
    step: &TweakExecutionStep,
) -> Result<RegistryTweakSpec, TweakExecutionError> {
    spec_for_id(step.tweak_id())
        .ok_or_else(|| TweakExecutionError::UnsupportedTweak(step.tweak_id().to_string()))
}
