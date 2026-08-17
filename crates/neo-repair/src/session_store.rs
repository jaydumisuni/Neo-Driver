use crate::{RepairError, RepairExecutionSession};
use cap_fs_ext::{DirExt, FollowSymlinks, OpenOptionsFollowExt};
use cap_std::ambient_authority;
use cap_std::fs::{Dir, File as CapFile, OpenOptions};
use neo_transaction::TransactionStage;
use neo_vault::{VaultLayout, MANAGED_DIRECTORY_NAME};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::ffi::OsString;
use std::fs;
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const STORE_SCHEMA_VERSION: u32 = 1;
const SESSION_NAMESPACE: &str = "phase21-repair";
const SESSION_MARKER: &str = ".neo-repair-session.json";
const MAX_SESSION_RECORD_BYTES: u64 = 512 * 1024;
const MAX_SESSION_VERSIONS: u64 = 64;
static NEXT_TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RepairSessionOwner {
    pub(crate) kind: String,
    pub(crate) principal: String,
}

impl RepairSessionOwner {
    pub(crate) fn new(
        kind: impl Into<String>,
        principal: impl Into<String>,
    ) -> Result<Self, RepairError> {
        let owner = Self {
            kind: kind.into(),
            principal: principal.into(),
        };
        owner.validate()?;
        Ok(owner)
    }

    fn validate(&self) -> Result<(), RepairError> {
        require_text("resume-session owner kind", &self.kind, 64)?;
        require_text("resume-session owner principal", &self.principal, 160)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct StoredRepairSession {
    pub(crate) owner: RepairSessionOwner,
    pub(crate) session_id: String,
    pub(crate) version: u64,
    pub(crate) session: RepairExecutionSession,
}

#[derive(Debug, Clone)]
pub(crate) struct RepairResumeSessionStore {
    layout: VaultLayout,
}

impl RepairResumeSessionStore {
    pub(crate) fn new(layout: VaultLayout) -> Self {
        Self { layout }
    }

    pub(crate) fn store_root(&self) -> PathBuf {
        self.layout.sessions().join(SESSION_NAMESPACE)
    }

    pub(crate) fn load_latest(
        &self,
        session_id: &str,
    ) -> Result<Option<StoredRepairSession>, RepairError> {
        validate_session_id(session_id)?;
        let Some(root) = self.open_existing_store()? else {
            return Ok(None);
        };
        let key = session_key(session_id);
        let display = self.store_root().join(&key);
        let session_dir = match root.open_dir_nofollow(&key) {
            Ok(dir) => dir,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(classify_io(&display, error)),
        };
        let marker = read_marker(&session_dir, &display)?;
        if marker.session_key != key || marker.session_id != session_id {
            return Err(RepairError::InvalidPersistedSession(
                "session marker identity does not match requested session".to_string(),
            ));
        }
        let Some((version, envelope)) = latest_envelope(&session_dir, &display)? else {
            return Err(RepairError::InvalidPersistedSession(
                "session directory has no persisted state version".to_string(),
            ));
        };
        validate_envelope(&envelope, session_id, &marker.owner)?;
        Ok(Some(StoredRepairSession {
            owner: marker.owner,
            session_id: session_id.to_string(),
            version,
            session: envelope.session,
        }))
    }

    pub(crate) fn persist(
        &self,
        session_id: &str,
        owner: &RepairSessionOwner,
        session: &RepairExecutionSession,
    ) -> Result<StoredRepairSession, RepairError> {
        validate_session_id(session_id)?;
        owner.validate()?;
        session.validate()?;
        require_persistable_stage(session.stage())?;
        let plan_fingerprint = session.plan().transaction().fingerprint()?;
        let envelope = SessionEnvelope::new(
            session_id.to_string(),
            owner.clone(),
            plan_fingerprint,
            session.clone(),
        )?;
        let root = self.open_or_create_store()?;
        let key = session_key(session_id);
        let display = self.store_root().join(&key);
        let (session_dir, newly_created) = open_or_create_session_dir(&root, &key, &display)?;
        if newly_created {
            if let Err(error) = write_marker(&session_dir, &key, session_id, owner) {
                drop(session_dir);
                let _ = root.remove_dir_all(&key);
                return Err(error);
            }
        }
        let marker = read_marker(&session_dir, &display)?;
        if marker.session_key != key || marker.session_id != session_id || marker.owner != *owner {
            return Err(RepairError::InvalidPersistedSession(
                "session marker owner or identity differs from persisted authority".to_string(),
            ));
        }

        let latest = latest_envelope(&session_dir, &display)?;
        let next_version = match latest {
            None => 1,
            Some((version, ref previous)) => {
                validate_envelope(previous, session_id, owner)?;
                if previous.record_fingerprint == envelope.record_fingerprint {
                    return Ok(StoredRepairSession {
                        owner: owner.clone(),
                        session_id: session_id.to_string(),
                        version,
                        session: previous.session.clone(),
                    });
                }
                validate_append(previous, &envelope)?;
                version.checked_add(1).ok_or_else(|| {
                    RepairError::SessionStore("session version sequence exhausted".to_string())
                })?
            }
        };
        if next_version > MAX_SESSION_VERSIONS {
            return Err(RepairError::SessionStore(
                "session version ceiling exceeded".to_string(),
            ));
        }
        write_version(&session_dir, &display, next_version, &envelope)?;
        Ok(StoredRepairSession {
            owner: owner.clone(),
            session_id: session_id.to_string(),
            version: next_version,
            session: session.clone(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SessionMarker {
    schema_version: u32,
    session_key: String,
    session_id: String,
    owner: RepairSessionOwner,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SessionEnvelope {
    schema_version: u32,
    session_id: String,
    owner: RepairSessionOwner,
    plan_fingerprint: String,
    session: RepairExecutionSession,
    record_fingerprint: String,
}

#[derive(Serialize)]
struct FingerprintMaterial<'a> {
    schema_version: u32,
    session_id: &'a str,
    owner: &'a RepairSessionOwner,
    plan_fingerprint: &'a str,
    session: &'a RepairExecutionSession,
}

impl SessionEnvelope {
    fn new(
        session_id: String,
        owner: RepairSessionOwner,
        plan_fingerprint: String,
        session: RepairExecutionSession,
    ) -> Result<Self, RepairError> {
        let record_fingerprint = fingerprint_material(&FingerprintMaterial {
            schema_version: STORE_SCHEMA_VERSION,
            session_id: &session_id,
            owner: &owner,
            plan_fingerprint: &plan_fingerprint,
            session: &session,
        })?;
        Ok(Self {
            schema_version: STORE_SCHEMA_VERSION,
            session_id,
            owner,
            plan_fingerprint,
            session,
            record_fingerprint,
        })
    }
}

fn validate_envelope(
    envelope: &SessionEnvelope,
    session_id: &str,
    owner: &RepairSessionOwner,
) -> Result<(), RepairError> {
    if envelope.schema_version != STORE_SCHEMA_VERSION
        || envelope.session_id != session_id
        || envelope.owner != *owner
    {
        return Err(RepairError::InvalidPersistedSession(
            "stored session envelope identity mismatch".to_string(),
        ));
    }
    envelope.owner.validate()?;
    envelope.session.validate()?;
    let plan_fingerprint = envelope.session.plan().transaction().fingerprint()?;
    if envelope.plan_fingerprint != plan_fingerprint {
        return Err(RepairError::InvalidPersistedSession(
            "stored plan fingerprint differs from validated session".to_string(),
        ));
    }
    let expected = fingerprint_material(&FingerprintMaterial {
        schema_version: envelope.schema_version,
        session_id: &envelope.session_id,
        owner: &envelope.owner,
        plan_fingerprint: &envelope.plan_fingerprint,
        session: &envelope.session,
    })?;
    if envelope.record_fingerprint != expected {
        return Err(RepairError::InvalidPersistedSession(
            "stored session record fingerprint mismatch".to_string(),
        ));
    }
    require_persistable_stage(envelope.session.stage())
}

fn validate_append(previous: &SessionEnvelope, next: &SessionEnvelope) -> Result<(), RepairError> {
    if previous.session_id != next.session_id
        || previous.owner != next.owner
        || previous.plan_fingerprint != next.plan_fingerprint
    {
        return Err(RepairError::InvalidPersistedSession(
            "persisted session authority changed between versions".to_string(),
        ));
    }
    if is_terminal(previous.session.stage()) {
        return Err(RepairError::InvalidPersistedSession(
            "terminal persisted session cannot be advanced".to_string(),
        ));
    }
    let old_events = previous.session.checkpoint().events();
    let new_events = next.session.checkpoint().events();
    if new_events.len() < old_events.len() || new_events[..old_events.len()] != *old_events {
        return Err(RepairError::InvalidPersistedSession(
            "persisted transaction event history is not append-only".to_string(),
        ));
    }
    Ok(())
}

fn require_persistable_stage(stage: TransactionStage) -> Result<(), RepairError> {
    if matches!(
        stage,
        TransactionStage::Applying
            | TransactionStage::AwaitingReboot
            | TransactionStage::Blocked
            | TransactionStage::AwaitingRollbackReboot
            | TransactionStage::Complete
            | TransactionStage::RolledBack
            | TransactionStage::Failed
    ) {
        Ok(())
    } else {
        Err(RepairError::InvalidPersistedSession(format!(
            "stage {stage:?} is not a durable Phase 21 resume state"
        )))
    }
}

fn is_terminal(stage: TransactionStage) -> bool {
    matches!(
        stage,
        TransactionStage::Complete | TransactionStage::RolledBack | TransactionStage::Failed
    )
}

fn fingerprint_material(value: &FingerprintMaterial<'_>) -> Result<String, RepairError> {
    let encoded = serde_json::to_vec(value).map_err(|error| {
        RepairError::SessionStore(format!("session fingerprint serialization failed: {error}"))
    })?;
    let digest = Sha256::digest(encoded);
    Ok(hex_digest(&digest))
}

fn session_key(session_id: &str) -> String {
    hex_digest(&Sha256::digest(session_id.as_bytes()))
}

fn hex_digest(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn validate_session_id(value: &str) -> Result<(), RepairError> {
    require_text("session id", value, 512)
}

fn require_text(label: &str, value: &str, max: usize) -> Result<(), RepairError> {
    if value.trim().is_empty() || value.len() > max || value.chars().any(char::is_control) {
        return Err(RepairError::InvalidRequest(format!(
            "{label} must be non-empty, bounded text without control characters"
        )));
    }
    Ok(())
}

fn version_name(version: u64) -> String {
    format!("{version:020}.json")
}

fn parse_version_name(name: &str) -> Option<u64> {
    let digits = name.strip_suffix(".json")?;
    if digits.len() != 20 || !digits.bytes().all(|value| value.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

fn latest_envelope(
    dir: &Dir,
    display: &Path,
) -> Result<Option<(u64, SessionEnvelope)>, RepairError> {
    let mut latest = None;
    for entry in dir.entries().map_err(|error| classify_io(display, error))? {
        let entry = entry.map_err(|error| classify_io(display, error))?;
        let name = entry.file_name();
        let name_text = name.to_string_lossy();
        if name_text == SESSION_MARKER || name_text.starts_with(".tmp-") {
            continue;
        }
        let Some(version) = parse_version_name(name_text.as_ref()) else {
            return Err(RepairError::InvalidPersistedSession(format!(
                "unexpected session-store entry {}",
                display.join(&name).display()
            )));
        };
        let file_type = entry
            .file_type()
            .map_err(|error| classify_io(display, error))?;
        if file_type.is_symlink() || !file_type.is_file() {
            return Err(RepairError::InvalidPersistedSession(format!(
                "session version is not a regular file: {}",
                display.join(&name).display()
            )));
        }
        if latest
            .as_ref()
            .is_none_or(|(current, _)| version > *current)
        {
            latest = Some((version, read_envelope(dir, display, name_text.as_ref())?));
        }
    }
    Ok(latest)
}

fn read_envelope(dir: &Dir, display: &Path, name: &str) -> Result<SessionEnvelope, RepairError> {
    let file = open_read_file_nofollow(dir, name)?;
    let metadata = file
        .metadata()
        .map_err(|error| classify_io(&display.join(name), error))?;
    if metadata.len() > MAX_SESSION_RECORD_BYTES {
        return Err(RepairError::InvalidPersistedSession(format!(
            "session record exceeds {} bytes",
            MAX_SESSION_RECORD_BYTES
        )));
    }
    let mut bytes = Vec::new();
    file.take(MAX_SESSION_RECORD_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| classify_io(&display.join(name), error))?;
    if bytes.len() as u64 > MAX_SESSION_RECORD_BYTES {
        return Err(RepairError::InvalidPersistedSession(
            "session record exceeded bounded read".to_string(),
        ));
    }
    serde_json::from_slice(&bytes).map_err(|error| {
        RepairError::InvalidPersistedSession(format!("invalid session record JSON: {error}"))
    })
}

fn write_version(
    dir: &Dir,
    display: &Path,
    version: u64,
    envelope: &SessionEnvelope,
) -> Result<(), RepairError> {
    let encoded = serde_json::to_vec_pretty(envelope).map_err(|error| {
        RepairError::SessionStore(format!("session serialization failed: {error}"))
    })?;
    if encoded.len() as u64 > MAX_SESSION_RECORD_BYTES {
        return Err(RepairError::SessionStore(
            "session record exceeds durable size bound".to_string(),
        ));
    }
    let final_name = version_name(version);
    let temporary = format!(
        ".tmp-{}-{}-{}",
        version,
        std::process::id(),
        NEXT_TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    );
    let mut file = create_new_file_nofollow(dir, &temporary)?;
    let write_result = (|| -> Result<(), RepairError> {
        file.write_all(&encoded)
            .map_err(|error| classify_io(&display.join(&temporary), error))?;
        file.sync_all()
            .map_err(|error| classify_io(&display.join(&temporary), error))?;
        drop(file);
        let reloaded = read_envelope(dir, display, &temporary)?;
        if reloaded.record_fingerprint != envelope.record_fingerprint {
            return Err(RepairError::InvalidPersistedSession(
                "staged session record changed before publication".to_string(),
            ));
        }
        match dir.hard_link(&temporary, dir, &final_name) {
            Ok(()) => {
                // Publication already succeeded. Temporary cleanup is best-effort so a cleanup
                // failure cannot turn a durable successful version into a false API failure.
                let _ = dir.remove_file(&temporary);
                Ok(())
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                Err(RepairError::InvalidPersistedSession(format!(
                    "session version publication collided at {}",
                    display.join(&final_name).display()
                )))
            }
            Err(error) => Err(classify_io(&display.join(&final_name), error)),
        }
    })();
    if write_result.is_err() {
        let _ = dir.remove_file(&temporary);
    }
    write_result
}

fn write_marker(
    dir: &Dir,
    key: &str,
    session_id: &str,
    owner: &RepairSessionOwner,
) -> Result<(), RepairError> {
    let marker = SessionMarker {
        schema_version: STORE_SCHEMA_VERSION,
        session_key: key.to_string(),
        session_id: session_id.to_string(),
        owner: owner.clone(),
    };
    let encoded = serde_json::to_vec_pretty(&marker).map_err(|error| {
        RepairError::SessionStore(format!("marker serialization failed: {error}"))
    })?;
    let mut file = create_new_file_nofollow(dir, SESSION_MARKER)?;
    file.write_all(&encoded)
        .map_err(|error| RepairError::SessionStore(error.to_string()))?;
    file.sync_all()
        .map_err(|error| RepairError::SessionStore(error.to_string()))?;
    Ok(())
}

fn read_marker(dir: &Dir, display: &Path) -> Result<SessionMarker, RepairError> {
    let file = open_read_file_nofollow(dir, SESSION_MARKER)?;
    let marker: SessionMarker = serde_json::from_reader(file).map_err(|error| {
        RepairError::InvalidPersistedSession(format!("invalid session marker JSON: {error}"))
    })?;
    if marker.schema_version != STORE_SCHEMA_VERSION {
        return Err(RepairError::InvalidPersistedSession(
            "unsupported session marker schema".to_string(),
        ));
    }
    marker.owner.validate()?;
    if session_key(&marker.session_id) != marker.session_key {
        return Err(RepairError::InvalidPersistedSession(format!(
            "session marker key mismatch at {}",
            display.display()
        )));
    }
    Ok(marker)
}

fn open_or_create_session_dir(
    root: &Dir,
    key: &str,
    display: &Path,
) -> Result<(Dir, bool), RepairError> {
    match root.open_dir_nofollow(key) {
        Ok(dir) => Ok((dir, false)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let newly_created = match root.create_dir(key) {
                Ok(()) => true,
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => false,
                Err(error) => return Err(classify_io(display, error)),
            };
            let dir = root
                .open_dir_nofollow(key)
                .map_err(|error| classify_io(display, error))?;
            Ok((dir, newly_created))
        }
        Err(error) => Err(classify_io(display, error)),
    }
}

impl RepairResumeSessionStore {
    fn open_or_create_store(&self) -> Result<Dir, RepairError> {
        let application = open_absolute_dir_nofollow(self.layout.application_root())?;
        let managed = open_or_create_child_dir(
            &application,
            MANAGED_DIRECTORY_NAME,
            self.layout.managed_root(),
        )?;
        let sessions = open_or_create_child_dir(&managed, "sessions", self.layout.sessions())?;
        open_or_create_child_dir(&sessions, SESSION_NAMESPACE, &self.store_root())
    }

    fn open_existing_store(&self) -> Result<Option<Dir>, RepairError> {
        let application = open_absolute_dir_nofollow(self.layout.application_root())?;
        let managed = match application.open_dir_nofollow(MANAGED_DIRECTORY_NAME) {
            Ok(dir) => dir,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(classify_io(self.layout.managed_root(), error)),
        };
        let sessions = match managed.open_dir_nofollow("sessions") {
            Ok(dir) => dir,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(classify_io(self.layout.sessions(), error)),
        };
        match sessions.open_dir_nofollow(SESSION_NAMESPACE) {
            Ok(dir) => Ok(Some(dir)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(classify_io(&self.store_root(), error)),
        }
    }
}

fn create_new_file_nofollow(dir: &Dir, name: impl AsRef<Path>) -> Result<CapFile, RepairError> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    options.follow(FollowSymlinks::No);
    dir.open_with(name, &options)
        .map_err(|error| RepairError::SessionStore(error.to_string()))
}

fn open_read_file_nofollow(dir: &Dir, name: impl AsRef<Path>) -> Result<CapFile, RepairError> {
    let mut options = OpenOptions::new();
    options.read(true);
    options.follow(FollowSymlinks::No);
    dir.open_with(name, &options)
        .map_err(|error| RepairError::SessionStore(error.to_string()))
}

fn open_or_create_child_dir(parent: &Dir, name: &str, display: &Path) -> Result<Dir, RepairError> {
    match parent.open_dir_nofollow(name) {
        Ok(dir) => Ok(dir),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            match parent.create_dir(name) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(classify_io(display, error)),
            }
            parent
                .open_dir_nofollow(name)
                .map_err(|error| classify_io(display, error))
        }
        Err(error) => Err(classify_io(display, error)),
    }
}

fn open_absolute_dir_nofollow(path: &Path) -> Result<Dir, RepairError> {
    if !path.is_absolute() {
        return Err(RepairError::SessionStore(format!(
            "application root is not absolute: {}",
            path.display()
        )));
    }
    let (root, components) = split_absolute_dir(path)?;
    let mut current = Dir::open_ambient_dir(&root, ambient_authority())
        .map_err(|error| classify_io(&root, error))?;
    let mut display = root;
    for component in components {
        display.push(&component);
        current = current
            .open_dir_nofollow(&component)
            .map_err(|error| classify_io(&display, error))?;
    }
    Ok(current)
}

fn split_absolute_dir(path: &Path) -> Result<(PathBuf, Vec<OsString>), RepairError> {
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
                return Err(RepairError::SessionStore(
                    "parent traversal is not valid session-store authority".to_string(),
                ))
            }
            Component::Normal(name) => names.push(name.to_os_string()),
        }
    }
    if !saw_root {
        return Err(RepairError::SessionStore(
            "session-store root has no absolute root component".to_string(),
        ));
    }
    Ok((root, names))
}

fn classify_io(path: &Path, error: std::io::Error) -> RepairError {
    if diagnostic_link_like(path) {
        RepairError::SessionStore(format!(
            "unsafe symlink/reparse substitution at {}",
            path.display()
        ))
    } else {
        RepairError::SessionStore(format!("{}: {error}", path.display()))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::{RepairExecutionSession, RepairExecutorCapability};
    use crate::host::testsupport::FakeRepairHost;
    use crate::model::{
        ComponentStoreState, FeatureDesiredState, SupportedWindowsFeature, SystemFileState,
        WindowsFeatureState,
    };
    use crate::operation::RepairOperation;
    use neo_transaction::TransactionAuthorization;
    use neo_vault::VaultMode;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_layout() -> (PathBuf, VaultLayout) {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "neo-repair-session-store-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        let layout = VaultLayout::new(VaultMode::Portable, &root).unwrap();
        (root, layout)
    }

    fn pending_session() -> RepairExecutionSession {
        let feature = SupportedWindowsFeature::WindowsSubsystemLinux;
        let host = FakeRepairHost::new(ComponentStoreState::Healthy, SystemFileState::Healthy);
        host.set_feature(feature, WindowsFeatureState::Disabled);
        *host.pending_feature_transition.borrow_mut() = true;
        let mut session = RepairExecutionSession::prepare_with_host(
            RepairOperation::SetWindowsFeature {
                feature,
                desired: FeatureDesiredState::Enabled,
            },
            "mission",
            &host,
        )
        .unwrap();
        let capability = RepairExecutorCapability::for_rpc();
        let fingerprint = session.plan().transaction().fingerprint().unwrap();
        session
            .authorize(
                &capability,
                TransactionAuthorization {
                    plan_fingerprint: fingerprint,
                    approved_action_ids: vec![session.plan().action_id()],
                    manual_override_action_ids: vec![],
                    high_risk_ack_action_ids: vec![],
                    irreversible_acknowledgements: vec![],
                },
            )
            .unwrap();
        session.apply_with_host(&capability, &host).unwrap();
        assert_eq!(session.stage(), TransactionStage::AwaitingReboot);
        session
    }

    #[test]
    fn pending_session_is_append_only_and_reloadable() {
        let (root, layout) = temp_layout();
        let store = RepairResumeSessionStore::new(layout);
        let owner = RepairSessionOwner::new("oracle", "owner").unwrap();
        let session = pending_session();
        let first = store
            .persist("phase21:test:session", &owner, &session)
            .unwrap();
        assert_eq!(first.version, 1);
        let loaded = store.load_latest("phase21:test:session").unwrap().unwrap();
        assert_eq!(loaded.owner, owner);
        assert_eq!(loaded.version, 1);
        assert_eq!(loaded.session.stage(), TransactionStage::AwaitingReboot);
        let second = store
            .persist("phase21:test:session", &owner, &session)
            .unwrap();
        assert_eq!(
            second.version, 1,
            "identical persistence must be idempotent"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn persisted_owner_rejects_unknown_fields() {
        let value = r#"{"kind":"oracle","principal":"owner","extra":"injected"}"#;
        assert!(serde_json::from_str::<RepairSessionOwner>(value).is_err());
    }

    #[test]
    fn existing_session_directory_is_never_reported_as_newly_created() {
        let (root_path, layout) = temp_layout();
        let store = RepairResumeSessionStore::new(layout);
        let root = store.open_or_create_store().unwrap();
        let key = session_key("phase21:test:creation-race");
        let display = store.store_root().join(&key);

        let (_first, first_created) = open_or_create_session_dir(&root, &key, &display).unwrap();
        let (_second, second_created) = open_or_create_session_dir(&root, &key, &display).unwrap();

        assert!(first_created);
        assert!(!second_created);
        assert!(display.exists());
        let _ = fs::remove_dir_all(root_path);
    }

    #[test]
    fn caller_identity_cannot_be_changed_between_versions() {
        let (root, layout) = temp_layout();
        let store = RepairResumeSessionStore::new(layout);
        let session = pending_session();
        store
            .persist(
                "phase21:test:owner",
                &RepairSessionOwner::new("oracle", "owner-a").unwrap(),
                &session,
            )
            .unwrap();
        assert!(store
            .persist(
                "phase21:test:owner",
                &RepairSessionOwner::new("oracle", "owner-b").unwrap(),
                &session,
            )
            .is_err());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn version_publication_never_replaces_an_existing_record() {
        let (root_path, layout) = temp_layout();
        let store = RepairResumeSessionStore::new(layout);
        let root = store.open_or_create_store().unwrap();
        let owner = RepairSessionOwner::new("oracle", "owner").unwrap();
        let session = pending_session();
        let plan_fingerprint = session.plan().transaction().fingerprint().unwrap();

        let first_id = "phase21:test:no-replace";
        let key = session_key(first_id);
        let display = store.store_root().join(&key);
        let (dir, newly_created) = open_or_create_session_dir(&root, &key, &display).unwrap();
        assert!(newly_created);
        write_marker(&dir, &key, first_id, &owner).unwrap();

        let first = SessionEnvelope::new(
            first_id.to_string(),
            owner.clone(),
            plan_fingerprint.clone(),
            session.clone(),
        )
        .unwrap();
        write_version(&dir, &display, 1, &first).unwrap();
        let final_path = display.join(version_name(1));
        let original = fs::read(&final_path).unwrap();

        let competitor = SessionEnvelope::new(
            "phase21:test:competing-writer".to_string(),
            owner,
            plan_fingerprint,
            session,
        )
        .unwrap();
        assert!(matches!(
            write_version(&dir, &display, 1, &competitor),
            Err(RepairError::InvalidPersistedSession(_))
        ));
        assert_eq!(
            fs::read(&final_path).unwrap(),
            original,
            "a racing writer must never replace an already-published version"
        );
        let _ = fs::remove_dir_all(root_path);
    }
}
