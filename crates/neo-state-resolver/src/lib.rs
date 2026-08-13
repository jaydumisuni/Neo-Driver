mod error;
mod model;
mod resolver;

pub use error::StateResolverError;
pub use model::{ReaderId, StateBinding, StateBindings};
pub use resolver::{resolve_selected_evidence, CapturedStates};

#[cfg(test)]
mod tests;
