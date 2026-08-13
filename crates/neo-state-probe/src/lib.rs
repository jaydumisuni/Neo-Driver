mod error;
mod model;
mod probe;

pub use error::StateProbeError;
pub use model::{
    RegistryHive, RegistryValueKind, RegistryView, WindowsStateBinding, WindowsStateBindings,
    WindowsStateSource,
};
pub use probe::{probe_selected_tweaks, StateProbeHost};

#[cfg(windows)]
mod windows_host;
#[cfg(windows)]
pub use windows_host::WindowsStateProbe;

#[cfg(test)]
mod tests;
