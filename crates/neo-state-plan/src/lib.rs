//! Typed, read-only state planning contracts for Neo Driver.
//!
//! Phase 9 uses this crate for the first bounded Tweaks-domain child. The crate
//! contains no operating-system mutation code; it validates exact tweak intent
//! and emits reversible Phase 4 transactions from supplied current-state evidence.

mod error;
mod model;
mod plan;

pub use error::StatePlanError;
pub use model::{
    ObservedState, TweakCatalogue, TweakDefinition, TweakEvidence, TweakObservation,
    TweakOperation, TweakTarget, TweakValue,
};
pub use plan::{build_tweak_plan, TweakPlanBundle};

#[cfg(test)]
mod tests;
