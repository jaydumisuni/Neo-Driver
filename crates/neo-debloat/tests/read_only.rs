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
            let entry = entry.expect("fixture directory entry must be readable");
            let name = entry.file_name().to_string_lossy().into_owned();
            let bytes = fs::read(entry.path()).expect("fixture file must be readable");
            (name, bytes)
        })
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| left.0.cmp(&right.0));
    entries
}

#[test]
fn assessment_binary_reports_no_changes_and_preserves_fixture_tree() {
    let catalogue = fixture_path("catalogue.json");
    let evidence = fixture_path("evidence.json");
    let fixture_dir = catalogue.parent().expect("catalogue must have parent");
    let before = directory_snapshot(fixture_dir);

    let output = Command::new(env!("CARGO_BIN_EXE_neo-debloat-assess"))
        .arg(&catalogue)
        .arg(&evidence)
        .arg("gaming")
        .arg("appx.contoso.optional,appx.contoso.gaming,appx.contoso.system")
        .output()
        .expect("debloat assessment binary must execute");

    assert!(
        output.status.success(),
        "assessment failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("assessment output must be UTF-8");
    assert!(stdout.contains("Machine changes: none"));
    assert!(stdout.contains("RemovalCandidate"));
    assert!(stdout.contains("BlockedByProfile"));
    assert!(stdout.contains("BlockedProtected"));

    let after = directory_snapshot(fixture_dir);
    assert_eq!(before, after, "read-only assessment changed fixture state");
}
