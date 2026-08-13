use crate::{RuntimeExecutorError, RuntimeInvocation, RuntimeProcessResult};
use neo_runtime::RuntimeInventory;

/// Narrow host boundary used by the runtime executor.
///
/// `execute` may return an operational error only before a child process is
/// created. Once process creation succeeds it must return a `RuntimeProcessResult`
/// even if waiting for or observing the child later fails, so Neo can retain the
/// conservative `machine_changed=true` obligation.
pub(crate) trait RuntimeHost {
    fn inventory(&self) -> Result<RuntimeInventory, RuntimeExecutorError>;

    fn execute(
        &self,
        invocation: &RuntimeInvocation,
    ) -> Result<RuntimeProcessResult, RuntimeExecutorError>;
}
