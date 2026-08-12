#!/usr/bin/env python3
from pathlib import Path


def replace_once(path: Path, old: str, new: str) -> None:
    text = path.read_text(encoding="utf-8")
    if text.count(old) != 1:
        raise SystemExit(f"anchor mismatch in {path}: {old[:80]!r}")
    path.write_text(text.replace(old, new, 1), encoding="utf-8")


def replace_exact(path: Path, old: str, new: str, count: int) -> None:
    text = path.read_text(encoding="utf-8")
    if text.count(old) != count:
        raise SystemExit(f"expected {count} anchors in {path}: {old[:80]!r}")
    path.write_text(text.replace(old, new), encoding="utf-8")


executor = Path("crates/neo-driverstore/src/executor.rs")
replace_once(
    executor,
    """use neo_transaction::{\n    ApplyOutcome, ApplyRecord, BaselineSnapshot, Observation, ObservedValue, RollbackRecord,\n    TransactionAuthorization, TransactionCheckpoint, TransactionStage,\n};\nuse serde::{Deserialize, Serialize};\nuse std::collections::BTreeSet;\n""",
    """use neo_transaction::{\n    ApplyOutcome, ApplyRecord, Observation, ObservedValue, RollbackRecord,\n    TransactionAuthorization, TransactionCheckpoint, TransactionStage,\n};\nuse serde::{Deserialize, Serialize};\n""",
)
replace_once(
    executor,
    """\n#[allow(dead_code)]\nfn _baseline_is_exact(_baseline: &BaselineSnapshot) {}\n""",
    "\n",
)
replace_exact(
    executor,
    "&[self.driver_plan.expected_signature.catalog_file.clone()]",
    "std::slice::from_ref(&self.driver_plan.expected_signature.catalog_file)",
    2,
)

tests = Path("crates/neo-driverstore/src/tests.rs")
replace_once(
    tests,
    """        if state.install_changes {\n            for device in &mut state.inventory.devices {\n                if state\n                    .compatible\n                    .iter()\n                    .any(|id| id.eq_ignore_ascii_case(device.instance_id.as_str()))\n                {\n""",
    """        if state.install_changes {\n            let compatible = state.compatible.clone();\n            let target_problem_code = state.target_problem_code;\n            for device in &mut state.inventory.devices {\n                if compatible\n                    .iter()\n                    .any(|id| id.eq_ignore_ascii_case(device.instance_id.as_str()))\n                {\n""",
)
replace_once(
    tests,
    """                    device.problem_code = state.target_problem_code;\n""",
    """                    device.problem_code = target_problem_code;\n""",
)
replace_once(
    tests,
    """    fixture.host.configure(|state| {\n        state.compatible.push(\"USB\\\\VID_9999&PID_0001\\\\B\".to_string());\n        state\n            .inventory\n            .devices\n            .push(fixture_device(\"USB\\\\VID_9999&PID_0001\\\\B\", None));\n    });\n""",
    """    fixture.host.configure(|state| {\n        state.compatible.push(\"USB\\\\VID_9999&PID_0001\\\\B\".to_string());\n        let mut incompatible = fixture_device(\"USB\\\\VID_9999&PID_0001\\\\B\", None);\n        incompatible.ids = OrderedDeviceIds {\n            hardware_ids: vec![OpaqueDeviceId::new(\"USB\\\\VID_9999&PID_0001\").unwrap()],\n            compatible_ids: vec![OpaqueDeviceId::new(\"USB\\\\Class_00\").unwrap()],\n        };\n        state.inventory.devices.push(incompatible);\n    });\n""",
)

plan = Path("crates/neo-driverstore/src/plan.rs")
replace_once(
    plan,
    """pub fn prepare_driver_install<H: DriverHost>(\n    host: &H,\n    catalogue: &Catalogue,\n    package_root: impl AsRef<Path>,\n    package_id: &str,\n    inf_path: &str,\n    architecture: &str,\n    windows_build: u32,\n    action_id: &str,\n    mission_id: &str,\n) -> Result<PreparedDriverInstall, DriverStoreError> {\n""",
    """#[derive(Debug, Clone, PartialEq, Eq)]\npub struct DriverInstallRequest {\n    pub package_root: PathBuf,\n    pub package_id: String,\n    pub inf_path: String,\n    pub architecture: String,\n    pub windows_build: u32,\n    pub action_id: String,\n    pub mission_id: String,\n}\n\npub fn prepare_driver_install<H: DriverHost>(\n    host: &H,\n    catalogue: &Catalogue,\n    request: &DriverInstallRequest,\n) -> Result<PreparedDriverInstall, DriverStoreError> {\n    let package_root = request.package_root.as_path();\n    let package_id = request.package_id.as_str();\n    let inf_path = request.inf_path.as_str();\n    let architecture = request.architecture.as_str();\n    let windows_build = request.windows_build;\n    let action_id = request.action_id.as_str();\n    let mission_id = request.mission_id.as_str();\n""",
)
replace_once(
    plan,
    "resolve_source_inf(package_root.as_ref(), inf_path)?",
    "resolve_source_inf(package_root, inf_path)?",
)

lib = Path("crates/neo-driverstore/src/lib.rs")
replace_once(
    lib,
    "pub use plan::prepare_driver_install;\n",
    "pub use plan::{prepare_driver_install, DriverInstallRequest};\n",
)

replace_once(
    tests,
    """        prepare_driver_install(\n            &self.host,\n            &fixture_catalogue(),\n            &self.root,\n            \"neo.fixture.driver\",\n            \"drivers/fixture.inf\",\n            \"x64\",\n            26100,\n            \"install.fixture.driver\",\n            \"mission.fixture\",\n        )\n""",
    """        prepare_driver_install(\n            &self.host,\n            &fixture_catalogue(),\n            &DriverInstallRequest {\n                package_root: self.root.clone(),\n                package_id: \"neo.fixture.driver\".to_string(),\n                inf_path: \"drivers/fixture.inf\".to_string(),\n                architecture: \"x64\".to_string(),\n                windows_build: 26100,\n                action_id: \"install.fixture.driver\".to_string(),\n                mission_id: \"mission.fixture\".to_string(),\n            },\n        )\n""",
)
replace_once(
    tests,
    """    let error = prepare_driver_install(\n        &fixture.host,\n        &fixture_catalogue(),\n        &fixture.root,\n        \"neo.fixture.driver\",\n        \"drivers/fixture.inf\",\n        \"x64\",\n        26100,\n        \"install.fixture.driver\",\n        \"mission.fixture\",\n    )\n""",
    """    let error = prepare_driver_install(\n        &fixture.host,\n        &fixture_catalogue(),\n        &DriverInstallRequest {\n            package_root: fixture.root.clone(),\n            package_id: \"neo.fixture.driver\".to_string(),\n            inf_path: \"drivers/fixture.inf\".to_string(),\n            architecture: \"x64\".to_string(),\n            windows_build: 26100,\n            action_id: \"install.fixture.driver\".to_string(),\n            mission_id: \"mission.fixture\".to_string(),\n        },\n    )\n""",
)
