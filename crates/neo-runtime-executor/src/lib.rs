//! Phase 8 bounded runtime execution for Neo Driver.
//!
//! This crate is the internal machine-changing boundary for exact, already
//! present runtime payloads. It deliberately exposes no network acquisition,
//! archive extraction, Windows-feature mutation, shell execution, or public CLI
//! apply path.

mod error;
mod executor;
mod host;
mod model;
mod plan;

#[cfg(windows)]
mod windows;

pub use error::RuntimeExecutorError;
pub use executor::RuntimeExecutionSession;
pub use host::RuntimeHost;
pub use model::{
    RuntimeExecutionOperation, RuntimeExecutionPlan, RuntimeInvocation, RuntimeProcessResult,
};
pub use plan::{prepare_runtime_execution, PreparedRuntimeExecution};

#[cfg(windows)]
pub use windows::WindowsRuntimeHost;

#[cfg(test)]
mod tests;
