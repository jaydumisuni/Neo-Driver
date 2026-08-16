//! Neo Driver managed package vault.
//!
//! The vault is rooted beneath an application root supplied by THETECHGUY
//! Software Builder (or the portable Neo folder). It deliberately does not
//! choose Program Files/ProgramData itself. Phase 6 provides deterministic,
//! owned storage and local/offline package intake only; network acquisition is
//! not enabled here.

mod error;
mod layout;
mod source;
mod store;
mod types;

pub use error::VaultError;
pub use layout::{
    VaultLayout, VaultMode, HISTORY_DIRECTORY_NAME, MANAGED_DIRECTORY_NAME, STAGING_MARKER_NAME,
};
pub use source::{DriverSource, DriverSourceMap, SourcePackageKind, SOURCE_MAP_SCHEMA_VERSION};
pub use store::{sha256_file, ImportDisposition, ImportReceipt, PackClass, VaultStore};
pub use types::{Sha256Digest, VaultSegment};

#[cfg(test)]
mod tests;
