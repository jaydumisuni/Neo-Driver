#![cfg(target_os = "windows")]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("fixtures")
        .join("debloat")
        .join(name)
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
fn live_windows_inventory_is_read_only_to_fixture_state() {
    let catalogue = fixture_path("catalogue.json");
    let fixture_dir = catalogue.parent().expect("catalogue must have parent");
    let before = directory_snapshot(fixture_dir);

    let output = Command::new(env!("CARGO_BIN_EXE_neo-debloat-live-scan"))
        .arg(&catalogue)
        .output()
        .expect("live debloat inventory binary must execute");

    assert!(
        output.status.success(),
        "live inventory failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("live inventory output must be UTF-8");
    assert!(stdout.contains("Machine changes: none"));
    assert!(stdout.contains("Contoso.Optional"));

    let after = directory_snapshot(fixture_dir);
    assert_eq!(before, after, "live read-only inventory changed fixture state");
}
