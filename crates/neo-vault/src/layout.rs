use crate::{VaultError, VaultSegment};
use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};

pub const MANAGED_DIRECTORY_NAME: &str = "NeoData";
pub const HISTORY_DIRECTORY_NAME: &str = "history";
pub const STAGING_MARKER_NAME: &str = ".neo-owned-staging.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VaultMode {
    Installed,
    Portable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VaultLayout {
    mode: VaultMode,
    application_root: PathBuf,
    managed_root: PathBuf,
    catalogue: PathBuf,
    driver_packs: PathBuf,
    packages: PathBuf,
    runtimes: PathBuf,
    staging: PathBuf,
    sessions: PathBuf,
    backups: PathBuf,
    logs: PathBuf,
    cache: PathBuf,
    history: PathBuf,
}

impl VaultLayout {
    pub fn new(mode: VaultMode, application_root: impl AsRef<Path>) -> Result<Self, VaultError> {
        let application_root = normalize_absolute(application_root.as_ref())?;
        let managed_root = application_root.join(MANAGED_DIRECTORY_NAME);
        Ok(Self {
            mode,
            application_root,
            catalogue: managed_root.join("catalogue"),
            driver_packs: managed_root.join("driver-packs"),
            packages: managed_root.join("packages"),
            runtimes: managed_root.join("runtimes"),
            staging: managed_root.join("staging"),
            sessions: managed_root.join("sessions"),
            backups: managed_root.join("backups"),
            logs: managed_root.join("logs"),
            cache: managed_root.join("cache"),
            history: managed_root.join(HISTORY_DIRECTORY_NAME),
            managed_root,
        })
    }

    pub fn mode(&self) -> VaultMode {
        self.mode
    }

    pub fn application_root(&self) -> &Path {
        &self.application_root
    }

    pub fn managed_root(&self) -> &Path {
        &self.managed_root
    }

    pub fn catalogue(&self) -> &Path {
        &self.catalogue
    }

    pub fn driver_packs(&self) -> &Path {
        &self.driver_packs
    }

    pub fn packages(&self) -> &Path {
        &self.packages
    }

    pub fn runtimes(&self) -> &Path {
        &self.runtimes
    }

    pub fn staging(&self) -> &Path {
        &self.staging
    }

    pub fn sessions(&self) -> &Path {
        &self.sessions
    }

    pub fn backups(&self) -> &Path {
        &self.backups
    }

    pub fn logs(&self) -> &Path {
        &self.logs
    }

    pub fn cache(&self) -> &Path {
        &self.cache
    }

    pub fn history(&self) -> &Path {
        &self.history
    }

    pub fn all_managed_directories(&self) -> [&Path; 11] {
        [
            &self.managed_root,
            &self.catalogue,
            &self.driver_packs,
            &self.packages,
            &self.runtimes,
            &self.staging,
            &self.sessions,
            &self.backups,
            &self.logs,
            &self.cache,
            &self.history,
        ]
    }

    pub fn staging_session(&self, session: &VaultSegment) -> PathBuf {
        self.staging.join(session.as_str())
    }

    pub fn driver_pack_destination(
        &self,
        package_id: &VaultSegment,
        version: &VaultSegment,
        sha256: &str,
    ) -> PathBuf {
        self.driver_packs
            .join(package_id.as_str())
            .join(version.as_str())
            .join(format!("{}.pack", sha256.to_ascii_lowercase()))
    }

    pub fn runtime_pack_destination(
        &self,
        package_id: &VaultSegment,
        version: &VaultSegment,
        sha256: &str,
    ) -> PathBuf {
        self.runtimes
            .join(package_id.as_str())
            .join(version.as_str())
            .join(format!("{}.pack", sha256.to_ascii_lowercase()))
    }

    pub fn ensure_managed(&self, path: impl AsRef<Path>) -> Result<PathBuf, VaultError> {
        let path = normalize_absolute(path.as_ref())?;
        if path_starts_with(&path, &self.managed_root) {
            Ok(path)
        } else {
            Err(VaultError::OutsideManagedRoot(path))
        }
    }

    pub fn ensure_cleanup_target(&self, path: impl AsRef<Path>) -> Result<PathBuf, VaultError> {
        let path = self.ensure_managed(path)?;
        if path_starts_with(&path, &self.staging) || path_starts_with(&path, &self.cache) {
            if same_path(&path, &self.staging) || same_path(&path, &self.cache) {
                return Err(VaultError::OutsideManagedRoot(path));
            }
            Ok(path)
        } else {
            Err(VaultError::OutsideManagedRoot(path))
        }
    }
}

pub(crate) fn normalize_absolute(path: &Path) -> Result<PathBuf, VaultError> {
    if !path.is_absolute() {
        return Err(VaultError::ApplicationRootNotAbsolute(path.to_path_buf()));
    }

    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::ParentDir => return Err(VaultError::ParentTraversal(path.to_path_buf())),
            Component::CurDir => {}
            other => normalized.push(other.as_os_str()),
        }
    }
    Ok(normalized)
}

fn path_starts_with(path: &Path, root: &Path) -> bool {
    let path_components: Vec<_> = path.components().collect();
    let root_components: Vec<_> = root.components().collect();
    if root_components.len() > path_components.len() {
        return false;
    }
    root_components
        .iter()
        .zip(path_components.iter())
        .all(|(left, right)| component_eq(left, right))
}

fn same_path(left: &Path, right: &Path) -> bool {
    let left_components: Vec<_> = left.components().collect();
    let right_components: Vec<_> = right.components().collect();
    left_components.len() == right_components.len()
        && left_components
            .iter()
            .zip(right_components.iter())
            .all(|(a, b)| component_eq(a, b))
}

#[cfg(windows)]
fn component_eq(left: &Component<'_>, right: &Component<'_>) -> bool {
    left.as_os_str()
        .to_string_lossy()
        .eq_ignore_ascii_case(&right.as_os_str().to_string_lossy())
}

#[cfg(not(windows))]
fn component_eq(left: &Component<'_>, right: &Component<'_>) -> bool {
    left == right
}
