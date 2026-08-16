use crate::model::{
    DebloatHistoryRecordId, HistoryRecordDisposition, HistoryRecordWriteReceipt, StagingMarker,
    StoredDebloatRemovalReceipt, StoredDebloatRemovalSummary, StoredReceiptEnvelope,
    DEBLOAT_HISTORY_STORE_SCHEMA_VERSION, MAX_HISTORY_RECORD_BYTES, RECORD_FILE_NAME,
    STAGING_DIRECTORY_NAME, STAGING_MARKER_NAME,
};
use crate::DebloatHistoryStoreError;
use cap_fs_ext::{DirExt, FollowSymlinks, OpenOptionsFollowExt};
use cap_std::ambient_authority;
use cap_std::fs::{Dir, File as CapFile, OpenOptions};
use neo_debloat_executor::DebloatExecutionSession;
use neo_debloat_history::{
    prepare_restore_from_inventory, prepare_windows_restore_from_receipt,
    receipt_from_completed_execution, DebloatRemovalReceipt, DebloatRestorePreparedTransaction,
};
use neo_debloat_plan::ExactAppxInventory;
use neo_vault::VaultLayout;
use std::ffi::OsString;
use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_STAGING_SEQUENCE: AtomicU64 = AtomicU64::new(1);
const DEBLOAT_REMOVALS_DIRECTORY_NAME: &str = "debloat-removals";
const STAGED_RECORD_DIRECTORY_NAME: &str = "record";

#[derive(Debug, Clone)]
pub struct DebloatHistoryStore {
    layout: VaultLayout,
}

impl DebloatHistoryStore {
    pub fn new(layout: VaultLayout) -> Self {
        Self { layout }
    }

    pub fn layout(&self) -> &VaultLayout {
        &self.layout
    }

    pub fn history_root(&self) -> PathBuf {
        self.layout.history().to_path_buf()
    }

    pub fn records_root(&self) -> PathBuf {
        self.history_root().join(DEBLOAT_REMOVALS_DIRECTORY_NAME)
    }

    pub fn ensure_layout(&self) -> Result<(), DebloatHistoryStoreError> {
        self.open_or_create_store()?;
        Ok(())
    }

    pub fn record_completed_execution(
        &self,
        session: &DebloatExecutionSession,
    ) -> Result<HistoryRecordWriteReceipt, DebloatHistoryStoreError> {
        let receipt = receipt_from_completed_execution(session)?;
        self.record_validated_receipt(&receipt)
    }

    pub fn load(
        &self,
        record_id: &DebloatHistoryRecordId,
    ) -> Result<StoredDebloatRemovalReceipt, DebloatHistoryStoreError> {
        let handles = self
            .open_existing_store()?
            .ok_or_else(|| DebloatHistoryStoreError::RecordNotFound(record_id.to_string()))?;
        load_record_from_root(&handles.records, &self.records_root(), record_id)
    }

    pub fn list(&self) -> Result<Vec<StoredDebloatRemovalSummary>, DebloatHistoryStoreError> {
        let Some(handles) = self.open_existing_store()? else {
            return Ok(Vec::new());
        };
        let mut summaries = Vec::new();
        for entry in handles.records.entries()? {
            let entry = entry?;
            let name = entry.file_name();
            let display = self.records_root().join(&name);
            let name_text = name.to_string_lossy();
            if name_text == STAGING_DIRECTORY_NAME {
                handles
                    .records
                    .open_dir_nofollow(&name)
                    .map_err(|error| classify_link_error(&display, error))?;
                continue;
            }
            let file_type = entry.file_type()?;
            if file_type.is_symlink() || !file_type.is_dir() {
                return Err(DebloatHistoryStoreError::UnexpectedEntry(display));
            }
            let record_id = DebloatHistoryRecordId::new(name_text.as_ref().to_string())
                .map_err(|_| DebloatHistoryStoreError::UnexpectedEntry(display.clone()))?;
            summaries.push(
                load_record_from_root(&handles.records, &self.records_root(), &record_id)?
                    .summary(),
            );
        }
        summaries.sort_by(|left, right| left.record_id.as_str().cmp(right.record_id.as_str()));
        Ok(summaries)
    }

    pub fn audit(&self) -> Result<(), DebloatHistoryStoreError> {
        let Some(handles) = self.open_existing_store()? else {
            return Ok(());
        };
        let _ = self.list()?;
        if let Some(staging) = &handles.staging {
            audit_staging(staging, &self.records_root().join(STAGING_DIRECTORY_NAME))?;
        }
        Ok(())
    }

    pub fn prepare_restore_from_inventory_by_id(
        &self,
        record_id: &DebloatHistoryRecordId,
        inventory: &ExactAppxInventory,
        mission_id: impl Into<String>,
    ) -> Result<DebloatRestorePreparedTransaction, DebloatHistoryStoreError> {
        let stored = self.load(record_id)?;
        Ok(prepare_restore_from_inventory(
            stored.receipt(),
            inventory,
            mission_id,
        )?)
    }

    pub fn prepare_windows_restore_by_id(
        &self,
        record_id: &DebloatHistoryRecordId,
        mission_id: impl Into<String>,
    ) -> Result<DebloatRestorePreparedTransaction, DebloatHistoryStoreError> {
        let stored = self.load(record_id)?;
        Ok(prepare_windows_restore_from_receipt(
            stored.receipt(),
            mission_id,
        )?)
    }

    fn record_validated_receipt(
        &self,
        receipt: &DebloatRemovalReceipt,
    ) -> Result<HistoryRecordWriteReceipt, DebloatHistoryStoreError> {
        receipt.validate()?;
        let record_id = DebloatHistoryRecordId::from_receipt(receipt)?;
        let handles = self.open_or_create_store()?;
        if let Some(existing) =
            try_load_record_from_root(&handles.records, &self.records_root(), &record_id)?
        {
            return existing_write_receipt(self, &record_id, receipt, existing);
        }

        let envelope = StoredReceiptEnvelope::new(receipt.clone())?;
        let encoded = serde_json::to_vec_pretty(&envelope)?;
        if encoded.len() as u64 > MAX_HISTORY_RECORD_BYTES {
            return Err(DebloatHistoryStoreError::RecordTooLarge {
                path: self.records_root().join(record_id.as_str()),
                limit: MAX_HISTORY_RECORD_BYTES,
            });
        }

        let staging = handles
            .staging
            .as_ref()
            .expect("write path creates staging");
        let (staging_name, staging_dir) = begin_unique_staging(
            staging,
            &self.records_root().join(STAGING_DIRECTORY_NAME),
            &record_id,
        )?;
        let staging_display = self
            .records_root()
            .join(STAGING_DIRECTORY_NAME)
            .join(&staging_name);
        let staged_record_display = staging_display.join(STAGED_RECORD_DIRECTORY_NAME);
        let write_result = (|| -> Result<(), DebloatHistoryStoreError> {
            write_staging_marker(&staging_dir, &staging_name, &record_id)?;
            staging_dir.create_dir(STAGED_RECORD_DIRECTORY_NAME)?;
            let staged_record_dir = staging_dir
                .open_dir_nofollow(STAGED_RECORD_DIRECTORY_NAME)
                .map_err(|error| classify_link_error(&staged_record_display, error))?;
            let mut file = create_new_file_nofollow(&staged_record_dir, RECORD_FILE_NAME)?;
            file.write_all(&encoded)?;
            file.sync_all()?;
            drop(file);
            let reloaded =
                load_envelope_from_dir(&staged_record_dir, &staged_record_display, &record_id)?;
            if reloaded.receipt() != receipt {
                return Err(DebloatHistoryStoreError::RecordConflict(
                    record_id.to_string(),
                ));
            }
            validate_staging_marker(&staging_dir, &staging_name, &staging_display, &record_id)?;
            drop(staged_record_dir);
            Ok(())
        })();

        if let Err(error) = write_result {
            drop(staging_dir);
            cleanup_owned_staging(staging, &staging_name, &staging_display, &record_id)?;
            return Err(error);
        }

        let promotion = staging_dir.rename(
            STAGED_RECORD_DIRECTORY_NAME,
            &handles.records,
            record_id.as_str(),
        );
        drop(staging_dir);

        match promotion {
            Ok(()) => {
                cleanup_owned_staging(staging, &staging_name, &staging_display, &record_id)?;
                let stored =
                    load_record_from_root(&handles.records, &self.records_root(), &record_id)?;
                if stored.receipt() != receipt {
                    return Err(DebloatHistoryStoreError::RecordConflict(
                        record_id.to_string(),
                    ));
                }
                Ok(HistoryRecordWriteReceipt {
                    record_id: record_id.clone(),
                    disposition: HistoryRecordDisposition::Recorded,
                    path: self.records_root().join(record_id.as_str()),
                })
            }
            Err(rename_error) => {
                let existing =
                    try_load_record_from_root(&handles.records, &self.records_root(), &record_id);
                cleanup_owned_staging(staging, &staging_name, &staging_display, &record_id)?;
                match existing? {
                    Some(stored) => existing_write_receipt(self, &record_id, receipt, stored),
                    None => Err(DebloatHistoryStoreError::Io(rename_error)),
                }
            }
        }
    }

    fn open_or_create_store(&self) -> Result<StoreHandles, DebloatHistoryStoreError> {
        let application = open_absolute_dir_nofollow(self.layout.application_root())?;
        let managed = open_or_create_child_dir(
            &application,
            neo_vault::MANAGED_DIRECTORY_NAME,
            self.layout.managed_root(),
        )?;
        let history_display = self.history_root();
        let history = open_or_create_child_dir(&managed, "history", &history_display)?;
        let records_display = self.records_root();
        let records =
            open_or_create_child_dir(&history, DEBLOAT_REMOVALS_DIRECTORY_NAME, &records_display)?;
        let staging_display = records_display.join(STAGING_DIRECTORY_NAME);
        let staging = open_or_create_child_dir(&records, STAGING_DIRECTORY_NAME, &staging_display)?;
        Ok(StoreHandles {
            records,
            staging: Some(staging),
        })
    }

    fn open_existing_store(&self) -> Result<Option<StoreHandles>, DebloatHistoryStoreError> {
        let application = open_absolute_dir_nofollow(self.layout.application_root())?;
        let Some(managed) = open_optional_child_dir(
            &application,
            neo_vault::MANAGED_DIRECTORY_NAME,
            self.layout.managed_root(),
        )?
        else {
            return Ok(None);
        };
        let history_display = self.history_root();
        let Some(history) = open_optional_child_dir(&managed, "history", &history_display)? else {
            return Ok(None);
        };
        let records_display = self.records_root();
        let Some(records) =
            open_optional_child_dir(&history, DEBLOAT_REMOVALS_DIRECTORY_NAME, &records_display)?
        else {
            return Ok(None);
        };
        let staging_display = records_display.join(STAGING_DIRECTORY_NAME);
        let staging = open_optional_child_dir(&records, STAGING_DIRECTORY_NAME, &staging_display)?;
        Ok(Some(StoreHandles { records, staging }))
    }

    #[cfg(test)]
    pub(crate) fn record_validated_receipt_for_tests(
        &self,
        receipt: &DebloatRemovalReceipt,
    ) -> Result<HistoryRecordWriteReceipt, DebloatHistoryStoreError> {
        self.record_validated_receipt(receipt)
    }
}

struct StoreHandles {
    records: Dir,
    staging: Option<Dir>,
}

fn existing_write_receipt(
    store: &DebloatHistoryStore,
    record_id: &DebloatHistoryRecordId,
    expected: &DebloatRemovalReceipt,
    existing: StoredDebloatRemovalReceipt,
) -> Result<HistoryRecordWriteReceipt, DebloatHistoryStoreError> {
    if existing.receipt() != expected {
        return Err(DebloatHistoryStoreError::RecordConflict(
            record_id.to_string(),
        ));
    }
    Ok(HistoryRecordWriteReceipt {
        record_id: record_id.clone(),
        disposition: HistoryRecordDisposition::AlreadyPresent,
        path: store.records_root().join(record_id.as_str()),
    })
}

fn try_load_record_from_root(
    records: &Dir,
    records_display: &Path,
    record_id: &DebloatHistoryRecordId,
) -> Result<Option<StoredDebloatRemovalReceipt>, DebloatHistoryStoreError> {
    let display = records_display.join(record_id.as_str());
    match records.open_dir_nofollow(record_id.as_str()) {
        Ok(record_dir) => Ok(Some(load_envelope_from_dir(
            &record_dir,
            &display,
            record_id,
        )?)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(classify_link_error(&display, error)),
    }
}

fn load_record_from_root(
    records: &Dir,
    records_display: &Path,
    record_id: &DebloatHistoryRecordId,
) -> Result<StoredDebloatRemovalReceipt, DebloatHistoryStoreError> {
    try_load_record_from_root(records, records_display, record_id)?
        .ok_or_else(|| DebloatHistoryStoreError::RecordNotFound(record_id.to_string()))
}

fn load_envelope_from_dir(
    record_dir: &Dir,
    display: &Path,
    record_id: &DebloatHistoryRecordId,
) -> Result<StoredDebloatRemovalReceipt, DebloatHistoryStoreError> {
    validate_record_directory_entries(record_dir, display)?;
    load_envelope_file_from_dir(record_dir, display, record_id)
}

fn load_envelope_file_from_dir(
    record_dir: &Dir,
    display: &Path,
    record_id: &DebloatHistoryRecordId,
) -> Result<StoredDebloatRemovalReceipt, DebloatHistoryStoreError> {
    let file_display = display.join(RECORD_FILE_NAME);
    let file = open_read_file_nofollow(record_dir, RECORD_FILE_NAME)
        .map_err(|error| map_file_error(&file_display, error))?;
    let metadata = file.metadata()?;
    if metadata.len() > MAX_HISTORY_RECORD_BYTES {
        return Err(DebloatHistoryStoreError::RecordTooLarge {
            path: file_display,
            limit: MAX_HISTORY_RECORD_BYTES,
        });
    }
    let envelope: StoredReceiptEnvelope = serde_json::from_reader(file)?;
    envelope.validate(record_id)
}

fn validate_record_directory_entries(
    record_dir: &Dir,
    display: &Path,
) -> Result<(), DebloatHistoryStoreError> {
    let mut saw_record = false;
    for entry in record_dir.entries()? {
        let entry = entry?;
        let name = entry.file_name();
        let child = display.join(&name);
        if name != OsString::from(RECORD_FILE_NAME) {
            return Err(DebloatHistoryStoreError::UnexpectedEntry(child));
        }
        let file_type = entry.file_type()?;
        if file_type.is_symlink() || !file_type.is_file() {
            return Err(DebloatHistoryStoreError::UnexpectedEntry(child));
        }
        saw_record = true;
    }
    if !saw_record {
        return Err(DebloatHistoryStoreError::InvalidRecord(format!(
            "missing {RECORD_FILE_NAME} in {}",
            display.display()
        )));
    }
    Ok(())
}

fn begin_unique_staging(
    staging: &Dir,
    staging_display: &Path,
    record_id: &DebloatHistoryRecordId,
) -> Result<(String, Dir), DebloatHistoryStoreError> {
    loop {
        let sequence = NEXT_STAGING_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let name = format!(
            "record-{}-{}-{sequence}",
            &record_id.as_str()[..16],
            std::process::id()
        );
        let display = staging_display.join(&name);
        match staging.create_dir(&name) {
            Ok(()) => {
                let dir = staging
                    .open_dir_nofollow(&name)
                    .map_err(|error| classify_link_error(&display, error))?;
                return Ok((name, dir));
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(DebloatHistoryStoreError::Io(error)),
        }
    }
}

fn write_staging_marker(
    staging_dir: &Dir,
    staging_name: &str,
    record_id: &DebloatHistoryRecordId,
) -> Result<(), DebloatHistoryStoreError> {
    let marker = StagingMarker {
        schema_version: DEBLOAT_HISTORY_STORE_SCHEMA_VERSION,
        staging_name: staging_name.to_string(),
        record_id: record_id.clone(),
    };
    let bytes = serde_json::to_vec_pretty(&marker)?;
    let mut file = create_new_file_nofollow(staging_dir, STAGING_MARKER_NAME)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    Ok(())
}

fn validate_staging_marker(
    staging_dir: &Dir,
    staging_name: &str,
    display: &Path,
    expected_record_id: &DebloatHistoryRecordId,
) -> Result<(), DebloatHistoryStoreError> {
    let marker_display = display.join(STAGING_MARKER_NAME);
    let file = open_read_file_nofollow(staging_dir, STAGING_MARKER_NAME)
        .map_err(|error| map_file_error(&marker_display, error))?;
    let marker: StagingMarker = serde_json::from_reader(file)?;
    if marker.schema_version != DEBLOAT_HISTORY_STORE_SCHEMA_VERSION
        || marker.staging_name != staging_name
        || marker.record_id != *expected_record_id
    {
        return Err(DebloatHistoryStoreError::InvalidRecord(format!(
            "staging marker mismatch at {}",
            display.display()
        )));
    }
    Ok(())
}

fn cleanup_owned_staging(
    staging: &Dir,
    staging_name: &str,
    display: &Path,
    record_id: &DebloatHistoryRecordId,
) -> Result<(), DebloatHistoryStoreError> {
    let staging_dir = staging
        .open_dir_nofollow(staging_name)
        .map_err(|error| classify_link_error(display, error))?;
    validate_staging_marker(&staging_dir, staging_name, display, record_id)?;
    drop(staging_dir);
    staging.remove_dir_all(staging_name)?;
    Ok(())
}

fn audit_staging(staging: &Dir, display: &Path) -> Result<(), DebloatHistoryStoreError> {
    for entry in staging.entries()? {
        let entry = entry?;
        let name = entry.file_name();
        let child = display.join(&name);
        let file_type = entry.file_type()?;
        if file_type.is_symlink() || !file_type.is_dir() {
            return Err(DebloatHistoryStoreError::UnexpectedEntry(child));
        }
        let staging_name = name.to_string_lossy().to_string();
        let staging_dir = staging
            .open_dir_nofollow(&name)
            .map_err(|error| classify_link_error(&child, error))?;
        let marker_display = child.join(STAGING_MARKER_NAME);
        let marker_file = open_read_file_nofollow(&staging_dir, STAGING_MARKER_NAME)
            .map_err(|error| map_file_error(&marker_display, error))?;
        let marker: StagingMarker = serde_json::from_reader(marker_file)?;
        let expected_prefix = format!("record-{}-", &marker.record_id.as_str()[..16]);
        if marker.schema_version != DEBLOAT_HISTORY_STORE_SCHEMA_VERSION
            || marker.staging_name != staging_name
            || !staging_name.starts_with(&expected_prefix)
        {
            return Err(DebloatHistoryStoreError::InvalidRecord(format!(
                "unowned or mismatched staging directory {}",
                child.display()
            )));
        }
        for nested in staging_dir.entries()? {
            let nested = nested?;
            let nested_name = nested.file_name();
            let nested_display = child.join(&nested_name);
            if nested_name == OsString::from(STAGING_MARKER_NAME) {
                let nested_type = nested.file_type()?;
                if nested_type.is_symlink() || !nested_type.is_file() {
                    return Err(DebloatHistoryStoreError::UnexpectedEntry(nested_display));
                }
                continue;
            }
            if nested_name == OsString::from(STAGED_RECORD_DIRECTORY_NAME) {
                let nested_type = nested.file_type()?;
                if nested_type.is_symlink() || !nested_type.is_dir() {
                    return Err(DebloatHistoryStoreError::UnexpectedEntry(nested_display));
                }
                let staged_record_dir = staging_dir
                    .open_dir_nofollow(&nested_name)
                    .map_err(|error| classify_link_error(&nested_display, error))?;
                load_envelope_from_dir(&staged_record_dir, &nested_display, &marker.record_id)?;
                continue;
            }
            return Err(DebloatHistoryStoreError::UnexpectedEntry(nested_display));
        }
    }
    Ok(())
}

fn create_new_file_nofollow(
    dir: &Dir,
    name: impl AsRef<Path>,
) -> Result<CapFile, DebloatHistoryStoreError> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    options.follow(FollowSymlinks::No);
    Ok(dir.open_with(name, &options)?)
}

fn open_read_file_nofollow(
    dir: &Dir,
    name: impl AsRef<Path>,
) -> Result<CapFile, DebloatHistoryStoreError> {
    let mut options = OpenOptions::new();
    options.read(true);
    options.follow(FollowSymlinks::No);
    Ok(dir.open_with(name, &options)?)
}

fn open_or_create_child_dir(
    parent: &Dir,
    name: &str,
    display: &Path,
) -> Result<Dir, DebloatHistoryStoreError> {
    match parent.open_dir_nofollow(name) {
        Ok(dir) => Ok(dir),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            match parent.create_dir(name) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(DebloatHistoryStoreError::Io(error)),
            }
            parent
                .open_dir_nofollow(name)
                .map_err(|error| classify_link_error(display, error))
        }
        Err(error) => Err(classify_link_error(display, error)),
    }
}

fn open_optional_child_dir(
    parent: &Dir,
    name: &str,
    display: &Path,
) -> Result<Option<Dir>, DebloatHistoryStoreError> {
    match parent.open_dir_nofollow(name) {
        Ok(dir) => Ok(Some(dir)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(classify_link_error(display, error)),
    }
}

fn open_absolute_dir_nofollow(path: &Path) -> Result<Dir, DebloatHistoryStoreError> {
    if !path.is_absolute() {
        return Err(DebloatHistoryStoreError::StoreUnavailable(
            path.to_path_buf(),
        ));
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

fn split_absolute_dir(path: &Path) -> Result<(PathBuf, Vec<OsString>), DebloatHistoryStoreError> {
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
            Component::ParentDir => {
                return Err(DebloatHistoryStoreError::StoreUnavailable(
                    path.to_path_buf(),
                ))
            }
            Component::Normal(name) => names.push(name.to_os_string()),
        }
    }
    if !saw_root {
        return Err(DebloatHistoryStoreError::StoreUnavailable(
            path.to_path_buf(),
        ));
    }
    Ok((root, names))
}

fn classify_link_error(path: &Path, error: std::io::Error) -> DebloatHistoryStoreError {
    if diagnostic_link_like(path) {
        DebloatHistoryStoreError::UnsafeLink(path.to_path_buf())
    } else if error.kind() == std::io::ErrorKind::NotFound {
        DebloatHistoryStoreError::StoreUnavailable(path.to_path_buf())
    } else {
        DebloatHistoryStoreError::Io(error)
    }
}

fn map_file_error(path: &Path, error: DebloatHistoryStoreError) -> DebloatHistoryStoreError {
    match error {
        DebloatHistoryStoreError::Io(io_error) => classify_link_error(path, io_error),
        other => other,
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
