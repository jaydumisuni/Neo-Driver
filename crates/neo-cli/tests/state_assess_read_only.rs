use neo_core::{EvidenceVerdict, RebootRequirement, RecommendationState, RiskLevel};
use neo_state_plan::{
    ObservedState, TweakCatalogue, TweakDefinition, TweakEvidence, TweakObservation,
    TweakOperation, TweakTarget, TweakValue,
};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq, Eq)]
struct SnapshotEntry {
    is_dir: bool,
    bytes: Vec<u8>,
    modified: Option<SystemTime>,
}

fn snapshot_tree(root: &Path) -> BTreeMap<PathBuf, SnapshotEntry> {
    fn visit(root: &Path, path: &Path, entries: &mut BTreeMap<PathBuf, SnapshotEntry>) {
        let metadata = fs::symlink_metadata(path).expect("fixture metadata must be readable");
        let relative = path
            .strip_prefix(root)
            .expect("fixture path must stay below root")
            .to_path_buf();
        let is_dir = metadata.is_dir();
        let bytes = if metadata.is_file() {
            fs::read(path).expect("fixture file must be readable")
        } else {
            Vec::new()
        };
        entries.insert(
            relative,
            SnapshotEntry {
                is_dir,
                bytes,
                modified: metadata.modified().ok(),
            },
        );
        if is_dir {
            let mut children = fs::read_dir(path)
                .expect("fixture directory must be readable")
                .map(|entry| entry.expect("fixture directory entry must be readable").path())
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
        .expect("system clock must be after Unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "neo-phase9-read-only-{}-{nonce}",
        std::process::id()
    ))
}

fn fixture_catalogue() -> TweakCatalogue {
    TweakCatalogue::new(vec![TweakDefinition {
        id: "fixture.enabled".to_string(),
        title: "Fixture preference".to_string(),
        category: "fixture".to_string(),
        benefit: "Exercises read-only assessment.".to_string(),
        tradeoff: "Fixture data only.".to_string(),
        risk: RiskLevel::Low,
        recommendation: RecommendationState::Recommended,
        verdict: EvidenceVerdict::Certified,
        selected_by_default: true,
        requires_admin: false,
        reboot: RebootRequirement::None,
        target: TweakTarget {
            key: "fixture.target".to_string(),
        },
        operation: TweakOperation::Set {
            value: TweakValue::U32(1),
        },
        warnings: vec![],
    }])
    .expect("fixture catalogue must be valid")
}

fn fixture_evidence() -> TweakEvidence {
    TweakEvidence::new(vec![TweakObservation {
        target: TweakTarget {
            key: "fixture.target".to_string(),
        },
        state: ObservedState::Present {
            value: TweakValue::U32(0),
        },
        source: "phase9-read-only-fixture".to_string(),
    }])
    .expect("fixture evidence must be valid")
}

#[test]
fn state_assess_subcommands_leave_isolated_fixture_tree_unchanged() {
    let root = unique_root();
    let work = root.join("work");
    fs::create_dir_all(&work).expect("isolated fixture root must be creatable");

    let catalogue_path = root.join("catalogue.json");
    let evidence_path = root.join("evidence.json");
    fs::write(
        &catalogue_path,
        serde_json::to_vec_pretty(&fixture_catalogue()).expect("catalogue must serialize"),
    )
    .expect("catalogue fixture must be writable");
    fs::write(
        &evidence_path,
        serde_json::to_vec_pretty(&fixture_evidence()).expect("evidence must serialize"),
    )
    .expect("evidence fixture must be writable");

    let before = snapshot_tree(&root);
    let binary = env!("CARGO_BIN_EXE_neo-state-assess");

    let validate = Command::new(binary)
        .current_dir(&work)
        .arg("validate")
        .arg(&catalogue_path)
        .arg("--json")
        .output()
        .expect("validate command must start");
    assert!(
        validate.status.success(),
        "validate failed: {}",
        String::from_utf8_lossy(&validate.stderr)
    );

    let assess = Command::new(binary)
        .current_dir(&work)
        .arg("assess")
        .arg("--catalogue")
        .arg(&catalogue_path)
        .arg("--evidence")
        .arg(&evidence_path)
        .arg("--select")
        .arg("fixture.enabled")
        .arg("--json")
        .output()
        .expect("assess command must start");
    assert!(
        assess.status.success(),
        "assess failed: {}",
        String::from_utf8_lossy(&assess.stderr)
    );

    let after = snapshot_tree(&root);
    assert_eq!(before, after, "Phase 9 CLI modified the isolated fixture tree");

    fs::remove_dir_all(root).expect("isolated fixture root must be removable");
}
