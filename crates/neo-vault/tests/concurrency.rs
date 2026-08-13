use neo_vault::{
    sha256_file, ImportDisposition, PackClass, VaultError, VaultLayout, VaultMode, VaultSegment,
    VaultStore,
};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

struct TempRoot {
    path: PathBuf,
}

impl TempRoot {
    fn new(label: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "neo-vault-integration-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("create temp root");
        Self { path }
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn concurrent_same_pack_import_never_overwrites_or_leaves_staging_noise() {
    const WORKERS: usize = 8;

    let root = TempRoot::new("concurrent");
    let source = root.path.join("source.zip");
    fs::write(&source, b"concurrent approved Neo pack").unwrap();
    let digest = sha256_file(&source).unwrap();
    let layout = VaultLayout::new(VaultMode::Installed, &root.path).unwrap();
    let store = VaultStore::new(layout);
    let package = VaultSegment::new("android-pack").unwrap();
    let version = VaultSegment::new("v1").unwrap();
    let barrier = Arc::new(Barrier::new(WORKERS));

    let mut handles = Vec::new();
    for _ in 0..WORKERS {
        let worker_store = store.clone();
        let worker_source = source.clone();
        let worker_package = package.clone();
        let worker_version = version.clone();
        let worker_digest = digest.clone();
        let worker_barrier = Arc::clone(&barrier);
        handles.push(thread::spawn(move || {
            worker_barrier.wait();
            worker_store.import_pack_file(
                PackClass::Driver,
                worker_source,
                &worker_package,
                &worker_version,
                &worker_digest,
            )
        }));
    }

    let mut imported = 0;
    let mut already_present = 0;
    let mut busy = 0;
    for handle in handles {
        match handle.join().expect("worker thread") {
            Ok(receipt) if receipt.disposition == ImportDisposition::Imported => imported += 1,
            Ok(receipt) if receipt.disposition == ImportDisposition::AlreadyPresent => {
                already_present += 1
            }
            Err(VaultError::ImportBusy(_)) => busy += 1,
            Ok(receipt) => panic!("unexpected import disposition: {:?}", receipt.disposition),
            Err(error) => panic!("unexpected import error: {error}"),
        }
    }

    assert_eq!(imported, 1, "exactly one worker may promote the pack");
    assert_eq!(imported + already_present + busy, WORKERS);
    let destination = store
        .layout()
        .driver_pack_destination(&package, &version, digest.as_str());
    assert_eq!(sha256_file(&destination).unwrap(), digest);
    assert_eq!(
        fs::read_dir(store.layout().staging()).unwrap().count(),
        0,
        "successful, idempotent, and busy imports must leave no staging noise"
    );
}

#[cfg(unix)]
#[test]
fn audit_rejects_symlink_application_root() {
    use std::os::unix::fs::symlink;

    let root = TempRoot::new("app-root-link");
    let real_app = root.path.join("real-app");
    fs::create_dir(&real_app).unwrap();
    let linked_app = root.path.join("linked-app");
    symlink(&real_app, &linked_app).unwrap();

    let layout = VaultLayout::new(VaultMode::Installed, &linked_app).unwrap();
    let store = VaultStore::new(layout);
    assert!(matches!(
        store.audit_existing_tree(),
        Err(VaultError::UnsafeLink(path)) if path == linked_app
    ));
}

#[cfg(unix)]
#[test]
fn import_rejects_symlinked_package_directory_without_writing_outside() {
    use std::os::unix::fs::symlink;

    let root = TempRoot::new("package-link");
    let source = root.path.join("source.zip");
    fs::write(&source, b"approved pack bytes").unwrap();
    let digest = sha256_file(&source).unwrap();
    let layout = VaultLayout::new(VaultMode::Installed, &root.path).unwrap();
    let store = VaultStore::new(layout);
    store.ensure_layout().unwrap();

    let outside = root.path.join("outside");
    fs::create_dir(&outside).unwrap();
    let package = VaultSegment::new("android-pack").unwrap();
    let version = VaultSegment::new("v1").unwrap();
    let package_link = store.layout().driver_packs().join(package.as_str());
    symlink(&outside, &package_link).unwrap();

    assert!(matches!(
        store.import_pack_file(PackClass::Driver, &source, &package, &version, &digest),
        Err(VaultError::UnsafeLink(_)) | Err(VaultError::Io(_))
    ));
    assert_eq!(fs::read_dir(&outside).unwrap().count(), 0);
}
