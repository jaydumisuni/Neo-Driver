use crate::engine::TweakHost;
use crate::model::{RegistrySnapshot, RegistryTweakSpec};
use crate::TweakExecutionError;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{
    CloseHandle, ERROR_FILE_NOT_FOUND, ERROR_SUCCESS, HANDLE, WAIT_ABANDONED, WAIT_OBJECT_0,
    WAIT_TIMEOUT,
};
use windows::Win32::System::Registry::{
    RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW, HKEY,
    HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_DWORD, REG_VALUE_TYPE,
};
use windows::Win32::System::Threading::{CreateMutexW, ReleaseMutex, WaitForSingleObject};

const TWEAK_MUTEX_NAME: &str = "Local\\THETECHGUY.NeoDriver.TweakExecutor.v1";
const TWEAK_MUTEX_TIMEOUT_MS: u32 = 300_000;

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

pub(crate) struct TweakExecutionMutex {
    handle: HANDLE,
    acquired: bool,
}

impl TweakExecutionMutex {
    pub(crate) fn acquire() -> Result<Self, TweakExecutionError> {
        let name = wide(TWEAK_MUTEX_NAME);
        let handle =
            unsafe { CreateMutexW(None, false, PCWSTR(name.as_ptr())) }.map_err(|error| {
                TweakExecutionError::Registry(format!("mutex creation failed: {error}"))
            })?;
        let wait = unsafe { WaitForSingleObject(handle, TWEAK_MUTEX_TIMEOUT_MS) };
        if wait == WAIT_OBJECT_0 || wait == WAIT_ABANDONED {
            return Ok(Self {
                handle,
                acquired: true,
            });
        }
        unsafe {
            let _ = CloseHandle(handle);
        }
        if wait == WAIT_TIMEOUT {
            return Err(TweakExecutionError::Registry(format!(
                "tweak executor mutex wait timed out after {TWEAK_MUTEX_TIMEOUT_MS} ms"
            )));
        }
        Err(TweakExecutionError::Registry(format!(
            "tweak executor mutex wait failed with status {wait:?}"
        )))
    }
}

impl Drop for TweakExecutionMutex {
    fn drop(&mut self) {
        unsafe {
            if self.acquired {
                let _ = ReleaseMutex(self.handle);
            }
            let _ = CloseHandle(self.handle);
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

#[cfg(test)]
mod tests {
    use super::TweakExecutionMutex;

    #[test]
    fn mutex_acquires_without_registry_mutation() {
        let _lock = TweakExecutionMutex::acquire().expect("Phase 11 mutex should be acquirable");
    }
}
