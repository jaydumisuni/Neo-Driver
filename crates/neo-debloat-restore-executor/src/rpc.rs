use crate::model::{DebloatRestoreExecutionSession, DebloatRestoreExecutorCapability};
use crate::{prepare_debloat_restore_execution, DebloatRestoreExecutionError};
#[cfg(test)]
use crate::engine::{apply_with_host, authorize_with_host, DebloatRestoreHost};
use neo_debloat_history::DebloatHistoryError;
use neo_debloat_history_store::{
    DebloatHistoryRecordId, DebloatHistoryStore, DebloatHistoryStoreError,
};
#[cfg(test)]
use neo_debloat_plan::ExactAppxInventory;
use neo_transaction::{TransactionAuthorization, TransactionStage};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

pub const NEO_DEBLOAT_RPC_SCHEMA_VERSION: &str = "neo-debloat-rpc-v1";
pub const MCP_DEBLOAT_RESTORE_PREPARE_TOOL: &str = "neo_debloat_restore_prepare";
pub const MCP_DEBLOAT_RESTORE_APPLY_TOOL: &str = "neo_debloat_restore_apply";
pub const RPC_DEBLOAT_RESTORE_PREPARE_METHOD: &str = "neo.debloat.restore.prepare";
pub const RPC_DEBLOAT_RESTORE_APPLY_METHOD: &str = "neo.debloat.restore.apply";
pub const DEBLOAT_RESTORE_PREPARE_PERMISSION_SCOPE: &str = "neo.debloat.restore.prepare";
pub const DEBLOAT_RESTORE_APPLY_PERMISSION_SCOPE: &str =
    "neo.debloat.restore.low-risk.apply";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DebloatRestoreRpcCallerKind {
    Hunter,
    Oracle,
    Gui,
    Internal,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct DebloatRestoreRpcCaller {
    pub kind: DebloatRestoreRpcCallerKind,
    pub principal: String,
}

impl DebloatRestoreRpcCaller {
    fn validate(&self) -> Result<(), DebloatRestoreRpcError> {
        require_text("caller principal", &self.principal, 160)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DebloatRestoreRpcContext {
    pub caller: DebloatRestoreRpcCaller,
    pub granted_scopes: Vec<String>,
}

impl DebloatRestoreRpcContext {
    fn validate(&self) -> Result<(), DebloatRestoreRpcError> {
        self.caller.validate()?;
        unique_text_set("permission scope", &self.granted_scopes, 160)?;
        Ok(())
    }

    fn has_scope(&self, scope: &str) -> bool {
        self.granted_scopes
            .iter()
            .any(|candidate| candidate == scope)
    }
}

#[derive(Debug, Clone)]
pub struct DebloatRestoreRpcPolicy {
    allowed_callers: BTreeSet<DebloatRestoreRpcCaller>,
}

impl DebloatRestoreRpcPolicy {
    pub fn new(
        allowed_callers: Vec<DebloatRestoreRpcCaller>,
    ) -> Result<Self, DebloatRestoreRpcError> {
        if allowed_callers.is_empty() {
            return Err(DebloatRestoreRpcError::InvalidRequest(
                "RPC policy must allow at least one caller".to_string(),
            ));
        }
        let mut normalized = BTreeSet::new();
        for caller in allowed_callers {
            caller.validate()?;
            if !normalized.insert(caller.clone()) {
                return Err(DebloatRestoreRpcError::InvalidRequest(format!(
                    "duplicate allowed RPC caller: {}",
                    caller.principal
                )));
            }
        }
        Ok(Self {
            allowed_callers: normalized,
        })
    }

    fn allows(&self, caller: &DebloatRestoreRpcCaller) -> bool {
        self.allowed_callers.contains(caller)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DebloatRestoreRpcPrepareRequest {
    pub request_id: String,
    pub mission_id: String,
    pub record_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DebloatRestoreRpcApplyRequest {
    pub request_id: String,
    pub session_id: String,
    pub plan_fingerprint: String,
    pub approved_action_ids: Vec<String>,
    pub confirmed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DebloatRestoreRpcPrepared {
    pub schema_version: &'static str,
    pub request_id: String,
    pub session_id: String,
    pub mission_id: String,
    pub record_id: String,
    pub receipt_fingerprint: String,
    pub plan_fingerprint: String,
    pub action_id: String,
    pub package_id: String,
    pub package_full_name: String,
    pub dependency_full_names: Vec<String>,
    pub confirmation_required: bool,
    pub stage: TransactionStage,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DebloatRestoreRpcExecutionReceipt {
    pub schema_version: &'static str,
    pub request_id: String,
    pub session_id: String,
    pub mission_id: String,
    pub record_id: String,
    pub receipt_fingerprint: String,
    pub plan_fingerprint: String,
    pub changed_action_ids: Vec<String>,
    pub stage: TransactionStage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DebloatRestoreRpcErrorCode {
    InvalidRequest,
    UnauthorizedCaller,
    PermissionDenied,
    ConfirmationRequired,
    SessionNotFound,
    ServiceStateExhausted,
    CallerMismatch,
    PlanMismatch,
    HistoryUnavailable,
    RestoreNotReady,
    UnsupportedPlatform,
    ExecutionFailed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DebloatRestoreRpcErrorPayload {
    pub code: DebloatRestoreRpcErrorCode,
    pub message: String,
}

#[derive(Debug, Error)]
pub enum DebloatRestoreRpcError {
    #[error("invalid MCP/RPC Debloat restore request: {0}")]
    InvalidRequest(String),
    #[error("RPC caller is not allowed by Neo policy: {0}")]
    UnauthorizedCaller(String),
    #[error("RPC caller lacks required Neo permission scope: {0}")]
    PermissionDenied(String),
    #[error("explicit confirmation bound to the prepared Debloat restore plan is required")]
    ConfirmationRequired,
    #[error("prepared RPC Debloat restore session was not found: {0}")]
    SessionNotFound(String),
    #[error("RPC service session sequence is exhausted")]
    ServiceStateExhausted,
    #[error("RPC caller differs from the caller that prepared the Debloat restore session")]
    CallerMismatch,
    #[error("RPC apply request does not match the prepared Debloat restore transaction")]
    PlanMismatch,
    #[error(transparent)]
    History(#[from] DebloatHistoryStoreError),
    #[error(transparent)]
    Execution(#[from] DebloatRestoreExecutionError),
}

impl DebloatRestoreRpcError {
    pub fn code(&self) -> DebloatRestoreRpcErrorCode {
        match self {
            Self::InvalidRequest(_) => DebloatRestoreRpcErrorCode::InvalidRequest,
            Self::UnauthorizedCaller(_) => DebloatRestoreRpcErrorCode::UnauthorizedCaller,
            Self::PermissionDenied(_) => DebloatRestoreRpcErrorCode::PermissionDenied,
            Self::ConfirmationRequired => DebloatRestoreRpcErrorCode::ConfirmationRequired,
            Self::SessionNotFound(_) => DebloatRestoreRpcErrorCode::SessionNotFound,
            Self::ServiceStateExhausted => DebloatRestoreRpcErrorCode::ServiceStateExhausted,
            Self::CallerMismatch => DebloatRestoreRpcErrorCode::CallerMismatch,
            Self::PlanMismatch => DebloatRestoreRpcErrorCode::PlanMismatch,
            Self::History(DebloatHistoryStoreError::InvalidRecordId(_)) => {
                DebloatRestoreRpcErrorCode::InvalidRequest
            }
            Self::History(DebloatHistoryStoreError::History(
                DebloatHistoryError::UnsupportedPlatform,
            )) => DebloatRestoreRpcErrorCode::UnsupportedPlatform,
            Self::History(DebloatHistoryStoreError::History(
                DebloatHistoryError::RestoreNotReady(_)
                | DebloatHistoryError::AlreadyRestored
                | DebloatHistoryError::InventoryConflict(_),
            )) => DebloatRestoreRpcErrorCode::RestoreNotReady,
            Self::History(_) => DebloatRestoreRpcErrorCode::HistoryUnavailable,
            Self::Execution(DebloatRestoreExecutionError::UnsupportedPlatform) => {
                DebloatRestoreRpcErrorCode::UnsupportedPlatform
            }
            Self::Execution(_) => DebloatRestoreRpcErrorCode::ExecutionFailed,
        }
    }

    pub fn payload(&self) -> DebloatRestoreRpcErrorPayload {
        DebloatRestoreRpcErrorPayload {
            code: self.code(),
            message: self.to_string(),
        }
    }
}

struct PendingDebloatRestoreRpcSession {
    caller: DebloatRestoreRpcCaller,
    record_id: DebloatHistoryRecordId,
    mission_id: String,
    plan_fingerprint: String,
    session: DebloatRestoreExecutionSession,
}

pub struct DebloatRestoreRpcService {
    store: DebloatHistoryStore,
    policy: DebloatRestoreRpcPolicy,
    service_instance_id: String,
    next_session_sequence: u64,
    pending: BTreeMap<String, PendingDebloatRestoreRpcSession>,
}

impl DebloatRestoreRpcService {
    pub fn new(
        store: DebloatHistoryStore,
        policy: DebloatRestoreRpcPolicy,
        service_instance_id: impl Into<String>,
    ) -> Result<Self, DebloatRestoreRpcError> {
        let service_instance_id = service_instance_id.into();
        require_text("service instance id", &service_instance_id, 160)?;
        Ok(Self {
            store,
            policy,
            service_instance_id,
            next_session_sequence: 0,
            pending: BTreeMap::new(),
        })
    }

    pub fn pending_session_count(&self) -> usize {
        self.pending.len()
    }

    pub fn prepare(
        &mut self,
        context: &DebloatRestoreRpcContext,
        request: DebloatRestoreRpcPrepareRequest,
    ) -> Result<DebloatRestoreRpcPrepared, DebloatRestoreRpcError> {
        let record_id = self.validate_prepare(context, &request)?;
        let prepared = self
            .store
            .prepare_windows_restore_by_id(&record_id, request.mission_id.clone())?;
        let session = prepare_debloat_restore_execution(&prepared)?;
        self.store_prepared(context, request, record_id, session)
    }

    pub fn apply(
        &mut self,
        context: &DebloatRestoreRpcContext,
        request: DebloatRestoreRpcApplyRequest,
    ) -> Result<DebloatRestoreRpcExecutionReceipt, DebloatRestoreRpcError> {
        let authorization = self.validate_apply(context, &request)?;
        let pending = self
            .pending
            .remove(&request.session_id)
            .ok_or_else(|| DebloatRestoreRpcError::SessionNotFound(request.session_id.clone()))?;

        #[cfg(windows)]
        {
            let mut pending = pending;
            let capability = DebloatRestoreExecutorCapability::for_rpc();
            pending.session.authorize(&capability, authorization)?;
            pending.session.apply(&capability)?;
            Ok(execution_receipt(request, pending))
        }

        #[cfg(not(windows))]
        {
            let _ = authorization;
            let _ = pending;
            Err(DebloatRestoreExecutionError::UnsupportedPlatform.into())
        }
    }

    fn validate_prepare(
        &self,
        context: &DebloatRestoreRpcContext,
        request: &DebloatRestoreRpcPrepareRequest,
    ) -> Result<DebloatHistoryRecordId, DebloatRestoreRpcError> {
        self.validate_context(context, DEBLOAT_RESTORE_PREPARE_PERMISSION_SCOPE)?;
        require_text("request id", &request.request_id, 160)?;
        require_text("mission id", &request.mission_id, 240)?;
        require_text("history record id", &request.record_id, 64)?;
        let record_id = DebloatHistoryRecordId::new(request.record_id.clone())?;
        if request.record_id != record_id.as_str() {
            return Err(DebloatRestoreRpcError::InvalidRequest(
                "history record id must use canonical lowercase spelling".to_string(),
            ));
        }
        Ok(record_id)
    }

    fn validate_context(
        &self,
        context: &DebloatRestoreRpcContext,
        required_scope: &str,
    ) -> Result<(), DebloatRestoreRpcError> {
        context.validate()?;
        if !self.policy.allows(&context.caller) {
            return Err(DebloatRestoreRpcError::UnauthorizedCaller(
                context.caller.principal.clone(),
            ));
        }
        if !context.has_scope(required_scope) {
            return Err(DebloatRestoreRpcError::PermissionDenied(
                required_scope.to_string(),
            ));
        }
        Ok(())
    }

    fn store_prepared(
        &mut self,
        context: &DebloatRestoreRpcContext,
        request: DebloatRestoreRpcPrepareRequest,
        record_id: DebloatHistoryRecordId,
        session: DebloatRestoreExecutionSession,
    ) -> Result<DebloatRestoreRpcPrepared, DebloatRestoreRpcError> {
        let plan_fingerprint = session.plan().transaction().fingerprint()?;
        let sequence = self
            .next_session_sequence
            .checked_add(1)
            .ok_or(DebloatRestoreRpcError::ServiceStateExhausted)?;
        let session_id = format!(
            "phase20:{}:{}:{plan_fingerprint}",
            self.service_instance_id, sequence
        );
        let step = session.plan().step();
        let response = DebloatRestoreRpcPrepared {
            schema_version: NEO_DEBLOAT_RPC_SCHEMA_VERSION,
            request_id: request.request_id,
            session_id: session_id.clone(),
            mission_id: request.mission_id.clone(),
            record_id: record_id.to_string(),
            receipt_fingerprint: session.plan().receipt_fingerprint().to_string(),
            plan_fingerprint: plan_fingerprint.clone(),
            action_id: step.action_id(),
            package_id: step.package_id().to_string(),
            package_full_name: step.package_full_name().to_string(),
            dependency_full_names: step.dependency_full_names().to_vec(),
            confirmation_required: true,
            stage: session.stage(),
        };

        self.next_session_sequence = sequence;
        self.pending
            .retain(|_, pending| pending.caller != context.caller);
        self.pending.insert(
            session_id,
            PendingDebloatRestoreRpcSession {
                caller: context.caller.clone(),
                record_id,
                mission_id: request.mission_id,
                plan_fingerprint,
                session,
            },
        );
        Ok(response)
    }

    fn validate_apply(
        &self,
        context: &DebloatRestoreRpcContext,
        request: &DebloatRestoreRpcApplyRequest,
    ) -> Result<TransactionAuthorization, DebloatRestoreRpcError> {
        self.validate_context(context, DEBLOAT_RESTORE_APPLY_PERMISSION_SCOPE)?;
        require_text("request id", &request.request_id, 160)?;
        require_text("session id", &request.session_id, 512)?;
        require_text("plan fingerprint", &request.plan_fingerprint, 128)?;
        let pending = self
            .pending
            .get(&request.session_id)
            .ok_or_else(|| DebloatRestoreRpcError::SessionNotFound(request.session_id.clone()))?;
        if pending.caller != context.caller {
            return Err(DebloatRestoreRpcError::CallerMismatch);
        }
        if !request.confirmed {
            return Err(DebloatRestoreRpcError::ConfirmationRequired);
        }
        if request.plan_fingerprint != pending.plan_fingerprint {
            return Err(DebloatRestoreRpcError::PlanMismatch);
        }
        if request.approved_action_ids.len() != 1 {
            return Err(DebloatRestoreRpcError::PlanMismatch);
        }
        let approved = unique_text_set("approved action id", &request.approved_action_ids, 240)?;
        let expected = BTreeSet::from([pending.session.plan().step().action_id()]);
        if approved != expected {
            return Err(DebloatRestoreRpcError::PlanMismatch);
        }
        Ok(TransactionAuthorization {
            plan_fingerprint: pending.plan_fingerprint.clone(),
            approved_action_ids: request.approved_action_ids.clone(),
            manual_override_action_ids: vec![],
            high_risk_ack_action_ids: vec![],
            irreversible_acknowledgements: vec![],
        })
    }

    #[cfg(test)]
    fn prepare_with_inventory(
        &mut self,
        context: &DebloatRestoreRpcContext,
        request: DebloatRestoreRpcPrepareRequest,
        inventory: &ExactAppxInventory,
    ) -> Result<DebloatRestoreRpcPrepared, DebloatRestoreRpcError> {
        let record_id = self.validate_prepare(context, &request)?;
        let prepared = self.store.prepare_restore_from_inventory_by_id(
            &record_id,
            inventory,
            request.mission_id.clone(),
        )?;
        let session = prepare_debloat_restore_execution(&prepared)?;
        self.store_prepared(context, request, record_id, session)
    }

    #[cfg(test)]
    fn apply_with_test_host<H: DebloatRestoreHost>(
        &mut self,
        context: &DebloatRestoreRpcContext,
        request: DebloatRestoreRpcApplyRequest,
        host: &mut H,
    ) -> Result<DebloatRestoreRpcExecutionReceipt, DebloatRestoreRpcError> {
        let authorization = self.validate_apply(context, &request)?;
        let mut pending = self
            .pending
            .remove(&request.session_id)
            .ok_or_else(|| DebloatRestoreRpcError::SessionNotFound(request.session_id.clone()))?;
        let _capability = DebloatRestoreExecutorCapability::for_rpc();
        authorize_with_host(&mut pending.session, authorization, host)?;
        apply_with_host(&mut pending.session, host)?;
        Ok(execution_receipt(request, pending))
    }

    #[cfg(test)]
    fn set_next_session_sequence_for_tests(&mut self, value: u64) {
        self.next_session_sequence = value;
    }
}

#[cfg(any(windows, test))]
fn execution_receipt(
    request: DebloatRestoreRpcApplyRequest,
    pending: PendingDebloatRestoreRpcSession,
) -> DebloatRestoreRpcExecutionReceipt {
    DebloatRestoreRpcExecutionReceipt {
        schema_version: NEO_DEBLOAT_RPC_SCHEMA_VERSION,
        request_id: request.request_id,
        session_id: request.session_id,
        mission_id: pending.mission_id,
        record_id: pending.record_id.to_string(),
        receipt_fingerprint: pending.session.plan().receipt_fingerprint().to_string(),
        plan_fingerprint: pending.plan_fingerprint,
        changed_action_ids: vec![pending.session.plan().step().action_id()],
        stage: pending.session.stage(),
    }
}

fn require_text(
    label: &str,
    value: &str,
    max_len: usize,
) -> Result<(), DebloatRestoreRpcError> {
    if value.trim().is_empty() || value.len() > max_len || value.chars().any(char::is_control) {
        return Err(DebloatRestoreRpcError::InvalidRequest(format!(
            "{label} must be non-empty, bounded, and contain no control characters"
        )));
    }
    Ok(())
}

fn unique_text_set(
    label: &str,
    values: &[String],
    max_len: usize,
) -> Result<BTreeSet<String>, DebloatRestoreRpcError> {
    let mut result = BTreeSet::new();
    for value in values {
        require_text(label, value, max_len)?;
        if !result.insert(value.clone()) {
            return Err(DebloatRestoreRpcError::InvalidRequest(format!(
                "duplicate {label}: {value}"
            )));
        }
    }
    Ok(result)
}

#[cfg(test)]
#[path = "rpc_tests.rs"]
mod tests;
