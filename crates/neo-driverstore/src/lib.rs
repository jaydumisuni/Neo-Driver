//! Controlled Windows driver installation foundation for Neo Driver.
//!
//! Phase 5 preserves Windows best-match policy on the forward path and uses
//! specific-device installation only as a rollback primitive for an exact
//! captured baseline binding. No force-install API is exposed by this crate.

mod error;
mod executor;
mod host;
mod model;
mod plan;

#[cfg(windows)]
mod windows;

pub use error::DriverStoreError;
pub use executor::DriverInstallSession;
pub use host::DriverHost;
pub use model::{
    DriverBackendResult, DriverBindingBaseline, DriverInstallImpact, DriverInstallPlan,
    DriverInventory, DriverStoreBaseline, PreparedDriverInstall, StoredDriverPackage,
    VerifiedInfSignature,
};
pub use plan::{prepare_driver_install, DriverInstallRequest};

#[cfg(windows)]
pub use windows::WindowsDriverHost;

#[cfg(test)]
mod tests;
