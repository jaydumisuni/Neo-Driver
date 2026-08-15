#![cfg(target_os = "windows")]

use neo_debloat_plan::scan_windows_exact_appx_inventory;
use std::fs;
use std::path::{Path, PathBuf};

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("fixtures")
        .join("debloat")
}

fn directory_snapshot(path: &Path) -> Vec<(String, Vec<u8>)> {
    let mut entries = fs::read_dir(path)
        .expect("fixture directory must exist")
        .map(|entry| {
            let entry = entry.expect("fixture entry must be readable");
            let name = entry.file_name().to_string_lossy().into_owned();
            let bytes = fs::read(entry.path()).expect("fixture file must be readable");
            (name, bytes)
        })
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| left.0.cmp(&right.0));
    entries
}

#[test]
fn native_exact_appx_identity_scan_is_read_only_to_fixture_state() {
    let fixtures = fixture_dir();
    let before = directory_snapshot(&fixtures);
    let inventory =
        scan_windows_exact_appx_inventory().expect("native exact inventory must execute");
    assert!(!inventory.machine_changes);
    assert!(inventory
        .current_user
        .iter()
        .all(|package| !package.name.trim().is_empty()
            && !package.full_name.trim().is_empty()
            && !package.family_name.trim().is_empty()));
    assert!(inventory
        .provisioned
        .iter()
        .all(|package| !package.name.trim().is_empty()
            && !package.full_name.trim().is_empty()
            && !package.family_name.trim().is_empty()));
    let after = directory_snapshot(&fixtures);
    assert_eq!(
        before, after,
        "native exact AppX inventory changed fixture state"
    );
}
