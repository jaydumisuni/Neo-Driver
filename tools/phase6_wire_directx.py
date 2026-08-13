from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected exactly one anchor, found {count}")
    return text.replace(old, new, 1)


directx_path = ROOT / "crates/neo-directx-legacy/src/lib.rs"
directx = directx_path.read_text(encoding="utf-8")
directx = replace_once(
    directx,
    "        return scan_at(&system_root, architecture, usize::BITS as u8);\n",
    "        scan_at(&system_root, architecture, usize::BITS as u8)\n",
    "clippy needless return",
)
directx_path.write_text(directx, encoding="utf-8")

probe_path = ROOT / "crates/neo-runtime-probe/src/lib.rs"
probe = probe_path.read_text(encoding="utf-8")
probe = replace_once(
    probe,
    "use neo_probe::{\n",
    "use neo_directx_legacy::{\n    scan_current as scan_legacy_directx, LegacyDirectXReport, LegacyDirectXState,\n    WindowsArchitecture as LegacyDirectXArchitecture,\n};\nuse neo_probe::{\n",
    "directx import",
)
probe = replace_once(
    probe,
    "        let canonical_architecture = canonical_architecture(architecture)\n            .ok_or_else(|| RuntimeProbeError::UnsupportedArchitecture(architecture.to_string()))?;\n\n        let vc_x86_native = self.capture(\n",
    "        let canonical_architecture = canonical_architecture(architecture)\n            .ok_or_else(|| RuntimeProbeError::UnsupportedArchitecture(architecture.to_string()))?;\n        let directx_architecture = LegacyDirectXArchitecture::parse(canonical_architecture)\n            .map_err(|_| RuntimeProbeError::UnsupportedArchitecture(architecture.to_string()))?;\n        let legacy_directx = scan_legacy_directx(directx_architecture);\n\n        let vc_x86_native = self.capture(\n",
    "directx scan",
)
probe = replace_once(
    probe,
    "            unknown_observation(\n                RuntimeComponent::DirectXLegacyJune2010,\n                \"neo-runtime-probe:directx-legacy-predicate-pending\",\n                \"Modern DirectX capability and the June 2010 side-by-side legacy package are distinct; Neo does not infer legacy completeness from the OS DirectX version.\",\n            ),\n",
    "            classify_legacy_directx(&legacy_directx),\n",
    "directx observation",
)
probe = replace_once(
    probe,
    "fn classify_registry_runtime(\n",
    "fn classify_legacy_directx(report: &LegacyDirectXReport) -> RuntimeObservation {\n    let state = match report.state {\n        LegacyDirectXState::Installed => RuntimeState::Installed,\n        LegacyDirectXState::Partial => RuntimeState::Partial,\n        LegacyDirectXState::Missing => RuntimeState::Missing,\n        LegacyDirectXState::Unknown => RuntimeState::Unknown,\n    };\n    let mut details = vec![\n        format!(\"expected_files={}\", report.expected_files),\n        format!(\"present_files={}\", report.present_files),\n    ];\n    for architecture in &report.architectures {\n        details.push(format!(\n            \"architecture={:?};present={}/{};missing={}\",\n            architecture.architecture,\n            architecture.present_files,\n            architecture.expected_files,\n            architecture.missing_files.join(\",\")\n        ));\n    }\n    details.extend(report.warnings.iter().cloned());\n    RuntimeObservation {\n        component: RuntimeComponent::DirectXLegacyJune2010,\n        state,\n        detected_version: (state == RuntimeState::Installed).then_some(\"June 2010 legacy framework component set\".to_string()),\n        source: report.source.clone(),\n        details,\n    }\n}\n\nfn classify_registry_runtime(\n",
    "directx classifier",
)
probe = replace_once(
    probe,
    "        let warnings = command_evidence\n",
    "        let mut warnings: Vec<String> = command_evidence\n",
    "warnings mutable",
)
probe = replace_once(
    probe,
    "            .collect();\n\n        Ok(RuntimeProbeReport {\n",
    "            .collect();\n        warnings.extend(legacy_directx.warnings.iter().map(|warning| {\n            format!(\"legacy DirectX probe could not certify completeness: {warning}\")\n        }));\n\n        Ok(RuntimeProbeReport {\n",
    "directx warnings",
)
probe_path.write_text(probe, encoding="utf-8")

review_path = ROOT / "tools/phase6_static_review.py"
review = review_path.read_text(encoding="utf-8")
review = replace_once(
    review,
    'RUNTIME = (ROOT / "crates/neo-runtime/src/lib.rs").read_text(encoding="utf-8")\n',
    'RUNTIME = (ROOT / "crates/neo-runtime/src/lib.rs").read_text(encoding="utf-8")\nDIRECTX = (ROOT / "crates/neo-directx-legacy/src/lib.rs").read_text(encoding="utf-8")\n',
    "directx review source",
)
review = replace_once(
    review,
    '    {"crates/neo-runtime", "crates/neo-runtime-probe"}.issubset(members),\n',
    '    {"crates/neo-runtime", "crates/neo-directx-legacy", "crates/neo-runtime-probe"}.issubset(members),\n',
    "workspace review membership",
)
review = replace_once(
    review,
    '            and {"neo-probe", "neo-runtime"}.issubset(probe_deps),\n',
    '            and {"neo-directx-legacy", "neo-probe", "neo-runtime"}.issubset(probe_deps),\n',
    "probe dependency review",
)
review = replace_once(
    review,
    '            "unproven-legacy-predicates-stay-unknown",\n',
    '            "directx-proven-other-legacy-predicates-stay-unknown",\n',
    "lane16 name",
)
review = replace_once(
    review,
    '                    "directx-legacy-predicate-pending",\n                    "xna-predicate-pending",\n',
    '                    "classify_legacy_directx",\n                    "scan_legacy_directx",\n                    "xna-predicate-pending",\n',
    "lane16 markers",
)
review = replace_once(
    review,
    '            "DirectX legacy/XNA/OpenAL/PhysX predicates remain Unknown until independently proven",\n',
    '            "DirectX June 2010 framework-component completeness is proven by the compiled detector; XNA/OpenAL/PhysX predicates remain Unknown until independently proven",\n',
    "lane16 detail",
)
review = replace_once(
    review,
    '            contains_all(\n                PROBE,\n                [\n                    "classify_legacy_directx",\n',
    '            contains_all(DIRECTX, ["GetWindowsDirectoryW", "expected_component_files", "D3DCompiler_", "D3DX9_", "XAudio2_", "XInput1_"])\n            and \"std::env::var_os(\\\"SystemRoot\\\")\" not in DIRECTX\n            and contains_all(\n                PROBE,\n                [\n                    "classify_legacy_directx",\n',
    "lane16 detector proof",
)
review_path.write_text(review, encoding="utf-8")
