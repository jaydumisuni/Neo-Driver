use super::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_TEMP: AtomicU64 = AtomicU64::new(1);

struct TempRoot {
    path: PathBuf,
}

impl TempRoot {
    fn new(label: &str) -> Self {
        let id = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "neo-vault-test-{label}-{}-{id}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("create temp root");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn layout(root: &TempRoot) -> VaultLayout {
    VaultLayout::new(VaultMode::Installed, root.path()).expect("valid absolute test root")
}

#[test]
fn layout_is_rooted_under_supplied_application_root() {
    let root = TempRoot::new("layout");
    let layout = layout(&root);
    assert_eq!(layout.application_root(), root.path());
    assert_eq!(layout.managed_root(), root.path().join("NeoData"));
    assert_eq!(layout.driver_packs(), root.path().join("NeoData/driver-packs"));
    assert_eq!(layout.staging(), root.path().join("NeoData/staging"));
}

#[test]
fn installed_and_portable_modes_share_the_same_child_layout() {
    let root = TempRoot::new("modes");
    let installed = VaultLayout::new(VaultMode::Installed, root.path()).unwrap();
    let portable = VaultLayout::new(VaultMode::Portable, root.path()).unwrap();
    assert_eq!(installed.managed_root(), portable.managed_root());
    assert_ne!(installed.mode(), portable.mode());
}

#[test]
fn relative_application_root_is_rejected() {
    assert!(matches!(
        VaultLayout::new(VaultMode::Portable, Path::new("relative/neo")),
        Err(VaultError::ApplicationRootNotAbsolute(_))
    ));
}

#[test]
fn vault_segments_reject_traversal_and_windows_separators() {
    for value in ["", ".", "..", "../escape", "a/b", r"a\b", "C:", "name ", "name."] {
        assert!(VaultSegment::new(value).is_err(), "accepted {value:?}");
    }
    assert!(VaultSegment::new("android.adb-1_0+usb").is_ok());
}

#[test]
fn sha256_digest_is_normalized_and_validated() {
    let digest = Sha256Digest::new("A".repeat(64)).unwrap();
    assert_eq!(digest.as_str(), "a".repeat(64));
    assert!(Sha256Digest::new("abc").is_err());
}

#[test]
fn direct_source_map_deserialization_runs_validation() {
    let json = r#"{
        "schema_version":1,
        "sources":[{
            "id":"android",
            "family":"android",
            "kind":"driver_pack",
            "repository":"bad repository",
            "release_tag":"v1",
            "asset_name":"drivers.zip",
            "sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "size_bytes":1
        }]
    }"#;
    assert!(serde_json::from_str::<DriverSourceMap>(json).is_err());
}

#[test]
fn source_map_rejects_case_insensitive_duplicate_asset_identity() {
    let make = |id: &str, asset: &str| DriverSource {
        id: VaultSegment::new(id).unwrap(),
        family: VaultSegment::new("android").unwrap(),
        kind: SourcePackageKind::DriverPack,
        repository: "jaydumisuni/android-drivers".to_string(),
        release_tag: "v1".to_string(),
        asset_name: asset.to_string(),
        sha256: Sha256Digest::new("a".repeat(64)).unwrap(),
        size_bytes: 1,
    };
    assert!(matches!(
        DriverSourceMap::new(vec![make("one", "Drivers.zip"), make("two", "drivers.ZIP")]),
        Err(VaultError::DuplicateSourceAsset(_))
    ));
}

#[test]
fn staging_cleanup_requires_neo_ownership_marker() {
    let root = TempRoot::new("unowned");
    let store = VaultStore::new(layout(&root));
    store.ensure_layout().unwrap();
    let session = VaultSegment::new("session-a").unwrap();
    let path = store.layout().staging_session(&session);
    fs::create_dir(&path).unwrap();
    fs::write(path.join("foreign.txt"), b"do not delete").unwrap();

    assert!(matches!(
        store.cleanup_staging(&session),
        Err(VaultError::UnownedStaging(_))
    ));
    assert!(path.join("foreign.txt").exists());
}

#[test]
fn staging_marker_binds_cleanup_to_exact_session() {
    let root = TempRoot::new("marker");
    let store = VaultStore::new(layout(&root));
    let session = VaultSegment::new("session-a").unwrap();
    let path = store.begin_staging(&session).unwrap();
    let marker = path.join(STAGING_MARKER_NAME);
    fs::write(
        &marker,
        r#"{"schema_version":1,"session":"different-session"}"#,
    )
    .unwrap();
    assert!(matches!(
        store.cleanup_staging(&session),
        Err(VaultError::StagingMarkerMismatch { .. })
    ));
    assert!(path.exists());
}

#[test]
fn import_pack_hashes_before_and_after_copy_and_cleans_staging() {
    let root = TempRoot::new("import");
    let source = root.path().join("source.zip");
    fs::write(&source, b"neo approved driver pack").unwrap();
    let expected = sha256_file(&source).unwrap();
    let store = VaultStore::new(layout(&root));
    let receipt = store
        .import_pack_file(
            PackClass::Driver,
            &source,
            &VaultSegment::new("android-pack").unwrap(),
            &VaultSegment::new("v1").unwrap(),
            &expected,
        )
        .unwrap();

    assert_eq!(receipt.disposition, ImportDisposition::Imported);
    assert!(receipt.destination.exists());
    assert_eq!(sha256_file(&receipt.destination).unwrap(), expected);
    assert_eq!(fs::read_dir(store.layout().staging()).unwrap().count(), 0);
    assert!(source.exists(), "Neo must not delete the operator source file");
}

#[test]
fn importing_the_same_pack_is_idempotent() {
    let root = TempRoot::new("idempotent");
    let source = root.path().join("source.zip");
    fs::write(&source, b"same pack").unwrap();
    let expected = sha256_file(&source).unwrap();
    let store = VaultStore::new(layout(&root));
    let package = VaultSegment::new("pack").unwrap();
    let version = VaultSegment::new("v1").unwrap();
    store
        .import_pack_file(PackClass::Driver, &source, &package, &version, &expected)
        .unwrap();
    let second = store
        .import_pack_file(PackClass::Driver, &source, &package, &version, &expected)
        .unwrap();
    assert_eq!(second.disposition, ImportDisposition::AlreadyPresent);
}

#[test]
fn promoted_pack_is_never_overwritten_when_content_drifted() {
    let root = TempRoot::new("conflict");
    let source = root.path().join("source.zip");
    fs::write(&source, b"approved bytes").unwrap();
    let expected = sha256_file(&source).unwrap();
    let store = VaultStore::new(layout(&root));
    let package = VaultSegment::new("pack").unwrap();
    let version = VaultSegment::new("v1").unwrap();
    let first = store
        .import_pack_file(PackClass::Driver, &source, &package, &version, &expected)
        .unwrap();
    fs::write(&first.destination, b"tampered").unwrap();

    assert!(matches!(
        store.import_pack_file(PackClass::Driver, &source, &package, &version, &expected),
        Err(VaultError::DestinationConflict(_))
    ));
    assert_eq!(fs::read(&first.destination).unwrap(), b"tampered");
}

#[test]
fn hash_mismatch_fails_before_vault_mutation() {
    let root = TempRoot::new("hash-mismatch");
    let source = root.path().join("source.zip");
    fs::write(&source, b"wrong bytes").unwrap();
    let store = VaultStore::new(layout(&root));
    let expected = Sha256Digest::new("a".repeat(64)).unwrap();
    assert!(matches!(
        store.import_pack_file(
            PackClass::Driver,
            &source,
            &VaultSegment::new("pack").unwrap(),
            &VaultSegment::new("v1").unwrap(),
            &expected,
        ),
        Err(VaultError::HashMismatch { .. })
    ));
    assert!(!store.layout().managed_root().exists());
}

#[cfg(unix)]
#[test]
fn audit_rejects_symlink_inside_managed_tree() {
    use std::os::unix::fs::symlink;

    let root = TempRoot::new("symlink");
    let store = VaultStore::new(layout(&root));
    store.ensure_layout().unwrap();
    let outside = root.path().join("outside.txt");
    fs::write(&outside, b"outside").unwrap();
    let link = store.layout().cache().join("outside-link");
    symlink(&outside, &link).unwrap();
    assert!(matches!(
        store.audit_existing_tree(),
        Err(VaultError::UnsafeLink(path)) if path == link
    ));
}
