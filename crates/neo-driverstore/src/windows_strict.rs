//! Fail-closed public Windows driver host.
//!
//! The established Phase 5 backend remains the mutation implementation. This wrapper owns the
//! public inventory boundary so SetupAPI absence, sizing, type, and decoding failures cannot be
//! collapsed into fabricated empty evidence before Phase 5 or Phase 22 consumes the inventory.

use neo_device::{DeviceRecord, DriverBinding, OpaqueDeviceId, OrderedDeviceIds};
use std::path::Path;
use windows::core::{Error as WinError, HRESULT};
use windows::Win32::Devices::DeviceAndDriverInstallation::{
    CM_Get_DevNode_Status, SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInfo,
    SetupDiGetClassDevsW, SetupDiGetDeviceInstanceIdW, SetupDiGetDevicePropertyW,
    CM_DEVNODE_STATUS_FLAGS, CM_PROB, CONFIGRET, CR_SUCCESS, DIGCF_ALLCLASSES, DIGCF_PRESENT,
    DN_HAS_PROBLEM, HDEVINFO, SP_DEVINFO_DATA,
};
use windows::Win32::Devices::Properties::{
    DEVPKEY_Device_Class, DEVPKEY_Device_CompatibleIds, DEVPKEY_Device_DeviceDesc,
    DEVPKEY_Device_DriverInfPath, DEVPKEY_Device_HardwareIds, DEVPKEY_Device_LowerFilters,
    DEVPKEY_Device_Manufacturer, DEVPKEY_Device_UpperFilters, DEVPROPTYPE, DEVPROP_TYPE_STRING,
    DEVPROP_TYPE_STRING_LIST,
};
use windows::Win32::Foundation::{ERROR_INSUFFICIENT_BUFFER, ERROR_NOT_FOUND, ERROR_NO_MORE_ITEMS};

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
        let hardware_ids = device_property_multisz(set.0, &data, &DEVPKEY_Device_HardwareIds)?
            .into_iter()
            .map(opaque_id)
            .collect::<Result<Vec<_>, _>>()?;
        let compatible_ids = device_property_multisz(set.0, &data, &DEVPKEY_Device_CompatibleIds)?
            .into_iter()
            .map(opaque_id)
            .collect::<Result<Vec<_>, _>>()?;
        let published_name = device_property_string(set.0, &data, &DEVPKEY_Device_DriverInfPath)?;
        let problem_code = problem_code(&data)?;
        let upper_filters = device_property_multisz(set.0, &data, &DEVPKEY_Device_UpperFilters)?;
        let lower_filters = device_property_multisz(set.0, &data, &DEVPKEY_Device_LowerFilters)?;

        devices.push(DeviceRecord {
            instance_id: opaque_id(instance_id)?,
            description: device_property_string(set.0, &data, &DEVPKEY_Device_DeviceDesc)?,
            manufacturer: device_property_string(set.0, &data, &DEVPKEY_Device_Manufacturer)?,
            class_name: device_property_string(set.0, &data, &DEVPKEY_Device_Class)?,
            class_guid: class_guid_from_devinfo(&data),
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
        Err(error) if is_insufficient_device_property_buffer(&error) && required > 0 => {}
        Err(error) => return Err(win_error("SetupDiGetDeviceInstanceIdW sizing", error)),
    }

    let mut buffer = vec![0u16; required as usize];
    unsafe { SetupDiGetDeviceInstanceIdW(set, data, Some(&mut buffer), Some(&mut required)) }
        .map_err(|error| win_error("SetupDiGetDeviceInstanceIdW", error))?;
    if required as usize > buffer.len() {
        return Err(DriverStoreError::Windows(
            "SetupDiGetDeviceInstanceIdW returned a size larger than the supplied buffer"
                .to_string(),
        ));
    }
    utf16_array(&buffer)
}

fn device_property_string(
    set: HDEVINFO,
    data: &SP_DEVINFO_DATA,
    property: &windows::Win32::Foundation::DEVPROPKEY,
) -> Result<Option<String>, DriverStoreError> {
    Ok(
        device_property_wide(set, data, property, DEVPROP_TYPE_STRING)?
            .map(|values| utf16_array(&values))
            .transpose()?
            .and_then(nonempty),
    )
}

fn device_property_multisz(
    set: HDEVINFO,
    data: &SP_DEVINFO_DATA,
    property: &windows::Win32::Foundation::DEVPROPKEY,
) -> Result<Vec<String>, DriverStoreError> {
    let values = match device_property_wide(set, data, property, DEVPROP_TYPE_STRING_LIST)? {
        Some(values) => utf16_multisz(&values)?,
        None => Vec::new(),
    };
    Ok(stable_unique(values))
}

fn device_property_wide(
    set: HDEVINFO,
    data: &SP_DEVINFO_DATA,
    property: &windows::Win32::Foundation::DEVPROPKEY,
    expected_property_type: DEVPROPTYPE,
) -> Result<Option<Vec<u16>>, DriverStoreError> {
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
        Ok(()) if required == 0 => {
            ensure_device_property_type(property_type, expected_property_type)?;
            return Ok(Some(Vec::new()));
        }
        Ok(()) => {}
        Err(error) if is_missing_device_property(&error) => return Ok(None),
        Err(error) if is_insufficient_device_property_buffer(&error) && required > 0 => {}
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
    ensure_device_property_type(property_type, expected_property_type)?;
    if required as usize > bytes.len() {
        return Err(DriverStoreError::Windows(
            "SetupDiGetDevicePropertyW returned a size larger than the supplied buffer".to_string(),
        ));
    }
    bytes.truncate(required as usize);
    bytes_to_u16(&bytes).map(Some)
}

fn ensure_device_property_type(
    actual: DEVPROPTYPE,
    expected: DEVPROPTYPE,
) -> Result<(), DriverStoreError> {
    if actual == expected {
        Ok(())
    } else {
        Err(DriverStoreError::Windows(format!(
            "SetupDiGetDevicePropertyW returned device property type {}, expected {}",
            actual.0, expected.0
        )))
    }
}

fn class_guid_from_devinfo(data: &SP_DEVINFO_DATA) -> Option<String> {
    let guid = data.ClassGuid;
    if guid.data1 == 0
        && guid.data2 == 0
        && guid.data3 == 0
        && guid.data4.iter().all(|byte| *byte == 0)
    {
        return None;
    }

    Some(format!(
        "{{{:08X}-{:04X}-{:04X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}}}",
        guid.data1,
        guid.data2,
        guid.data3,
        guid.data4[0],
        guid.data4[1],
        guid.data4[2],
        guid.data4[3],
        guid.data4[4],
        guid.data4[5],
        guid.data4[6],
        guid.data4[7]
    ))
}

fn problem_code(data: &SP_DEVINFO_DATA) -> Result<Option<u32>, DriverStoreError> {
    let mut status = CM_DEVNODE_STATUS_FLAGS(0);
    let mut problem = CM_PROB(0);
    let result = unsafe { CM_Get_DevNode_Status(&mut status, &mut problem, data.DevInst, 0) };
    decode_problem_code(result, status, problem)
}

fn decode_problem_code(
    result: CONFIGRET,
    status: CM_DEVNODE_STATUS_FLAGS,
    problem: CM_PROB,
) -> Result<Option<u32>, DriverStoreError> {
    if result != CR_SUCCESS {
        return Err(DriverStoreError::Windows(format!(
            "CM_Get_DevNode_Status failed: CONFIGRET {}",
            result.0
        )));
    }

    let has_problem = status.0 & DN_HAS_PROBLEM.0 != 0;
    match (has_problem, problem.0) {
        (false, 0) => Ok(None),
        (true, code) if code != 0 => Ok(Some(code)),
        (true, 0) => Err(DriverStoreError::Windows(
            "CM_Get_DevNode_Status set DN_HAS_PROBLEM without a nonzero problem code".to_string(),
        )),
        (false, code) => Err(DriverStoreError::Windows(format!(
            "CM_Get_DevNode_Status returned problem code {code} without DN_HAS_PROBLEM"
        ))),
    }
}

fn stable_unique(values: Vec<String>) -> Vec<String> {
    let mut unique = Vec::with_capacity(values.len());
    for value in values {
        if !unique
            .iter()
            .any(|existing: &String| existing.eq_ignore_ascii_case(&value))
        {
            unique.push(value);
        }
    }
    unique
}

fn bytes_to_u16(bytes: &[u8]) -> Result<Vec<u16>, DriverStoreError> {
    let (pairs, remainder) = bytes.as_chunks::<2>();
    if !remainder.is_empty() {
        return Err(DriverStoreError::Windows(
            "SetupAPI returned an odd byte count for UTF-16 evidence".to_string(),
        ));
    }
    Ok(pairs.iter().map(|pair| u16::from_le_bytes(*pair)).collect())
}

fn utf16_array(value: &[u16]) -> Result<String, DriverStoreError> {
    let Some((&terminator, payload)) = value.split_last() else {
        return Err(DriverStoreError::Windows(
            "SetupAPI returned unterminated UTF-16 evidence".to_string(),
        ));
    };
    if terminator != 0 || payload.contains(&0) {
        return Err(DriverStoreError::Windows(
            "SetupAPI returned non-canonical UTF-16 evidence termination".to_string(),
        ));
    }
    String::from_utf16(payload).map_err(|error| {
        DriverStoreError::Windows(format!(
            "SetupAPI returned invalid UTF-16 evidence: {error}"
        ))
    })
}

fn utf16_multisz(value: &[u16]) -> Result<Vec<String>, DriverStoreError> {
    if value == [0] {
        return Ok(Vec::new());
    }
    if value.len() < 3 || !value.ends_with(&[0, 0]) {
        return Err(DriverStoreError::Windows(
            "SetupAPI returned unterminated UTF-16 string-list evidence value".to_string(),
        ));
    }

    let mut result = Vec::new();
    let mut start = 0usize;
    for (index, code) in value[..value.len() - 1].iter().copied().enumerate() {
        if code != 0 {
            continue;
        }
        if index == start {
            return Err(DriverStoreError::Windows(
                "SetupAPI returned trailing data after a UTF-16 string-list terminator".to_string(),
            ));
        }
        result.push(String::from_utf16(&value[start..index]).map_err(|error| {
            DriverStoreError::Windows(format!(
                "SetupAPI returned invalid UTF-16 string-list evidence: {error}"
            ))
        })?);
        start = index + 1;
    }
    Ok(result)
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

fn is_missing_device_property(error: &WinError) -> bool {
    error.code() == HRESULT::from_win32(ERROR_NOT_FOUND.0)
}

fn is_insufficient_device_property_buffer(error: &WinError) -> bool {
    error.code() == HRESULT::from_win32(ERROR_INSUFFICIENT_BUFFER.0)
}

fn win_error(context: &str, error: WinError) -> DriverStoreError {
    DriverStoreError::Windows(format!("{context}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_manager_problem_decode_requires_status_flag_and_code_consistency() {
        assert_eq!(
            decode_problem_code(CR_SUCCESS, CM_DEVNODE_STATUS_FLAGS(0), CM_PROB(0)).unwrap(),
            None
        );
        assert_eq!(
            decode_problem_code(CR_SUCCESS, DN_HAS_PROBLEM, CM_PROB(28)).unwrap(),
            Some(28)
        );
        assert!(decode_problem_code(CR_SUCCESS, DN_HAS_PROBLEM, CM_PROB(0)).is_err());
        assert!(decode_problem_code(CR_SUCCESS, CM_DEVNODE_STATUS_FLAGS(0), CM_PROB(28)).is_err());
        assert!(
            decode_problem_code(CONFIGRET(13), CM_DEVNODE_STATUS_FLAGS(0), CM_PROB(0)).is_err()
        );
    }

    #[test]
    fn setupapi_id_normalization_removes_case_only_duplicates_without_reordering() {
        let values = vec![
            r"COMPUTER\{A}".to_string(),
            r"computer\{a}".to_string(),
            r"PCI\VEN_1234&DEV_5678".to_string(),
            r"Computer\{A}".to_string(),
        ];
        assert_eq!(
            stable_unique(values),
            vec![
                r"COMPUTER\{A}".to_string(),
                r"PCI\VEN_1234&DEV_5678".to_string(),
            ]
        );
    }

    #[test]
    fn utf16_evidence_rejects_odd_byte_count() {
        assert!(bytes_to_u16(&[0x41]).is_err());
        assert_eq!(bytes_to_u16(&[0x41, 0x00]).unwrap(), vec![0x0041]);
    }

    #[test]
    fn malformed_utf16_evidence_is_rejected() {
        assert!(utf16_array(&[0xD800, 0]).is_err());
        assert!(utf16_multisz(&[0xD800, 0, 0]).is_err());
    }

    #[test]
    fn string_evidence_requires_exact_termination() {
        assert_eq!(utf16_array(&[0x41, 0]).unwrap(), "A");
        assert!(utf16_array(&[0x41]).is_err());
        assert!(utf16_array(&[0x41, 0, 0x42, 0]).is_err());
    }

    #[test]
    fn string_list_evidence_requires_final_list_terminator_and_no_trailing_data() {
        assert!(utf16_multisz(&[0]).unwrap().is_empty());
        assert_eq!(utf16_multisz(&[0x41, 0, 0]).unwrap(), vec!["A"]);
        assert!(utf16_multisz(&[0x41, 0]).is_err());
        assert!(utf16_multisz(&[0x41, 0, 0, 0x42, 0, 0]).is_err());
    }
}
