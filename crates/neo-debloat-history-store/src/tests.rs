use super::*;
use neo_debloat_history::DebloatRemovalReceipt;
use neo_debloat_plan::{ExactAppxInventory, ExactPackageIdentity};
use neo_vault::{VaultLayout, VaultMode};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Barrier};

static NEXT_TEMP: AtomicU64 = AtomicU64::new(1);
const FIXTURE: &str = include_str!("../../../fixtures/debloat/phase19_receipt.json");

struct TempRoot {
    path: PathBuf,
}

impl TempRoot {
    fn new(label: &str) -> Self {
        let id = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "neo-debloat-history-store-{label}-{}-{id}",
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

fn fixture_receipt() -> DebloatRemovalReceipt {
    DebloatRemovalReceipt::from_json_str(FIXTURE).expect("Phase 17-derived fixture must validate")
}

fn history_store(root: &TempRoot, mode: VaultMode) -> DebloatHistoryStore {
    DebloatHistoryStore::new(VaultLayout::new(mode, root.path()).expect("absolute test root"))
}

fn record_fixture(store: &DebloatHistoryStore) -> (DebloatRemovalReceipt, DebloatHistoryRecordId) {
    let receipt = fixture_receipt();
    let record_id = DebloatHistoryRecordId::from_receipt(&receipt).expect("valid record id");
    let written = store
        .record_validated_receipt_for_tests(&receipt)
        .expect("fixture must persist");
    assert_eq!(written.record_id, record_id);
    assert_eq!(written.disposition, HistoryRecordDisposition::Recorded);
    (receipt, record_id)
}

#[test]
fn record_ids_reject_non_fingerprint_and_traversal_like_input() {
    for value in [
        "",
        ".",
        "..",
        "../escape",
        "a/b",
        r"a\b",
        "g000000000000000000000000000000000000000000000000000000000000000",
        "abc",
    ] {
        assert!(
            DebloatHistoryRecordId::new(value).is_err(),
            "accepted invalid record id {value:?}"
        );
    }
    let upper = "A".repeat(64);
    assert_eq!(
        DebloatHistoryRecordId::new(upper).unwrap().as_str(),
        "a".repeat(64)
    );
}

#[test]
fn installed_and_portable_modes_share_canonical_history_child() {
    let root = TempRoot::new("mode-parity");
    let installed = history_store(&root, VaultMode::Installed);
    let portable = history_store(&root, VaultMode::Portable);
    let expected = root.path().join("NeoData").join("history");
    assert_eq!(installed.history_root(), expected);
    assert_eq!(portable.history_root(), expected);
    assert_eq!(installed.records_root(), portable.records_root());
}

#[test]
fn empty_store_listing_is_read_only() {
    let root = TempRoot::new("empty-read");
    let store = history_store(&root, VaultMode::Portable);
    assert_eq!(store.list().unwrap(), Vec::new());
    assert!(
        !store.layout().managed_root().exists(),
        "read-only listing must not create NeoData"
    );
}

#[test]
fn valid_history_promotes_to_one_final_file_and_is_idempotent() {
    let root = TempRoot::new("record");
    let store = history_store(&root, VaultMode::Installed);
    let (receipt, record_id) = record_fixture(&store);

    let final_dir = store.records_root().join(record_id.as_str());
    let mut names: Vec<String> = fs::read_dir(&final_dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().to_string())
        .collect();
    names.sort();
    assert_eq!(names, vec!["receipt.json"]);
    assert_eq!(
        fs::read_dir(store.records_root().join(".staging"))
            .unwrap()
            .count(),
        0,
        "completed promotion must not leave staging noise"
    );

    let loaded = store.load(&record_id).expect("stored receipt must load");
    assert_eq!(loaded.record_id(), &record_id);
    assert_eq!(loaded.receipt(), &receipt);
    let summaries = store.list().expect("stored receipt must enumerate");
    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].record_id, record_id);

    let second = store
        .record_validated_receipt_for_tests(&receipt)
        .expect("identical receipt must be idempotent");
    assert_eq!(second.disposition, HistoryRecordDisposition::AlreadyPresent);
}

#[test]
fn concurrent_identical_writers_converge_on_one_valid_record() {
    let root = TempRoot::new("concurrent");
    let store = history_store(&root, VaultMode::Installed);
    let receipt = fixture_receipt();
    let record_id = DebloatHistoryRecordId::from_receipt(&receipt).expect("valid record id");
    let barrier = Arc::new(Barrier::new(3));

    let dispositions = std::thread::scope(|scope| {
        let left_store = store.clone();
        let left_receipt = receipt.clone();
        let left_barrier = Arc::clone(&barrier);
        let left = scope.spawn(move || {
            left_barrier.wait();
            left_store
                .record_validated_receipt_for_tests(&left_receipt)
                .expect("left writer must converge")
                .disposition
        });

        let right_store = store.clone();
        let right_receipt = receipt.clone();
        let right_barrier = Arc::clone(&barrier);
        let right = scope.spawn(move || {
            right_barrier.wait();
            right_store
                .record_validated_receipt_for_tests(&right_receipt)
                .expect("right writer must converge")
                .disposition
        });

        barrier.wait();
        [left.join().unwrap(), right.join().unwrap()]
    });

    assert_eq!(
        dispositions
            .iter()
            .filter(|value| **value == HistoryRecordDisposition::Recorded)
            .count(),
        1
    );
    assert_eq!(
        dispositions
            .iter()
            .filter(|value| **value == HistoryRecordDisposition::AlreadyPresent)
            .count(),
        1
    );
    assert_eq!(
        fs::read_dir(store.records_root().join(".staging"))
            .unwrap()
            .count(),
        0,
        "concurrent convergence must not leave staging noise"
    );
    assert_eq!(
        store
            .load(&record_id)
            .expect("final record must load")
            .receipt(),
        &receipt
    );
}

#[test]
fn tampered_final_record_fails_closed_and_is_never_repaired_by_recording_again() {
    let root = TempRoot::new("tamper");
    let store = history_store(&root, VaultMode::Installed);
    let (receipt, record_id) = record_fixture(&store);
    let path = store
        .records_root()
        .join(record_id.as_str())
        .join("receipt.json");
    let mut value: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    value["receipt"]["package_id"] = Value::String("Contoso.Tampered".to_string());
    fs::write(&path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
    let tampered = fs::read(&path).unwrap();

    assert!(store.load(&record_id).is_err());
    assert!(store.record_validated_receipt_for_tests(&receipt).is_err());
    assert_eq!(fs::read(&path).unwrap(), tampered);
}

#[test]
fn oversized_and_identity_mismatched_records_fail_before_selection() {
    let root = TempRoot::new("record-validation");
    let store = history_store(&root, VaultMode::Installed);
    let (_receipt, record_id) = record_fixture(&store);
    let path = store
        .records_root()
        .join(record_id.as_str())
        .join("receipt.json");

    fs::write(&path, vec![b'x'; (MAX_HISTORY_RECORD_BYTES + 1) as usize]).unwrap();
    assert!(matches!(
        store.load(&record_id),
        Err(DebloatHistoryStoreError::RecordTooLarge { .. })
    ));

    let mut envelope = serde_json::json!({
        "schema_version": DEBLOAT_HISTORY_STORE_SCHEMA_VERSION,
        "record_id": "a".repeat(64),
        "receipt": serde_json::from_str::<Value>(FIXTURE).unwrap()
    });
    envelope["receipt"]["receipt_fingerprint"] = Value::String(record_id.as_str().to_string());
    fs::write(&path, serde_json::to_vec_pretty(&envelope).unwrap()).unwrap();
    assert!(matches!(
        store.load(&record_id),
        Err(DebloatHistoryStoreError::InvalidRecord(_))
    ));
}

#[test]
fn unexpected_final_or_staging_entries_fail_audit_closed() {
    let root = TempRoot::new("audit");
    let store = history_store(&root, VaultMode::Installed);
    let (_receipt, record_id) = record_fixture(&store);
    let final_dir = store.records_root().join(record_id.as_str());
    fs::write(final_dir.join("foreign.txt"), b"foreign").unwrap();
    assert!(matches!(
        store.audit(),
        Err(DebloatHistoryStoreError::UnexpectedEntry(_))
    ));
    fs::remove_file(final_dir.join("foreign.txt")).unwrap();

    let foreign_staging = store.records_root().join(".staging").join("foreign");
    fs::create_dir(&foreign_staging).unwrap();
    assert!(store.audit().is_err());
}

#[test]
fn trusted_selection_by_id_preserves_phase17_fresh_restore_readiness() {
    let root = TempRoot::new("prepare");
    let store = history_store(&root, VaultMode::Installed);
    let (receipt, record_id) = record_fixture(&store);
    let dependency = receipt.dependencies()[0].clone();
    let dependency_identity = ExactPackageIdentity {
        name: dependency.name,
        full_name: dependency.full_name,
        family_name: dependency.family_name,
        is_framework: true,
        is_resource: false,
        is_bundle: false,
        is_optional: false,
        dependencies: Vec::new(),
    };
    let inventory = ExactAppxInventory::new(
        Vec::new(),
        vec![receipt.main().clone(), dependency_identity],
        "phase19-store-test",
    )
    .expect("valid exact inventory");

    let prepared = store
        .prepare_restore_from_inventory_by_id(&record_id, &inventory, "phase19-prepare")
        .expect("store-selected exact receipt should prepare read-only restore");
    assert_eq!(prepared.receipt_fingerprint(), record_id.as_str());
    assert!(!prepared.machine_changes());
}

#[cfg(unix)]
#[test]
fn receipt_symlink_substitution_is_rejected() {
    use std::os::unix::fs::symlink;

    let root = TempRoot::new("symlink");
    let store = history_store(&root, VaultMode::Installed);
    let (_receipt, record_id) = record_fixture(&store);
    let final_file = store
        .records_root()
        .join(record_id.as_str())
        .join("receipt.json");
    let outside = root.path().join("outside.json");
    fs::write(&outside, FIXTURE.as_bytes()).unwrap();
    fs::remove_file(&final_file).unwrap();
    symlink(&outside, &final_file).unwrap();

    assert!(matches!(
        store.load(&record_id),
        Err(DebloatHistoryStoreError::UnsafeLink(_))
            | Err(DebloatHistoryStoreError::UnexpectedEntry(_))
    ));
}
