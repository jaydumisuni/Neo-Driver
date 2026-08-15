use crate::model::TweakExecutionSession;
#[cfg(any(windows, test))]
use crate::model::TweakExecutorCapability;
use crate::{prepare_windows_tweaks, RegistrySnapshot, TweakExecutionError};
#[cfg(any(windows, test))]
use crate::{
    engine::{prepare_with_host, TweakHost},
    session::{apply_with_host, authorize_with_host},
};
use neo_state_plan::TweakCatalogue;
use neo_transaction::{TransactionAuthorization, TransactionStage};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

pub const NEO_RPC_SCHEMA_VERSION: &str = "neo-rpc-v1";
pub const MCP_TWEAK_PREPARE_TOOL: &str = "neo_tweaks_prepare";
pub const MCP_TWEAK_APPLY_TOOL: &str = "neo_tweaks_apply";
pub const RPC_TWEAK_PREPARE_METHOD: &str = "neo.tweaks.prepare";
pub const RPC_TWEAK_APPLY_METHOD: &str = "neo.tweaks.apply";
pub const TWEAK_PREPARE_PERMISSION_SCOPE: &str = "neo.tweaks.prepare";
pub const TWEAK_APPLY_PERMISSION_SCOPE: &str = "neo.tweaks.low-risk.apply";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TweakRpcCallerKind {
    Hunter,
    Oracle,
    Gui,
    Internal,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TweakRpcCaller {
    pub kind: TweakRpcCallerKind,
    pub principal: String,
}

impl TweakRpcCaller {
    fn validate(&self) -> Result<(), TweakRpcError> {
        require_text("caller principal", &self.principal, 160)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TweakRpcContext {
    pub caller: TweakRpcCaller,
    pub granted_scopes: Vec<String>,
}

impl TweakRpcContext {
    fn validate(&self) -> Result<(), TweakRpcError> {
        self.caller.validate()?;
        unique_text_set("permission scope", &self.granted_scopes, 160)?;
        Ok(())
    }

    fn has_scope(&self, scope: &str) -> bool {
        self.granted_scopes.iter().any(|candidate| candidate == scope)
    }
}

#[derive(Debug, Clone)]
pub struct TweakRpcPolicy {
    allowed_callers: BTreeSet<TweakRpcCaller>,
}

impl TweakRpcPolicy {
    pub fn new(allowed_callers: Vec<TweakRpcCaller>) -> Result<Self, TweakRpcError> {
        if allowed_callers.is_empty() {
            return Err(TweakRpcError::InvalidRequest(
                "RPC policy must allow at least one caller".to_string(),
            ));
        }
        let mut normalized = BTreeSet::new();
        for caller in allowed_callers {
            caller.validate()?;
            if !normalized.insert(caller.clone()) {
                return Err(TweakRpcError::InvalidRequest(format!(
                    "duplicate allowed RPC caller: {}",
                    caller.principal
                )));
            }
        }
        Ok(Self {
            allowed_callers: normalized,
        })
    }

    fn allows(&self, caller: &TweakRpcCaller) -> bool {
        self.allowed_callers.contains(caller)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TweakRpcPrepareRequest {
    pub request_id: String,
    pub mission_id: String,
    pub selected_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TweakRpcApplyRequest {
    pub request_id: String,
    pub session_id: String,
    pub plan_fingerprint: String,
    pub approved_action_ids: Vec<String>,
    pub confirmed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TweakRpcPreparedAction {
    pub tweak_id: String,
    pub desired_dword: u32,
    pub baseline: RegistrySnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TweakRpcPrepared {
    pub schema_version: &'static str,
    pub request_id: String,
    pub session_id: String,
    pub mission_id: String,
    pub plan_fingerprint: String,
    pub actions: Vec<TweakRpcPreparedAction>,
    pub confirmation_required: bool,
    pub stage: TransactionStage,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TweakRpcExecutionReceipt {
    pub schema_version: &'static str,
    pub request_id: String,
    pub session_id: String,
    pub mission_id: String,
    pub plan_fingerprint: String,
    pub changed_action_ids: Vec<String>,
    pub stage: TransactionStage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TweakRpcErrorCode {
    InvalidRequest,
    UnauthorizedCaller,
    PermissionDenied,
    ConfirmationRequired,
    SessionNotFound,
    SessionConflict,
    CallerMismatch,
    PlanMismatch,
    NoChange,
    UnsupportedPlatform,
    ExecutionFailed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TweakRpcErrorPayload {
    pub code: TweakRpcErrorCode,
    pub message: String,
}

#[derive(Debug, Error)]
pub enum TweakRpcError {
    #[error("invalid MCP/RPC tweak request: {0}")]
    InvalidRequest(String),
    #[error("RPC caller is not allowed by Neo policy: {0}")]
    UnauthorizedCaller(String),
    #[error("RPC caller lacks required Neo permission scope: {0}")]
    PermissionDenied(String),
    #[error("explicit confirmation bound to the prepared plan is required")]
    ConfirmationRequired,
    #[error("prepared RPC tweak session was not found: {0}")]
    SessionNotFound(String),
    #[error("prepared RPC tweak session already exists: {0}")]
    SessionConflict(String),
    #[error("RPC caller differs from the caller that prepared the session")]
    CallerMismatch,
    #[error("RPC apply request does not match the prepared transaction fingerprint or action set")]
    PlanMismatch,
    #[error(transparent)]
    Execution(#[from] TweakExecutionError),
}

impl TweakRpcError {
    pub fn code(&self) -> TweakRpcErrorCode {
        match self {
            Self::InvalidRequest(_) => TweakRpcErrorCode::InvalidRequest,
            Self::UnauthorizedCaller(_) => TweakRpcErrorCode::UnauthorizedCaller,
            Self::PermissionDenied(_) => TweakRpcErrorCode::PermissionDenied,
            Self::ConfirmationRequired => TweakRpcErrorCode::ConfirmationRequired,
            Self::SessionNotFound(_) => TweakRpcErrorCode::SessionNotFound,
            Self::SessionConflict(_) => TweakRpcErrorCode::SessionConflict,
            Self::CallerMismatch => TweakRpcErrorCode::CallerMismatch,
            Self::PlanMismatch => TweakRpcErrorCode::PlanMismatch,
            Self::Execution(TweakExecutionError::NothingToChange) => TweakRpcErrorCode::NoChange,
            Self::Execution(TweakExecutionError::UnsupportedPlatform) => {
                TweakRpcErrorCode::UnsupportedPlatform
            }
            Self::Execution(
                TweakExecutionError::InvalidRequest(_)
                | TweakExecutionError::UnsupportedTweak(_)
                | TweakExecutionError::TargetMismatch(_)
                | TweakExecutionError::UnsupportedOperation(_)
                | TweakExecutionError::NonCertifiedTweak(_),
            ) => TweakRpcErrorCode::InvalidRequest,
            Self::Execution(_) => TweakRpcErrorCode::ExecutionFailed,
        }
    }

    pub fn payload(&self) -> TweakRpcErrorPayload {
        TweakRpcErrorPayload {
            code: self.code(),
            message: self.to_string(),
        }
    }
}

struct PendingTweakRpcSession {
    caller: TweakRpcCaller,
    mission_id: String,
    plan_fingerprint: String,
    session: TweakExecutionSession,
}

pub struct TweakRpcService {
    catalogue: TweakCatalogue,
    policy: TweakRpcPolicy,
    pending: BTreeMap<String, PendingTweakRpcSession>,
}

impl TweakRpcService {
    pub fn new(catalogue: TweakCatalogue, policy: TweakRpcPolicy) -> Result<Self, TweakRpcError> {
        catalogue.validate().map_err(TweakExecutionError::from)?;
        Ok(Self {
            catalogue,
            policy,
            pending: BTreeMap::new(),
        })
    }

    pub fn pending_session_count(&self) -> usize {
        self.pending.len()
    }

    pub fn prepare(
        &mut self,
        context: &TweakRpcContext,
        request: TweakRpcPrepareRequest,
    ) -> Result<TweakRpcPrepared, TweakRpcError> {
        self.validate_prepare(context, &request)?;
        let session = prepare_windows_tweaks(
            &self.catalogue,
            &request.selected_ids,
            request.mission_id.clone(),
        )?;
        self.store_prepared(context, request, session)
    }

    pub fn apply(
        &mut self,
        context: &TweakRpcContext,
        request: TweakRpcApplyRequest,
    ) -> Result<TweakRpcExecutionReceipt, TweakRpcError> {
        let authorization = self.validate_apply(context, &request)?;
        let pending = self
            .pending
            .remove(&request.session_id)
            .ok_or_else(|| TweakRpcError::SessionNotFound(request.session_id.clone()))?;

        #[cfg(windows)]
        {
            let mut pending = pending;
            let capability = TweakExecutorCapability::for_rpc();
            pending.session.authorize(&capability, authorization)?;
            pending.session.apply(&capability)?;
            return Ok(execution_receipt(request, pending));
        }

        #[cfg(not(windows))]
        {
            let _ = authorization;
            let _ = pending;
            Err(TweakExecutionError::UnsupportedPlatform.into())
        }
    }

    fn validate_prepare(
        &self,
        context: &TweakRpcContext,
        request: &TweakRpcPrepareRequest,
    ) -> Result<(), TweakRpcError> {
        self.validate_context(context, TWEAK_PREPARE_PERMISSION_SCOPE)?;
        require_text("request id", &request.request_id, 160)?;
        require_text("mission id", &request.mission_id, 240)?;
        if request.selected_ids.is_empty() {
            return Err(TweakRpcError::InvalidRequest(
                "selected tweak ids must not be empty".to_string(),
            ));
        }
        unique_text_set("selected tweak id", &request.selected_ids, 240)?;
        Ok(())
    }

    fn validate_context(
        &self,
        context: &TweakRpcContext,
        required_scope: &str,
    ) -> Result<(), TweakRpcError> {
        context.validate()?;
        if !self.policy.allows(&context.caller) {
            return Err(TweakRpcError::UnauthorizedCaller(
                context.caller.principal.clone(),
            ));
        }
        if !context.has_scope(required_scope) {
            return Err(TweakRpcError::PermissionDenied(required_scope.to_string()));
        }
        Ok(())
    }

    fn store_prepared(
        &mut self,
        context: &TweakRpcContext,
        request: TweakRpcPrepareRequest,
        session: TweakExecutionSession,
    ) -> Result<TweakRpcPrepared, TweakRpcError> {
        let plan_fingerprint = session
            .plan()
            .transaction()
            .fingerprint()
            .map_err(TweakExecutionError::from)?;
        let session_id = format!("phase12:{}:{plan_fingerprint}", request.request_id);
        if self.pending.contains_key(&session_id) {
            return Err(TweakRpcError::SessionConflict(session_id));
        }
        let actions = session
            .plan()
            .steps()
            .iter()
            .map(|step| TweakRpcPreparedAction {
                tweak_id: step.tweak_id().to_string(),
                desired_dword: step.desired_dword(),
                baseline: step.baseline(),
            })
            .collect();
        let prepared = TweakRpcPrepared {
            schema_version: NEO_RPC_SCHEMA_VERSION,
            request_id: request.request_id,
            session_id: session_id.clone(),
            mission_id: request.mission_id.clone(),
            plan_fingerprint: plan_fingerprint.clone(),
            actions,
            confirmation_required: true,
            stage: session.stage(),
        };
        self.pending.insert(
            session_id,
            PendingTweakRpcSession {
                caller: context.caller.clone(),
                mission_id: request.mission_id,
                plan_fingerprint,
                session,
            },
        );
        Ok(prepared)
    }

    fn validate_apply(
        &self,
        context: &TweakRpcContext,
        request: &TweakRpcApplyRequest,
    ) -> Result<TransactionAuthorization, TweakRpcError> {
        self.validate_context(context, TWEAK_APPLY_PERMISSION_SCOPE)?;
        require_text("request id", &request.request_id, 160)?;
        require_text("session id", &request.session_id, 512)?;
        require_text("plan fingerprint", &request.plan_fingerprint, 128)?;
        let pending = self
            .pending
            .get(&request.session_id)
            .ok_or_else(|| TweakRpcError::SessionNotFound(request.session_id.clone()))?;
        if pending.caller != context.caller {
            return Err(TweakRpcError::CallerMismatch);
        }
        if !request.confirmed {
            return Err(TweakRpcError::ConfirmationRequired);
        }
        if request.plan_fingerprint != pending.plan_fingerprint {
            return Err(TweakRpcError::PlanMismatch);
        }
        let approved = unique_text_set("approved action id", &request.approved_action_ids, 240)?;
        let expected = pending
            .session
            .plan()
            .steps()
            .iter()
            .map(|step| step.tweak_id().to_string())
            .collect::<BTreeSet<_>>();
        if approved != expected {
            return Err(TweakRpcError::PlanMismatch);
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
    fn prepare_with_test_host<H: TweakHost>(
        &mut self,
        context: &TweakRpcContext,
        request: TweakRpcPrepareRequest,
        host: &H,
    ) -> Result<TweakRpcPrepared, TweakRpcError> {
        self.validate_prepare(context, &request)?;
        let session = prepare_with_host(
            &self.catalogue,
            &request.selected_ids,
            request.mission_id.clone(),
            host,
        )?;
        self.store_prepared(context, request, session)
    }

    #[cfg(test)]
    fn apply_with_test_host<H: TweakHost>(
        &mut self,
        context: &TweakRpcContext,
        request: TweakRpcApplyRequest,
        host: &mut H,
    ) -> Result<TweakRpcExecutionReceipt, TweakRpcError> {
        let authorization = self.validate_apply(context, &request)?;
        let mut pending = self
            .pending
            .remove(&request.session_id)
            .ok_or_else(|| TweakRpcError::SessionNotFound(request.session_id.clone()))?;
        let _capability = TweakExecutorCapability::for_rpc();
        authorize_with_host(&mut pending.session, authorization, host)?;
        apply_with_host(&mut pending.session, host)?;
        Ok(execution_receipt(request, pending))
    }
}

#[cfg(any(windows, test))]
fn execution_receipt(
    request: TweakRpcApplyRequest,
    pending: PendingTweakRpcSession,
) -> TweakRpcExecutionReceipt {
    TweakRpcExecutionReceipt {
        schema_version: NEO_RPC_SCHEMA_VERSION,
        request_id: request.request_id,
        session_id: request.session_id,
        mission_id: pending.mission_id,
        plan_fingerprint: pending.plan_fingerprint,
        changed_action_ids: pending.session.changed_ids.iter().cloned().collect(),
        stage: pending.session.stage(),
    }
}

fn require_text(label: &str, value: &str, max_len: usize) -> Result<(), TweakRpcError> {
    if value.trim().is_empty() || value.len() > max_len || value.chars().any(char::is_control) {
        return Err(TweakRpcError::InvalidRequest(format!(
            "{label} must be non-empty, bounded, and contain no control characters"
        )));
    }
    Ok(())
}

fn unique_text_set(
    label: &str,
    values: &[String],
    max_len: usize,
) -> Result<BTreeSet<String>, TweakRpcError> {
    let mut result = BTreeSet::new();
    for value in values {
        require_text(label, value, max_len)?;
        if !result.insert(value.clone()) {
            return Err(TweakRpcError::InvalidRequest(format!(
                "duplicate {label}: {value}"
            )));
        }
    }
    Ok(result)
}

#[cfg(test)]
#[path = "rpc_tests.rs"]
mod tests;
