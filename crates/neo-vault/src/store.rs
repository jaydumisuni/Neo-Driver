use crate::{
    layout::STAGING_MARKER_NAME, Sha256Digest, VaultError, VaultLayout, VaultSegment,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackClass {
    Driver,
    Runtime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportDisposition {
    Imported,
    AlreadyPresent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportReceipt {
    pub disposition: ImportDisposition,
    pub destination: PathBuf,
    pub sha256: Sha256Digest,
    pub bytes: u64,
}

#[derive(Debug, Clone)]
pub struct VaultStore {
    layout: VaultLayout,
}

impl VaultStore {
    pub fn new(layout: VaultLayout) -> Self {
        Self { layout }
    }

    pub fn layout(&self) -> &VaultLayout {
        &self.layout
    }

    pub fn ensure_layout(&self) -> Result<(), VaultError> {
        let app_root = self.layout.application_root();
        if !app_root.exists() || !app_root.is_dir() {
            return Err(VaultError::ApplicationRootUnavailable(app_root.to_path_buf()));
        }
        reject_link_like(app_root)?;

        for path in self.layout.all_managed_directories() {
            self.layout.ensure_managed(path)?;
            ensure_directory_chain(app_root, path)?;
        }
        Ok(())
    }

    pub fn begin_staging(&self, session: &VaultSegment) -> Result<PathBuf, VaultError> {
        self.ensure_layout()?;
        let path = self.layout.staging_session(session);
        self.layout.ensure_cleanup_target(&path)?;

        if path.exists() {
            reject_link_like(&path)?;
            self.validate_staging_marker(&path, session)?;
            return Ok(path);
        }

        fs::create_dir(&path)?;
        let marker = StagingMarker {
            schema_version: 1,
            session: session.clone(),
        };
        let marker_path = path.join(STAGING_MARKER_NAME);
        let encoded = serde_json::to_vec_pretty(&marker)?;
        let mut file = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&marker_path)?;
        file.write_all(&encoded)?;
        file.sync_all()?;
        Ok(path)
    }

    pub fn cleanup_staging(&self, session: &VaultSegment) -> Result<bool, VaultError> {
        let path = self.layout.staging_session(session);
        self.layout.ensure_cleanup_target(&path)?;
        if !path.exists() {
            return Ok(false);
        }
        reject_link_like(&path)?;
        self.validate_staging_marker(&path, session)?;
        fs::remove_dir_all(&path)?;
        Ok(true)
    }

    pub fn import_pack_file(
        &self,
        class: PackClass,
        source: impl AsRef<Path>,
        package_id: &VaultSegment,
        version: &VaultSegment,
        expected_sha256: &Sha256Digest,
    ) -> Result<ImportReceipt, VaultError> {
        let source = source.as_ref();
        let metadata = fs::symlink_metadata(source)?;
        if !metadata.is_file() {
            return Err(VaultError::SourceNotFile(source.to_path_buf()));
        }
        reject_link_like(source)?;

        let observed = sha256_file(source)?;
        if observed != *expected_sha256 {
            return Err(VaultError::HashMismatch {
                path: source.to_path_buf(),
                expected: expected_sha256.to_string(),
                observed: observed.to_string(),
            });
        }

        self.ensure_layout()?;
        let destination = match class {
            PackClass::Driver => self.layout.driver_pack_destination(
                package_id,
                version,
                expected_sha256.as_str(),
            ),
            PackClass::Runtime => self.layout.runtime_pack_destination(
                package_id,
                version,
                expected_sha256.as_str(),
            ),
        };
        self.layout.ensure_managed(&destination)?;

        if destination.exists() {
            reject_link_like(&destination)?;
            let destination_hash = sha256_file(&destination)?;
            if destination_hash != *expected_sha256 {
                return Err(VaultError::DestinationConflict(destination));
            }
            return Ok(ImportReceipt {
                disposition: ImportDisposition::AlreadyPresent,
                destination,
                sha256: expected_sha256.clone(),
                bytes: metadata.len(),
            });
        }

        let session = VaultSegment::new(format!(
            "import-{}-{}",
            package_id.as_str(),
            &expected_sha256.as_str()[..16]
        ))?;
        let staging = self.begin_staging(&session)?;
        let staged = staging.join("payload.pack");
        if staged.exists() {
            reject_link_like(&staged)?;
            fs::remove_file(&staged)?;
        }
        fs::copy(source, &staged)?;
        let staged_hash = sha256_file(&staged)?;
        if staged_hash != *expected_sha256 {
            let _ = self.cleanup_staging(&session);
            return Err(VaultError::HashMismatch {
                path: staged,
                expected: expected_sha256.to_string(),
                observed: staged_hash.to_string(),
            });
        }

        let parent = destination
            .parent()
            .ok_or_else(|| VaultError::OutsideManagedRoot(destination.clone()))?;
        self.layout.ensure_managed(parent)?;
        ensure_directory_chain(self.layout.managed_root(), parent)?;

        if destination.exists() {
            let _ = self.cleanup_staging(&session);
            return Err(VaultError::DestinationConflict(destination));
        }
        fs::rename(&staged, &destination)?;
        self.cleanup_staging(&session)?;

        Ok(ImportReceipt {
            disposition: ImportDisposition::Imported,
            destination,
            sha256: expected_sha256.clone(),
            bytes: metadata.len(),
        })
    }

    pub fn verify_pack(
        &self,
        path: impl AsRef<Path>,
        expected_sha256: &Sha256Digest,
    ) -> Result<(), VaultError> {
        let path = self.layout.ensure_managed(path)?;
        reject_link_like(&path)?;
        let observed = sha256_file(&path)?;
        if observed == *expected_sha256 {
            Ok(())
        } else {
            Err(VaultError::HashMismatch {
                path,
                expected: expected_sha256.to_string(),
                observed: observed.to_string(),
            })
        }
    }

    pub fn audit_existing_tree(&self) -> Result<(), VaultError> {
        if !self.layout.managed_root().exists() {
            return Ok(());
        }
        self.layout.ensure_managed(self.layout.managed_root())?;
        audit_tree(self.layout.managed_root())
    }

    fn validate_staging_marker(
        &self,
        staging: &Path,
        session: &VaultSegment,
    ) -> Result<(), VaultError> {
        let marker_path = staging.join(STAGING_MARKER_NAME);
        if !marker_path.exists() {
            return Err(VaultError::UnownedStaging(staging.to_path_buf()));
        }
        reject_link_like(&marker_path)?;
        let marker: StagingMarker = serde_json::from_slice(&fs::read(&marker_path)?)?;
        if marker.schema_version != 1 || marker.session != *session {
            return Err(VaultError::StagingMarkerMismatch {
                session: session.to_string(),
                path: staging.to_path_buf(),
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct StagingMarker {
    schema_version: u32,
    session: VaultSegment,
}

pub fn sha256_file(path: impl AsRef<Path>) -> Result<Sha256Digest, VaultError> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 1024 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Sha256Digest::new(format!("{:x}", hasher.finalize()))
}

fn ensure_directory_chain(base: &Path, target: &Path) -> Result<(), VaultError> {
    if !base.exists() || !base.is_dir() {
        return Err(VaultError::ApplicationRootUnavailable(base.to_path_buf()));
    }
    reject_link_like(base)?;
    let relative = target
        .strip_prefix(base)
        .map_err(|_| VaultError::OutsideManagedRoot(target.to_path_buf()))?;
    let mut current = base.to_path_buf();
    for component in relative.components() {
        current.push(component.as_os_str());
        if current.exists() {
            reject_link_like(&current)?;
            if !current.is_dir() {
                return Err(VaultError::UnsafeLink(current));
            }
        } else {
            fs::create_dir(&current)?;
        }
    }
    Ok(())
}

fn audit_tree(path: &Path) -> Result<(), VaultError> {
    reject_link_like(path)?;
    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            audit_tree(&entry.path())?;
        }
    }
    Ok(())
}

fn reject_link_like(path: &Path) -> Result<(), VaultError> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || has_reparse_point(&metadata) {
        Err(VaultError::UnsafeLink(path.to_path_buf()))
    } else {
        Ok(())
    }
}

#[cfg(windows)]
fn has_reparse_point(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn has_reparse_point(_metadata: &fs::Metadata) -> bool {
    false
}
