use crate::model::appx_target;
use crate::{DebloatRestoreExecutionError, DebloatRestoreExecutionSession};
use neo_debloat_plan::{ExactAppxInventory, ExactPackageDependency, ExactPackageIdentity};
use neo_transaction::{
    ApplyOutcome, ApplyRecord, CapturedValue, Observation, ObservedValue, RollbackRecord,
    StateTarget, TransactionAuthorization, TransactionStage,
};

pub(crate) trait DebloatRestoreHost {
    fn current_inventory(&self) -> Result<ExactAppxInventory, DebloatRestoreExecutionError>;
    fn register_current_user(
        &mut self,
        package_full_name: &str,
        dependency_full_names: &[String],
    ) -> Result<(), DebloatRestoreExecutionError>;
    fn remove_current_user(
        &mut self,
        package_full_name: &str,
    ) -> Result<(), DebloatRestoreExecutionError>;
}

pub(crate) fn authorize_with_host<H: DebloatRestoreHost>(
    session: &mut DebloatRestoreExecutionSession,
    authorization: TransactionAuthorization,
    host: &H,
) -> Result<(), DebloatRestoreExecutionError> {
    ensure_execution_state_unchanged(session, host)?;
    session.checkpoint.authorize(authorization)?;
    Ok(())
}

pub(crate) fn apply_with_host<H: DebloatRestoreHost>(
    session: &mut DebloatRestoreExecutionSession,
    host: &mut H,
) -> Result<(), DebloatRestoreExecutionError> {
    ensure_execution_state_unchanged(session, host)?;
    session.checkpoint.begin_apply()?;

    let step = session.plan.step().clone();
    let action_id = step.action_id();
    session.checkpoint.assert_action_pending(&action_id)?;
    let restore_result =
        host.register_current_user(step.package_full_name(), step.dependency_full_names());
    let observed_after = observe_all(session, host);
    let post_write_observation_error = observed_after.as_ref().err().map(ToString::to_string);
    let machine_changed = observed_after
        .as_ref()
        .map(|observations| any_target_changed_from_baseline(session, observations))
        .unwrap_or(true);

    match restore_result {
        Ok(()) => {
            session.checkpoint.record_apply_result(ApplyRecord {
                action_id: action_id.clone(),
                outcome: ApplyOutcome::Success,
                detail: "native staged full-name current-user AppX restore completed".to_string(),
                machine_changed,
                reboot_required: false,
            })?;
        }
        Err(error) => {
            session.checkpoint.record_apply_result(ApplyRecord {
                action_id: action_id.clone(),
                outcome: ApplyOutcome::Failure,
                detail: format!(
                    "native staged full-name current-user AppX restore failed: {error}"
                ),
                machine_changed,
                reboot_required: false,
            })?;
            if session.stage() == TransactionStage::RollingBack {
                if let Err(rollback_error) = rollback_with_host(session, host) {
                    return Err(DebloatRestoreExecutionError::NativeDeployment(format!(
                        "restore failed: {error}; rollback also failed: {rollback_error}"
                    )));
                }
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
        if let Some(reason) = post_write_observation_error {
            return Err(DebloatRestoreExecutionError::Observation(format!(
                "post-write AppX restore observation failed: {reason}; restore-time baseline was restored"
            )));
        }
        return Err(DebloatRestoreExecutionError::Observation(
            "post-success AppX restore postconditions were not proven; restore-time baseline was restored"
                .to_string(),
        ));
    }

    if session.stage() != TransactionStage::Complete {
        return Err(DebloatRestoreExecutionError::InvalidPreparedState(format!(
            "unexpected terminal stage after AppX restore: {:?}",
            session.stage()
        )));
    }
    Ok(())
}

fn rollback_with_host<H: DebloatRestoreHost>(
    session: &mut DebloatRestoreExecutionSession,
    host: &mut H,
) -> Result<(), DebloatRestoreExecutionError> {
    let action_id = session.plan.step().action_id();
    let rollback_result = apply_restore_time_baseline(session, host);
    match rollback_result {
        Ok(()) => session.checkpoint.record_rollback_result(RollbackRecord {
            action_id,
            outcome: ApplyOutcome::Success,
            detail: "removed restored main and only restore-introduced dependencies".to_string(),
            reboot_required: false,
        })?,
        Err(error) => {
            session.checkpoint.record_rollback_result(RollbackRecord {
                action_id,
                outcome: ApplyOutcome::Failure,
                detail: format!("restore-time AppX baseline recovery failed: {error}"),
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
        return Err(DebloatRestoreExecutionError::Observation(
            "restore-time AppX baseline was not proven after rollback".to_string(),
        ));
    }
    Ok(())
}

fn apply_restore_time_baseline<H: DebloatRestoreHost>(
    session: &DebloatRestoreExecutionSession,
    host: &mut H,
) -> Result<(), DebloatRestoreExecutionError> {
    let inventory = host.current_inventory()?;
    let step = session.plan.step();
    let baseline = session.checkpoint.baseline().ok_or_else(|| {
        DebloatRestoreExecutionError::InvalidPreparedState(
            "restore-time baseline is missing during rollback".to_string(),
        )
    })?;

    if has_current_full_name(&inventory, step.package_full_name()) {
        host.remove_current_user(step.package_full_name())?;
    }

    for dependency in step.dependencies().iter().rev() {
        let target = appx_target(&dependency.full_name);
        match baseline.get(&target) {
            Some(CapturedValue::Present(_)) => {
                if !has_current_full_name(&host.current_inventory()?, &dependency.full_name) {
                    host.register_current_user(&dependency.full_name, &[])?;
                }
            }
            Some(CapturedValue::Absent) => {
                if has_current_full_name(&host.current_inventory()?, &dependency.full_name) {
                    host.remove_current_user(&dependency.full_name)?;
                }
            }
            _ => {
                return Err(DebloatRestoreExecutionError::InvalidPreparedState(format!(
                    "rollback baseline for dependency {} is unavailable or missing",
                    dependency.full_name
                )))
            }
        }
    }
    Ok(())
}

fn ensure_execution_state_unchanged<H: DebloatRestoreHost>(
    session: &DebloatRestoreExecutionSession,
    host: &H,
) -> Result<(), DebloatRestoreExecutionError> {
    let inventory = host.current_inventory()?;
    inventory.validate()?;
    ensure_current_baseline_matches(session, &inventory)?;
    ensure_no_side_by_side_current_conflicts(session, &inventory)?;
    ensure_exact_staged_route(session, &inventory)?;
    Ok(())
}

fn ensure_current_baseline_matches(
    session: &DebloatRestoreExecutionSession,
    inventory: &ExactAppxInventory,
) -> Result<(), DebloatRestoreExecutionError> {
    for target in session_targets(session) {
        let actual = captured_value_for_target(session, inventory, &target)?;
        let expected = session
            .checkpoint
            .baseline()
            .and_then(|baseline| baseline.get(&target))
            .ok_or_else(|| {
                DebloatRestoreExecutionError::InvalidPreparedState(format!(
                    "missing restore-time captured baseline for {}",
                    target.key
                ))
            })?;
        if &actual != expected {
            return Err(DebloatRestoreExecutionError::BaselineDrift(target.key));
        }
    }
    Ok(())
}

fn ensure_no_side_by_side_current_conflicts(
    session: &DebloatRestoreExecutionSession,
    inventory: &ExactAppxInventory,
) -> Result<(), DebloatRestoreExecutionError> {
    let step = session.plan.step();
    for current in &inventory.current_user {
        if current.name.eq_ignore_ascii_case(&step.main().name)
            || current
                .family_name
                .eq_ignore_ascii_case(&step.main().family_name)
        {
            return Err(DebloatRestoreExecutionError::BaselineDrift(format!(
                "conflicting current-user main identity {} appeared after Phase 17 preparation",
                current.full_name
            )));
        }
    }
    for dependency in step.dependencies() {
        for current in &inventory.current_user {
            if current
                .full_name
                .eq_ignore_ascii_case(&dependency.full_name)
            {
                continue;
            }
            if current.name.eq_ignore_ascii_case(&dependency.name)
                || current
                    .family_name
                    .eq_ignore_ascii_case(&dependency.family_name)
            {
                return Err(DebloatRestoreExecutionError::BaselineDrift(format!(
                    "conflicting current-user dependency identity {} appeared after Phase 17 preparation",
                    current.full_name
                )));
            }
        }
    }
    Ok(())
}

fn ensure_exact_staged_route(
    session: &DebloatRestoreExecutionSession,
    inventory: &ExactAppxInventory,
) -> Result<(), DebloatRestoreExecutionError> {
    let step = session.plan.step();
    let main_matches = inventory
        .provisioned
        .iter()
        .filter(|package| {
            package
                .full_name
                .eq_ignore_ascii_case(step.package_full_name())
                && package
                    .family_name
                    .eq_ignore_ascii_case(step.package_family_name())
        })
        .collect::<Vec<_>>();
    let main = match main_matches.as_slice() {
        [package] => *package,
        [] => {
            return Err(DebloatRestoreExecutionError::RestoreRouteDrift(format!(
                "exact staged main {} disappeared after Phase 17 preparation",
                step.package_full_name()
            )))
        }
        _ => {
            return Err(DebloatRestoreExecutionError::RestoreRouteDrift(format!(
                "multiple exact staged main identities match {}",
                step.package_full_name()
            )))
        }
    };
    if !same_main_shape(main, step.main()) {
        return Err(DebloatRestoreExecutionError::RestoreRouteDrift(format!(
            "exact staged main {} changed identity shape after Phase 17 preparation",
            step.package_full_name()
        )));
    }

    for dependency in step.dependencies() {
        let matches = inventory
            .provisioned
            .iter()
            .filter(|package| {
                package
                    .full_name
                    .eq_ignore_ascii_case(&dependency.full_name)
                    && package
                        .family_name
                        .eq_ignore_ascii_case(&dependency.family_name)
            })
            .collect::<Vec<_>>();
        match matches.as_slice() {
            [package] if package.name.eq_ignore_ascii_case(&dependency.name) => {}
            [package] => {
                return Err(DebloatRestoreExecutionError::RestoreRouteDrift(format!(
                    "staged dependency {} changed package name to {}",
                    dependency.full_name, package.name
                )))
            }
            [] => {
                return Err(DebloatRestoreExecutionError::RestoreRouteDrift(format!(
                    "exact staged dependency {} disappeared after Phase 17 preparation",
                    dependency.full_name
                )))
            }
            _ => {
                return Err(DebloatRestoreExecutionError::RestoreRouteDrift(format!(
                    "multiple exact staged dependency identities match {}",
                    dependency.full_name
                )))
            }
        }
    }
    Ok(())
}

fn observe_all<H: DebloatRestoreHost>(
    session: &DebloatRestoreExecutionSession,
    host: &H,
) -> Result<Vec<Observation>, DebloatRestoreExecutionError> {
    let inventory = host.current_inventory()?;
    session_targets(session)
        .into_iter()
        .map(|target| {
            let value = observed_value_for_target(session, &inventory, &target)?;
            Ok(Observation { target, value })
        })
        .collect()
}

fn unavailable_observations(
    session: &DebloatRestoreExecutionSession,
    reason: &str,
) -> Vec<Observation> {
    session_targets(session)
        .into_iter()
        .map(|target| Observation {
            target,
            value: ObservedValue::Unavailable(reason.to_string()),
        })
        .collect()
}

fn any_target_changed_from_baseline(
    session: &DebloatRestoreExecutionSession,
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

fn session_targets(session: &DebloatRestoreExecutionSession) -> Vec<StateTarget> {
    let step = session.plan.step();
    std::iter::once(step.package_full_name())
        .chain(step.dependency_full_names().iter().map(String::as_str))
        .map(appx_target)
        .collect()
}

fn captured_value_for_target(
    session: &DebloatRestoreExecutionSession,
    inventory: &ExactAppxInventory,
    target: &StateTarget,
) -> Result<CapturedValue, DebloatRestoreExecutionError> {
    Ok(
        match observed_value_for_target(session, inventory, target)? {
            ObservedValue::Present(value) => CapturedValue::Present(value),
            ObservedValue::Absent => CapturedValue::Absent,
            ObservedValue::Unavailable(reason) => CapturedValue::Unavailable(reason),
        },
    )
}

fn observed_value_for_target(
    session: &DebloatRestoreExecutionSession,
    inventory: &ExactAppxInventory,
    target: &StateTarget,
) -> Result<ObservedValue, DebloatRestoreExecutionError> {
    let full_name = target
        .key
        .strip_prefix("current_user:")
        .ok_or_else(|| DebloatRestoreExecutionError::InvalidPreparedState(target.key.clone()))?;
    let matches = inventory
        .current_user
        .iter()
        .filter(|package| package.full_name.eq_ignore_ascii_case(full_name))
        .collect::<Vec<_>>();
    let package = match matches.as_slice() {
        [] => return Ok(ObservedValue::Absent),
        [package] => *package,
        _ => {
            return Err(DebloatRestoreExecutionError::Observation(format!(
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

fn has_current_full_name(inventory: &ExactAppxInventory, full_name: &str) -> bool {
    inventory
        .current_user
        .iter()
        .any(|package| package.full_name.eq_ignore_ascii_case(full_name))
}

fn same_main_shape(left: &ExactPackageIdentity, right: &ExactPackageIdentity) -> bool {
    left.name.eq_ignore_ascii_case(&right.name)
        && left.full_name.eq_ignore_ascii_case(&right.full_name)
        && left.family_name.eq_ignore_ascii_case(&right.family_name)
        && left.is_framework == right.is_framework
        && left.is_resource == right.is_resource
        && left.is_bundle == right.is_bundle
        && left.is_optional == right.is_optional
        && left.dependencies.len() == right.dependencies.len()
        && left
            .dependencies
            .iter()
            .zip(&right.dependencies)
            .all(|(left, right)| {
                left.name.eq_ignore_ascii_case(&right.name)
                    && left.full_name.eq_ignore_ascii_case(&right.full_name)
                    && left.family_name.eq_ignore_ascii_case(&right.family_name)
            })
}
