use crate::engine::{baseline_snapshot, registry_state_target, spec_for_step, TweakHost};
use crate::model::{RegistrySnapshot, TweakExecutionSession};
use crate::TweakExecutionError;
use neo_transaction::{
    ApplyOutcome, ApplyRecord, Observation, ObservedValue, RollbackRecord, TransactionAuthorization,
    TransactionStage,
};

pub(crate) fn authorize_with_host<H: TweakHost>(
    session: &mut TweakExecutionSession,
    authorization: TransactionAuthorization,
    host: &H,
) -> Result<(), TweakExecutionError> {
    ensure_baseline_unchanged(session, host)?;
    session.checkpoint.authorize(authorization)?;
    Ok(())
}

pub(crate) fn apply_with_host<H: TweakHost>(
    session: &mut TweakExecutionSession,
    host: &mut H,
) -> Result<(), TweakExecutionError> {
    ensure_baseline_unchanged(session, host)?;
    session.checkpoint.begin_apply()?;

    for step in session.plan.steps().to_vec() {
        let spec = spec_for_step(&step)?;
        session.checkpoint.assert_action_pending(step.tweak_id())?;
        let write_result = host.write_dword(spec, step.desired_dword());
        let observed_after = host.read(spec);
        let machine_changed = observed_after
            .as_ref()
            .map(|actual| *actual != step.baseline())
            .unwrap_or(true);
        if machine_changed {
            session.changed_ids.insert(step.tweak_id().to_string());
        }

        match write_result {
            Ok(()) => {
                session.checkpoint.record_apply_result(ApplyRecord {
                    action_id: step.tweak_id().to_string(),
                    outcome: ApplyOutcome::Success,
                    detail: "curated HKCU DWORD write returned success".to_string(),
                    machine_changed,
                    reboot_required: false,
                })?;
            }
            Err(error) => {
                session.checkpoint.record_apply_result(ApplyRecord {
                    action_id: step.tweak_id().to_string(),
                    outcome: ApplyOutcome::Failure,
                    detail: format!("curated HKCU DWORD write failed: {error}"),
                    machine_changed,
                    reboot_required: false,
                })?;
                if session.stage() == TransactionStage::RollingBack {
                    rollback_with_host(session, host)?;
                }
                return Err(error);
            }
        }
    }

    if session.stage() == TransactionStage::Verifying {
        let observations = observe_steps(session, host);
        session.checkpoint.verify_postconditions(observations)?;
    }
    if session.stage() == TransactionStage::RollingBack {
        rollback_with_host(session, host)?;
    }
    Ok(())
}

fn ensure_baseline_unchanged<H: TweakHost>(
    session: &TweakExecutionSession,
    host: &H,
) -> Result<(), TweakExecutionError> {
    for step in session.plan.steps() {
        let spec = spec_for_step(step)?;
        let current = host.read(spec)?;
        if current != baseline_snapshot(session, step.tweak_id())? {
            return Err(TweakExecutionError::BaselineDrift(
                step.tweak_id().to_string(),
            ));
        }
    }
    Ok(())
}

fn observe_steps<H: TweakHost>(
    session: &TweakExecutionSession,
    host: &H,
) -> Vec<Observation> {
    session
        .plan
        .steps()
        .iter()
        .map(|step| {
            let spec = spec_for_step(step).expect("prepared steps are curated");
            let value = match host.read(spec) {
                Ok(snapshot) => snapshot_to_observed_value(snapshot),
                Err(error) => ObservedValue::Unavailable(error.to_string()),
            };
            Observation {
                target: registry_state_target(spec),
                value,
            }
        })
        .collect()
}

fn rollback_with_host<H: TweakHost>(
    session: &mut TweakExecutionSession,
    host: &mut H,
) -> Result<(), TweakExecutionError> {
    let changed = session.changed_ids.clone();
    for step in session.plan.steps().to_vec() {
        if !changed.contains(step.tweak_id()) {
            continue;
        }
        let spec = spec_for_step(&step)?;
        match host.restore(spec, step.baseline()) {
            Ok(()) => session.checkpoint.record_rollback_result(RollbackRecord {
                action_id: step.tweak_id().to_string(),
                outcome: ApplyOutcome::Success,
                detail: "captured HKCU registry baseline restored".to_string(),
                reboot_required: false,
            })?,
            Err(error) => {
                session.checkpoint.record_rollback_result(RollbackRecord {
                    action_id: step.tweak_id().to_string(),
                    outcome: ApplyOutcome::Failure,
                    detail: format!("captured registry baseline restore failed: {error}"),
                    reboot_required: false,
                })?;
                return Err(error);
            }
        }
    }

    if session.stage() == TransactionStage::RollingBack {
        session.checkpoint.verify_rollback(observe_steps(session, host))?;
    }
    Ok(())
}

fn snapshot_to_observed_value(snapshot: RegistrySnapshot) -> ObservedValue {
    match snapshot {
        RegistrySnapshot::Absent => ObservedValue::Absent,
        RegistrySnapshot::Dword(_) => ObservedValue::Present(
            snapshot
                .encoded()
                .expect("RegistrySnapshot serialization is infallible"),
        ),
    }
}
