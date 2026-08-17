use crate::executor::{RepairExecutionSession, RepairExecutorCapability};
use crate::host::RepairHost;
use crate::operation::RepairOperation;
use crate::session_store::{RepairResumeSessionStore, RepairSessionOwner};
use crate::{RepairBaseline, RepairError};
use neo_transaction::{ActionAcknowledgement, TransactionAuthorization, TransactionStage};
use neo_vault::VaultLayout;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

pub const REPAIR_RPC_SCHEMA: &str = "neo-repair-rpc-v1";
pub const REPAIR_RPC_PREPARE_TOOL: &str = "neo_repair_prepare";
pub const REPAIR_RPC_PREPARE_METHOD: &str = "neo.repair.prepare";
pub const REPAIR_RPC_APPLY_TOOL: &str = "neo_repair_apply";
pub const REPAIR_RPC_APPLY_METHOD: &str = "neo.repair.apply";
pub const REPAIR_RPC_RESUME_TOOL: &str = "neo_repair_resume";
pub const REPAIR_RPC_RESUME_METHOD: &str = "neo.repair.resume";
pub const REPAIR_INSPECT_SCOPE: &str = "repair.inspect";
pub const REPAIR_APPLY_SCOPE: &str = "repair.apply";
pub const WINDOWS_FEATURES_INSPECT_SCOPE: &str = "windows_features.inspect";
pub const WINDOWS_FEATURES_APPLY_SCOPE: &str = "windows_features.apply";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RepairRpcCallerKind {
    Hunter,
    Oracle,
    Gui,
    Internal,
}

impl RepairRpcCallerKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Hunter => "hunter",
            Self::Oracle => "oracle",
            Self::Gui => "gui",
            Self::Internal => "internal",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RepairRpcCallerContext {
    pub caller_kind: RepairRpcCallerKind,
    pub principal: String,
    pub granted_scopes: BTreeSet<String>,
}

impl RepairRpcCallerContext {
    pub fn new(
        caller_kind: RepairRpcCallerKind,
        principal: impl Into<String>,
        granted_scopes: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<Self, RepairRpcError> {
        let context = Self {
            caller_kind,
            principal: principal.into(),
            granted_scopes: granted_scopes.into_iter().map(Into::into).collect(),
        };
        context.validate()?;
        Ok(context)
    }

    fn validate(&self) -> Result<(), RepairRpcError> {
        validate_text("principal", &self.principal, 160)?;
        if self.granted_scopes.len() > 16 {
            return Err(RepairRpcError::InvalidRequest(
                "trusted caller scope count exceeds Phase 21 bound".to_string(),
            ));
        }
        for scope in &self.granted_scopes {
            validate_text("scope", scope, 96)?;
        }
        Ok(())
    }

    fn owner(&self) -> Result<RepairSessionOwner, RepairRpcError> {
        RepairSessionOwner::new(self.caller_kind.as_str(), self.principal.clone())
            .map_err(RepairRpcError::Repair)
    }
}

#[derive(Debug, Clone, Default)]
pub struct RepairRpcPolicy {
    allowed: BTreeMap<RepairRpcCallerKind, BTreeSet<String>>,
}

impl RepairRpcPolicy {
    pub fn allow(
        mut self,
        caller: RepairRpcCallerKind,
        scopes: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.allowed
            .entry(caller)
            .or_default()
            .extend(scopes.into_iter().map(Into::into));
        self
    }

    fn authorize(
        &self,
        context: &RepairRpcCallerContext,
        scope: &str,
    ) -> Result<(), RepairRpcError> {
        context.validate()?;
        let allowed = self
            .allowed
            .get(&context.caller_kind)
            .ok_or(RepairRpcError::UnauthorizedCaller)?;
        if !allowed.contains(scope) || !context.granted_scopes.contains(scope) {
            return Err(RepairRpcError::PermissionDenied(scope.to_string()));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepairRpcAuthorityClass {
    Repair,
    WindowsFeatures,
}

impl RepairRpcAuthorityClass {
    fn for_operation(operation: RepairOperation) -> Self {
        match operation {
            RepairOperation::RestoreComponentStore | RepairOperation::RepairSystemFiles => {
                Self::Repair
            }
            RepairOperation::SetWindowsFeature { .. } => Self::WindowsFeatures,
        }
    }

    fn prepare_scope(self) -> &'static str {
        match self {
            Self::Repair => REPAIR_INSPECT_SCOPE,
            Self::WindowsFeatures => WINDOWS_FEATURES_INSPECT_SCOPE,
        }
    }

    fn apply_scope(self) -> &'static str {
        match self {
            Self::Repair => REPAIR_APPLY_SCOPE,
            Self::WindowsFeatures => WINDOWS_FEATURES_APPLY_SCOPE,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepairRpcPrepareRequest {
    pub request_id: String,
    pub mission_id: String,
    pub operation: RepairOperation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepairRpcApplyRequest {
    pub request_id: String,
    pub session_id: String,
    pub authority: RepairRpcAuthorityClass,
    pub plan_fingerprint: String,
    pub approved_action_ids: Vec<String>,
    pub confirmed: bool,
    pub irreversible_acknowledged: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepairRpcResumeRequest {
    pub request_id: String,
    pub session_id: String,
    pub authority: RepairRpcAuthorityClass,
    pub plan_fingerprint: String,
    pub expected_version: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairRpcPrepared {
    pub schema: String,
    pub tool: String,
    pub method: String,
    pub request_id: String,
    pub session_id: String,
    pub authority: RepairRpcAuthorityClass,
    pub operation: RepairOperation,
    pub baseline: RepairBaseline,
    pub action_id: String,
    pub plan_fingerprint: String,
    pub confirmation_required: bool,
    pub irreversible_acknowledgement_required: bool,
    pub machine_changes: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairRpcExecutionReceipt {
    pub schema: String,
    pub tool: String,
    pub method: String,
    pub request_id: String,
    pub session_id: String,
    pub authority: RepairRpcAuthorityClass,
    pub action_id: String,
    pub plan_fingerprint: String,
    pub stage: String,
    pub persisted_version: u64,
    pub resume_required: bool,
    pub machine_changes: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepairRpcErrorPayload {
    pub schema: String,
    pub code: String,
    pub message: String,
    pub retryable: bool,
}

#[derive(Debug, Error)]
pub enum RepairRpcError {
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    #[error("caller is not permitted to use Phase 21 repair RPC")]
    UnauthorizedCaller,
    #[error("required scope is missing: {0}")]
    PermissionDenied(String),
    #[error("explicit confirmation is required")]
    ConfirmationRequired,
    #[error("irreversible Windows repair acknowledgement is required")]
    IrreversibleAcknowledgementRequired,
    #[error("repair RPC session not found")]
    SessionNotFound,
    #[error("repair RPC session belongs to another trusted caller")]
    CallerMismatch,
    #[error("repair RPC session authority class differs from request")]
    AuthorityMismatch,
    #[error("repair RPC plan fingerprint differs from prepared session")]
    PlanMismatch,
    #[error("repair RPC approval differs from prepared action")]
    ApprovalMismatch,
    #[error("repair resume version differs from latest persisted state")]
    VersionMismatch,
    #[error("repair RPC session is terminal and cannot be resumed")]
    SessionNotResumable,
    #[error("repair RPC service sequence exhausted")]
    SequenceExhausted,
    #[error("Phase 21 repair service failure: {0}")]
    Repair(#[from] RepairError),
}

impl RepairRpcError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidRequest(_) => "invalid_request",
            Self::UnauthorizedCaller => "unauthorized_caller",
            Self::PermissionDenied(_) => "permission_denied",
            Self::ConfirmationRequired => "confirmation_required",
            Self::IrreversibleAcknowledgementRequired => "irreversible_ack_required",
            Self::SessionNotFound => "session_not_found",
            Self::CallerMismatch => "caller_mismatch",
            Self::AuthorityMismatch => "authority_mismatch",
            Self::PlanMismatch => "plan_mismatch",
            Self::ApprovalMismatch => "approval_mismatch",
            Self::VersionMismatch => "version_mismatch",
            Self::SessionNotResumable => "session_not_resumable",
            Self::SequenceExhausted => "sequence_exhausted",
            Self::Repair(RepairError::UnsupportedPlatform) => "unsupported_platform",
            Self::Repair(RepairError::ElevationRequired) => "elevation_required",
            Self::Repair(RepairError::NothingToRepair(_) | RepairError::NothingToChange(_)) => {
                "nothing_to_do"
            }
            Self::Repair(
                RepairError::StateUnavailable(_) | RepairError::FeatureNotReversible(_),
            ) => "state_unavailable",
            Self::Repair(
                RepairError::SessionStore(_) | RepairError::InvalidPersistedSession(_),
            ) => "session_store_failed",
            Self::Repair(RepairError::BaselineDrift(_)) => "baseline_drift",
            Self::Repair(_) => "execution_failed",
        }
    }

    pub fn payload(&self) -> RepairRpcErrorPayload {
        let (message, retryable) = match self {
            Self::InvalidRequest(_) => ("The repair request is invalid.", false),
            Self::UnauthorizedCaller => {
                ("This caller is not authorized for repair service.", false)
            }
            Self::PermissionDenied(_) => ("The required repair permission is missing.", false),
            Self::ConfirmationRequired => ("Explicit repair confirmation is required.", false),
            Self::IrreversibleAcknowledgementRequired => (
                "Explicit irreversible-repair acknowledgement is required.",
                false,
            ),
            Self::SessionNotFound => ("The prepared repair session was not found.", false),
            Self::CallerMismatch => ("The repair session belongs to another caller.", false),
            Self::AuthorityMismatch => (
                "The repair authority class does not match the session.",
                false,
            ),
            Self::PlanMismatch => ("The repair plan fingerprint does not match.", false),
            Self::ApprovalMismatch => ("The approved repair action does not match.", false),
            Self::VersionMismatch => ("A newer repair session state already exists.", false),
            Self::SessionNotResumable => ("The repair session is already terminal.", false),
            Self::SequenceExhausted => ("The repair service session sequence is exhausted.", false),
            Self::Repair(RepairError::UnsupportedPlatform) => (
                "Windows repair authority is unavailable on this platform.",
                false,
            ),
            Self::Repair(RepairError::ElevationRequired) => {
                ("Elevated Windows servicing authority is required.", true)
            }
            Self::Repair(RepairError::NothingToRepair(_) | RepairError::NothingToChange(_)) => {
                ("The selected operation is already satisfied.", false)
            }
            Self::Repair(
                RepairError::StateUnavailable(_) | RepairError::FeatureNotReversible(_),
            ) => (
                "The required Windows state is unavailable for this operation.",
                true,
            ),
            Self::Repair(
                RepairError::SessionStore(_) | RepairError::InvalidPersistedSession(_),
            ) => (
                "The trusted repair session store is unavailable or invalid.",
                true,
            ),
            Self::Repair(RepairError::BaselineDrift(_)) => (
                "Windows state changed after repair preparation; prepare again.",
                true,
            ),
            Self::Repair(_) => ("The repair operation could not be completed safely.", true),
        };
        RepairRpcErrorPayload {
            schema: REPAIR_RPC_SCHEMA.to_string(),
            code: self.code().to_string(),
            message: message.to_string(),
            retryable,
        }
    }
}

struct PendingRepairRpcSession {
    owner: RepairSessionOwner,
    authority: RepairRpcAuthorityClass,
    plan_fingerprint: String,
    action_id: String,
    session: RepairExecutionSession,
}

pub struct RepairRpcService {
    policy: RepairRpcPolicy,
    store: RepairResumeSessionStore,
    service_instance_id: String,
    sequence: u64,
    pending: BTreeMap<String, PendingRepairRpcSession>,
}

impl RepairRpcService {
    pub fn new(
        layout: VaultLayout,
        policy: RepairRpcPolicy,
        service_instance_id: impl Into<String>,
    ) -> Result<Self, RepairRpcError> {
        let service_instance_id = service_instance_id.into();
        validate_text("service instance id", &service_instance_id, 128)?;
        Ok(Self {
            policy,
            store: RepairResumeSessionStore::new(layout),
            service_instance_id,
            sequence: 0,
            pending: BTreeMap::new(),
        })
    }

    #[cfg(windows)]
    pub fn prepare(
        &mut self,
        context: &RepairRpcCallerContext,
        request: RepairRpcPrepareRequest,
    ) -> Result<RepairRpcPrepared, RepairRpcError> {
        let host = crate::host::WindowsRepairHost::new()?;
        self.prepare_with_host(context, request, &host)
    }

    #[cfg(not(windows))]
    pub fn prepare(
        &mut self,
        _context: &RepairRpcCallerContext,
        _request: RepairRpcPrepareRequest,
    ) -> Result<RepairRpcPrepared, RepairRpcError> {
        Err(RepairError::UnsupportedPlatform.into())
    }

    #[cfg(windows)]
    pub fn apply(
        &mut self,
        context: &RepairRpcCallerContext,
        request: RepairRpcApplyRequest,
    ) -> Result<RepairRpcExecutionReceipt, RepairRpcError> {
        let host = crate::host::WindowsRepairHost::new()?;
        self.apply_with_host(context, request, &host)
    }

    #[cfg(not(windows))]
    pub fn apply(
        &mut self,
        _context: &RepairRpcCallerContext,
        _request: RepairRpcApplyRequest,
    ) -> Result<RepairRpcExecutionReceipt, RepairRpcError> {
        Err(RepairError::UnsupportedPlatform.into())
    }

    #[cfg(windows)]
    pub fn resume(
        &mut self,
        context: &RepairRpcCallerContext,
        request: RepairRpcResumeRequest,
    ) -> Result<RepairRpcExecutionReceipt, RepairRpcError> {
        let host = crate::host::WindowsRepairHost::new()?;
        self.resume_with_host(context, request, &host)
    }

    #[cfg(not(windows))]
    pub fn resume(
        &mut self,
        _context: &RepairRpcCallerContext,
        _request: RepairRpcResumeRequest,
    ) -> Result<RepairRpcExecutionReceipt, RepairRpcError> {
        Err(RepairError::UnsupportedPlatform.into())
    }

    pub(crate) fn prepare_with_host<H: RepairHost>(
        &mut self,
        context: &RepairRpcCallerContext,
        request: RepairRpcPrepareRequest,
        host: &H,
    ) -> Result<RepairRpcPrepared, RepairRpcError> {
        let authority = RepairRpcAuthorityClass::for_operation(request.operation);
        self.policy.authorize(context, authority.prepare_scope())?;
        validate_text("request id", &request.request_id, 160)?;
        validate_text("mission id", &request.mission_id, 160)?;
        let session =
            RepairExecutionSession::prepare_with_host(request.operation, request.mission_id, host)?;
        let plan_fingerprint = session
            .plan()
            .transaction()
            .fingerprint()
            .map_err(RepairError::from)?;
        let action_id = session.plan().action_id();
        let baseline = session.plan().baseline();
        let owner = context.owner()?;
        let session_id = self.next_session_id(&plan_fingerprint)?;
        self.pending.retain(|_, pending| pending.owner != owner);
        self.pending.insert(
            session_id.clone(),
            PendingRepairRpcSession {
                owner,
                authority,
                plan_fingerprint: plan_fingerprint.clone(),
                action_id: action_id.clone(),
                session,
            },
        );
        Ok(RepairRpcPrepared {
            schema: REPAIR_RPC_SCHEMA.to_string(),
            tool: REPAIR_RPC_PREPARE_TOOL.to_string(),
            method: REPAIR_RPC_PREPARE_METHOD.to_string(),
            request_id: request.request_id,
            session_id,
            authority,
            operation: request.operation,
            baseline,
            action_id,
            plan_fingerprint,
            confirmation_required: true,
            irreversible_acknowledgement_required: matches!(
                request.operation,
                RepairOperation::RestoreComponentStore | RepairOperation::RepairSystemFiles
            ),
            machine_changes: false,
        })
    }

    pub(crate) fn apply_with_host<H: RepairHost>(
        &mut self,
        context: &RepairRpcCallerContext,
        request: RepairRpcApplyRequest,
        host: &H,
    ) -> Result<RepairRpcExecutionReceipt, RepairRpcError> {
        self.policy
            .authorize(context, request.authority.apply_scope())?;
        validate_text("request id", &request.request_id, 160)?;
        validate_text("session id", &request.session_id, 512)?;
        validate_text("plan fingerprint", &request.plan_fingerprint, 128)?;
        if !request.confirmed {
            return Err(RepairRpcError::ConfirmationRequired);
        }
        let owner = context.owner()?;
        let pending = self
            .pending
            .get(&request.session_id)
            .ok_or(RepairRpcError::SessionNotFound)?;
        if pending.owner != owner {
            return Err(RepairRpcError::CallerMismatch);
        }
        if pending.authority != request.authority {
            return Err(RepairRpcError::AuthorityMismatch);
        }
        if pending.plan_fingerprint != request.plan_fingerprint {
            return Err(RepairRpcError::PlanMismatch);
        }
        if request.approved_action_ids != vec![pending.action_id.clone()] {
            return Err(RepairRpcError::ApprovalMismatch);
        }
        let irreversible = matches!(
            pending.session.plan().operation(),
            RepairOperation::RestoreComponentStore | RepairOperation::RepairSystemFiles
        );
        if irreversible && !request.irreversible_acknowledged {
            return Err(RepairRpcError::IrreversibleAcknowledgementRequired);
        }
        if !irreversible && request.irreversible_acknowledged {
            return Err(RepairRpcError::InvalidRequest(
                "irreversible acknowledgement is not valid for a reversible feature action"
                    .to_string(),
            ));
        }

        let mut pending = self
            .pending
            .remove(&request.session_id)
            .expect("validated pending session exists");
        let capability = RepairExecutorCapability::for_rpc();
        pending.session.authorize(
            &capability,
            TransactionAuthorization {
                plan_fingerprint: pending.plan_fingerprint.clone(),
                approved_action_ids: vec![pending.action_id.clone()],
                manual_override_action_ids: Vec::new(),
                high_risk_ack_action_ids: Vec::new(),
                irreversible_acknowledgements: if irreversible {
                    vec![ActionAcknowledgement {
                        action_id: pending.action_id.clone(),
                        reason: "Confirmed through Phase 21 trusted repair RPC".to_string(),
                    }]
                } else {
                    Vec::new()
                },
            },
        )?;
        pending.session.begin_apply_with_host(&capability, host)?;
        let write_ahead =
            self.store
                .persist(&request.session_id, &pending.owner, &pending.session)?;
        let execution_result = pending
            .session
            .execute_applying_with_host(&capability, host);
        let persisted =
            self.store
                .persist(&request.session_id, &pending.owner, &pending.session)?;
        if let Err(error) = execution_result {
            return Err(error.into());
        }
        if persisted.version < write_ahead.version {
            return Err(RepairRpcError::Repair(RepairError::SessionStore(
                "persisted repair version regressed".to_string(),
            )));
        }
        Ok(execution_receipt(
            REPAIR_RPC_APPLY_TOOL,
            REPAIR_RPC_APPLY_METHOD,
            request.request_id,
            request.session_id,
            request.authority,
            pending.action_id,
            pending.plan_fingerprint,
            &persisted.session,
            persisted.version,
        ))
    }

    pub(crate) fn resume_with_host<H: RepairHost>(
        &mut self,
        context: &RepairRpcCallerContext,
        request: RepairRpcResumeRequest,
        host: &H,
    ) -> Result<RepairRpcExecutionReceipt, RepairRpcError> {
        self.policy
            .authorize(context, request.authority.apply_scope())?;
        validate_text("request id", &request.request_id, 160)?;
        validate_text("session id", &request.session_id, 512)?;
        validate_text("plan fingerprint", &request.plan_fingerprint, 128)?;
        if request.expected_version == 0 {
            return Err(RepairRpcError::InvalidRequest(
                "expected version must be positive".to_string(),
            ));
        }
        let owner = context.owner()?;
        let mut stored = self
            .store
            .load_latest(&request.session_id)?
            .ok_or(RepairRpcError::SessionNotFound)?;
        if stored.session_id != request.session_id {
            return Err(RepairRpcError::InvalidRequest(
                "persisted session identity differs from resume request".to_string(),
            ));
        }
        if stored.owner != owner {
            return Err(RepairRpcError::CallerMismatch);
        }
        if stored.version != request.expected_version {
            return Err(RepairRpcError::VersionMismatch);
        }
        let authority = RepairRpcAuthorityClass::for_operation(stored.session.plan().operation());
        if authority != request.authority {
            return Err(RepairRpcError::AuthorityMismatch);
        }
        let fingerprint = stored
            .session
            .plan()
            .transaction()
            .fingerprint()
            .map_err(RepairError::from)?;
        if fingerprint != request.plan_fingerprint {
            return Err(RepairRpcError::PlanMismatch);
        }
        if is_terminal(stored.session.stage()) {
            return Err(RepairRpcError::SessionNotResumable);
        }
        let action_id = stored.session.plan().action_id();
        let capability = RepairExecutorCapability::for_rpc();
        let resume_result = stored.session.resume_with_host(&capability, host);
        let persisted = self
            .store
            .persist(&request.session_id, &stored.owner, &stored.session)?;
        if let Err(error) = resume_result {
            return Err(error.into());
        }
        Ok(execution_receipt(
            REPAIR_RPC_RESUME_TOOL,
            REPAIR_RPC_RESUME_METHOD,
            request.request_id,
            request.session_id,
            request.authority,
            action_id,
            fingerprint,
            &persisted.session,
            persisted.version,
        ))
    }

    fn next_session_id(&mut self, plan_fingerprint: &str) -> Result<String, RepairRpcError> {
        loop {
            self.sequence = self
                .sequence
                .checked_add(1)
                .ok_or(RepairRpcError::SequenceExhausted)?;
            let session_id = format!(
                "phase21:{}:{}:{}",
                self.service_instance_id, self.sequence, plan_fingerprint
            );
            if !self.pending.contains_key(&session_id)
                && self.store.load_latest(&session_id)?.is_none()
            {
                return Ok(session_id);
            }
        }
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "fixed Phase 21 RPC receipt mirrors the protocol envelope"
)]
fn execution_receipt(
    tool: &str,
    method: &str,
    request_id: String,
    session_id: String,
    authority: RepairRpcAuthorityClass,
    action_id: String,
    plan_fingerprint: String,
    session: &RepairExecutionSession,
    persisted_version: u64,
) -> RepairRpcExecutionReceipt {
    RepairRpcExecutionReceipt {
        schema: REPAIR_RPC_SCHEMA.to_string(),
        tool: tool.to_string(),
        method: method.to_string(),
        request_id,
        session_id,
        authority,
        action_id,
        plan_fingerprint,
        stage: stage_name(session.stage()).to_string(),
        persisted_version,
        resume_required: matches!(
            session.stage(),
            TransactionStage::Applying
                | TransactionStage::AwaitingReboot
                | TransactionStage::Blocked
                | TransactionStage::AwaitingRollbackReboot
        ),
        machine_changes: true,
    }
}

fn stage_name(stage: TransactionStage) -> &'static str {
    match stage {
        TransactionStage::Planned => "planned",
        TransactionStage::BaselineCaptured => "baseline_captured",
        TransactionStage::Authorized => "authorized",
        TransactionStage::Applying => "applying",
        TransactionStage::AwaitingReboot => "awaiting_reboot",
        TransactionStage::Verifying => "verifying",
        TransactionStage::RollingBack => "rolling_back",
        TransactionStage::AwaitingRollbackReboot => "awaiting_rollback_reboot",
        TransactionStage::Complete => "complete",
        TransactionStage::RolledBack => "rolled_back",
        TransactionStage::Failed => "failed",
        TransactionStage::Blocked => "blocked",
    }
}

fn is_terminal(stage: TransactionStage) -> bool {
    matches!(
        stage,
        TransactionStage::Complete | TransactionStage::RolledBack | TransactionStage::Failed
    )
}

fn validate_text(label: &str, value: &str, max_len: usize) -> Result<(), RepairRpcError> {
    if value.trim().is_empty() || value.len() > max_len || value.chars().any(char::is_control) {
        return Err(RepairRpcError::InvalidRequest(format!(
            "{label} must be non-empty bounded text without control characters"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host::testsupport::FakeRepairHost;
    use crate::{
        ComponentStoreState, FeatureDesiredState, SupportedWindowsFeature, SystemFileState,
        WindowsFeatureState,
    };
    use neo_vault::VaultMode;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn fixture() -> (
        PathBuf,
        RepairRpcService,
        RepairRpcCallerContext,
        FakeRepairHost,
    ) {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("neo-repair-rpc-{}-{nonce}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let layout = VaultLayout::new(VaultMode::Portable, &root).unwrap();
        let policy = RepairRpcPolicy::default().allow(
            RepairRpcCallerKind::Oracle,
            [
                REPAIR_INSPECT_SCOPE,
                REPAIR_APPLY_SCOPE,
                WINDOWS_FEATURES_INSPECT_SCOPE,
                WINDOWS_FEATURES_APPLY_SCOPE,
            ],
        );
        let service = RepairRpcService::new(layout, policy, "test-service").unwrap();
        let caller = RepairRpcCallerContext::new(
            RepairRpcCallerKind::Oracle,
            "owner",
            [
                REPAIR_INSPECT_SCOPE,
                REPAIR_APPLY_SCOPE,
                WINDOWS_FEATURES_INSPECT_SCOPE,
                WINDOWS_FEATURES_APPLY_SCOPE,
            ],
        )
        .unwrap();
        let host = FakeRepairHost::new(
            ComponentStoreState::Repairable,
            SystemFileState::IntegrityViolations,
        );
        (root, service, caller, host)
    }

    #[test]
    fn raw_requests_reject_trusted_context_injection() {
        let json = r#"{
            "request_id":"req",
            "mission_id":"mission",
            "operation":{"kind":"restore_component_store"},
            "caller_kind":"oracle",
            "granted_scopes":["repair.apply"]
        }"#;
        assert!(serde_json::from_str::<RepairRpcPrepareRequest>(json).is_err());
    }

    #[test]
    fn authorization_happens_before_machine_evidence_lookup() {
        let (root, mut service, _caller, host) = fixture();
        let denied = RepairRpcCallerContext::new(
            RepairRpcCallerKind::Gui,
            "gui-user",
            [REPAIR_INSPECT_SCOPE],
        )
        .unwrap();
        let error = service
            .prepare_with_host(
                &denied,
                RepairRpcPrepareRequest {
                    request_id: "req".to_string(),
                    mission_id: "mission".to_string(),
                    operation: RepairOperation::RestoreComponentStore,
                },
                &host,
            )
            .unwrap_err();
        assert!(matches!(error, RepairRpcError::UnauthorizedCaller));
        assert!(host.observed.borrow().is_empty());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn malformed_confirmation_does_not_consume_prepared_session() {
        let (root, mut service, caller, host) = fixture();
        let prepared = service
            .prepare_with_host(
                &caller,
                RepairRpcPrepareRequest {
                    request_id: "prepare".to_string(),
                    mission_id: "mission".to_string(),
                    operation: RepairOperation::RestoreComponentStore,
                },
                &host,
            )
            .unwrap();
        let bad = service.apply_with_host(
            &caller,
            RepairRpcApplyRequest {
                request_id: "apply-bad".to_string(),
                session_id: prepared.session_id.clone(),
                authority: prepared.authority,
                plan_fingerprint: prepared.plan_fingerprint.clone(),
                approved_action_ids: vec![prepared.action_id.clone(), prepared.action_id.clone()],
                confirmed: true,
                irreversible_acknowledged: true,
            },
            &host,
        );
        assert!(matches!(bad, Err(RepairRpcError::ApprovalMismatch)));
        assert!(service.pending.contains_key(&prepared.session_id));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn valid_apply_is_consumed_and_persisted_before_and_after_execution() {
        let (root, mut service, caller, host) = fixture();
        let prepared = service
            .prepare_with_host(
                &caller,
                RepairRpcPrepareRequest {
                    request_id: "prepare".to_string(),
                    mission_id: "mission".to_string(),
                    operation: RepairOperation::RestoreComponentStore,
                },
                &host,
            )
            .unwrap();
        let receipt = service
            .apply_with_host(
                &caller,
                RepairRpcApplyRequest {
                    request_id: "apply".to_string(),
                    session_id: prepared.session_id.clone(),
                    authority: prepared.authority,
                    plan_fingerprint: prepared.plan_fingerprint.clone(),
                    approved_action_ids: vec![prepared.action_id.clone()],
                    confirmed: true,
                    irreversible_acknowledged: true,
                },
                &host,
            )
            .unwrap();
        assert_eq!(receipt.stage, "complete");
        assert!(receipt.persisted_version >= 2);
        assert!(!service.pending.contains_key(&prepared.session_id));
        assert_eq!(host.executed.borrow().len(), 1);
        let replay = service.apply_with_host(
            &caller,
            RepairRpcApplyRequest {
                request_id: "replay".to_string(),
                session_id: prepared.session_id,
                authority: prepared.authority,
                plan_fingerprint: prepared.plan_fingerprint,
                approved_action_ids: vec![prepared.action_id],
                confirmed: true,
                irreversible_acknowledged: true,
            },
            &host,
        );
        assert!(matches!(replay, Err(RepairRpcError::SessionNotFound)));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn pending_feature_resume_is_version_bound_and_single_use() {
        let (root, mut service, caller, host) = fixture();
        let feature = SupportedWindowsFeature::WindowsSubsystemLinux;
        host.set_feature(feature, WindowsFeatureState::Disabled);
        *host.pending_feature_transition.borrow_mut() = true;
        let prepared = service
            .prepare_with_host(
                &caller,
                RepairRpcPrepareRequest {
                    request_id: "prepare-feature".to_string(),
                    mission_id: "mission-feature".to_string(),
                    operation: RepairOperation::SetWindowsFeature {
                        feature,
                        desired: FeatureDesiredState::Enabled,
                    },
                },
                &host,
            )
            .unwrap();
        let apply = service
            .apply_with_host(
                &caller,
                RepairRpcApplyRequest {
                    request_id: "apply-feature".to_string(),
                    session_id: prepared.session_id.clone(),
                    authority: prepared.authority,
                    plan_fingerprint: prepared.plan_fingerprint.clone(),
                    approved_action_ids: vec![prepared.action_id.clone()],
                    confirmed: true,
                    irreversible_acknowledged: false,
                },
                &host,
            )
            .unwrap();
        assert_eq!(apply.stage, "awaiting_reboot");
        assert!(apply.resume_required);
        *host.pending_feature_transition.borrow_mut() = false;
        host.set_feature(feature, WindowsFeatureState::Enabled);
        let resume = service
            .resume_with_host(
                &caller,
                RepairRpcResumeRequest {
                    request_id: "resume-feature".to_string(),
                    session_id: prepared.session_id.clone(),
                    authority: prepared.authority,
                    plan_fingerprint: prepared.plan_fingerprint.clone(),
                    expected_version: apply.persisted_version,
                },
                &host,
            )
            .unwrap();
        assert_eq!(resume.stage, "complete");
        assert!(resume.persisted_version > apply.persisted_version);
        let replay = service.resume_with_host(
            &caller,
            RepairRpcResumeRequest {
                request_id: "resume-replay".to_string(),
                session_id: prepared.session_id,
                authority: prepared.authority,
                plan_fingerprint: prepared.plan_fingerprint,
                expected_version: apply.persisted_version,
            },
            &host,
        );
        assert!(matches!(replay, Err(RepairRpcError::VersionMismatch)));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn caller_safe_error_payload_does_not_leak_internal_session_path() {
        let error = RepairRpcError::Repair(RepairError::SessionStore(
            r"C:\secret\NeoData\sessions\phase21".to_string(),
        ));
        let payload = error.payload();
        assert_eq!(payload.code, "session_store_failed");
        assert!(!payload.message.contains("secret"));
        assert!(!payload.message.contains("NeoData"));
    }
}
