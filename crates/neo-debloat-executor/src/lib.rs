//! Phase 16 transaction-bound current-user AppX executor.
//!
//! This crate consumes exactly one Phase 15 prepared current-user debloat transaction. The real
//! Windows backend uses PackageManager full-name removal and staged full-name re-registration,
//! while the shared Phase 4 checkpoint remains the authority for authorization, postcondition
//! verification, and rollback verification. Mutation methods require an opaque capability with no
//! public constructor. Phase 16 does not issue that capability through CLI, GUI, plugin, MCP, or
//! RPC surfaces and does not deprovision packages for all users.

#[cfg(any(windows, test))]
mod engine;
mod error;
mod model;
#[cfg(windows)]
mod windows;

pub use error::DebloatExecutionError;
pub use model::{
    DebloatExecutionPlan, DebloatExecutionSession, DebloatExecutionStep, DebloatExecutorCapability,
};

use neo_debloat_plan::DebloatPreparedTransaction;
use neo_transaction::TransactionAuthorization;

pub fn prepare_debloat_execution(
    prepared: &DebloatPreparedTransaction,
) -> Result<DebloatExecutionSession, DebloatExecutionError> {
    DebloatExecutionSession::from_prepared(prepared)
}

impl DebloatExecutionSession {
    #[cfg(windows)]
    pub fn authorize(
        &mut self,
        _capability: &DebloatExecutorCapability,
        authorization: TransactionAuthorization,
    ) -> Result<(), DebloatExecutionError> {
        let host = windows::WindowsDebloatHost;
        engine::authorize_with_host(self, authorization, &host)
    }

    #[cfg(not(windows))]
    pub fn authorize(
        &mut self,
        _capability: &DebloatExecutorCapability,
        _authorization: TransactionAuthorization,
    ) -> Result<(), DebloatExecutionError> {
        Err(DebloatExecutionError::UnsupportedPlatform)
    }

    #[cfg(windows)]
    pub fn apply(
        &mut self,
        _capability: &DebloatExecutorCapability,
    ) -> Result<(), DebloatExecutionError> {
        let _execution_lock = windows::DebloatExecutionMutex::acquire()?;
        let mut host = windows::WindowsDebloatHost;
        engine::apply_with_host(self, &mut host)
    }

    #[cfg(not(windows))]
    pub fn apply(
        &mut self,
        _capability: &DebloatExecutorCapability,
    ) -> Result<(), DebloatExecutionError> {
        Err(DebloatExecutionError::UnsupportedPlatform)
    }
}

#[cfg(test)]
mod tests;
