use crate::engine::TweakHost;
use crate::model::{RegistrySnapshot, RegistryTweakSpec};
use crate::TweakExecutionError;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
use windows::Win32::System::Registry::{
    RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW, HKEY,
    HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_DWORD, REG_VALUE_TYPE,
};

pub(crate) struct WindowsRegistryHost;

impl TweakHost for WindowsRegistryHost {
    fn read(&self, spec: RegistryTweakSpec) -> Result<RegistrySnapshot, TweakExecutionError> {
        let key = OpenKey::open(spec, KEY_QUERY_VALUE)?;
        let name = wide(spec.value_name);
        let mut value_type = REG_VALUE_TYPE::default();
        let mut data = [0u8; 4];
        let mut size = data.len() as u32;
        let status = unsafe {
            RegQueryValueExW(
                key.raw,
                PCWSTR(name.as_ptr()),
                None,
                Some(&mut value_type),
                Some(data.as_mut_ptr()),
                Some(&mut size),
            )
        };
        if status == ERROR_FILE_NOT_FOUND {
            return Ok(RegistrySnapshot::Absent);
        }
        if status != ERROR_SUCCESS {
            return Err(TweakExecutionError::Registry(format!(
                "RegQueryValueExW({}) returned {}",
                spec.value_name, status.0
            )));
        }
        if value_type != REG_DWORD || size != 4 {
            return Err(TweakExecutionError::UnsupportedRegistryState(
                spec.id.to_string(),
            ));
        }
        Ok(RegistrySnapshot::Dword(u32::from_le_bytes(data)))
    }

    fn write_dword(
        &mut self,
        spec: RegistryTweakSpec,
        value: u32,
    ) -> Result<(), TweakExecutionError> {
        let key = OpenKey::open(spec, KEY_SET_VALUE)?;
        let name = wide(spec.value_name);
        let bytes = value.to_le_bytes();
        let status = unsafe {
            RegSetValueExW(
                key.raw,
                PCWSTR(name.as_ptr()),
                None,
                REG_DWORD,
                Some(&bytes),
            )
        };
        if status != ERROR_SUCCESS {
            return Err(TweakExecutionError::Registry(format!(
                "RegSetValueExW({}) returned {}",
                spec.value_name, status.0
            )));
        }
        Ok(())
    }

    fn restore(
        &mut self,
        spec: RegistryTweakSpec,
        baseline: RegistrySnapshot,
    ) -> Result<(), TweakExecutionError> {
        match baseline {
            RegistrySnapshot::Dword(value) => self.write_dword(spec, value),
            RegistrySnapshot::Absent => {
                let key = OpenKey::open(spec, KEY_SET_VALUE)?;
                let name = wide(spec.value_name);
                let status = unsafe { RegDeleteValueW(key.raw, PCWSTR(name.as_ptr())) };
                if status != ERROR_SUCCESS && status != ERROR_FILE_NOT_FOUND {
                    return Err(TweakExecutionError::Registry(format!(
                        "RegDeleteValueW({}) returned {}",
                        spec.value_name, status.0
                    )));
                }
                Ok(())
            }
        }
    }
}

struct OpenKey {
    raw: HKEY,
}

impl OpenKey {
    fn open(
        spec: RegistryTweakSpec,
        access: windows::Win32::System::Registry::REG_SAM_FLAGS,
    ) -> Result<Self, TweakExecutionError> {
        let path = wide(spec.subkey);
        let mut raw = HKEY::default();
        let status = unsafe {
            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                PCWSTR(path.as_ptr()),
                None,
                access,
                &mut raw,
            )
        };
        if status != ERROR_SUCCESS {
            return Err(TweakExecutionError::Registry(format!(
                "RegOpenKeyExW({}) returned {}",
                spec.subkey, status.0
            )));
        }
        Ok(Self { raw })
    }
}

impl Drop for OpenKey {
    fn drop(&mut self) {
        unsafe {
            let _ = RegCloseKey(self.raw);
        }
    }
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}
