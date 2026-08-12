#!/usr/bin/env python3
from pathlib import Path


def replace_once(path: Path, old: str, new: str) -> None:
    text = path.read_text(encoding="utf-8")
    if text.count(old) != 1:
        raise SystemExit(f"anchor mismatch in {path}: {old[:120]!r}")
    path.write_text(text.replace(old, new, 1), encoding="utf-8")


executor = Path("crates/neo-driverstore/src/executor.rs")
replace_once(
    executor,
    '''        let backend = if operational_error.is_none() {
            match self.target_package.as_ref() {
                Some(package) => match host.install_best_match(&package.driver_store_inf) {
                    Ok(result) => Some(result),
                    Err(error) => {
                        operational_error =
                            Some(format!("Windows best-match install failed: {error}"));
                        None
                    }
                },
                None => None,
            }
        } else {
            None
        };
''',
    '''        let mut reboot_required = false;
        if operational_error.is_none() {
            for impact in &self.driver_plan.impacts {
                match host.install_best_match(&impact.instance_id) {
                    Ok(result) => reboot_required |= result.reboot_required,
                    Err(error) => {
                        operational_error = Some(format!(
                            "Windows per-device best-match install failed for {}: {error}",
                            impact.instance_id
                        ));
                        break;
                    }
                }
            }
        }
''',
)
replace_once(
    executor,
    '''            reboot_required: backend.is_some_and(|result| result.reboot_required),
''',
    '''            reboot_required,
''',
)

tests = Path("crates/neo-driverstore/src/tests.rs")
replace_once(
    tests,
    '''    fn install_best_match(
        &self,
        driver_store_inf: &Path,
    ) -> Result<DriverBackendResult, DriverStoreError> {
        let mut state = self.state.borrow_mut();
        let package = state
            .packages
            .values()
            .find(|package| package.driver_store_inf == driver_store_inf)
            .cloned()
            .ok_or_else(|| DriverStoreError::Windows("target package missing".to_string()))?;
        if state.install_changes {
            let compatible = state.compatible.clone();
            let target_problem_code = state.target_problem_code;
            for device in &mut state.inventory.devices {
                if compatible
                    .iter()
                    .any(|id| id.eq_ignore_ascii_case(device.instance_id.as_str()))
                {
                    let mut binding = device.active_driver.clone().unwrap_or_default();
                    binding.published_name = Some(package.published_inf.clone());
                    binding.original_name = Some("fixture.inf".to_string());
                    binding.provider = Some("Neo Fixture Vendor".to_string());
                    binding.version = Some("2.0.0.0".to_string());
                    binding.signer = Some("Neo Fixture Signer".to_string());
                    binding.catalog_file = Some("fixture.cat".to_string());
                    device.active_driver = Some(binding);
                    device.problem_code = target_problem_code;
                }
            }
        }
''',
    '''    fn install_best_match(
        &self,
        instance_id: &str,
    ) -> Result<DriverBackendResult, DriverStoreError> {
        let mut state = self.state.borrow_mut();
        let package = state
            .packages
            .get("oem42.inf")
            .cloned()
            .ok_or_else(|| DriverStoreError::Windows("target package missing".to_string()))?;
        if state.install_changes {
            let target_problem_code = state.target_problem_code;
            let device = state
                .inventory
                .devices
                .iter_mut()
                .find(|device| device.instance_id.as_str().eq_ignore_ascii_case(instance_id))
                .ok_or_else(|| DriverStoreError::Windows(format!("device disappeared: {instance_id}")))?;
            let mut binding = device.active_driver.clone().unwrap_or_default();
            binding.published_name = Some(package.published_inf.clone());
            binding.original_name = Some("fixture.inf".to_string());
            binding.provider = Some("Neo Fixture Vendor".to_string());
            binding.version = Some("2.0.0.0".to_string());
            binding.signer = Some("Neo Fixture Signer".to_string());
            binding.catalog_file = Some("fixture.cat".to_string());
            device.active_driver = Some(binding);
            device.problem_code = target_problem_code;
        }
''',
)

windows = Path("crates/neo-driverstore/src/windows.rs")
replace_once(
    windows,
    '''//! Forward installation always calls `DiInstallDriverW` with flags=0 so Windows
//! retains best-match authority. `DiInstallDevice` is used only to restore an
//! exact captured baseline published INF during rollback. Package removal uses
//! `SetupUninstallOEMInfW` with flags=0; force deletion is intentionally absent.
''',
    '''//! Forward installation stages the approved package, then calls `DiInstallDevice`
//! with `DriverInfoData = NULL` for each already-authorized device so Windows searches
//! the preinstalled Driver Store and selects that device's best match. Rollback is the
//! only path that supplies a specific captured driver node. Package removal uses
//! `SetupUninstallOEMInfW` with flags=0; force installation/deletion is absent.
''',
)
replace_once(
    windows,
    '''    CM_Get_DevNode_Status, DiInstallDevice, DiInstallDriverW, SetupCopyOEMInfW,
''',
    '''    CM_Get_DevNode_Status, DiInstallDevice, SetupCopyOEMInfW,
''',
)
replace_once(
    windows,
    '''    DIGCF_PRESENT, DIINSTALLDEVICE_FLAGS, DIINSTALLDRIVER_FLAGS, DI_ENUMSINGLEINF,
''',
    '''    DIGCF_PRESENT, DIINSTALLDEVICE_FLAGS, DI_ENUMSINGLEINF,
''',
)
replace_once(
    windows,
    '''    fn install_best_match(
        &self,
        driver_store_inf: &Path,
    ) -> Result<DriverBackendResult, DriverStoreError> {
        let wide = wide_path(driver_store_inf)?;
        let mut reboot = windows::core::BOOL(0);
        unsafe {
            DiInstallDriverW(
                None,
                PCWSTR(wide.as_ptr()),
                DIINSTALLDRIVER_FLAGS(0),
                Some(&mut reboot),
            )
        }
        .map_err(|error| win_error("DiInstallDriverW", error))?;
        Ok(DriverBackendResult {
            reboot_required: reboot.as_bool(),
        })
    }
''',
    '''    fn install_best_match(
        &self,
        instance_id: &str,
    ) -> Result<DriverBackendResult, DriverStoreError> {
        let set = present_device_set()?;
        let mut index = 0u32;
        loop {
            let mut data = devinfo_data();
            match unsafe { SetupDiEnumDeviceInfo(set.0, index, &mut data) } {
                Ok(()) => {}
                Err(error) if is_no_more_items(&error) => {
                    return Err(DriverStoreError::Windows(format!(
                        "authorized device disappeared before best-match install: {instance_id}"
                    )));
                }
                Err(error) => return Err(win_error("SetupDiEnumDeviceInfo", error)),
            }
            index += 1;
            if !device_instance_id(set.0, &data)?.eq_ignore_ascii_case(instance_id) {
                continue;
            }
            let mut reboot = windows::core::BOOL(0);
            unsafe {
                DiInstallDevice(
                    None,
                    set.0,
                    &data,
                    None,
                    DIINSTALLDEVICE_FLAGS(0),
                    Some(&mut reboot),
                )
            }
            .map_err(|error| win_error("DiInstallDevice best-match", error))?;
            return Ok(DriverBackendResult {
                reboot_required: reboot.as_bool(),
            });
        }
    }
''',
)
