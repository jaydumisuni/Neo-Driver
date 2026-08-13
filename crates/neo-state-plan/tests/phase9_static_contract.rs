#[test]
fn phase9_twenty_lane_static_contract() {
    let workspace = include_str!("../../../Cargo.toml");
    let manifest = include_str!("../Cargo.toml");
    let production = concat!(
        include_str!("../src/assessment.rs"),
        include_str!("../src/error.rs"),
        include_str!("../src/model.rs"),
        include_str!("../src/model/catalogue.rs"),
        include_str!("../src/model/definition.rs"),
        include_str!("../src/model/evidence.rs"),
        include_str!("../src/model/value.rs"),
    );
    let proof_cli = include_str!("../../neo-cli/src/state_assess_cli.rs");
    let review = include_str!("../../../docs/PHASE9_20_LANE_REVIEW.md");

    let checks = [
        (
            "workspace membership",
            workspace.contains("\"crates/neo-state-plan\""),
        ),
        ("no Windows dependency", !manifest.to_ascii_lowercase().contains("windows")),
        ("no transaction dependency", !manifest.contains("neo-transaction")),
        ("no process execution", !production.contains("std::process::Command")),
        (
            "typed values",
            production.contains("enum TweakValue")
                && production.contains("U32")
                && production.contains("U64"),
        ),
        (
            "validated opaque target",
            production.contains("struct TweakTarget") && production.contains("canonical_key"),
        ),
        (
            "catalogue Serde validation",
            production.contains("TweakCatalogueWire"),
        ),
        (
            "evidence Serde validation",
            production.contains("TweakEvidenceWire"),
        ),
        ("duplicate ID gate", production.contains("DuplicateId")),
        ("duplicate target gate", production.contains("DuplicateTarget")),
        (
            "duplicate observation gate",
            production.contains("DuplicateObservation"),
        ),
        (
            "high-risk default gate",
            production.contains("HighRiskPreselected"),
        ),
        (
            "Certified default gate",
            production.contains("NonCertifiedPreselected"),
        ),
        (
            "safe recommendation gate",
            production.contains("UnsafeRecommendationPreselected"),
        ),
        ("explicit selection gate", production.contains("EmptySelection")),
        (
            "duplicate selection gate",
            production.contains("DuplicateSelection"),
        ),
        ("unknown selection gate", production.contains("UnknownTweak")),
        ("Rejected selection gate", production.contains("RejectedTweak")),
        (
            "observation hard gates",
            production.contains("MissingObservation")
                && production.contains("UnavailableObservation"),
        ),
        (
            "read-only reporting boundary",
            proof_cli.contains("Machine changes: none")
                && review.to_ascii_lowercase().contains("machine change"),
        ),
    ];

    for (lane, passed) in checks {
        assert!(passed, "Phase 9 static lane failed: {lane}");
    }
}
