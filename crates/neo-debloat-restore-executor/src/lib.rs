//! Phase 18 transaction-bound post-success current-user AppX restore executor.
//!
//! This crate consumes exactly one Phase 17 prepared inverse transaction. It re-proves the fresh
//! restore-time baseline and exact local staged package/dependency route both before authorization
//! and immediately before mutation. The Windows backend registers only the exact staged full-name
//! identities already proven by Phase 17. Forward verification covers the restored main package and
//! every direct dependency. Failure recovery returns to the Phase 17 restore-time baseline: the main
//! package is absent again, dependencies that were already present are preserved, and only
//! dependencies introduced by the failed restore are removed.
//!
//! Mutation methods require an opaque capability with no public constructor. Phase 20 adds typed
//! MCP/RPC orchestration that may issue that capability only after trusted Phase 19 history selection,
//! fresh Phase 17 preparation, trusted caller policy/scopes, explicit confirmation, exact plan
//! fingerprint/action binding, and single-use service-session validation. No public CLI mutation
//! surface is introduced.

#[cfg(any(windows, test))]
mod engine;
mod error;
mod model;
mod rpc;
#[cfg(windows)]
mod windows;

pub use error::DebloatRestoreExecutionError;
pub use model::{
    DebloatRestoreExecutionPlan, DebloatRestoreExecutionSession, DebloatRestoreExecutionStep,
    DebloatRestoreExecutorCapability,
};
pub use rpc::{
    DebloatRestoreRpcApplyRequest, DebloatRestoreRpcCaller, DebloatRestoreRpcCallerKind,
    DebloatRestoreRpcContext, DebloatRestoreRpcError, DebloatRestoreRpcErrorCode,
    DebloatRestoreRpcErrorPayload, DebloatRestoreRpcExecutionReceipt, DebloatRestoreRpcPolicy,
    DebloatRestoreRpcPrepareRequest, DebloatRestoreRpcPrepared, DebloatRestoreRpcService,
    DEBLOAT_RESTORE_APPLY_PERMISSION_SCOPE, DEBLOAT_RESTORE_PREPARE_PERMISSION_SCOPE,
    MCP_DEBLOAT_RESTORE_APPLY_TOOL, MCP_DEBLOAT_RESTORE_PREPARE_TOOL,
    NEO_DEBLOAT_RPC_SCHEMA_VERSION, RPC_DEBLOAT_RESTORE_APPLY_METHOD,
    RPC_DEBLOAT_RESTORE_PREPARE_METHOD,
};

use neo_debloat_history::DebloatRestorePreparedTransaction;
use neo_transaction::TransactionAuthorization;

pub fn prepare_debloat_restore_execution(
    prepared: &DebloatRestorePreparedTransaction,
) -> Result<DebloatRestoreExecutionSession, DebloatRestoreExecutionError> {
    DebloatRestoreExecutionSession::from_prepared(prepared)
}

impl DebloatRestoreExecutionSession {
    #[cfg(windows)]
    pub fn authorize(
        &mut self,
        _capability: &DebloatRestoreExecutorCapability,
        authorization: TransactionAuthorization,
    ) -> Result<(), DebloatRestoreExecutionError> {
        let host = windows::WindowsDebloatRestoreHost;
        engine::authorize_with_host(self, authorization, &host)
    }

    #[cfg(not(windows))]
    pub fn authorize(
        &mut self,
        _capability: &DebloatRestoreExecutorCapability,
        _authorization: TransactionAuthorization,
    ) -> Result<(), DebloatRestoreExecutionError> {
        Err(DebloatRestoreExecutionError::UnsupportedPlatform)
    }

    #[cfg(windows)]
    pub fn apply(
        &mut self,
        _capability: &DebloatRestoreExecutorCapability,
    ) -> Result<(), DebloatRestoreExecutionError> {
        let _execution_lock = windows::DebloatRestoreExecutionMutex::acquire()?;
        let mut host = windows::WindowsDebloatRestoreHost;
        engine::apply_with_host(self, &mut host)
    }

    #[cfg(not(windows))]
    pub fn apply(
        &mut self,
        _capability: &DebloatRestoreExecutorCapability,
    ) -> Result<(), DebloatRestoreExecutionError> {
        Err(DebloatRestoreExecutionError::UnsupportedPlatform)
    }
}

#[cfg(test)]
mod tests;
