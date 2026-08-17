use crate::error::RepairError;
use crate::host::RepairHost;
use crate::model::{
    ComponentStoreState, FeatureDesiredState, SystemFileState, WindowsFeatureState,
};
use crate::operation::{RepairBaseline, RepairOperation};
use crate::plan::{feature_baseline_state, target_for, target_value, RepairExecutionPlan};
use neo_transaction::{
    ApplyOutcome, ApplyRecord, Observation, ObservedValue, RollbackRecord,
    TransactionAuthorization, TransactionCheckpoint, TransactionStage,
};
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct RepairExecutorCapability {
    pub(crate) _private: (),
}

impl RepairExecutorCapability {
    pub(crate) fn for_rpc() -> Self {
        Self { _private: () }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepairExecutionSession {
    plan: RepairExecutionPlan,
    checkpoint: TransactionCheckpoint,
}

impl RepairExecutionSession {
    pub(crate) fn prepare_with_host<H: RepairHost>(
        operation: RepairOperation,
        mission_id: impl Into<String>,
        host: &H,
    ) -> Result<Self, RepairError> {
        let mission_id = mission_id.into();
        let plan = match operation {
            RepairOperation::RestoreComponentStore => RepairExecutionPlan::from_component_store(
                &host.observe_component_store()?,
                mission_id,
            )?,
            RepairOperation::RepairSystemFiles => {
                RepairExecutionPlan::from_system_files(&host.observe_system_files()?, mission_id)?
            }
            RepairOperation::SetWindowsFeature { feature, desired } => {
                RepairExecutionPlan::from_feature(
                    &host.observe_feature(feature)?,
                    desired,
                    mission_id,
                )?
            }
        };
        let checkpoint = plan.checkpoint()?;
        let session = Self { plan, checkpoint };
        session.validate()?;
        Ok(session)
    }

    pub fn plan(&self) -> &RepairExecutionPlan {
        &self.plan
    }

    pub fn checkpoint(&self) -> &TransactionCheckpoint {
        &self.checkpoint
    }

    pub fn stage(&self) -> TransactionStage {
        self.checkpoint.stage()
    }

    pub fn validate(&self) -> Result<(), RepairError> {
        self.plan.validate()?;
        let plan_fingerprint = self.plan.transaction().fingerprint()?;
        if self.checkpoint.plan_fingerprint() != plan_fingerprint
            || self.checkpoint.plan().fingerprint()? != plan_fingerprint
            || self.checkpoint.plan().transaction_id() != self.plan.transaction().transaction_id()
            || self.checkpoint.plan().mission_id() != self.plan.transaction().mission_id()
        {
            return Err(RepairError::InvalidRequest(
                "repair session checkpoint does not match the frozen execution plan".to_string(),
            ));
        }
        Ok(())
    }

    pub fn from_json_str(input: &str) -> Result<Self, RepairError> {
        let session: Self = serde_json::from_str(input).map_err(|error| {
            RepairError::InvalidRequest(format!("invalid persisted repair session JSON: {error}"))
        })?;
        session.validate()?;
        Ok(session)
    }

    pub(crate) fn authorize(
        &mut self,
        _capability: &RepairExecutorCapability,
        authorization: TransactionAuthorization,
    ) -> Result<(), RepairError> {
        self.validate()?;
        self.checkpoint.authorize(authorization)?;
        Ok(())
    }

    /// Perform every non-mutating preflight, then enter `Applying` without
    /// launching a Windows servicing command. The RPC layer persists this
    /// write-ahead checkpoint before calling `execute_applying_with_host`.
    pub(crate) fn begin_apply_with_host<H: RepairHost>(
        &mut self,
        _capability: &RepairExecutorCapability,
        host: &H,
    ) -> Result<(), RepairError> {
        self.validate()?;
        if self.stage() != TransactionStage::Authorized {
            return Err(RepairError::InvalidRequest(format!(
                "repair apply requires Authorized stage, found {:?}",
                self.stage()
            )));
        }
        self.assert_fresh_baseline(host)?;
        self.checkpoint.begin_apply()?;
        self.checkpoint
            .assert_action_pending(&self.plan.action_id())?;
        Ok(())
    }

    /// Execute one already write-ahead-recorded Phase 21 operation.
    pub(crate) fn execute_applying_with_host<H: RepairHost>(
        &mut self,
        _capability: &RepairExecutorCapability,
        host: &H,
    ) -> Result<(), RepairError> {
        self.validate()?;
        if self.stage() != TransactionStage::Applying {
            return Err(RepairError::InvalidRequest(format!(
                "repair command execution requires Applying stage, found {:?}",
                self.stage()
            )));
        }
        let action_id = self.plan.action_id();
        self.checkpoint.assert_action_pending(&action_id)?;
        let execution = match host.execute(self.plan.operation()) {
            Ok(value) => value,
            Err(error) => {
                self.checkpoint.record_apply_result(ApplyRecord {
                    action_id,
                    outcome: ApplyOutcome::Failure,
                    detail: format!("repair command could not be executed: {error}"),
                    machine_changed: false,
                    reboot_required: false,
                })?;
                return Err(error);
            }
        };

        let command_started = execution.start_error.is_none();
        if !execution.succeeded() {
            let detail = command_detail(&execution);
            self.checkpoint.record_apply_result(ApplyRecord {
                action_id,
                outcome: ApplyOutcome::Failure,
                detail: detail.clone(),
                machine_changed: command_started,
                reboot_required: false,
            })?;
            if self.stage() == TransactionStage::RollingBack {
                self.rollback_feature_with_host(host)?;
            }
            return Err(RepairError::CommandFailed(detail));
        }

        let post = self.observe_current(host)?;
        let reboot_required = operation_pending(self.plan.operation(), &post);
        self.checkpoint.record_apply_result(ApplyRecord {
            action_id,
            outcome: ApplyOutcome::Success,
            detail: command_detail(&execution),
            machine_changed: true,
            reboot_required,
        })?;
        if self.stage() == TransactionStage::Verifying {
            self.verify_current_with_observation(post)?;
            if self.stage() == TransactionStage::RollingBack {
                self.rollback_feature_with_host(host)?;
                return Err(RepairError::CommandFailed(
                    "Windows feature postcondition failed and the captured baseline was restored"
                        .to_string(),
                ));
            }
        }
        terminal_result(self.stage())
    }

    #[cfg(test)]
    pub(crate) fn apply_with_host<H: RepairHost>(
        &mut self,
        capability: &RepairExecutorCapability,
        host: &H,
    ) -> Result<(), RepairError> {
        self.begin_apply_with_host(capability, host)?;
        self.execute_applying_with_host(capability, host)
    }

    pub(crate) fn resume_with_host<H: RepairHost>(
        &mut self,
        capability: &RepairExecutorCapability,
        host: &H,
    ) -> Result<(), RepairError> {
        self.validate()?;
        if self.stage() == TransactionStage::Applying {
            return self.recover_applying_with_host(capability, host);
        }
        let observed = self.observe_current(host)?;
        match self.stage() {
            TransactionStage::AwaitingReboot => {
                self.checkpoint.resume_after_reboot(vec![observed])?;
                if self.stage() == TransactionStage::Verifying {
                    let observed = self.observe_current(host)?;
                    self.verify_current_with_observation(observed)?;
                }
                if self.stage() == TransactionStage::RollingBack {
                    self.rollback_feature_with_host(host)?;
                    return Err(RepairError::CommandFailed(
                        "post-reboot feature verification failed and rollback was required"
                            .to_string(),
                    ));
                }
            }
            TransactionStage::Blocked => {
                self.checkpoint.reprobe_after_block(vec![observed])?;
                if self.stage() == TransactionStage::Verifying {
                    let observed = self.observe_current(host)?;
                    self.verify_current_with_observation(observed)?;
                }
                if self.stage() == TransactionStage::RollingBack {
                    self.rollback_feature_with_host(host)?;
                    return Err(RepairError::CommandFailed(
                        "blocked feature verification remained unproven and rollback was required"
                            .to_string(),
                    ));
                }
            }
            TransactionStage::AwaitingRollbackReboot => {
                self.checkpoint
                    .resume_after_rollback_reboot(vec![observed])?;
            }
            other => {
                return Err(RepairError::InvalidRequest(format!(
                    "repair resume requires Applying/reboot/blocked stage, found {other:?}"
                )))
            }
        }
        terminal_result(self.stage())
    }

    fn recover_applying_with_host<H: RepairHost>(
        &mut self,
        _capability: &RepairExecutorCapability,
        host: &H,
    ) -> Result<(), RepairError> {
        self.validate()?;
        if self.stage() != TransactionStage::Applying {
            return Err(RepairError::InvalidRequest(
                "in-flight recovery requires Applying stage".to_string(),
            ));
        }
        let action_id = self.plan.action_id();
        self.checkpoint.assert_action_pending(&action_id)?;
        let observed = self.observe_current(host)?;
        let reaches_target = observation_reaches_target(self.plan.operation(), &observed);
        let pending = operation_pending(self.plan.operation(), &observed);
        if reaches_target || pending {
            self.checkpoint.record_apply_result(ApplyRecord {
                action_id,
                outcome: ApplyOutcome::Success,
                detail: "recovered an interrupted Phase 21 apply from fresh machine state"
                    .to_string(),
                machine_changed: true,
                reboot_required: pending,
            })?;
            if self.stage() == TransactionStage::Verifying {
                self.verify_current_with_observation(observed)?;
            }
            return terminal_result(self.stage());
        }

        let baseline_unchanged = observation_matches_baseline(self.plan.baseline(), &observed);
        let machine_changed = match self.plan.operation() {
            RepairOperation::RestoreComponentStore | RepairOperation::RepairSystemFiles => true,
            RepairOperation::SetWindowsFeature { .. } => !baseline_unchanged,
        };
        self.checkpoint.record_apply_result(ApplyRecord {
            action_id,
            outcome: ApplyOutcome::Failure,
            detail: if baseline_unchanged {
                "interrupted Phase 21 apply recovered at its captured baseline".to_string()
            } else {
                "interrupted Phase 21 apply recovered in an unexpected machine state".to_string()
            },
            machine_changed,
            reboot_required: false,
        })?;
        if self.stage() == TransactionStage::RollingBack {
            self.rollback_feature_with_host(host)?;
        }
        Err(RepairError::CommandFailed(
            "interrupted Phase 21 apply could not be proven successful from fresh state"
                .to_string(),
        ))
    }

    fn assert_fresh_baseline<H: RepairHost>(&self, host: &H) -> Result<(), RepairError> {
        let observed = baseline_from_host(self.plan.operation(), host)?;
        if observed != self.plan.baseline() {
            return Err(RepairError::BaselineDrift(format!(
                "prepared {:?}, freshly observed {:?}",
                self.plan.baseline(),
                observed
            )));
        }
        Ok(())
    }

    fn observe_current<H: RepairHost>(&self, host: &H) -> Result<Observation, RepairError> {
        observation_from_host(self.plan.operation(), host)
    }

    fn verify_current_with_observation(
        &mut self,
        observed: Observation,
    ) -> Result<(), RepairError> {
        self.checkpoint.verify_postconditions(vec![observed])?;
        Ok(())
    }

    fn rollback_feature_with_host<H: RepairHost>(&mut self, host: &H) -> Result<(), RepairError> {
        let Some((feature, baseline_state)) = feature_baseline_state(self.plan.baseline()) else {
            return Err(RepairError::InvalidRequest(
                "irreversible repair entered rollback state".to_string(),
            ));
        };
        let desired = match baseline_state {
            WindowsFeatureState::Enabled => FeatureDesiredState::Enabled,
            WindowsFeatureState::Disabled => FeatureDesiredState::Disabled,
            _ => {
                return Err(RepairError::FeatureNotReversible(format!(
                    "rollback baseline is {baseline_state:?}"
                )))
            }
        };
        let rollback_operation = RepairOperation::SetWindowsFeature { feature, desired };
        let execution = host.execute(rollback_operation)?;
        let observed = observation_from_host(rollback_operation, host)?;
        let reboot_required = operation_pending(rollback_operation, &observed);
        self.checkpoint.record_rollback_result(RollbackRecord {
            action_id: self.plan.action_id(),
            outcome: if execution.succeeded() {
                ApplyOutcome::Success
            } else {
                ApplyOutcome::Failure
            },
            detail: command_detail(&execution),
            reboot_required,
        })?;
        if self.stage() == TransactionStage::RollingBack {
            self.checkpoint.verify_rollback(vec![observed])?;
        }
        if self.stage() == TransactionStage::Failed {
            return Err(RepairError::CommandFailed(
                "Windows feature rollback failed or could not be verified".to_string(),
            ));
        }
        Ok(())
    }
}

fn baseline_from_host<H: RepairHost>(
    operation: RepairOperation,
    host: &H,
) -> Result<RepairBaseline, RepairError> {
    match operation {
        RepairOperation::RestoreComponentStore => {
            let observed = host.observe_component_store()?;
            unavailable_component(observed.state, &observed.detail)?;
            Ok(RepairBaseline::ComponentStore(observed.state))
        }
        RepairOperation::RepairSystemFiles => {
            let observed = host.observe_system_files()?;
            unavailable_system_files(observed.state, &observed.detail)?;
            Ok(RepairBaseline::SystemFiles(observed.state))
        }
        RepairOperation::SetWindowsFeature { feature, .. } => {
            let observed = host.observe_feature(feature)?;
            unavailable_feature(observed.state, &observed.detail)?;
            Ok(RepairBaseline::WindowsFeature {
                feature,
                state: observed.state,
            })
        }
    }
}

fn observation_from_host<H: RepairHost>(
    operation: RepairOperation,
    host: &H,
) -> Result<Observation, RepairError> {
    let target = target_for(operation);
    let value = match operation {
        RepairOperation::RestoreComponentStore => {
            let observed = host.observe_component_store()?;
            match observed.state {
                ComponentStoreState::Unavailable => ObservedValue::Unavailable(observed.detail),
                ComponentStoreState::Healthy => ObservedValue::Present("healthy".to_string()),
                ComponentStoreState::Repairable => ObservedValue::Present("repairable".to_string()),
                ComponentStoreState::Unrepairable => {
                    ObservedValue::Present("unrepairable".to_string())
                }
            }
        }
        RepairOperation::RepairSystemFiles => {
            let observed = host.observe_system_files()?;
            match observed.state {
                SystemFileState::Unavailable => ObservedValue::Unavailable(observed.detail),
                SystemFileState::Healthy => ObservedValue::Present("healthy".to_string()),
                SystemFileState::IntegrityViolations => {
                    ObservedValue::Present("integrity_violations".to_string())
                }
            }
        }
        RepairOperation::SetWindowsFeature { feature, .. } => {
            let observed = host.observe_feature(feature)?;
            match observed.state {
                WindowsFeatureState::Unavailable => ObservedValue::Unavailable(observed.detail),
                state => ObservedValue::Present(state.as_transaction_value().to_string()),
            }
        }
    };
    Ok(Observation { target, value })
}

fn observation_reaches_target(operation: RepairOperation, observed: &Observation) -> bool {
    matches!(
        observed.value,
        ObservedValue::Present(ref value) if value == target_value(operation)
    )
}

fn observation_matches_baseline(baseline: RepairBaseline, observed: &Observation) -> bool {
    matches!(
        observed.value,
        ObservedValue::Present(ref value) if value == baseline.transaction_value()
    )
}

fn operation_pending(operation: RepairOperation, observed: &Observation) -> bool {
    matches!(operation, RepairOperation::SetWindowsFeature { .. })
        && matches!(
            observed.value,
            ObservedValue::Present(ref value)
                if value == WindowsFeatureState::EnablePending.as_transaction_value()
                    || value == WindowsFeatureState::DisablePending.as_transaction_value()
        )
}

fn unavailable_component(state: ComponentStoreState, detail: &str) -> Result<(), RepairError> {
    if state == ComponentStoreState::Unavailable {
        unavailable_detail(detail)
    } else {
        Ok(())
    }
}

fn unavailable_system_files(state: SystemFileState, detail: &str) -> Result<(), RepairError> {
    if state == SystemFileState::Unavailable {
        unavailable_detail(detail)
    } else {
        Ok(())
    }
}

fn unavailable_feature(state: WindowsFeatureState, detail: &str) -> Result<(), RepairError> {
    if state == WindowsFeatureState::Unavailable {
        unavailable_detail(detail)
    } else {
        Ok(())
    }
}

fn unavailable_detail(detail: &str) -> Result<(), RepairError> {
    if detail.to_ascii_lowercase().contains("elevated") {
        Err(RepairError::ElevationRequired)
    } else {
        Err(RepairError::StateUnavailable(detail.to_string()))
    }
}

fn command_detail(evidence: &crate::model::BoundedCommandEvidence) -> String {
    match (&evidence.start_error, evidence.exit_code) {
        (Some(error), _) => format!("command start failed: {error}"),
        (None, Some(code)) => format!("trusted Windows command exited with code {code}"),
        (None, None) => "trusted Windows command exit status unavailable".to_string(),
    }
}

fn terminal_result(stage: TransactionStage) -> Result<(), RepairError> {
    match stage {
        TransactionStage::Complete
        | TransactionStage::AwaitingReboot
        | TransactionStage::AwaitingRollbackReboot
        | TransactionStage::RolledBack
        | TransactionStage::Blocked => Ok(()),
        TransactionStage::Failed => Err(RepairError::CommandFailed(
            "Phase 21 transaction reached Failed after execution/verification".to_string(),
        )),
        other => Err(RepairError::InvalidRequest(format!(
            "Phase 21 transaction stopped in unexpected stage {other:?}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host::testsupport::FakeRepairHost;
    use crate::model::{SupportedWindowsFeature, WindowsFeatureState};
    use neo_transaction::{ActionAcknowledgement, TransactionAuthorization};

    fn authorization(session: &RepairExecutionSession) -> TransactionAuthorization {
        let action_id = session.plan.action_id();
        let irreversible = matches!(
            session.plan.operation(),
            RepairOperation::RestoreComponentStore | RepairOperation::RepairSystemFiles
        );
        TransactionAuthorization {
            plan_fingerprint: session.plan.transaction().fingerprint().unwrap(),
            approved_action_ids: vec![action_id.clone()],
            manual_override_action_ids: vec![],
            high_risk_ack_action_ids: vec![],
            irreversible_acknowledgements: if irreversible {
                vec![ActionAcknowledgement {
                    action_id,
                    reason: "Owner approved irreversible Windows repair".to_string(),
                }]
            } else {
                vec![]
            },
        }
    }

    fn authorized_feature_session(
        host: &FakeRepairHost,
        feature: SupportedWindowsFeature,
    ) -> (RepairExecutionSession, RepairExecutorCapability) {
        host.set_feature(feature, WindowsFeatureState::Disabled);
        let mut session = RepairExecutionSession::prepare_with_host(
            RepairOperation::SetWindowsFeature {
                feature,
                desired: FeatureDesiredState::Enabled,
            },
            "mission",
            host,
        )
        .unwrap();
        let capability = RepairExecutorCapability::for_rpc();
        let authorization = authorization(&session);
        session.authorize(&capability, authorization).unwrap();
        (session, capability)
    }

    #[test]
    fn fresh_baseline_drift_blocks_before_mutation() {
        let host = FakeRepairHost::new(ComponentStoreState::Repairable, SystemFileState::Healthy);
        let mut session = RepairExecutionSession::prepare_with_host(
            RepairOperation::RestoreComponentStore,
            "mission",
            &host,
        )
        .unwrap();
        let capability = RepairExecutorCapability::for_rpc();
        let authorization = authorization(&session);
        session.authorize(&capability, authorization).unwrap();
        host.set_component(ComponentStoreState::Healthy);
        assert!(matches!(
            session.apply_with_host(&capability, &host),
            Err(RepairError::BaselineDrift(_))
        ));
        assert!(host.executed.borrow().is_empty());
    }

    #[test]
    fn irreversible_repair_completes_only_after_fresh_healthy_observation() {
        let host = FakeRepairHost::new(ComponentStoreState::Repairable, SystemFileState::Healthy);
        let mut session = RepairExecutionSession::prepare_with_host(
            RepairOperation::RestoreComponentStore,
            "mission",
            &host,
        )
        .unwrap();
        let capability = RepairExecutorCapability::for_rpc();
        let authorization = authorization(&session);
        session.authorize(&capability, authorization).unwrap();
        session.apply_with_host(&capability, &host).unwrap();
        assert_eq!(session.stage(), TransactionStage::Complete);
        assert_eq!(host.executed.borrow().len(), 1);
    }

    #[test]
    fn pending_feature_transition_requires_resume() {
        let feature = SupportedWindowsFeature::WindowsSubsystemLinux;
        let host = FakeRepairHost::new(ComponentStoreState::Healthy, SystemFileState::Healthy);
        let (mut session, capability) = authorized_feature_session(&host, feature);
        *host.pending_feature_transition.borrow_mut() = true;
        session.apply_with_host(&capability, &host).unwrap();
        assert_eq!(session.stage(), TransactionStage::AwaitingReboot);
        *host.pending_feature_transition.borrow_mut() = false;
        host.set_feature(feature, WindowsFeatureState::Enabled);
        session.resume_with_host(&capability, &host).unwrap();
        assert_eq!(session.stage(), TransactionStage::Complete);
    }

    #[test]
    fn applying_write_ahead_recovers_success_without_rerunning_feature_command() {
        let feature = SupportedWindowsFeature::VirtualMachinePlatform;
        let host = FakeRepairHost::new(ComponentStoreState::Healthy, SystemFileState::Healthy);
        let (mut session, capability) = authorized_feature_session(&host, feature);
        session.begin_apply_with_host(&capability, &host).unwrap();
        assert_eq!(session.stage(), TransactionStage::Applying);
        host.set_feature(feature, WindowsFeatureState::Enabled);
        session.resume_with_host(&capability, &host).unwrap();
        assert_eq!(session.stage(), TransactionStage::Complete);
        assert!(host.executed.borrow().is_empty());
    }

    #[test]
    fn interrupted_irreversible_repair_at_old_baseline_fails_closed_without_rerun() {
        let host = FakeRepairHost::new(ComponentStoreState::Repairable, SystemFileState::Healthy);
        let mut session = RepairExecutionSession::prepare_with_host(
            RepairOperation::RestoreComponentStore,
            "mission",
            &host,
        )
        .unwrap();
        let capability = RepairExecutorCapability::for_rpc();
        let authorization = authorization(&session);
        session.authorize(&capability, authorization).unwrap();
        session.begin_apply_with_host(&capability, &host).unwrap();
        assert!(session.resume_with_host(&capability, &host).is_err());
        assert_eq!(session.stage(), TransactionStage::Failed);
        assert!(host.executed.borrow().is_empty());
    }
}
