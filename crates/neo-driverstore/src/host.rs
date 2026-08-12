use std::path::Path;

use crate::{
    DriverBackendResult, DriverInventory, DriverStoreError, StoredDriverPackage,
    VerifiedInfSignature,
};

pub trait DriverHost {
    /// Read-only inventory of present devices and their active bindings.
    fn inventory(&self) -> Result<DriverInventory, DriverStoreError>;

    /// Verify the actual selected INF and return Windows signer/catalogue evidence.
    fn verify_inf_signature(&self, inf: &Path) -> Result<VerifiedInfSignature, DriverStoreError>;

    /// Read-only lookup for an already materialized equivalent package in the Driver Store.
    fn find_equivalent_package(
        &self,
        source_inf: &Path,
        catalogue_files: &[String],
    ) -> Result<Option<StoredDriverPackage>, DriverStoreError>;

    /// Stage the exact selected source INF and return its Windows-published identity.
    fn stage_driver(&self, source_inf: &Path) -> Result<StoredDriverPackage, DriverStoreError>;

    /// Normal forward lane. Implementations must preserve Windows ranking (no force flag).
    fn install_best_match(
        &self,
        driver_store_inf: &Path,
    ) -> Result<DriverBackendResult, DriverStoreError>;

    /// Rollback-only primitive: restore the captured published INF on one captured device.
    fn restore_specific_driver(
        &self,
        instance_id: &str,
        published_inf: &str,
    ) -> Result<DriverBackendResult, DriverStoreError>;

    /// Remove only the exact OEM package Neo introduced, without force deletion.
    fn remove_published_package(&self, published_inf: &str) -> Result<(), DriverStoreError>;
}
