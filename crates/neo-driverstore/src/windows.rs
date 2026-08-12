//! Windows SetupAPI/NewDev backend.
//!
//! The concrete implementation is added only after the platform-neutral
//! transaction/session core passes its independent proof gate.

#[derive(Debug, Default, Clone, Copy)]
pub struct WindowsDriverHost;
