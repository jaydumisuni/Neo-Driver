from pathlib import Path

root = Path(r"D:\projects\neo-host-setup\neo-phase19-20260816")

def replace_once(path: Path, old: str, new: str, label: str):
    text = path.read_text(encoding="utf-8")
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected 1 anchor, found {count}")
    path.write_text(text.replace(old, new), encoding="utf-8")

layout = root / "crates" / "neo-vault" / "src" / "layout.rs"
replace_once(
    layout,
    'pub const MANAGED_DIRECTORY_NAME: &str = "NeoData";\npub const STAGING_MARKER_NAME: &str = ".neo-owned-staging.json";',
    'pub const MANAGED_DIRECTORY_NAME: &str = "NeoData";\npub const HISTORY_DIRECTORY_NAME: &str = "history";\npub const STAGING_MARKER_NAME: &str = ".neo-owned-staging.json";',
    "layout constant",
)
replace_once(layout, 'history: managed_root.join("history"),', 'history: managed_root.join(HISTORY_DIRECTORY_NAME),', "layout history usage")

vault_lib = root / "crates" / "neo-vault" / "src" / "lib.rs"
replace_once(
    vault_lib,
    'pub use layout::{VaultLayout, VaultMode, MANAGED_DIRECTORY_NAME, STAGING_MARKER_NAME};',
    '''pub use layout::{
    VaultLayout, VaultMode, HISTORY_DIRECTORY_NAME, MANAGED_DIRECTORY_NAME, STAGING_MARKER_NAME,
};''',
    "vault re-export",
)

store_path = root / "crates" / "neo-debloat-history-store" / "src" / "store.rs"
replace_once(store_path, 'use std::io::Write;', 'use std::io::{Read, Write};', "store Read import")
replace_once(
    store_path,
    'let history = open_or_create_child_dir(&managed, "history", &history_display)?;',
    '''let history = open_or_create_child_dir(
            &managed,
            neo_vault::HISTORY_DIRECTORY_NAME,
            &history_display,
        )?;''',
    "store create history constant",
)
replace_once(
    store_path,
    'let Some(history) = open_optional_child_dir(&managed, "history", &history_display)? else {',
    '''let Some(history) = open_optional_child_dir(
            &managed,
            neo_vault::HISTORY_DIRECTORY_NAME,
            &history_display,
        )?
        else {''',
    "store existing history constant",
)
replace_once(
    store_path,
    '''        if let Err(error) = write_result {
            drop(staging_dir);
            cleanup_owned_staging(staging, &staging_name, &staging_display, &record_id)?;
            return Err(error);
        }''',
    '''        if let Err(error) = write_result {
            drop(staging_dir);
            // Preserve the primary write/validation failure. Any cleanup residue remains inert
            // staging and is surfaced by the existing audit boundary.
            let _ = cleanup_owned_staging(staging, &staging_name, &staging_display, &record_id);
            return Err(error);
        }''',
    "write failure cleanup semantics",
)
replace_once(
    store_path,
    '''            Ok(()) => {
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
            }''',
    '''            Ok(()) => {
                let stored =
                    load_record_from_root(&handles.records, &self.records_root(), &record_id)?;
                if stored.receipt() != receipt {
                    return Err(DebloatHistoryStoreError::RecordConflict(
                        record_id.to_string(),
                    ));
                }
                // The final record has already been promoted and revalidated. Cleanup is
                // best-effort so inert staging residue cannot turn a successful publication
                // into a false failure.
                let _ = cleanup_owned_staging(staging, &staging_name, &staging_display, &record_id);
                Ok(HistoryRecordWriteReceipt {
                    record_id: record_id.clone(),
                    disposition: HistoryRecordDisposition::Recorded,
                    path: self.records_root().join(record_id.as_str()),
                })
            }''',
    "promotion success cleanup semantics",
)
replace_once(
    store_path,
    '''            Err(rename_error) => {
                let existing =
                    try_load_record_from_root(&handles.records, &self.records_root(), &record_id);
                cleanup_owned_staging(staging, &staging_name, &staging_display, &record_id)?;
                match existing? {
                    Some(stored) => existing_write_receipt(self, &record_id, receipt, stored),
                    None => Err(DebloatHistoryStoreError::Io(rename_error)),
                }
            }''',
    '''            Err(rename_error) => {
                let existing =
                    try_load_record_from_root(&handles.records, &self.records_root(), &record_id);
                // Preserve the promotion/convergence result. Any owned residue is inert and
                // remains visible to audit rather than replacing the primary outcome.
                let _ = cleanup_owned_staging(staging, &staging_name, &staging_display, &record_id);
                match existing? {
                    Some(stored) => existing_write_receipt(self, &record_id, receipt, stored),
                    None => Err(DebloatHistoryStoreError::Io(rename_error)),
                }
            }''',
    "rename cleanup semantics",
)
replace_once(
    store_path,
    '''    let envelope: StoredReceiptEnvelope = serde_json::from_reader(file)?;
    envelope.validate(record_id)''',
    '''    let mut bounded = file.take(MAX_HISTORY_RECORD_BYTES + 1);
    let mut encoded = Vec::with_capacity(metadata.len() as usize);
    bounded.read_to_end(&mut encoded)?;
    if encoded.len() as u64 > MAX_HISTORY_RECORD_BYTES {
        return Err(DebloatHistoryStoreError::RecordTooLarge {
            path: file_display,
            limit: MAX_HISTORY_RECORD_BYTES,
        });
    }
    let envelope: StoredReceiptEnvelope = serde_json::from_slice(&encoded)?;
    envelope.validate(record_id)''',
    "bounded record read",
)

tests_path = root / "crates" / "neo-debloat-history-store" / "src" / "tests.rs"
tests = tests_path.read_text(encoding="utf-8")
if "fn receipt_windows_reparse_substitution_is_rejected()" in tests:
    raise SystemExit("windows reparse test already exists")
windows_test = r'''

#[cfg(windows)]
#[test]
fn receipt_windows_reparse_substitution_is_rejected() {
    use std::io::ErrorKind;
    use std::os::windows::fs::symlink_file;

    let root = TempRoot::new("windows-reparse");
    let store = history_store(&root, VaultMode::Installed);
    let (_receipt, record_id) = record_fixture(&store);
    let final_file = store
        .records_root()
        .join(record_id.as_str())
        .join("receipt.json");
    let outside = root.path().join("outside.json");
    fs::write(&outside, FIXTURE.as_bytes()).unwrap();
    fs::remove_file(&final_file).unwrap();

    if let Err(error) = symlink_file(&outside, &final_file) {
        if error.kind() == ErrorKind::PermissionDenied || error.raw_os_error() == Some(1314) {
            eprintln!(
                "Windows symlink/reparse creation unavailable for this runner; no-follow production code remains covered by static proof"
            );
            return;
        }
        panic!("create Windows receipt substitution link: {error}");
    }

    assert!(matches!(
        store.load(&record_id),
        Err(DebloatHistoryStoreError::UnsafeLink(_))
            | Err(DebloatHistoryStoreError::UnexpectedEntry(_))
    ));
}
'''
tests_path.write_text(tests.rstrip() + windows_test + "\n", encoding="utf-8")

decision = root / "docs" / "decisions" / "0019-PHASE19-DEBLOAT-HISTORY-STORE.md"
replace_once(
    decision,
    '''- the marker-owned session remains independently cleanable after successful promotion or a rename race;
- concurrent identical writers converge on one valid record plus idempotent already-present evidence;''',
    '''- the marker-owned session remains independently cleanable after successful promotion or a rename race;
- staging cleanup is best-effort after the primary write/promotion outcome: cleanup failure never masks an earlier write/validation failure and never converts an already-promoted, revalidated final record into a reported publication failure; inert residue remains visible to `audit()`;
- concurrent identical writers converge on one valid record plus idempotent already-present evidence;''',
    "decision cleanup semantics",
)
replace_once(
    decision,
    '''A crash may leave an inert marker-owned staging session containing no nested record or one validated nested `record/`. Final record enumeration never treats staging as history evidence. The store does not delete an unowned or marker-mismatched staging directory.''',
    '''A crash may leave an inert marker-owned staging session containing no nested record or one validated nested `record/`. Final record enumeration never treats staging as history evidence. The store does not delete an unowned or marker-mismatched staging directory.

`receipt.json` is file-synced before namespace promotion, but Phase 19 does **not** claim platform-independent power-loss durability for parent-directory metadata or the rename itself. The atomic rename is the live namespace/process-crash publication boundary; persistence of directory entries across sudden power loss remains filesystem/platform dependent and requires a stronger platform-specific durability design if that guarantee is introduced later.''',
    "decision durability nonclaim",
)

review = root / "docs" / "PHASE19_20_LANE_REVIEW.md"
replace_once(
    review,
    '''9. **Staged promotion** — a new record is written beneath a marker-owned unique staging session as `record/receipt.json`, synced, re-read/validated, then only the marker-free nested `record/` directory is namespace-promoted into the final record-id directory; the ownership marker stays with the staging session until cleanup.''',
    '''9. **Staged promotion** — a new record is written beneath a marker-owned unique staging session as `record/receipt.json`, file-synced, re-read/validated, then only the marker-free nested `record/` directory is namespace-promoted into the final record-id directory; the ownership marker stays with the staging session until best-effort cleanup. Cleanup cannot mask the primary write/promotion outcome, and Phase 19 does not claim platform-independent power-loss durability for directory metadata.''',
    "review lane 9",
)
replace_once(
    review,
    '''12. **Crash/staging isolation** — a crash may leave an inert marker-owned session containing no nested record or one validated nested `record/`; staging is never enumerated or selected as completed history, and unexpected/unowned staging or final-tree entries fail audit closed.''',
    '''12. **Crash/staging isolation** — a crash or best-effort cleanup failure may leave an inert marker-owned session containing no nested record or one validated nested `record/`; staging is never enumerated or selected as completed history, and unexpected/unowned staging or final-tree entries fail audit closed.''',
    "review lane 12",
)

static_path = root / "tools" / "phase19_static_review.py"
static = static_path.read_text(encoding="utf-8")
old_single = '''        'history: managed_root.join("history")' in VAULT_LAYOUT
        and 'pub fn history(&self)' in VAULT_LAYOUT
        and 'self.layout.history().to_path_buf()' in STORE'''
new_single = '''        'pub const HISTORY_DIRECTORY_NAME: &str = "history";' in VAULT_LAYOUT
        and 'history: managed_root.join(HISTORY_DIRECTORY_NAME)' in VAULT_LAYOUT
        and 'pub fn history(&self)' in VAULT_LAYOUT
        and 'self.layout.history().to_path_buf()' in STORE
        and STORE.count('neo_vault::HISTORY_DIRECTORY_NAME') == 2'''
if static.count(old_single) != 1:
    raise SystemExit(f"static single-root anchor count {static.count(old_single)}")
static = static.replace(old_single, new_single)
old_bound = '''        has_all(MODEL, ('DEBLOAT_HISTORY_STORE_SCHEMA_VERSION', 'MAX_HISTORY_RECORD_BYTES', 'StoredReceiptEnvelope'))
        and 'metadata.len() > MAX_HISTORY_RECORD_BYTES' in STORE,'''
new_bound = '''        has_all(MODEL, ('DEBLOAT_HISTORY_STORE_SCHEMA_VERSION', 'MAX_HISTORY_RECORD_BYTES', 'StoredReceiptEnvelope'))
        and 'metadata.len() > MAX_HISTORY_RECORD_BYTES' in STORE
        and 'file.take(MAX_HISTORY_RECORD_BYTES + 1)' in STORE
        and 'bounded.read_to_end(&mut encoded)?' in STORE
        and 'serde_json::from_slice(&encoded)?' in STORE,'''
if static.count(old_bound) != 1:
    raise SystemExit(f"static bounded anchor count {static.count(old_bound)}")
static = static.replace(old_bound, new_bound)
old_crash = '''        has_all(STORE, ('audit_staging', 'validate_staging_marker', 'STAGED_RECORD_DIRECTORY_NAME'))
        and 'owned_incomplete_staging_is_inert_and_not_history' in TESTS
        and 'staging is never enumerated or selected as completed history' in REVIEW,'''
new_crash = '''        has_all(STORE, ('audit_staging', 'validate_staging_marker', 'STAGED_RECORD_DIRECTORY_NAME'))
        and 'owned_incomplete_staging_is_inert_and_not_history' in TESTS
        and STORE.count('let _ = cleanup_owned_staging(') == 3
        and 'does **not** claim platform-independent power-loss durability' in DECISION
        and 'staging is never enumerated or selected as completed history' in REVIEW,'''
if static.count(old_crash) != 1:
    raise SystemExit(f"static crash anchor count {static.count(old_crash)}")
static = static.replace(old_crash, new_crash)
old_names = "                'receipt_symlink_substitution_is_rejected',"
new_names = "                'receipt_symlink_substitution_is_rejected',\n                'receipt_windows_reparse_substitution_is_rejected',"
if static.count(old_names) != 1:
    raise SystemExit(f"static symlink name anchor count {static.count(old_names)}")
static = static.replace(old_names, new_names)
old_tail = '''        )
        and 'same OS principal' in DECISION,'''
new_tail = '''        )
        and '#[cfg(unix)]\\n#[test]\\nfn receipt_symlink_substitution_is_rejected' in TESTS
        and '#[cfg(windows)]\\n#[test]\\nfn receipt_windows_reparse_substitution_is_rejected' in TESTS
        and 'same OS principal' in DECISION,'''
if static.count(old_tail) != 1:
    raise SystemExit(f"static adversarial tail count {static.count(old_tail)}")
static = static.replace(old_tail, new_tail)
static_path.write_text(static, encoding="utf-8")

print("PATCH_APPLIED=PASS")