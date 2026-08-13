//! Phase 8 bounded runtime execution for Neo Driver.
//!
//! This crate is the internal machine-changing boundary for exact, already
//! present runtime payloads. It deliberately exposes no network acquisition,
//! archive extraction, Windows-feature mutation, shell execution, or public CLI
//! apply path.
//!
//! Phase 8 intentionally exposes only validated planning/inspection contracts.
//! The session, host adapter, process invocation/result types, and Windows host
//! remain crate-private so an outside crate cannot bypass Phase 6 assessment,
//! Phase 7 vault authority, or Phase 4 transaction authorization.

mod error;
mod executor;
mod host;
mod model;
mod plan;

#[cfg(windows)]
mod windows;

pub use error::RuntimeExecutorError;
pub use model::{RuntimeExecutionOperation, RuntimeExecutionPlan};
pub use plan::{prepare_runtime_execution, PreparedRuntimeExecution};

pub(crate) use executor::RuntimeExecutionSession;
pub(crate) use host::RuntimeHost;
pub(crate) use model::{RuntimeInvocation, RuntimeProcessResult};

#[cfg(windows)]
pub(crate) use windows::WindowsRuntimeHost;

#[cfg(test)]
mod tests;
