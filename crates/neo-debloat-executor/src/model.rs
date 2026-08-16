use crate::DebloatExecutionError;
use neo_debloat_plan::{DebloatPreparedStep, DebloatPreparedTransaction, DebloatRestoreRoute};
use neo_transaction::{TransactionCheckpoint, TransactionPlan, TransactionStage};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DebloatExecutionStep {
    debloat_id: String,
    package_id: String,
    package_full_name: String,
    package_family_name: String,
    dependency_full_names: Vec<String>,
    restore: DebloatRestoreRoute,
}

impl DebloatExecutionStep {
    pub(crate) fn from_prepared(step: &DebloatPreparedStep) -> Result<Self, DebloatExecutionError> {
        if step.debloat_id.trim().is_empty()
            || step.package_id.trim().is_empty()
            || step.package_full_name.trim().is_empty()
            || step.package_family_name.trim().is_empty()
        {
            return Err(DebloatExecutionError::InvalidPreparedState(
                "prepared step contains an empty identity".to_string(),
            ));
        }
        let restore = step.restore.clone();
        let DebloatRestoreRoute::RegisterByFullNameFromProvisioned {
            package_full_name,
            package_family_name,
            dependency_full_names,
        } = &restore;
        if !package_full_name.eq_ignore_ascii_case(&step.package_full_name)
            || !package_family_name.eq_ignore_ascii_case(&step.package_family_name)
            || dependency_full_names.len() != step.dependency_full_names.len()
            || !dependency_full_names
                .iter()
                .zip(&step.dependency_full_names)
                .all(|(left, right)| left.eq_ignore_ascii_case(right))
        {
            return Err(DebloatExecutionError::InvalidPreparedState(
                "restore route does not match the prepared package/dependency identities"
                    .to_string(),
            ));
        }
        Ok(Self {
            debloat_id: step.debloat_id.clone(),
            package_id: step.package_id.clone(),
            package_full_name: step.package_full_name.clone(),
            package_family_name: step.package_family_name.clone(),
            dependency_full_names: step.dependency_full_names.clone(),
            restore,
        })
    }

    pub fn debloat_id(&self) -> &str {
        &self.debloat_id
    }

    pub fn package_id(&self) -> &str {
        &self.package_id
    }

    pub fn package_full_name(&self) -> &str {
        &self.package_full_name
    }

    pub fn package_family_name(&self) -> &str {
        &self.package_family_name
    }

    pub fn dependency_full_names(&self) -> &[String] {
        &self.dependency_full_names
    }

    pub fn restore(&self) -> &DebloatRestoreRoute {
        &self.restore
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DebloatExecutionPlan {
    step: DebloatExecutionStep,
    transaction: TransactionPlan,
}

impl DebloatExecutionPlan {
    pub(crate) fn from_prepared(
        prepared: &DebloatPreparedTransaction,
    ) -> Result<Self, DebloatExecutionError> {
        if prepared.machine_changes() {
            return Err(DebloatExecutionError::InvalidPreparedState(
                "Phase 15 prepared state cannot already claim machine mutation".to_string(),
            ));
        }
        if prepared.steps().len() != 1 || prepared.transaction().actions().len() != 1 {
            return Err(DebloatExecutionError::InvalidPreparedState(
                "Phase 16 accepts exactly one Phase 15 current-user action".to_string(),
            ));
        }
        if prepared.checkpoint().stage() != TransactionStage::BaselineCaptured {
            return Err(DebloatExecutionError::InvalidPreparedState(
                "Phase 15 checkpoint must be BaselineCaptured".to_string(),
            ));
        }
        prepared.transaction().validate()?;
        if prepared.checkpoint().plan_fingerprint() != prepared.transaction().fingerprint()? {
            return Err(DebloatExecutionError::InvalidPreparedState(
                "checkpoint fingerprint does not match the prepared transaction".to_string(),
            ));
        }
        let step = DebloatExecutionStep::from_prepared(&prepared.steps()[0])?;
        let action = &prepared.transaction().actions()[0];
        if action.action.id != step.debloat_id {
            return Err(DebloatExecutionError::InvalidPreparedState(
                "prepared action id does not match the debloat step".to_string(),
            ));
        }
        Ok(Self {
            step,
            transaction: prepared.transaction().clone(),
        })
    }

    pub fn step(&self) -> &DebloatExecutionStep {
        &self.step
    }

    pub fn transaction(&self) -> &TransactionPlan {
        &self.transaction
    }
}

#[derive(Debug, Clone)]
pub struct DebloatExecutionSession {
    pub(crate) plan: DebloatExecutionPlan,
    pub(crate) checkpoint: TransactionCheckpoint,
}

impl DebloatExecutionSession {
    pub(crate) fn from_prepared(
        prepared: &DebloatPreparedTransaction,
    ) -> Result<Self, DebloatExecutionError> {
        let plan = DebloatExecutionPlan::from_prepared(prepared)?;
        let checkpoint = prepared.checkpoint().clone();
        if checkpoint.plan_fingerprint() != plan.transaction.fingerprint()? {
            return Err(DebloatExecutionError::InvalidPreparedState(
                "execution session fingerprint mismatch".to_string(),
            ));
        }
        Ok(Self { plan, checkpoint })
    }

    pub fn plan(&self) -> &DebloatExecutionPlan {
        &self.plan
    }

    pub fn checkpoint(&self) -> &TransactionCheckpoint {
        &self.checkpoint
    }

    pub fn stage(&self) -> TransactionStage {
        self.checkpoint.stage()
    }
}

#[derive(Debug)]
pub struct DebloatExecutorCapability {
    _private: (),
}

impl DebloatExecutorCapability {
    #[cfg(test)]
    pub(crate) fn for_tests() -> Self {
        Self { _private: () }
    }
}
