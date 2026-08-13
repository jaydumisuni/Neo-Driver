#![cfg(windows)]

use neo_core::{EvidenceVerdict, RebootRequirement, RecommendationState, RiskLevel};
use neo_state_plan::{
    ReaderId, StateBinding, StateBindings, TweakCatalogue, TweakDefinition, TweakOperation,
    TweakTarget, TweakValue,
};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn snapshot_tree(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn visit(root: &Path, path: &Path, entries: &mut BTreeMap<PathBuf, Vec<u8>>) {
        let metadata = fs::symlink_metadata(path).expect("metadata must be readable");
        let relative = path.strip_prefix(root).unwrap().to_path_buf();
        entries.insert(
            relative,
            if metadata.is_file() {
                fs::read(path).unwrap()
            } else {
                Vec::new()
            },
        );
        if metadata.is_dir() {
            let mut children = fs::read_dir(path)
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .collect::<Vec<_>>();
            children.sort();
            for child in children {
                visit(root, &child, entries);
            }
        }
    }
    let mut entries = BTreeMap::new();
    visit(root, root, &mut entries);
    entries
}

fn unique_root() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("neo-phase10-live-{}-{nonce}", std::process::id()))
}

#[test]
fn live_state_assessment_reads_proven_system_evidence_without_mutation() {
    let root = unique_root();
    let work = root.join("work");
    fs::create_dir_all(&work).unwrap();

    let catalogue = TweakCatalogue::new(vec![TweakDefinition {
        id: "fixture.build".to_string(),
        title: "Build evidence".to_string(),
        category: "fixture".to_string(),
        benefit: "Exercises live read-only evidence.".to_string(),
        tradeoff: "Fixture only.".to_string(),
        risk: RiskLevel::Low,
        recommendation: RecommendationState::Manual,
        verdict: EvidenceVerdict::Certified,
        selected_by_default: false,
        requires_admin: false,
        reboot: RebootRequirement::None,
        target: TweakTarget {
            key: "fixture.build".to_string(),
        },
        operation: TweakOperation::Set {
            value: TweakValue::Text("0".to_string()),
        },
        warnings: vec![],
    }])
    .unwrap();
    let bindings = StateBindings::new(vec![StateBinding {
        target: TweakTarget {
            key: "fixture.build".to_string(),
        },
        reader: ReaderId::new("windows.os.current_build").unwrap(),
    }])
    .unwrap();

    let catalogue_path = root.join("catalogue.json");
    let bindings_path = root.join("bindings.json");
    fs::write(
        &catalogue_path,
        serde_json::to_vec_pretty(&catalogue).unwrap(),
    )
    .unwrap();
    fs::write(
        &bindings_path,
        serde_json::to_vec_pretty(&bindings).unwrap(),
    )
    .unwrap();

    let before = snapshot_tree(&root);
    let output = Command::new(env!("CARGO_BIN_EXE_neo-state-assess"))
        .current_dir(&work)
        .arg("live")
        .arg("--catalogue")
        .arg(&catalogue_path)
        .arg("--bindings")
        .arg(&bindings_path)
        .arg("--select")
        .arg("fixture.build")
        .output()
        .expect("live state assessment must start");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("Machine changes: none"));
    assert_eq!(before, snapshot_tree(&root));

    fs::remove_dir_all(root).unwrap();
}
