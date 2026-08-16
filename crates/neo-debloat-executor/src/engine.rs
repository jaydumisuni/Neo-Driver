use crate::{DebloatExecutionError, DebloatExecutionSession};
use neo_debloat_plan::{ExactAppxInventory, ExactPackageDependency};
use neo_transaction::{
    ApplyOutcome, ApplyRecord, CapturedValue, Observation, ObservedValue, RollbackRecord,
    StateTarget, StateTargetKind, TransactionAuthorization, TransactionStage,
};

pub(crate) trait DebloatHost {
    fn current_inventory(&self) -> Result<ExactAppxInventory, DebloatExecutionError>;
    fn remove_current_user(&mut self, package_full_name: &str)
        -> Result<(), DebloatExecutionError>;
    fn register_current_user(
        &mut self,
        package_full_name: &str,
        dependency_full_names: &[String],
    ) -> Result<(), DebloatExecutionError>;
}

pub(crate) fn authorize_with_host<H: DebloatHost>(
    session: &mut DebloatExecutionSession,
    authorization: TransactionAuthorization,
    host: &H,
) -> Result<(), DebloatExecutionError> {
    ensure_baseline_unchanged(session, host)?;
    session.checkpoint.authorize(authorization)?;
    Ok(())
}

pub(crate) fn apply_with_host<H: DebloatHost>(
    session: &mut DebloatExecutionSession,
    host: &mut H,
) -> Result<(), DebloatExecutionError> {
    ensure_baseline_unchanged(session, host)?;
    session.checkpoint.begin_apply()?;

    let step = session.plan.step().clone();
    session
        .checkpoint
        .assert_action_pending(step.debloat_id())?;
    let removal_result = host.remove_current_user(step.package_full_name());
    let observed_after = observe_all(session, host);
    let machine_changed = observed_after
        .as_ref()
        .map(|observations| any_target_changed_from_baseline(session, observations))
        .unwrap_or(true);

    match removal_result {
        Ok(()) => {
            session.checkpoint.record_apply_result(ApplyRecord {
                action_id: step.debloat_id().to_string(),
                outcome: ApplyOutcome::Success,
                detail: "native current-user PackageManager removal completed".to_string(),
                machine_changed,
                reboot_required: false,
            })?;
        }
        Err(error) => {
            session.checkpoint.record_apply_result(ApplyRecord {
                action_id: step.debloat_id().to_string(),
                outcome: ApplyOutcome::Failure,
                detail: format!("native current-user PackageManager removal failed: {error}"),
                machine_changed,
                reboot_required: false,
            })?;
            if session.stage() == TransactionStage::RollingBack {
                rollback_with_host(session, host)?;
            }
            return Err(error);
        }
    }

    if session.stage() == TransactionStage::Verifying {
        let observations = match observed_after {
            Ok(observations) => observations,
            Err(error) => unavailable_observations(session, &error.to_string()),
        };
        session.checkpoint.verify_postconditions(observations)?;
    }

    if session.stage() == TransactionStage::RollingBack {
        rollback_with_host(session, host)?;
        return Err(DebloatExecutionError::Observation(
            "current-user AppX removal postcondition was not proven; captured baseline was restored"
                .to_string(),
        ));
    }

    if session.stage() != TransactionStage::Complete {
        return Err(DebloatExecutionError::InvalidPreparedState(format!(
            "unexpected terminal stage after AppX apply: {:?}",
            session.stage()
        )));
    }
    Ok(())
}

fn rollback_with_host<H: DebloatHost>(
    session: &mut DebloatExecutionSession,
    host: &mut H,
) -> Result<(), DebloatExecutionError> {
    let step = session.plan.step().clone();
    let restore_result =
        host.register_current_user(step.package_full_name(), step.dependency_full_names());
    match restore_result {
        Ok(()) => session.checkpoint.record_rollback_result(RollbackRecord {
            action_id: step.debloat_id().to_string(),
            outcome: ApplyOutcome::Success,
            detail: "native staged full-name re-registration completed".to_string(),
            reboot_required: false,
        })?,
        Err(error) => {
            session.checkpoint.record_rollback_result(RollbackRecord {
                action_id: step.debloat_id().to_string(),
                outcome: ApplyOutcome::Failure,
                detail: format!("native staged full-name re-registration failed: {error}"),
                reboot_required: false,
            })?;
            return Err(error);
        }
    }

    let observations = match observe_all(session, host) {
        Ok(observations) => observations,
        Err(error) => unavailable_observations(session, &error.to_string()),
    };
    if session.stage() == TransactionStage::RollingBack {
        session.checkpoint.verify_rollback(observations)?;
    }
    if session.stage() != TransactionStage::RolledBack {
        return Err(DebloatExecutionError::Observation(
            "captured AppX baseline was not proven after rollback".to_string(),
        ));
    }
    Ok(())
}

fn ensure_baseline_unchanged<H: DebloatHost>(
    session: &DebloatExecutionSession,
    host: &H,
) -> Result<(), DebloatExecutionError> {
    let inventory = host.current_inventory()?;
    for target in session_targets(session) {
        let actual = captured_value_for_target(session, &inventory, &target)?;
        let expected = session
            .checkpoint
            .baseline()
            .and_then(|baseline| baseline.get(&target))
            .ok_or_else(|| {
                DebloatExecutionError::InvalidPreparedState(format!(
                    "missing captured baseline for {}",
                    target.key
                ))
            })?;
        if &actual != expected {
            return Err(DebloatExecutionError::BaselineDrift(target.key));
        }
    }
    Ok(())
}

fn observe_all<H: DebloatHost>(
    session: &DebloatExecutionSession,
    host: &H,
) -> Result<Vec<Observation>, DebloatExecutionError> {
    let inventory = host.current_inventory()?;
    session_targets(session)
        .into_iter()
        .map(|target| {
            let value = observed_value_for_target(session, &inventory, &target)?;
            Ok(Observation { target, value })
        })
        .collect()
}

fn unavailable_observations(session: &DebloatExecutionSession, reason: &str) -> Vec<Observation> {
    session_targets(session)
        .into_iter()
        .map(|target| Observation {
            target,
            value: ObservedValue::Unavailable(reason.to_string()),
        })
        .collect()
}

fn any_target_changed_from_baseline(
    session: &DebloatExecutionSession,
    observations: &[Observation],
) -> bool {
    observations.iter().any(|observation| {
        let Some(expected) = session
            .checkpoint
            .baseline()
            .and_then(|baseline| baseline.get(&observation.target))
        else {
            return true;
        };
        !observed_matches_captured(&observation.value, expected)
    })
}

fn observed_matches_captured(observed: &ObservedValue, captured: &CapturedValue) -> bool {
    match (observed, captured) {
        (ObservedValue::Present(left), CapturedValue::Present(right)) => left == right,
        (ObservedValue::Absent, CapturedValue::Absent) => true,
        _ => false,
    }
}

fn session_targets(session: &DebloatExecutionSession) -> Vec<StateTarget> {
    let step = session.plan.step();
    std::iter::once(step.package_full_name())
        .chain(step.dependency_full_names().iter().map(String::as_str))
        .map(appx_target)
        .collect()
}

fn captured_value_for_target(
    session: &DebloatExecutionSession,
    inventory: &ExactAppxInventory,
    target: &StateTarget,
) -> Result<CapturedValue, DebloatExecutionError> {
    Ok(
        match observed_value_for_target(session, inventory, target)? {
            ObservedValue::Present(value) => CapturedValue::Present(value),
            ObservedValue::Absent => CapturedValue::Absent,
            ObservedValue::Unavailable(reason) => CapturedValue::Unavailable(reason),
        },
    )
}

fn observed_value_for_target(
    session: &DebloatExecutionSession,
    inventory: &ExactAppxInventory,
    target: &StateTarget,
) -> Result<ObservedValue, DebloatExecutionError> {
    let full_name = target
        .key
        .strip_prefix("current_user:")
        .ok_or_else(|| DebloatExecutionError::InvalidPreparedState(target.key.clone()))?;
    let matches = inventory
        .current_user
        .iter()
        .filter(|package| package.full_name.eq_ignore_ascii_case(full_name))
        .collect::<Vec<_>>();
    let package = match matches.as_slice() {
        [] => return Ok(ObservedValue::Absent),
        [package] => *package,
        _ => {
            return Err(DebloatExecutionError::Observation(format!(
                "duplicate current-user full name {full_name}"
            )))
        }
    };

    if full_name.eq_ignore_ascii_case(session.plan.step().package_full_name()) {
        return Ok(ObservedValue::Present(serde_json::to_string(package)?));
    }

    let dependency = ExactPackageDependency {
        name: package.name.clone(),
        full_name: package.full_name.clone(),
        family_name: package.family_name.clone(),
    };
    Ok(ObservedValue::Present(serde_json::to_string(&dependency)?))
}

fn appx_target(full_name: &str) -> StateTarget {
    StateTarget {
        kind: StateTargetKind::AppxPackage,
        key: format!("current_user:{full_name}"),
    }
}
