//! Phase 11 transaction-bound tweak execution.
//!
//! The public surface exposes curated planning and inspection. Mutation methods require an opaque
//! capability with no public constructor. Raw registry bindings and the Windows host remain private.

mod engine;
mod error;
mod model;
mod session;

#[cfg(windows)]
mod windows;

pub use error::TweakExecutionError;
pub use model::{
    curated_tweak_ids, RegistrySnapshot, TweakExecutionPlan, TweakExecutionSession,
    TweakExecutionStep, TweakExecutorCapability, SHOW_FILE_EXTENSIONS, SHOW_HIDDEN_FILES,
    TASKBAR_CENTERED_ICONS,
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
mod tests;
