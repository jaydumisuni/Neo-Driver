//! Phase 8 bounded runtime execution for Neo Driver.
//!
//! This crate is the internal machine-changing boundary for exact, already
//! present runtime payloads. It deliberately exposes no network acquisition,
//! archive extraction, Windows-feature mutation, shell execution, or public CLI
//! apply path.
//!
//! Public callers may inspect validated plans/sessions, but mutation authority
//! requires an opaque `RuntimeExecutorCapability`. The capability has no public
//! constructor, while raw host/invocation/process/Windows-host types stay
//! crate-private. Safe outside code therefore cannot bypass Phase 6 assessment,
//! Phase 7 vault authority, or Phase 4 transaction authorization.

mod error;
mod executor;
#[cfg(any(windows, test))]
mod host;
mod model;
mod plan;

#[cfg(windows)]
mod windows;

pub use error::RuntimeExecutorError;
pub use executor::{RuntimeExecutionSession, RuntimeExecutorCapability};
pub use model::{RuntimeExecutionOperation, RuntimeExecutionPlan};
pub use plan::{prepare_runtime_execution, PreparedRuntimeExecution};

#[cfg(any(windows, test))]
pub(crate) use host::RuntimeHost;
#[cfg(any(windows, test))]
pub(crate) use model::{RuntimeInvocation, RuntimeProcessResult};

#[cfg(test)]
mod tests;
