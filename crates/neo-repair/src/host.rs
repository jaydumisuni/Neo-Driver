use crate::error::RepairError;
use crate::model::{
    BoundedCommandEvidence, ComponentStoreObservation, SupportedWindowsFeature,
    SystemFileObservation, WindowsFeatureObservation,
};
use crate::operation::RepairOperation;
#[cfg(windows)]
use crate::parse::{component_store_observation, feature_observation, system_file_observation};
#[cfg(windows)]
use neo_probe::{CommandEvidence, CommandRunner, SystemCommandRunner};
#[cfg(windows)]
use windows::core::PCWSTR;
#[cfg(windows)]
use windows::Win32::Foundation::{
    CloseHandle, HANDLE, WAIT_ABANDONED, WAIT_OBJECT_0, WAIT_TIMEOUT,
};
#[cfg(windows)]
use windows::Win32::System::Threading::{CreateMutexW, ReleaseMutex, WaitForSingleObject};

#[cfg(windows)]
const REPAIR_MUTEX_NAME: &str = "Local\\THETECHGUY.NeoDriver.RepairExecutor.v1";
#[cfg(windows)]
const REPAIR_MUTEX_TIMEOUT_MS: u32 = 300_000;

#[cfg(windows)]
pub(crate) struct WindowsRepairExecutionMutex {
    handle: HANDLE,
    acquired: bool,
}

#[cfg(windows)]
impl WindowsRepairExecutionMutex {
    pub(crate) fn acquire() -> Result<Self, RepairError> {
        let name = wide(REPAIR_MUTEX_NAME);
        let handle =
            unsafe { CreateMutexW(None, false, PCWSTR(name.as_ptr())) }.map_err(|error| {
                RepairError::CommandFailed(format!(
                    "repair execution mutex creation failed: {error}"
                ))
            })?;
        let wait = unsafe { WaitForSingleObject(handle, REPAIR_MUTEX_TIMEOUT_MS) };
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
            return Err(RepairError::CommandFailed(format!(
                "repair execution mutex wait timed out after {REPAIR_MUTEX_TIMEOUT_MS} ms"
            )));
        }
        Err(RepairError::CommandFailed(format!(
            "repair execution mutex wait failed with status {wait:?}"
        )))
    }
}

#[cfg(windows)]
impl Drop for WindowsRepairExecutionMutex {
    fn drop(&mut self) {
        unsafe {
            if self.acquired {
                let _ = ReleaseMutex(self.handle);
            }
            let _ = CloseHandle(self.handle);
        }
    }
}

#[cfg(windows)]
fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

pub(crate) trait RepairHost {
    fn observe_component_store(&self) -> Result<ComponentStoreObservation, RepairError>;
    fn observe_system_files(&self) -> Result<SystemFileObservation, RepairError>;
    fn observe_feature(
        &self,
        feature: SupportedWindowsFeature,
    ) -> Result<WindowsFeatureObservation, RepairError>;
    fn execute(&self, operation: RepairOperation) -> Result<BoundedCommandEvidence, RepairError>;
}

#[cfg(windows)]
pub(crate) struct WindowsRepairHost {
    runner: SystemCommandRunner,
    dism: String,
    sfc: String,
}

#[cfg(windows)]
impl WindowsRepairHost {
    pub(crate) fn new() -> Result<Self, RepairError> {
        let windows = trusted_windows_directory()?;
        let system32 = windows.join("System32");
        let dism = system32.join("dism.exe");
        let sfc = system32.join("sfc.exe");
        Ok(Self {
            runner: SystemCommandRunner,
            dism: path_text(&dism)?,
            sfc: path_text(&sfc)?,
        })
    }

    fn capture(&self, program: &str, args: &[&str]) -> BoundedCommandEvidence {
        let evidence = match self.runner.run(program, args) {
            Ok(value) => value,
            Err(error) => CommandEvidence::failed_to_start(program, args, &error),
        };
        BoundedCommandEvidence::from_command(evidence)
    }

    fn feature_info(&self, feature: SupportedWindowsFeature) -> WindowsFeatureObservation {
        let feature_arg = format!("/FeatureName:{}", feature.dism_name());
        let evidence = self.capture(
            &self.dism,
            &["/Online", "/Get-FeatureInfo", &feature_arg, "/English"],
        );
        feature_observation(feature, evidence)
    }
}

#[cfg(windows)]
impl RepairHost for WindowsRepairHost {
    fn observe_component_store(&self) -> Result<ComponentStoreObservation, RepairError> {
        Ok(component_store_observation(self.capture(
            &self.dism,
            &["/Online", "/Cleanup-Image", "/CheckHealth", "/English"],
        )))
    }

    fn observe_system_files(&self) -> Result<SystemFileObservation, RepairError> {
        Ok(system_file_observation(
            self.capture(&self.sfc, &["/verifyonly"]),
        ))
    }

    fn observe_feature(
        &self,
        feature: SupportedWindowsFeature,
    ) -> Result<WindowsFeatureObservation, RepairError> {
        Ok(self.feature_info(feature))
    }

    fn execute(&self, operation: RepairOperation) -> Result<BoundedCommandEvidence, RepairError> {
        let evidence = match operation {
            RepairOperation::RestoreComponentStore => self.capture(
                &self.dism,
                &[
                    "/Online",
                    "/NoRestart",
                    "/Cleanup-Image",
                    "/RestoreHealth",
                    "/English",
                ],
            ),
            RepairOperation::RepairSystemFiles => self.capture(&self.sfc, &["/scannow"]),
            RepairOperation::SetWindowsFeature { feature, desired } => {
                let feature_arg = format!("/FeatureName:{}", feature.dism_name());
                match desired {
                    crate::model::FeatureDesiredState::Enabled => self.capture(
                        &self.dism,
                        &[
                            "/Online",
                            "/NoRestart",
                            "/Enable-Feature",
                            &feature_arg,
                            "/English",
                        ],
                    ),
                    crate::model::FeatureDesiredState::Disabled => self.capture(
                        &self.dism,
                        &[
                            "/Online",
                            "/NoRestart",
                            "/Disable-Feature",
                            &feature_arg,
                            "/English",
                        ],
                    ),
                }
            }
        };
        Ok(evidence)
    }
}

#[cfg(windows)]
fn path_text(path: &std::path::Path) -> Result<String, RepairError> {
    path.to_str().map(ToOwned::to_owned).ok_or_else(|| {
        RepairError::WindowsDirectory("System32 path is not valid UTF-8".to_string())
    })
}

#[cfg(windows)]
fn trusted_windows_directory() -> Result<std::path::PathBuf, RepairError> {
    use windows::core::Error as WinError;
    use windows::Win32::System::SystemInformation::GetWindowsDirectoryW;

    let mut buffer = vec![0u16; 260];
    loop {
        let length = unsafe { GetWindowsDirectoryW(Some(&mut buffer)) } as usize;
        if length == 0 {
            return Err(RepairError::WindowsDirectory(
                WinError::from_thread().to_string(),
            ));
        }
        if length < buffer.len() {
            return Ok(std::path::PathBuf::from(String::from_utf16_lossy(
                &buffer[..length],
            )));
        }
        buffer.resize(length + 1, 0);
    }
}

#[cfg(test)]
pub(crate) mod testsupport {
    use super::*;
    use crate::model::{
        ComponentStoreState, FeatureDesiredState, SystemFileState, WindowsFeatureState,
    };
    use std::cell::RefCell;

    #[derive(Debug, Clone)]
    pub(crate) struct FakeRepairHost {
        component: RefCell<ComponentStoreState>,
        system_files: RefCell<SystemFileState>,
        features: RefCell<std::collections::BTreeMap<SupportedWindowsFeature, WindowsFeatureState>>,
        pub(crate) observed: RefCell<Vec<String>>,
        pub(crate) executed: RefCell<Vec<RepairOperation>>,
        pub(crate) fail_execution: RefCell<Option<String>>,
        pub(crate) fail_operation: RefCell<Option<RepairOperation>>,
        pub(crate) execution_exit_code: RefCell<i32>,
        pub(crate) pending_feature_transition: RefCell<bool>,
    }

    impl FakeRepairHost {
        pub(crate) fn new(component: ComponentStoreState, system_files: SystemFileState) -> Self {
            let features = SupportedWindowsFeature::all()
                .iter()
                .copied()
                .map(|feature| (feature, WindowsFeatureState::Disabled))
                .collect();
            Self {
                component: RefCell::new(component),
                system_files: RefCell::new(system_files),
                features: RefCell::new(features),
                observed: RefCell::new(Vec::new()),
                executed: RefCell::new(Vec::new()),
                fail_execution: RefCell::new(None),
                fail_operation: RefCell::new(None),
                execution_exit_code: RefCell::new(0),
                pending_feature_transition: RefCell::new(false),
            }
        }

        pub(crate) fn set_feature(
            &self,
            feature: SupportedWindowsFeature,
            state: WindowsFeatureState,
        ) {
            self.features.borrow_mut().insert(feature, state);
        }

        pub(crate) fn set_component(&self, state: ComponentStoreState) {
            *self.component.borrow_mut() = state;
        }

        fn evidence(program: &str, args: Vec<String>) -> BoundedCommandEvidence {
            BoundedCommandEvidence {
                program: program.to_string(),
                args,
                exit_code: Some(0),
                stdout: "fake Phase 21 evidence".to_string(),
                stderr: String::new(),
                start_error: None,
                stdout_truncated: false,
                stderr_truncated: false,
            }
        }
    }

    impl RepairHost for FakeRepairHost {
        fn observe_component_store(&self) -> Result<ComponentStoreObservation, RepairError> {
            self.observed
                .borrow_mut()
                .push("component_store".to_string());
            let state = *self.component.borrow();
            Ok(ComponentStoreObservation {
                state,
                elevation_required: false,
                detail: format!("fake component store state: {state:?}"),
                evidence: Self::evidence("dism.exe", vec!["/CheckHealth".to_string()]),
            })
        }

        fn observe_system_files(&self) -> Result<SystemFileObservation, RepairError> {
            self.observed.borrow_mut().push("system_files".to_string());
            let state = *self.system_files.borrow();
            Ok(SystemFileObservation {
                state,
                elevation_required: false,
                detail: format!("fake system file state: {state:?}"),
                evidence: Self::evidence("sfc.exe", vec!["/verifyonly".to_string()]),
            })
        }

        fn observe_feature(
            &self,
            feature: SupportedWindowsFeature,
        ) -> Result<WindowsFeatureObservation, RepairError> {
            self.observed
                .borrow_mut()
                .push(format!("feature:{}", feature.id()));
            let state = self
                .features
                .borrow()
                .get(&feature)
                .copied()
                .unwrap_or(WindowsFeatureState::Unavailable);
            Ok(WindowsFeatureObservation {
                feature,
                state,
                elevation_required: false,
                detail: format!("fake feature state: {state:?}"),
                evidence: Self::evidence("dism.exe", vec![feature.dism_name().to_string()]),
            })
        }

        fn execute(
            &self,
            operation: RepairOperation,
        ) -> Result<BoundedCommandEvidence, RepairError> {
            if self.fail_operation.borrow().as_ref() == Some(&operation) {
                self.fail_operation.borrow_mut().take();
                return Err(RepairError::CommandFailed(format!(
                    "configured command-start failure for {}",
                    operation.action_id()
                )));
            }
            if let Some(error) = self.fail_execution.borrow_mut().take() {
                return Err(RepairError::CommandFailed(error));
            }
            self.executed.borrow_mut().push(operation);
            match operation {
                RepairOperation::RestoreComponentStore => {
                    *self.component.borrow_mut() = ComponentStoreState::Healthy;
                }
                RepairOperation::RepairSystemFiles => {
                    *self.system_files.borrow_mut() = SystemFileState::Healthy;
                }
                RepairOperation::SetWindowsFeature { feature, desired } => {
                    let state = if *self.pending_feature_transition.borrow() {
                        match desired {
                            FeatureDesiredState::Enabled => WindowsFeatureState::EnablePending,
                            FeatureDesiredState::Disabled => WindowsFeatureState::DisablePending,
                        }
                    } else {
                        desired.target_state()
                    };
                    self.features.borrow_mut().insert(feature, state);
                }
            }
            let mut evidence = Self::evidence("trusted.exe", vec![operation.action_id()]);
            evidence.exit_code = Some(*self.execution_exit_code.borrow());
            Ok(evidence)
        }
    }
}
