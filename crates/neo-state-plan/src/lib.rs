mod assessment;
mod error;
mod model;
mod resolver;
mod windows_read;

pub use assessment::{assess_tweaks, TweakAssessment, TweakAssessmentItem};
pub use error::StatePlanError;
pub use model::{
    ObservedState, TweakCatalogue, TweakDefinition, TweakEvidence, TweakObservation,
    TweakOperation, TweakTarget, TweakValue,
};
pub use resolver::{
    resolve_selected_evidence, CapturedState, CapturedStates, ReaderId, StateBinding, StateBindings,
};
pub use windows_read::{
    RegistryHive, RegistryValueKind, RegistryView, WindowsReadSource, WindowsReaderSource,
    WindowsReaderSources,
};

#[cfg(test)]
mod tests;
