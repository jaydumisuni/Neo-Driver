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
//! Mutation methods require an opaque capability with no public constructor. Phase 18 does not issue
//! that capability through CLI, GUI, plugin, MCP, or RPC surfaces; it does not use Store/network
//! acquisition, provision/deprovision packages, or mutate all-user state.

#[cfg(any(windows, test))]
mod engine;
mod error;
mod model;
#[cfg(windows)]
mod windows;

pub use error::DebloatRestoreExecutionError;
pub use model::{
    DebloatRestoreExecutionPlan, DebloatRestoreExecutionSession, DebloatRestoreExecutionStep,
    DebloatRestoreExecutorCapability,
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
