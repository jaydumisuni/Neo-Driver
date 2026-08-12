#!/usr/bin/env python3
from pathlib import Path


def replace_once(path: Path, old: str, new: str) -> None:
    text = path.read_text(encoding="utf-8")
    if text.count(old) != 1:
        raise SystemExit(f"anchor mismatch in {path}: {old[:100]!r}")
    path.write_text(text.replace(old, new, 1), encoding="utf-8")


def replace_all(path: Path, old: str, new: str, expected: int) -> None:
    text = path.read_text(encoding="utf-8")
    if text.count(old) != expected:
        raise SystemExit(f"expected {expected} anchors in {path}: {old[:100]!r}")
    path.write_text(text.replace(old, new), encoding="utf-8")


plan = Path("crates/neo-driverstore/src/plan.rs")
replace_once(
    plan,
    '''        if binding
            .published_name
            .as_deref()
            .is_none_or(|value| value.trim().is_empty())
        {
            return Err(DriverStoreError::MissingBaselinePublishedInf(
                device.instance_id.to_string(),
            ));
        }
        impacts.push(DriverInstallImpact {
            instance_id: device.instance_id.to_string(),
            baseline: DriverBindingBaseline {
                binding,
                problem_code: device.problem_code,
            },
        });
''',
    '''        let published = binding
            .published_name
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| {
                DriverStoreError::MissingBaselinePublishedInf(device.instance_id.to_string())
            })?;
        let baseline_package = host
            .resolve_published_package(published)?
            .ok_or_else(|| DriverStoreError::MissingBaselinePackage(device.instance_id.to_string()))?;
        baseline_package.validate()?;
        impacts.push(DriverInstallImpact {
            instance_id: device.instance_id.to_string(),
            baseline: DriverBindingBaseline {
                binding,
                problem_code: device.problem_code,
            },
            baseline_package,
        });
''',
)

executor = Path("crates/neo-driverstore/src/executor.rs")
replace_once(
    executor,
    '''            if current != Some(impact.baseline.clone()) {
                return Err(DriverStoreError::PrestateDrift);
            }
        }
        match &self.driver_plan.store_baseline {
''',
    '''            if current != Some(impact.baseline.clone()) {
                return Err(DriverStoreError::PrestateDrift);
            }
            if host
                .resolve_published_package(&impact.baseline_package.published_inf)?
                .as_ref()
                != Some(&impact.baseline_package)
            {
                return Err(DriverStoreError::PrestateDrift);
            }
        }
        match &self.driver_plan.store_baseline {
''',
)
replace_once(
    executor,
    '''            let baseline_inf = impact
                .baseline
                .binding
                .published_name
                .as_deref()
                .ok_or_else(|| {
                    DriverStoreError::MissingBaselinePublishedInf(impact.instance_id.clone())
                })?;
            match host.restore_specific_driver(&impact.instance_id, baseline_inf) {
''',
    '''            let baseline_inf = impact.baseline_package.published_inf.as_str();
            match host.restore_specific_driver(&impact.instance_id, baseline_inf) {
''',
)

tests = Path("crates/neo-driverstore/src/tests.rs")
replace_once(
    tests,
    '''        let staged_inf = store_dir.join("oem42.inf");
        Self {
            state: RefCell::new(FakeState {
''',
    '''        let staged_inf = store_dir.join("oem42.inf");
        let baseline_inf = store_dir.join("oem1.inf");
        fs::write(&baseline_inf, b"baseline driver inf bytes\n").unwrap();
        let baseline_package = StoredDriverPackage {
            published_inf: "oem1.inf".to_string(),
            driver_store_inf: baseline_inf,
        };
        Self {
            state: RefCell::new(FakeState {
''',
)
replace_once(
    tests,
    '''                packages: BTreeMap::new(),
''',
    '''                packages: BTreeMap::from([("oem1.inf".to_string(), baseline_package)]),
''',
)
replace_once(
    tests,
    '''        Ok(self.state.borrow().packages.values().next().cloned())
''',
    '''        Ok(self
            .state
            .borrow()
            .packages
            .get("oem42.inf")
            .cloned())
''',
)
replace_all(
    tests,
    '''    assert!(fixture.host.state.borrow().packages.is_empty());
''',
    '''    let packages = &fixture.host.state.borrow().packages;
    assert!(packages.contains_key("oem1.inf"));
    assert!(!packages.contains_key("oem42.inf"));
''',
    4,
)
replace_once(
    tests,
    '''    assert!(!fixture.host.state.borrow().packages.is_empty());
''',
    '''    assert!(fixture.host.state.borrow().packages.contains_key("oem42.inf"));
''',
)
marker = '''#[test]
fn source_byte_drift_blocks_before_staging() {
'''
text = tests.read_text(encoding="utf-8")
if text.count(marker) != 1:
    raise SystemExit("baseline package regression insertion anchor mismatch")
added = '''#[test]
fn planner_refuses_missing_baseline_driver_package() {
    let fixture = Fixture::new(None);
    fixture.host.configure(|state| state.packages.clear());
    let error = prepare_driver_install(
        &fixture.host,
        &fixture_catalogue(),
        &DriverInstallRequest {
            package_root: fixture.root.clone(),
            package_id: "neo.fixture.driver".to_string(),
            inf_path: "drivers/fixture.inf".to_string(),
            architecture: "x64".to_string(),
            windows_build: 26100,
            action_id: "install.fixture.driver".to_string(),
            mission_id: "mission.fixture".to_string(),
        },
    )
    .unwrap_err();
    assert!(matches!(error, DriverStoreError::MissingBaselinePackage(_)));
}

'''
tests.write_text(text.replace(marker, added + marker, 1), encoding="utf-8")
