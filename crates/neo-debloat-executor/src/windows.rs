use crate::engine::DebloatHost;
use crate::DebloatExecutionError;
use neo_debloat_plan::{scan_windows_exact_appx_inventory, ExactAppxInventory};
use windows::core::{HSTRING, PCWSTR};
use windows::Management::Deployment::{DeploymentOptions, PackageManager};
use windows::Win32::Foundation::{
    CloseHandle, HANDLE, WAIT_ABANDONED, WAIT_OBJECT_0, WAIT_TIMEOUT,
};
use windows::Win32::System::Threading::{CreateMutexW, ReleaseMutex, WaitForSingleObject};
use windows_collections::IIterable;

const DEBLOAT_MUTEX_NAME: &str = "Local\\THETECHGUY.NeoDriver.DebloatExecutor.v1";
const DEBLOAT_MUTEX_TIMEOUT_MS: u32 = 300_000;

pub(crate) struct WindowsDebloatHost;

impl DebloatHost for WindowsDebloatHost {
    fn current_inventory(&self) -> Result<ExactAppxInventory, DebloatExecutionError> {
        scan_windows_exact_appx_inventory()
            .map_err(|error| DebloatExecutionError::Observation(error.to_string()))
    }

    fn remove_current_user(
        &mut self,
        package_full_name: &str,
    ) -> Result<(), DebloatExecutionError> {
        let manager = PackageManager::new().map_err(native_error("create PackageManager"))?;
        let operation = manager
            .RemovePackageAsync(&HSTRING::from(package_full_name))
            .map_err(native_error("start current-user package removal"))?;
        let result = operation
            .join()
            .map_err(native_error("await current-user package removal"))?;
        validate_deployment_result(&result, "current-user package removal")
    }

    fn register_current_user(
        &mut self,
        package_full_name: &str,
        dependency_full_names: &[String],
    ) -> Result<(), DebloatExecutionError> {
        let manager = PackageManager::new().map_err(native_error("create PackageManager"))?;
        let dependencies = IIterable::from(
            dependency_full_names
                .iter()
                .map(HSTRING::from)
                .collect::<Vec<_>>(),
        );
        let operation = manager
            .RegisterPackageByFullNameAsync(
                &HSTRING::from(package_full_name),
                &dependencies,
                DeploymentOptions::None,
            )
            .map_err(native_error("start staged full-name package registration"))?;
        let result = operation
            .join()
            .map_err(native_error("await staged full-name package registration"))?;
        validate_deployment_result(&result, "staged full-name package registration")
    }
}

fn validate_deployment_result(
    result: &windows::Management::Deployment::DeploymentResult,
    operation: &str,
) -> Result<(), DebloatExecutionError> {
    let extended = result
        .ExtendedErrorCode()
        .map_err(native_error("read deployment extended error"))?;
    if extended.is_err() {
        let text = result
            .ErrorText()
            .map(|value| value.to_string_lossy())
            .unwrap_or_else(|_| "deployment returned no error text".to_string());
        return Err(DebloatExecutionError::NativeDeployment(format!(
            "{operation} returned {extended:?}: {text}"
        )));
    }
    Ok(())
}

fn native_error(
    operation: &'static str,
) -> impl FnOnce(windows::core::Error) -> DebloatExecutionError {
    move |error| DebloatExecutionError::NativeDeployment(format!("{operation}: {error}"))
}

pub(crate) struct DebloatExecutionMutex {
    handle: HANDLE,
    acquired: bool,
}

impl DebloatExecutionMutex {
    pub(crate) fn acquire() -> Result<Self, DebloatExecutionError> {
        let name = wide(DEBLOAT_MUTEX_NAME);
        let handle =
            unsafe { CreateMutexW(None, false, PCWSTR(name.as_ptr())) }.map_err(|error| {
                DebloatExecutionError::NativeDeployment(format!("mutex creation failed: {error}"))
            })?;
        let wait = unsafe { WaitForSingleObject(handle, DEBLOAT_MUTEX_TIMEOUT_MS) };
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
            return Err(DebloatExecutionError::NativeDeployment(format!(
                "debloat executor mutex wait timed out after {DEBLOAT_MUTEX_TIMEOUT_MS} ms"
            )));
        }
        Err(DebloatExecutionError::NativeDeployment(format!(
            "debloat executor mutex wait failed with status {wait:?}"
        )))
    }
}

impl Drop for DebloatExecutionMutex {
    fn drop(&mut self) {
        unsafe {
            if self.acquired {
                let _ = ReleaseMutex(self.handle);
            }
            let _ = CloseHandle(self.handle);
        }
    }
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}
