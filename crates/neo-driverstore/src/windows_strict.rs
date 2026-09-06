//! Fail-closed public Windows driver host.
//!
//! The established Phase 5 backend remains the mutation implementation. This wrapper owns the
//! public read-only inventory boundary so SetupAPI sizing/absence failures cannot be collapsed
//! into fabricated empty evidence before Phase 5 or Phase 22 consumes the inventory.

use neo_device::{DeviceRecord, DriverBinding, OpaqueDeviceId, OrderedDeviceIds};
use std::path::Path;
use windows::core::{Error as WinError, HRESULT};
use windows::Win32::Devices::DeviceAndDriverInstallation::{
    CM_Get_DevNode_Status, SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInfo,
    SetupDiGetClassDevsW, SetupDiGetDeviceInstanceIdW, SetupDiGetDevicePropertyW,
    SetupDiGetDeviceRegistryPropertyW, CM_DEVNODE_STATUS_FLAGS, CM_PROB, CONFIGRET, CR_SUCCESS,
    DIGCF_ALLCLASSES, DIGCF_PRESENT, HDEVINFO, SPDRP_CLASS, SPDRP_CLASSGUID, SPDRP_COMPATIBLEIDS,
    SPDRP_DEVICEDESC, SPDRP_HARDWAREID, SPDRP_LOWERFILTERS, SPDRP_MFG, SPDRP_UPPERFILTERS,
    SP_DEVINFO_DATA,
};
use windows::Win32::Devices::Properties::{DEVPKEY_Device_DriverInfPath, DEVPROPTYPE};
use windows::Win32::Foundation::{
    ERROR_INSUFFICIENT_BUFFER, ERROR_INVALID_DATA, ERROR_NOT_FOUND, ERROR_NO_MORE_ITEMS,
};

use crate::{
    DriverBackendResult, DriverHost, DriverInventory, DriverStoreError, StoredDriverPackage,
    VerifiedInfSignature,
};

#[derive(Debug, Default, Clone, Copy)]
pub struct WindowsDriverHost;

fn mutation_host() -> crate::windows::WindowsDriverHost {
    crate::windows::WindowsDriverHost
}

impl DriverHost for WindowsDriverHost {
    fn windows_build(&self) -> Result<u32, DriverStoreError> {
        mutation_host().windows_build()
    }

    fn inventory(&self) -> Result<DriverInventory, DriverStoreError> {
        strict_inventory()
    }

    fn compatible_present_devices(&self, inf: &Path) -> Result<Vec<String>, DriverStoreError> {
        mutation_host().compatible_present_devices(inf)
    }

    fn verify_inf_signature(&self, inf: &Path) -> Result<VerifiedInfSignature, DriverStoreError> {
        mutation_host().verify_inf_signature(inf)
    }

    fn find_equivalent_package(
        &self,
        source_inf: &Path,
        catalogue_files: &[String],
    ) -> Result<Option<StoredDriverPackage>, DriverStoreError> {
        mutation_host().find_equivalent_package(source_inf, catalogue_files)
    }

    fn resolve_published_package(
        &self,
        published_inf: &str,
    ) -> Result<Option<StoredDriverPackage>, DriverStoreError> {
        mutation_host().resolve_published_package(published_inf)
    }

    fn stage_driver(&self, source_inf: &Path) -> Result<StoredDriverPackage, DriverStoreError> {
        mutation_host().stage_driver(source_inf)
    }

    fn install_best_match(
        &self,
        instance_id: &str,
    ) -> Result<DriverBackendResult, DriverStoreError> {
        mutation_host().install_best_match(instance_id)
    }

    fn restore_specific_driver(
        &self,
        instance_id: &str,
        published_inf: &str,
    ) -> Result<DriverBackendResult, DriverStoreError> {
        mutation_host().restore_specific_driver(instance_id, published_inf)
    }

    fn remove_published_package(&self, published_inf: &str) -> Result<(), DriverStoreError> {
        mutation_host().remove_published_package(published_inf)
    }
}

struct DeviceSet(HDEVINFO);

impl Drop for DeviceSet {
    fn drop(&mut self) {
        unsafe {
            let _ = SetupDiDestroyDeviceInfoList(self.0);
        }
    }
}

fn strict_inventory() -> Result<DriverInventory, DriverStoreError> {
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
        let published_name = device_property_string(set.0, &data, &DEVPKEY_Device_DriverInfPath)?;
        let problem_code = problem_code(&data)?;
        let upper_filters = registry_multisz(set.0, &data, SPDRP_UPPERFILTERS)?;
        let lower_filters = registry_multisz(set.0, &data, SPDRP_LOWERFILTERS)?;

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
            upper_filters,
            lower_filters,
        });
    }

    let inventory = DriverInventory { devices };
    inventory.validate()?;
    Ok(inventory)
}

fn present_device_set() -> Result<DeviceSet, DriverStoreError> {
    let set = unsafe {
        SetupDiGetClassDevsW(
            None,
            windows::core::PCWSTR::null(),
            None,
            DIGCF_PRESENT | DIGCF_ALLCLASSES,
        )
    }
    .map_err(|error| win_error("SetupDiGetClassDevsW", error))?;
    Ok(DeviceSet(set))
}

fn device_instance_id(set: HDEVINFO, data: &SP_DEVINFO_DATA) -> Result<String, DriverStoreError> {
    let mut required = 0u32;
    match unsafe { SetupDiGetDeviceInstanceIdW(set, data, None, Some(&mut required)) } {
        Ok(()) if required > 0 => {}
        Ok(()) => {
            return Err(DriverStoreError::Windows(
                "SetupDiGetDeviceInstanceIdW succeeded without reporting a size".to_string(),
            ))
        }
        Err(error) if is_insufficient_registry_buffer(&error) && required > 0 => {}
        Err(error) => return Err(win_error("SetupDiGetDeviceInstanceIdW sizing", error)),
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
    let values = registry_property_wide(set, data, property)?
        .map(|values| utf16_multisz(&values))
        .unwrap_or_default();
    Ok(stable_unique(values))
}

fn registry_property_wide(
    set: HDEVINFO,
    data: &SP_DEVINFO_DATA,
    property: windows::Win32::Devices::DeviceAndDriverInstallation::SETUP_DI_REGISTRY_PROPERTY,
) -> Result<Option<Vec<u16>>, DriverStoreError> {
    let mut required = 0u32;
    let sizing = unsafe {
        SetupDiGetDeviceRegistryPropertyW(set, data, property, None, None, Some(&mut required))
    };

    match sizing {
        Ok(()) if required == 0 => return Ok(Some(Vec::new())),
        Ok(()) => {}
        Err(error) if is_missing_registry_property(&error) => return Ok(None),
        Err(error) if is_insufficient_registry_buffer(&error) && required > 0 => {}
        Err(error) => return Err(win_error("SetupDiGetDeviceRegistryPropertyW sizing", error)),
    }

    if required == 0 {
        return Err(DriverStoreError::Windows(
            "SetupDiGetDeviceRegistryPropertyW reported an insufficient buffer without a size"
                .to_string(),
        ));
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
    bytes_to_u16(&bytes).map(Some)
}

fn device_property_string(
    set: HDEVINFO,
    data: &SP_DEVINFO_DATA,
    property: &windows::Win32::Foundation::DEVPROPKEY,
) -> Result<Option<String>, DriverStoreError> {
    let mut property_type = DEVPROPTYPE(0);
    let mut required = 0u32;
    let sizing = unsafe {
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

    match sizing {
        Ok(()) if required == 0 => return Ok(None),
        Ok(()) => {}
        Err(error) if is_missing_device_property(&error) => return Ok(None),
        Err(error) if is_insufficient_registry_buffer(&error) && required > 0 => {}
        Err(error) => return Err(win_error("SetupDiGetDevicePropertyW sizing", error)),
    }

    if required == 0 {
        return Err(DriverStoreError::Windows(
            "SetupDiGetDevicePropertyW reported an insufficient buffer without a size".to_string(),
        ));
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
    Ok(nonempty(utf16_array(&bytes_to_u16(&bytes)?)))
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

fn stable_unique(values: Vec<String>) -> Vec<String> {
    let mut unique = Vec::with_capacity(values.len());
    for value in values {
        if !unique.contains(&value) {
            unique.push(value);
        }
    }
    unique
}

fn bytes_to_u16(bytes: &[u8]) -> Result<Vec<u16>, DriverStoreError> {
    if bytes.len() % 2 != 0 {
        return Err(DriverStoreError::Windows(
            "SetupAPI returned an odd byte count for UTF-16 evidence".to_string(),
        ));
    }
    Ok(bytes
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect())
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

fn nonempty(value: String) -> Option<String> {
    (!value.trim().is_empty()).then_some(value)
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

fn is_no_more_items(error: &WinError) -> bool {
    error.code() == HRESULT::from_win32(ERROR_NO_MORE_ITEMS.0)
}

fn is_missing_registry_property(error: &WinError) -> bool {
    error.code() == HRESULT::from_win32(ERROR_INVALID_DATA.0)
}

fn is_missing_device_property(error: &WinError) -> bool {
    error.code() == HRESULT::from_win32(ERROR_NOT_FOUND.0)
}

fn is_insufficient_registry_buffer(error: &WinError) -> bool {
    error.code() == HRESULT::from_win32(ERROR_INSUFFICIENT_BUFFER.0)
}

fn win_error(context: &str, error: WinError) -> DriverStoreError {
    DriverStoreError::Windows(format!("{context}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_manager_problem_decode_remains_fail_closed() {
        assert_eq!(decode_problem_code(CR_SUCCESS, CM_PROB(0)).unwrap(), None);
        assert_eq!(
            decode_problem_code(CR_SUCCESS, CM_PROB(28)).unwrap(),
            Some(28)
        );
        assert!(decode_problem_code(CONFIGRET(13), CM_PROB(0)).is_err());
    }

    #[test]
    fn utf16_evidence_rejects_odd_byte_count() {
        assert!(bytes_to_u16(&[0x41]).is_err());
        assert_eq!(bytes_to_u16(&[0x41, 0x00]).unwrap(), vec![0x0041]);
    }
}
