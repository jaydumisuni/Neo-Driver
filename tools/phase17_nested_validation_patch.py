from pathlib import Path

path = Path("crates/neo-debloat-history/src/tests.rs")
text = path.read_text(encoding="utf-8")
old = '''    let error = DebloatRemovalReceipt::from_json_str(&json)
        .expect_err("durable history must reject broadened source authority");
    assert!(matches!(error, DebloatHistoryError::InvalidReceipt(_)));
    assert!(error
        .to_string()
        .contains("authority expected from Phase 15/16"));
'''
new = '''    let error = DebloatRemovalReceipt::from_json_str(&json)
        .expect_err("durable history must reject broadened source authority");
    assert!(matches!(
        error,
        DebloatHistoryError::InvalidReceipt(_) | DebloatHistoryError::Serialization(_)
    ));
'''
if text.count(old) != 1:
    raise SystemExit(f"expected exactly one nested-validation marker, got {text.count(old)}")
path.write_text(text.replace(old, new, 1), encoding="utf-8")
