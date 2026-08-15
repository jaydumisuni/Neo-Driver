//! Phase 11 transaction-bound tweak execution with Phase 12 MCP/RPC authority.
//!
//! The public surface exposes curated planning and typed RPC service contracts. Raw registry
//! bindings remain private. Mutation still requires the opaque capability, which Phase 12 issues
//! only inside the policy/scoped/confirmed RPC service path.

#[cfg(any(windows, test))]
mod engine;
mod error;
mod model;
mod rpc;
#[cfg(any(windows, test))]
mod session;

#[cfg(windows)]
mod windows;

pub use error::TweakExecutionError;
pub use model::{
    curated_tweak_ids, RegistrySnapshot, TweakExecutionPlan, TweakExecutionSession,
    TweakExecutionStep, TweakExecutorCapability, SHOW_FILE_EXTENSIONS, SHOW_HIDDEN_FILES,
    TASKBAR_CENTERED_ICONS,
};
pub use rpc::{
    TweakRpcApplyRequest, TweakRpcCaller, TweakRpcCallerKind, TweakRpcContext, TweakRpcError,
    TweakRpcErrorCode, TweakRpcErrorPayload, TweakRpcExecutionReceipt, TweakRpcPolicy,
    TweakRpcPrepareRequest, TweakRpcPrepared, TweakRpcPreparedAction, TweakRpcService,
    MCP_TWEAK_APPLY_TOOL, MCP_TWEAK_PREPARE_TOOL, NEO_RPC_SCHEMA_VERSION, RPC_TWEAK_APPLY_METHOD,
    RPC_TWEAK_PREPARE_METHOD, TWEAK_APPLY_PERMISSION_SCOPE, TWEAK_PREPARE_PERMISSION_SCOPE,
};

use neo_state_plan::TweakCatalogue;
use neo_transaction::TransactionAuthorization;

#[cfg(windows)]
pub fn prepare_windows_tweaks(
    catalogue: &TweakCatalogue,
    selected_ids: &[String],
    mission_id: impl Into<String>,
) -> Result<TweakExecutionSession, TweakExecutionError> {
    let host = windows::WindowsRegistryHost;
    engine::prepare_with_host(catalogue, selected_ids, mission_id, &host)
}

#[cfg(not(windows))]
pub fn prepare_windows_tweaks(
    _catalogue: &TweakCatalogue,
    _selected_ids: &[String],
    _mission_id: impl Into<String>,
) -> Result<TweakExecutionSession, TweakExecutionError> {
    Err(TweakExecutionError::UnsupportedPlatform)
}

impl TweakExecutionSession {
    #[cfg(windows)]
    pub fn authorize(
        &mut self,
        _capability: &TweakExecutorCapability,
        authorization: TransactionAuthorization,
    ) -> Result<(), TweakExecutionError> {
        let host = windows::WindowsRegistryHost;
        session::authorize_with_host(self, authorization, &host)
    }

    #[cfg(not(windows))]
    pub fn authorize(
        &mut self,
        _capability: &TweakExecutorCapability,
        _authorization: TransactionAuthorization,
    ) -> Result<(), TweakExecutionError> {
        Err(TweakExecutionError::UnsupportedPlatform)
    }

    #[cfg(windows)]
    pub fn apply(
        &mut self,
        _capability: &TweakExecutorCapability,
    ) -> Result<(), TweakExecutionError> {
        let _execution_lock = windows::TweakExecutionMutex::acquire()?;
        let mut host = windows::WindowsRegistryHost;
        session::apply_with_host(self, &mut host)
    }

    #[cfg(not(windows))]
    pub fn apply(
        &mut self,
        _capability: &TweakExecutorCapability,
    ) -> Result<(), TweakExecutionError> {
        Err(TweakExecutionError::UnsupportedPlatform)
    }
}

#[cfg(test)]
mod review_tests;
#[cfg(test)]
mod tests;
