use crate::{layout::STAGING_MARKER_NAME, Sha256Digest, VaultError, VaultLayout, VaultSegment};
use cap_fs_ext::{DirExt, FollowSymlinks, OpenOptionsFollowExt};
use cap_std::ambient_authority;
use cap_std::fs::{Dir, File as CapFile, OpenOptions};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::ffi::OsString;
use std::fs;
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_IMPORT_SESSION: AtomicU64 = AtomicU64::new(1);
const MANAGED_CHILDREN: [&str; 9] = [
    "catalogue",
    "driver-packs",
    "packages",
    "runtimes",
    "staging",
    "sessions",
    "backups",
    "logs",
    "cache",
];

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
        self.open_managed_handles()?;
        Ok(())
    }

    pub fn begin_staging(&self, session: &VaultSegment) -> Result<PathBuf, VaultError> {
        let handles = self.open_managed_handles()?;
        let display = self.layout.staging_session(session);
        match handles.staging.open_dir_nofollow(session.as_str()) {
            Ok(session_dir) => {
                validate_staging_marker(&session_dir, session, &display)?;
                Ok(display)
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                create_child_dir(&handles.staging, session.as_str(), &display)?;
                let session_dir = open_child_dir(&handles.staging, session.as_str(), &display)?;
                if let Err(error) = write_staging_marker(&session_dir, session) {
                    let _ = handles.staging.remove_dir_all(session.as_str());
                    return Err(error);
                }
                Ok(display)
            }
            Err(error) => Err(classify_link_error(&display, error)),
        }
    }

    pub fn cleanup_staging(&self, session: &VaultSegment) -> Result<bool, VaultError> {
        let Some(staging) = self.open_existing_managed_child("staging")? else {
            return Ok(false);
        };
        let display = self.layout.staging_session(session);
        let session_dir = match staging.open_dir_nofollow(session.as_str()) {
            Ok(dir) => dir,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(error) => return Err(classify_link_error(&display, error)),
        };
        validate_staging_marker(&session_dir, session, &display)?;
        drop(session_dir);
        staging.remove_dir_all(session.as_str())?;
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
        if !metadata.is_file() || metadata.file_type().is_symlink() || has_reparse_point(&metadata)
        {
            return Err(VaultError::SourceNotFile(source.to_path_buf()));
        }

        let observed = sha256_file(source)?;
        if observed != *expected_sha256 {
            return Err(VaultError::HashMismatch {
                path: source.to_path_buf(),
                expected: expected_sha256.to_string(),
                observed: observed.to_string(),
            });
        }

        let handles = self.open_managed_handles()?;
        let (pack_root, destination) = match class {
            PackClass::Driver => (
                &handles.driver_packs,
                self.layout
                    .driver_pack_destination(package_id, version, expected_sha256.as_str()),
            ),
            PackClass::Runtime => (
                &handles.runtimes,
                self.layout
                    .runtime_pack_destination(package_id, version, expected_sha256.as_str()),
            ),
        };
        self.layout.ensure_managed(&destination)?;

        let package_display = destination
            .parent()
            .and_then(Path::parent)
            .ok_or_else(|| VaultError::OutsideManagedRoot(destination.clone()))?;
        let version_display = destination
            .parent()
            .ok_or_else(|| VaultError::OutsideManagedRoot(destination.clone()))?;
        let package_dir =
            open_or_create_child_dir(pack_root, package_id.as_str(), package_display)?;
        let version_dir =
            open_or_create_child_dir(&package_dir, version.as_str(), version_display)?;
        let destination_name = format!("{}.pack", expected_sha256.as_str());

        if let Some(receipt) = existing_receipt(
            &version_dir,
            &destination_name,
            &destination,
            expected_sha256,
            metadata.len(),
        )? {
            return Ok(receipt);
        }

        let (session, session_dir) =
            self.begin_unique_import_staging(&handles.staging, package_id, expected_sha256)?;
        let session_display = self.layout.staging_session(&session);
        let staged_name = "payload.pack";

        let import_result = (|| -> Result<ImportReceipt, VaultError> {
            let mut source_file = fs::File::open(source)?;
            let mut staged_file = create_new_file_nofollow(&session_dir, staged_name)?;
            std::io::copy(&mut source_file, &mut staged_file)?;
            staged_file.sync_all()?;
            drop(staged_file);

            let staged_hash = sha256_cap_file(open_read_file_nofollow(&session_dir, staged_name)?)?;
            if staged_hash != *expected_sha256 {
                return Err(VaultError::HashMismatch {
                    path: session_display.join(staged_name),
                    expected: expected_sha256.to_string(),
                    observed: staged_hash.to_string(),
                });
            }

            let mut final_file = match create_new_file_nofollow(&version_dir, &destination_name) {
                Ok(file) => file,
                Err(VaultError::Io(error)) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    return Err(VaultError::ImportBusy(destination.clone()));
                }
                Err(error) => return Err(error),
            };

            let mut staged_reader = open_read_file_nofollow(&session_dir, staged_name)?;
            if let Err(error) = std::io::copy(&mut staged_reader, &mut final_file) {
                drop(final_file);
                let _ = version_dir.remove_file(&destination_name);
                return Err(VaultError::Io(error));
            }
            drop(staged_reader);
            if let Err(error) = final_file.sync_all() {
                drop(final_file);
                let _ = version_dir.remove_file(&destination_name);
                return Err(VaultError::Io(error));
            }
            drop(final_file);

            let promoted_hash =
                sha256_cap_file(open_read_file_nofollow(&version_dir, &destination_name)?)?;
            if promoted_hash != *expected_sha256 {
                let _ = version_dir.remove_file(&destination_name);
                return Err(VaultError::HashMismatch {
                    path: destination.clone(),
                    expected: expected_sha256.to_string(),
                    observed: promoted_hash.to_string(),
                });
            }

            session_dir.remove_file(staged_name)?;
            Ok(ImportReceipt {
                disposition: ImportDisposition::Imported,
                destination: destination.clone(),
                sha256: expected_sha256.clone(),
                bytes: metadata.len(),
            })
        })();

        drop(session_dir);
        let cleanup_result = self.cleanup_staging(&session);
        match (import_result, cleanup_result) {
            (Ok(receipt), Ok(_)) => Ok(receipt),
            (Err(error), _) => Err(error),
            (Ok(_), Err(error)) => Err(error),
        }
    }

    pub fn verify_pack(
        &self,
        path: impl AsRef<Path>,
        expected_sha256: &Sha256Digest,
    ) -> Result<(), VaultError> {
        let normalized = self.layout.ensure_managed(path)?;
        let managed = self.open_existing_managed_root()?.ok_or_else(|| {
            VaultError::ApplicationRootUnavailable(self.layout.managed_root().to_path_buf())
        })?;
        let relative = relative_components(self.layout.managed_root(), &normalized)?;
        let file = open_relative_file_nofollow(&managed, &relative, &normalized)?;
        let observed = sha256_cap_file(file)?;
        if observed == *expected_sha256 {
            Ok(())
        } else {
            Err(VaultError::HashMismatch {
                path: normalized,
                expected: expected_sha256.to_string(),
                observed: observed.to_string(),
            })
        }
    }

    pub fn audit_existing_tree(&self) -> Result<(), VaultError> {
        let app_root = open_absolute_dir_nofollow(self.layout.application_root())?;
        let managed = match app_root.open_dir_nofollow("NeoData") {
            Ok(dir) => dir,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(classify_link_error(self.layout.managed_root(), error)),
        };
        audit_dir(&managed, self.layout.managed_root())
    }

    fn open_managed_handles(&self) -> Result<VaultHandles, VaultError> {
        let application = open_absolute_dir_nofollow(self.layout.application_root())?;
        let managed =
            open_or_create_child_dir(&application, "NeoData", self.layout.managed_root())?;

        let mut created = Vec::with_capacity(MANAGED_CHILDREN.len());
        for name in MANAGED_CHILDREN {
            let display = self.layout.managed_root().join(name);
            created.push(open_or_create_child_dir(&managed, name, &display)?);
        }

        let mut iter = created.into_iter();
        let _catalogue = iter.next().expect("managed child count");
        let driver_packs = iter.next().expect("managed child count");
        let _packages = iter.next().expect("managed child count");
        let runtimes = iter.next().expect("managed child count");
        let staging = iter.next().expect("managed child count");
        let _sessions = iter.next().expect("managed child count");
        let _backups = iter.next().expect("managed child count");
        let _logs = iter.next().expect("managed child count");
        let _cache = iter.next().expect("managed child count");

        Ok(VaultHandles {
            driver_packs,
            runtimes,
            staging,
        })
    }

    fn open_existing_managed_root(&self) -> Result<Option<Dir>, VaultError> {
        let application = open_absolute_dir_nofollow(self.layout.application_root())?;
        match application.open_dir_nofollow("NeoData") {
            Ok(dir) => Ok(Some(dir)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(classify_link_error(self.layout.managed_root(), error)),
        }
    }

    fn open_existing_managed_child(&self, child: &str) -> Result<Option<Dir>, VaultError> {
        let Some(managed) = self.open_existing_managed_root()? else {
            return Ok(None);
        };
        let display = self.layout.managed_root().join(child);
        match managed.open_dir_nofollow(child) {
            Ok(dir) => Ok(Some(dir)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(classify_link_error(&display, error)),
        }
    }

    fn begin_unique_import_staging(
        &self,
        staging: &Dir,
        package_id: &VaultSegment,
        expected_sha256: &Sha256Digest,
    ) -> Result<(VaultSegment, Dir), VaultError> {
        loop {
            let session = unique_import_session(package_id, expected_sha256)?;
            let display = self.layout.staging_session(&session);
            match staging.create_dir(session.as_str()) {
                Ok(()) => {
                    let session_dir = open_child_dir(staging, session.as_str(), &display)?;
                    if let Err(error) = write_staging_marker(&session_dir, &session) {
                        let _ = staging.remove_dir_all(session.as_str());
                        return Err(error);
                    }
                    return Ok((session, session_dir));
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(VaultError::Io(error)),
            }
        }
    }
}

struct VaultHandles {
    driver_packs: Dir,
    runtimes: Dir,
    staging: Dir,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct StagingMarker {
    schema_version: u32,
    session: VaultSegment,
}

pub fn sha256_file(path: impl AsRef<Path>) -> Result<Sha256Digest, VaultError> {
    let mut file = fs::File::open(path)?;
    sha256_reader(&mut file)
}

fn sha256_cap_file(mut file: CapFile) -> Result<Sha256Digest, VaultError> {
    sha256_reader(&mut file)
}

fn sha256_reader(reader: &mut impl Read) -> Result<Sha256Digest, VaultError> {
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 1024 * 1024];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Sha256Digest::new(format!("{:x}", hasher.finalize()))
}

fn existing_receipt(
    version_dir: &Dir,
    destination_name: &str,
    destination: &Path,
    expected_sha256: &Sha256Digest,
    bytes: u64,
) -> Result<Option<ImportReceipt>, VaultError> {
    let file = match open_read_file_nofollow(version_dir, destination_name) {
        Ok(file) => file,
        Err(VaultError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(None);
        }
        Err(error) => return Err(error),
    };
    let destination_hash = sha256_cap_file(file)?;
    if destination_hash != *expected_sha256 {
        return Err(VaultError::DestinationConflict(destination.to_path_buf()));
    }
    Ok(Some(ImportReceipt {
        disposition: ImportDisposition::AlreadyPresent,
        destination: destination.to_path_buf(),
        sha256: expected_sha256.clone(),
        bytes,
    }))
}

fn write_staging_marker(session_dir: &Dir, session: &VaultSegment) -> Result<(), VaultError> {
    let marker = StagingMarker {
        schema_version: 1,
        session: session.clone(),
    };
    let encoded = serde_json::to_vec_pretty(&marker)?;
    let mut file = create_new_file_nofollow(session_dir, STAGING_MARKER_NAME)?;
    file.write_all(&encoded)?;
    file.sync_all()?;
    Ok(())
}

fn validate_staging_marker(
    session_dir: &Dir,
    session: &VaultSegment,
    display: &Path,
) -> Result<(), VaultError> {
    let marker_file = match open_read_file_nofollow(session_dir, STAGING_MARKER_NAME) {
        Ok(file) => file,
        Err(VaultError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(VaultError::UnownedStaging(display.to_path_buf()));
        }
        Err(error) => return Err(error),
    };
    let marker: StagingMarker = serde_json::from_reader(marker_file)?;
    if marker.schema_version != 1 || marker.session != *session {
        return Err(VaultError::StagingMarkerMismatch {
            session: session.to_string(),
            path: display.to_path_buf(),
        });
    }
    Ok(())
}

fn unique_import_session(
    package_id: &VaultSegment,
    expected_sha256: &Sha256Digest,
) -> Result<VaultSegment, VaultError> {
    let sequence = NEXT_IMPORT_SESSION.fetch_add(1, Ordering::Relaxed);
    VaultSegment::new(format!(
        "import-{}-{}-{}-{}",
        package_id.as_str(),
        &expected_sha256.as_str()[..16],
        std::process::id(),
        sequence
    ))
}

fn create_new_file_nofollow(dir: &Dir, name: impl AsRef<Path>) -> Result<CapFile, VaultError> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    options.follow(FollowSymlinks::No);
    dir.open_with(name, &options).map_err(VaultError::Io)
}

fn open_read_file_nofollow(dir: &Dir, name: impl AsRef<Path>) -> Result<CapFile, VaultError> {
    let mut options = OpenOptions::new();
    options.read(true);
    options.follow(FollowSymlinks::No);
    dir.open_with(name, &options).map_err(VaultError::Io)
}

fn open_or_create_child_dir(parent: &Dir, name: &str, display: &Path) -> Result<Dir, VaultError> {
    match parent.open_dir_nofollow(name) {
        Ok(dir) => Ok(dir),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            match parent.create_dir(name) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(VaultError::Io(error)),
            }
            open_child_dir(parent, name, display)
        }
        Err(error) => Err(classify_link_error(display, error)),
    }
}

fn create_child_dir(parent: &Dir, name: &str, display: &Path) -> Result<(), VaultError> {
    match parent.create_dir(name) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            open_child_dir(parent, name, display).map(|_| ())
        }
        Err(error) => Err(VaultError::Io(error)),
    }
}

fn open_child_dir(parent: &Dir, name: &str, display: &Path) -> Result<Dir, VaultError> {
    parent
        .open_dir_nofollow(name)
        .map_err(|error| classify_link_error(display, error))
}

fn open_absolute_dir_nofollow(path: &Path) -> Result<Dir, VaultError> {
    if !path.is_absolute() {
        return Err(VaultError::ApplicationRootNotAbsolute(path.to_path_buf()));
    }
    let (root, components) = split_absolute_dir(path)?;
    let mut current = Dir::open_ambient_dir(&root, ambient_authority())?;
    let mut display = root;
    for component in components {
        display.push(&component);
        current = current
            .open_dir_nofollow(&component)
            .map_err(|error| classify_link_error(&display, error))?;
    }
    Ok(current)
}

fn split_absolute_dir(path: &Path) -> Result<(PathBuf, Vec<OsString>), VaultError> {
    let mut root = PathBuf::new();
    let mut names = Vec::new();
    let mut saw_root = false;
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => root.push(prefix.as_os_str()),
            Component::RootDir => {
                root.push(component.as_os_str());
                saw_root = true;
            }
            Component::CurDir => {}
            Component::ParentDir => return Err(VaultError::ParentTraversal(path.to_path_buf())),
            Component::Normal(name) => names.push(name.to_os_string()),
        }
    }
    if !saw_root {
        return Err(VaultError::ApplicationRootNotAbsolute(path.to_path_buf()));
    }
    Ok((root, names))
}

fn relative_components(root: &Path, path: &Path) -> Result<Vec<OsString>, VaultError> {
    let root_len = root.components().count();
    let path_components: Vec<_> = path.components().collect();
    if path_components.len() <= root_len {
        return Err(VaultError::SourceNotFile(path.to_path_buf()));
    }
    Ok(path_components[root_len..]
        .iter()
        .filter_map(|component| match component {
            Component::Normal(name) => Some(name.to_os_string()),
            Component::CurDir => None,
            _ => None,
        })
        .collect())
}

fn open_relative_file_nofollow(
    root: &Dir,
    components: &[OsString],
    display: &Path,
) -> Result<CapFile, VaultError> {
    let (last, parents) = components
        .split_last()
        .ok_or_else(|| VaultError::SourceNotFile(display.to_path_buf()))?;
    let mut current = root.try_clone()?;
    for parent in parents {
        current = current
            .open_dir_nofollow(parent)
            .map_err(|error| classify_link_error(display, error))?;
    }
    open_read_file_nofollow(&current, last)
}

fn audit_dir(dir: &Dir, display: &Path) -> Result<(), VaultError> {
    for entry in dir.entries()? {
        let entry = entry?;
        let name = entry.file_name();
        let child_display = display.join(&name);
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            return Err(VaultError::UnsafeLink(child_display));
        }
        if file_type.is_dir() {
            let child = dir
                .open_dir_nofollow(&name)
                .map_err(|error| classify_link_error(&child_display, error))?;
            audit_dir(&child, &child_display)?;
        } else {
            open_read_file_nofollow(dir, &name).map_err(|error| match error {
                VaultError::Io(io_error) => classify_link_error(&child_display, io_error),
                other => other,
            })?;
        }
    }
    Ok(())
}

fn classify_link_error(path: &Path, error: std::io::Error) -> VaultError {
    if diagnostic_link_like(path) {
        VaultError::UnsafeLink(path.to_path_buf())
    } else if error.kind() == std::io::ErrorKind::NotFound {
        VaultError::ApplicationRootUnavailable(path.to_path_buf())
    } else {
        VaultError::Io(error)
    }
}

fn diagnostic_link_like(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .map(|metadata| metadata.file_type().is_symlink() || has_reparse_point(&metadata))
        .unwrap_or(false)
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
