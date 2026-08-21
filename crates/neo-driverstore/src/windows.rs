//! Windows SetupAPI/NewDev backend for controlled driver installation.
//!
//! Forward installation stages the approved package, then calls `DiInstallDevice`
//! with `DriverInfoData = NULL` for each already-authorized device so Windows searches
//! the preinstalled Driver Store and selects that device's best match. Rollback is the
//! only path that supplies a specific captured driver node. Package removal uses
//! `SetupUninstallOEMInfW` with flags=0; force installation/deletion is absent.

use neo_device::{DeviceRecord, DriverBinding, OpaqueDeviceId, OrderedDeviceIds};
use std::fs;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use windows::core::{Error as WinError, HRESULT, PCWSTR};
use windows::Win32::Devices::DeviceAndDriverInstallation::{
    CM_Get_DevNode_Status, DiInstallDevice, SetupCopyOEMInfW, SetupDiBuildDriverInfoList,
    SetupDiDestroyDeviceInfoList, SetupDiDestroyDriverInfoList, SetupDiEnumDeviceInfo,
    SetupDiEnumDriverInfoW, SetupDiGetClassDevsW, SetupDiGetDeviceInstallParamsW,
    SetupDiGetDeviceInstanceIdW, SetupDiGetDevicePropertyW, SetupDiGetDeviceRegistryPropertyW,
    SetupDiSetDeviceInstallParamsW, SetupGetInfDriverStoreLocationW, SetupGetInfPublishedNameW,
    SetupUninstallOEMInfW, SetupVerifyInfFileW, CM_DEVNODE_STATUS_FLAGS, CM_PROB, CONFIGRET,
    CR_SUCCESS, DIGCF_ALLCLASSES, DIGCF_PRESENT, DIINSTALLDEVICE_FLAGS, DI_ENUMSINGLEINF,
    DI_FLAGSEX_ALLOWEXCLUDEDDRVS, HDEVINFO, SPDIT_COMPATDRIVER, SPDRP_CLASS, SPDRP_CLASSGUID,
    SPDRP_COMPATIBLEIDS, SPDRP_DEVICEDESC, SPDRP_HARDWAREID, SPDRP_MFG, SPOST_PATH, SP_COPY_STYLE,
    SP_DEVINFO_DATA, SP_DEVINSTALL_PARAMS_W, SP_DRVINFO_DATA_V2_W, SP_INF_SIGNER_INFO_V2_W,
};
use windows::Win32::Devices::Properties::{DEVPKEY_Device_DriverInfPath, DEVPROPTYPE};
use windows::Win32::Foundation::ERROR_NO_MORE_ITEMS;
use windows::Win32::System::Registry::{
    RegGetValueW, HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ, RRF_ZEROONFAILURE,
};
use windows::Win32::System::SystemInformation::GetWindowsDirectoryW;

use crate::plan::signature_matches;
use crate::{
    DriverBackendResult, DriverHost, DriverInventory, DriverStoreError, StoredDriverPackage,
    VerifiedInfSignature,
};

#[derive(Debug, Default, Clone, Copy)]
pub struct WindowsDriverHost;

struct DeviceSet(HDEVINFO);

impl Drop for DeviceSet {
    fn drop(&mut self) {
        unsafe {
            let _ = SetupDiDestroyDeviceInfoList(self.0);
        }
    }
}

impl DriverHost for WindowsDriverHost {
    fn windows_build(&self) -> Result<u32, DriverStoreError> {
        windows_build_number()
    }

    fn inventory(&self) -> Result<DriverInventory, DriverStoreError> {
        let set = present_device_set()?;
        let mut devices = Vec::new();
        let mut index = 0u32;
        loop {
            let mut data = devinfo_data();
            match unsafe { SetupDiEnumDeviceInfo(set.0, index, &mut data) } {
                Ok(()) => {}
                Err(error) if is_no_more_items(&error) => break,
                Err(error) => return Err(win_error("SetupDiEnumDeviceInfo", error)),
            }
            index += 1;
            let instance_id = device_instance_id(set.0, &data)?;
            let hardware_ids = registry_multisz(set.0, &data, SPDRP_HARDWAREID)?
                .into_iter()
                .map(opaque_id)
                .collect::<Result<Vec<_>, _>>()?;
            let compatible_ids = registry_multisz(set.0, &data, SPDRP_COMPATIBLEIDS)?
                .into_iter()
                .map(opaque_id)
                .collect::<Result<Vec<_>, _>>()?;
            let published_name =
                device_property_string(set.0, &data, &DEVPKEY_Device_DriverInfPath)?;
            let problem_code = problem_code(&data)?;
            devices.push(DeviceRecord {
                instance_id: opaque_id(instance_id)?,
                description: registry_string(set.0, &data, SPDRP_DEVICEDESC)?,
                manufacturer: registry_string(set.0, &data, SPDRP_MFG)?,
                class_name: registry_string(set.0, &data, SPDRP_CLASS)?,
                class_guid: registry_string(set.0, &data, SPDRP_CLASSGUID)?,
                problem_code,
                disabled: None,
                ids: OrderedDeviceIds {
                    hardware_ids,
                    compatible_ids,
                },
                active_driver: published_name.map(|published_name| DriverBinding {
                    published_name: Some(published_name),
                    ..DriverBinding::default()
                }),
                upper_filters: vec![],
                lower_filters: vec![],
            });
        }
        let inventory = DriverInventory { devices };
        inventory.validate()?;
        Ok(inventory)
    }

    fn compatible_present_devices(&self, inf: &Path) -> Result<Vec<String>, DriverStoreError> {
        let set = present_device_set()?;
        let inf_wide = wide_path(inf)?;
        if inf_wide.len() > 260 {
            return Err(DriverStoreError::UnsafeInfPath);
        }
        let mut supported = Vec::new();
        let mut index = 0u32;
        loop {
            let mut data = devinfo_data();
            match unsafe { SetupDiEnumDeviceInfo(set.0, index, &mut data) } {
                Ok(()) => {}
                Err(error) if is_no_more_items(&error) => break,
                Err(error) => return Err(win_error("SetupDiEnumDeviceInfo", error)),
            }
            index += 1;
            configure_single_inf(set.0, &mut data, &inf_wide)?;
            unsafe { SetupDiBuildDriverInfoList(set.0, Some(&mut data), SPDIT_COMPATDRIVER) }
                .map_err(|error| win_error("SetupDiBuildDriverInfoList", error))?;
            let mut driver = drvinfo_data();
            let has_match = match unsafe {
                SetupDiEnumDriverInfoW(set.0, Some(&data), SPDIT_COMPATDRIVER, 0, &mut driver)
            } {
                Ok(()) => true,
                Err(error) if is_no_more_items(&error) => false,
                Err(error) => {
                    unsafe {
                        let _ =
                            SetupDiDestroyDriverInfoList(set.0, Some(&data), SPDIT_COMPATDRIVER);
                    }
                    return Err(win_error("SetupDiEnumDriverInfoW", error));
                }
            };
            unsafe {
                SetupDiDestroyDriverInfoList(set.0, Some(&data), SPDIT_COMPATDRIVER)
                    .map_err(|error| win_error("SetupDiDestroyDriverInfoList", error))?;
            }
            if has_match {
                supported.push(device_instance_id(set.0, &data)?);
            }
        }
        Ok(supported)
    }

    fn verify_inf_signature(&self, inf: &Path) -> Result<VerifiedInfSignature, DriverStoreError> {
        let wide = wide_path(inf)?;
        let mut signer = SP_INF_SIGNER_INFO_V2_W {
            cbSize: std::mem::size_of::<SP_INF_SIGNER_INFO_V2_W>() as u32,
            ..Default::default()
        };
        let ok = unsafe { SetupVerifyInfFileW(PCWSTR(wide.as_ptr()), None, &mut signer) };
        if !ok.as_bool() {
            return Err(last_error("SetupVerifyInfFileW"));
        }
        let evidence = VerifiedInfSignature {
            catalog_file: utf16_array(&signer.CatalogFile),
            signer: utf16_array(&signer.DigitalSigner),
            signer_version: nonempty(utf16_array(&signer.DigitalSignerVersion)),
        };
        evidence.validate()?;
        Ok(evidence)
    }

    fn find_equivalent_package(
        &self,
        source_inf: &Path,
        catalogue_files: &[String],
    ) -> Result<Option<StoredDriverPackage>, DriverStoreError> {
        let source_bytes = fs::read(source_inf)?;
        let source_signature = self.verify_inf_signature(source_inf)?;
        let source_catalog = source_catalog_path(source_inf, &source_signature.catalog_file)?;
        let source_catalog_bytes = fs::read(&source_catalog)?;
        if !catalogue_files.iter().any(|catalogue| {
            file_name(catalogue).eq_ignore_ascii_case(&file_name(&source_signature.catalog_file))
        }) {
            return Err(DriverStoreError::SignatureMismatch);
        }
        let inf_dir = windows_inf_dir()?;
        for entry in fs::read_dir(inf_dir)? {
            let entry = entry?;
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
                continue;
            };
            if !is_safe_published_name(name) || fs::read(&path)? != source_bytes {
                continue;
            }
            let candidate_signature = match self.verify_inf_signature(&path) {
                Ok(value) => value,
                Err(_) => continue,
            };
            if signature_matches(&candidate_signature, &source_signature) {
                let candidate_catalog = path.with_extension("cat");
                let candidate_catalog_bytes = match fs::read(candidate_catalog) {
                    Ok(bytes) => bytes,
                    Err(_) => continue,
                };
                if candidate_catalog_bytes != source_catalog_bytes {
                    continue;
                }
                if let Some(package) = self.resolve_published_package(name)? {
                    return Ok(Some(package));
                }
            }
        }
        Ok(None)
    }

    fn resolve_published_package(
        &self,
        published_inf: &str,
    ) -> Result<Option<StoredDriverPackage>, DriverStoreError> {
        if !is_safe_published_name(published_inf) {
            return Err(DriverStoreError::InvalidStoredPackage);
        }
        let published_path = windows_inf_dir()?.join(published_inf);
        if !published_path.is_file() {
            return Ok(None);
        }
        let driver_store_inf = driver_store_location(&published_path)?;
        let mapped_published = published_name_for_store_inf(&driver_store_inf)?;
        if !mapped_published.eq_ignore_ascii_case(published_inf) {
            return Err(DriverStoreError::InvalidStoredPackage);
        }
        let package = StoredDriverPackage {
            published_inf: published_inf.to_string(),
            driver_store_inf,
        };
        package.validate()?;
        Ok(Some(package))
    }

    fn stage_driver(&self, source_inf: &Path) -> Result<StoredDriverPackage, DriverStoreError> {
        let source_wide = wide_path(source_inf)?;
        let source_dir = source_inf.parent().ok_or(DriverStoreError::UnsafeInfPath)?;
        let source_dir_wide = wide_path(source_dir)?;
        let mut destination = vec![0u16; 32768];
        let mut required = 0u32;
        unsafe {
            SetupCopyOEMInfW(
                PCWSTR(source_wide.as_ptr()),
                PCWSTR(source_dir_wide.as_ptr()),
                SPOST_PATH,
                SP_COPY_STYLE(0),
                Some(destination.as_mut_slice()),
                Some(&mut required),
                None,
            )
        }
        .map_err(|error| win_error("SetupCopyOEMInfW", error))?;
        let destination = utf16_array(&destination);
        let published_inf = file_name(&destination);
        self.resolve_published_package(&published_inf)?
            .ok_or(DriverStoreError::StagedPackageMismatch)
    }

    fn install_best_match(
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

    fn restore_specific_driver(
        &self,
        instance_id: &str,
        published_inf: &str,
    ) -> Result<DriverBackendResult, DriverStoreError> {
        if !is_safe_published_name(published_inf) {
            return Err(DriverStoreError::InvalidStoredPackage);
        }
        let published_path = windows_inf_dir()?.join(published_inf);
        if !published_path.is_file() {
            return Err(DriverStoreError::MissingBaselinePackage(
                instance_id.to_string(),
            ));
        }
        let inf_wide = wide_path(&published_path)?;
        if inf_wide.len() > 260 {
            return Err(DriverStoreError::UnsafeInfPath);
        }
        let set = present_device_set()?;
        let mut index = 0u32;
        loop {
            let mut data = devinfo_data();
            match unsafe { SetupDiEnumDeviceInfo(set.0, index, &mut data) } {
                Ok(()) => {}
                Err(error) if is_no_more_items(&error) => {
                    return Err(DriverStoreError::RollbackBindingFailure(
                        instance_id.to_string(),
                    ));
                }
                Err(error) => return Err(win_error("SetupDiEnumDeviceInfo", error)),
            }
            index += 1;
            if !device_instance_id(set.0, &data)?.eq_ignore_ascii_case(instance_id) {
                continue;
            }
            configure_single_inf(set.0, &mut data, &inf_wide)?;
            unsafe { SetupDiBuildDriverInfoList(set.0, Some(&mut data), SPDIT_COMPATDRIVER) }
                .map_err(|error| win_error("SetupDiBuildDriverInfoList rollback", error))?;
            let mut driver = drvinfo_data();
            unsafe {
                SetupDiEnumDriverInfoW(set.0, Some(&data), SPDIT_COMPATDRIVER, 0, &mut driver)
            }
            .map_err(|error| win_error("SetupDiEnumDriverInfoW rollback", error))?;
            let mut reboot = windows::core::BOOL(0);
            let result = unsafe {
                DiInstallDevice(
                    None,
                    set.0,
                    &data,
                    Some(&driver),
                    DIINSTALLDEVICE_FLAGS(0),
                    Some(&mut reboot),
                )
            };
            unsafe {
                let _ = SetupDiDestroyDriverInfoList(set.0, Some(&data), SPDIT_COMPATDRIVER);
            }
            result.map_err(|error| win_error("DiInstallDevice rollback", error))?;
            return Ok(DriverBackendResult {
                reboot_required: reboot.as_bool(),
            });
        }
    }

    fn remove_published_package(&self, published_inf: &str) -> Result<(), DriverStoreError> {
        if !is_safe_published_name(published_inf) {
            return Err(DriverStoreError::InvalidStoredPackage);
        }
        let wide = wide_string(published_inf);
        let ok = unsafe { SetupUninstallOEMInfW(PCWSTR(wide.as_ptr()), 0, None) };
        if !ok.as_bool() {
            return Err(last_error("SetupUninstallOEMInfW"));
        }
        Ok(())
    }
}

fn present_device_set() -> Result<DeviceSet, DriverStoreError> {
    let set = unsafe {
        SetupDiGetClassDevsW(None, PCWSTR::null(), None, DIGCF_PRESENT | DIGCF_ALLCLASSES)
    }
    .map_err(|error| win_error("SetupDiGetClassDevsW", error))?;
    Ok(DeviceSet(set))
}

fn configure_single_inf(
    set: HDEVINFO,
    data: &mut SP_DEVINFO_DATA,
    inf_wide: &[u16],
) -> Result<(), DriverStoreError> {
    let mut params = SP_DEVINSTALL_PARAMS_W {
        cbSize: std::mem::size_of::<SP_DEVINSTALL_PARAMS_W>() as u32,
        ..Default::default()
    };
    unsafe { SetupDiGetDeviceInstallParamsW(set, Some(data), &mut params) }
        .map_err(|error| win_error("SetupDiGetDeviceInstallParamsW", error))?;
    params.Flags.0 |= DI_ENUMSINGLEINF.0;
    params.FlagsEx.0 |= DI_FLAGSEX_ALLOWEXCLUDEDDRVS.0;
    params.DriverPath.fill(0);
    params.DriverPath[..inf_wide.len()].copy_from_slice(inf_wide);
    unsafe { SetupDiSetDeviceInstallParamsW(set, Some(data), &params) }
        .map_err(|error| win_error("SetupDiSetDeviceInstallParamsW", error))
}

fn device_instance_id(set: HDEVINFO, data: &SP_DEVINFO_DATA) -> Result<String, DriverStoreError> {
    let mut required = 0u32;
    let _ = unsafe { SetupDiGetDeviceInstanceIdW(set, data, None, Some(&mut required)) };
    if required == 0 {
        return Err(DriverStoreError::Windows(
            "SetupDiGetDeviceInstanceIdW returned no size".to_string(),
        ));
    }
    let mut buffer = vec![0u16; required as usize];
    unsafe { SetupDiGetDeviceInstanceIdW(set, data, Some(&mut buffer), Some(&mut required)) }
        .map_err(|error| win_error("SetupDiGetDeviceInstanceIdW", error))?;
    Ok(utf16_array(&buffer))
}

fn registry_string(
    set: HDEVINFO,
    data: &SP_DEVINFO_DATA,
    property: windows::Win32::Devices::DeviceAndDriverInstallation::SETUP_DI_REGISTRY_PROPERTY,
) -> Result<Option<String>, DriverStoreError> {
    Ok(registry_property_wide(set, data, property)?
        .map(|values| utf16_array(&values))
        .and_then(nonempty))
}

fn registry_multisz(
    set: HDEVINFO,
    data: &SP_DEVINFO_DATA,
    property: windows::Win32::Devices::DeviceAndDriverInstallation::SETUP_DI_REGISTRY_PROPERTY,
) -> Result<Vec<String>, DriverStoreError> {
    Ok(registry_property_wide(set, data, property)?
        .map(|values| utf16_multisz(&values))
        .unwrap_or_default())
}

fn registry_property_wide(
    set: HDEVINFO,
    data: &SP_DEVINFO_DATA,
    property: windows::Win32::Devices::DeviceAndDriverInstallation::SETUP_DI_REGISTRY_PROPERTY,
) -> Result<Option<Vec<u16>>, DriverStoreError> {
    let mut required = 0u32;
    let _ = unsafe {
        SetupDiGetDeviceRegistryPropertyW(set, data, property, None, None, Some(&mut required))
    };
    if required == 0 {
        return Ok(None);
    }
    let mut bytes = vec![0u8; required as usize];
    unsafe {
        SetupDiGetDeviceRegistryPropertyW(
            set,
            data,
            property,
            None,
            Some(&mut bytes),
            Some(&mut required),
        )
    }
    .map_err(|error| win_error("SetupDiGetDeviceRegistryPropertyW", error))?;
    Ok(Some(bytes_to_u16(&bytes)))
}

fn device_property_string(
    set: HDEVINFO,
    data: &SP_DEVINFO_DATA,
    property: &windows::Win32::Foundation::DEVPROPKEY,
) -> Result<Option<String>, DriverStoreError> {
    let mut property_type = DEVPROPTYPE(0);
    let mut required = 0u32;
    let _ = unsafe {
        SetupDiGetDevicePropertyW(
            set,
            data,
            property,
            &mut property_type,
            None,
            Some(&mut required),
            0,
        )
    };
    if required == 0 {
        return Ok(None);
    }
    let mut bytes = vec![0u8; required as usize];
    unsafe {
        SetupDiGetDevicePropertyW(
            set,
            data,
            property,
            &mut property_type,
            Some(&mut bytes),
            Some(&mut required),
            0,
        )
    }
    .map_err(|error| win_error("SetupDiGetDevicePropertyW", error))?;
    Ok(nonempty(utf16_array(&bytes_to_u16(&bytes))))
}

fn problem_code(data: &SP_DEVINFO_DATA) -> Result<Option<u32>, DriverStoreError> {
    let mut status = CM_DEVNODE_STATUS_FLAGS(0);
    let mut problem = CM_PROB(0);
    let result = unsafe { CM_Get_DevNode_Status(&mut status, &mut problem, data.DevInst, 0) };
    decode_problem_code(result, problem)
}

fn decode_problem_code(
    result: CONFIGRET,
    problem: CM_PROB,
) -> Result<Option<u32>, DriverStoreError> {
    if result != CR_SUCCESS {
        return Err(DriverStoreError::Windows(format!(
            "CM_Get_DevNode_Status failed: CONFIGRET {}",
            result.0
        )));
    }
    Ok((problem.0 != 0).then_some(problem.0))
}

fn driver_store_location(published_inf: &Path) -> Result<PathBuf, DriverStoreError> {
    let wide = wide_path(published_inf)?;
    let mut buffer = vec![0u16; 32768];
    unsafe {
        SetupGetInfDriverStoreLocationW(
            PCWSTR(wide.as_ptr()),
            None,
            PCWSTR::null(),
            &mut buffer,
            None,
        )
    }
    .map_err(|error| win_error("SetupGetInfDriverStoreLocationW", error))?;
    Ok(PathBuf::from(utf16_array(&buffer)))
}

fn published_name_for_store_inf(driver_store_inf: &Path) -> Result<String, DriverStoreError> {
    let wide = wide_path(driver_store_inf)?;
    let mut buffer = vec![0u16; 32768];
    unsafe { SetupGetInfPublishedNameW(PCWSTR(wide.as_ptr()), &mut buffer, None) }
        .map_err(|error| win_error("SetupGetInfPublishedNameW", error))?;
    Ok(file_name(&utf16_array(&buffer)))
}

fn source_catalog_path(inf: &Path, catalog_file: &str) -> Result<PathBuf, DriverStoreError> {
    let name = Path::new(catalog_file)
        .file_name()
        .ok_or(DriverStoreError::InvalidSignatureEvidence)?;
    let parent = inf.parent().ok_or(DriverStoreError::UnsafeInfPath)?;
    let catalog = parent.join(name);
    if !catalog.is_file() {
        return Err(DriverStoreError::InvalidSignatureEvidence);
    }
    Ok(catalog)
}

fn windows_inf_dir() -> Result<PathBuf, DriverStoreError> {
    let mut buffer = vec![0u16; 260];
    loop {
        let length = unsafe { GetWindowsDirectoryW(Some(&mut buffer)) } as usize;
        if length == 0 {
            return Err(last_error("GetWindowsDirectoryW"));
        }
        if length < buffer.len() {
            return Ok(PathBuf::from(String::from_utf16_lossy(&buffer[..length])).join("INF"));
        }
        buffer.resize(length + 1, 0);
    }
}

fn windows_build_number() -> Result<u32, DriverStoreError> {
    let subkey = wide_string(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion");
    let value_name = wide_string("CurrentBuildNumber");
    let flags = RRF_RT_REG_SZ | RRF_ZEROONFAILURE;
    let mut bytes = 0u32;
    let status = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            PCWSTR(subkey.as_ptr()),
            PCWSTR(value_name.as_ptr()),
            flags,
            None,
            None,
            Some(&mut bytes),
        )
    };
    if status.0 != 0 || bytes < 2 {
        return Err(DriverStoreError::Windows(format!(
            "RegGetValueW CurrentBuildNumber sizing failed: {}",
            status.0
        )));
    }
    let mut buffer = vec![0u16; (bytes as usize).div_ceil(2)];
    let status = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            PCWSTR(subkey.as_ptr()),
            PCWSTR(value_name.as_ptr()),
            flags,
            None,
            Some(buffer.as_mut_ptr().cast()),
            Some(&mut bytes),
        )
    };
    if status.0 != 0 {
        return Err(DriverStoreError::Windows(format!(
            "RegGetValueW CurrentBuildNumber failed: {}",
            status.0
        )));
    }
    utf16_array(&buffer)
        .trim()
        .parse::<u32>()
        .map_err(|error| DriverStoreError::Windows(format!("invalid CurrentBuildNumber: {error}")))
}

fn wide_path(path: &Path) -> Result<Vec<u16>, DriverStoreError> {
    let mut value = path.as_os_str().encode_wide().collect::<Vec<_>>();
    if value.contains(&0) {
        return Err(DriverStoreError::UnsafeInfPath);
    }
    value.push(0);
    Ok(value)
}

fn wide_string(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn utf16_array(value: &[u16]) -> String {
    let end = value
        .iter()
        .position(|code| *code == 0)
        .unwrap_or(value.len());
    String::from_utf16_lossy(&value[..end])
}

fn utf16_multisz(value: &[u16]) -> Vec<String> {
    let mut result = Vec::new();
    let mut start = 0usize;
    for (index, code) in value.iter().copied().enumerate() {
        if code != 0 {
            continue;
        }
        if index == start {
            break;
        }
        result.push(String::from_utf16_lossy(&value[start..index]));
        start = index + 1;
    }
    result
}

fn bytes_to_u16(bytes: &[u8]) -> Vec<u16> {
    let (pairs, _) = bytes.as_chunks::<2>();
    pairs.iter().map(|pair| u16::from_le_bytes(*pair)).collect()
}

fn opaque_id(value: String) -> Result<OpaqueDeviceId, DriverStoreError> {
    OpaqueDeviceId::new(value).map_err(|error| DriverStoreError::Device(error.to_string()))
}

fn devinfo_data() -> SP_DEVINFO_DATA {
    SP_DEVINFO_DATA {
        cbSize: std::mem::size_of::<SP_DEVINFO_DATA>() as u32,
        ..Default::default()
    }
}

fn drvinfo_data() -> SP_DRVINFO_DATA_V2_W {
    SP_DRVINFO_DATA_V2_W {
        cbSize: std::mem::size_of::<SP_DRVINFO_DATA_V2_W>() as u32,
        ..Default::default()
    }
}

fn is_no_more_items(error: &WinError) -> bool {
    error.code() == HRESULT::from_win32(ERROR_NO_MORE_ITEMS.0)
}

fn win_error(context: &str, error: WinError) -> DriverStoreError {
    DriverStoreError::Windows(format!("{context}: {error}"))
}

fn last_error(context: &str) -> DriverStoreError {
    win_error(context, WinError::from_thread())
}

fn nonempty(value: String) -> Option<String> {
    (!value.trim().is_empty()).then_some(value)
}

fn file_name(value: &str) -> String {
    Path::new(value)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(value)
        .to_string()
}

fn is_safe_published_name(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    if value.contains(['\\', '/']) || !lower.starts_with("oem") || !lower.ends_with(".inf") {
        return false;
    }
    let digits = &lower[3..lower.len() - 4];
    !digits.is_empty() && digits.chars().all(|character| character.is_ascii_digit())
}

#[cfg(test)]
mod windows_tests {
    use super::*;

    #[test]
    fn published_name_requires_numeric_oem_index() {
        assert!(is_safe_published_name("oem0.inf"));
        assert!(is_safe_published_name("OEM42.INF"));
        assert!(!is_safe_published_name("oem.inf"));
        assert!(!is_safe_published_name("oemx.inf"));
        assert!(!is_safe_published_name(r"sub\oem1.inf"));
    }

    #[test]
    fn catalog_equivalence_requires_identical_bytes() {
        let root =
            std::env::temp_dir().join(format!("neo-driverstore-catalog-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let source = root.join("source.cat");
        let candidate = root.join("candidate.cat");
        std::fs::write(&source, b"catalog-a").unwrap();
        std::fs::write(&candidate, b"catalog-b").unwrap();
        assert_ne!(
            std::fs::read(&source).unwrap(),
            std::fs::read(&candidate).unwrap()
        );
        std::fs::write(&candidate, b"catalog-a").unwrap();
        assert_eq!(
            std::fs::read(&source).unwrap(),
            std::fs::read(&candidate).unwrap()
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn config_manager_status_failure_is_not_treated_as_healthy() {
        assert_eq!(decode_problem_code(CR_SUCCESS, CM_PROB(0)).unwrap(), None);
        assert_eq!(
            decode_problem_code(CR_SUCCESS, CM_PROB(10)).unwrap(),
            Some(10)
        );
        let error = decode_problem_code(CONFIGRET(13), CM_PROB(0)).unwrap_err();
        assert!(error.to_string().contains("CONFIGRET 13"));
    }
}
