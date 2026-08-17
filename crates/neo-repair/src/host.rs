#[cfg(windows)]
use crate::command::{
    component_store_inspection_command, feature_inspection_command, operation_command,
    system_files_inspection_command, TrustedCommand, TrustedProgram,
};
use crate::error::RepairError;
use crate::model::{
    BoundedCommandEvidence, ComponentStoreObservation, SupportedWindowsFeature,
    SystemFileObservation, WindowsFeatureObservation,
};
use crate::operation::RepairOperation;
#[cfg(windows)]
use crate::parse::{component_store_observation, feature_observation, system_file_observation};
#[cfg(windows)]
use neo_probe::CommandEvidence;
#[cfg(windows)]
use std::io::Read;
#[cfg(all(test, windows))]
use std::io::Write;
#[cfg(windows)]
use std::process::{Command, Stdio};
#[cfg(windows)]
use std::time::{Duration, Instant};
#[cfg(windows)]
use windows::core::PCWSTR;
#[cfg(windows)]
use windows::Win32::Foundation::{
    CloseHandle, LocalFree, HANDLE, HLOCAL, WAIT_ABANDONED, WAIT_OBJECT_0, WAIT_TIMEOUT,
};
#[cfg(windows)]
use windows::Win32::Security::Authorization::{
    ConvertStringSecurityDescriptorToSecurityDescriptorW, GetSecurityInfo, SE_KERNEL_OBJECT,
};
#[cfg(windows)]
use windows::Win32::Security::{
    EqualSid, GetTokenInformation, IsWellKnownSid, TokenOwner, WinBuiltinAdministratorsSid,
    WinLocalSystemSid, OWNER_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR, PSID, SECURITY_ATTRIBUTES,
    TOKEN_OWNER, TOKEN_QUERY,
};
#[cfg(windows)]
use windows::Win32::System::Threading::{
    CreateMutexW, GetCurrentProcess, OpenProcessToken, ReleaseMutex, WaitForSingleObject,
};

#[cfg(any(windows, test))]
const REPAIR_MUTEX_NAME: &str = "Global\\THETECHGUY.NeoDriver.RepairExecutor.v1";
#[cfg(any(windows, test))]
const REPAIR_MUTEX_SDDL: &str = "D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GA;;;OW)";
#[cfg(windows)]
const REPAIR_MUTEX_TIMEOUT_MS: u32 = 300_000;
#[cfg(windows)]
const REPAIR_COMMAND_TIMEOUT_SECONDS: u64 = 15 * 60;
#[cfg(windows)]
const REPAIR_COMMAND_POLL_MILLISECONDS: u64 = 100;

#[cfg(windows)]
pub(crate) struct WindowsRepairExecutionMutex {
    handle: HANDLE,
    acquired: bool,
}

#[cfg(windows)]
struct LocalSecurityDescriptor(PSECURITY_DESCRIPTOR);

#[cfg(windows)]
impl Drop for LocalSecurityDescriptor {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            unsafe {
                let _ = LocalFree(Some(HLOCAL(self.0 .0)));
            }
        }
    }
}

#[cfg(windows)]
fn repair_mutex_security_descriptor() -> Result<LocalSecurityDescriptor, RepairError> {
    let sddl = wide(REPAIR_MUTEX_SDDL);
    let mut descriptor = PSECURITY_DESCRIPTOR::default();
    unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            PCWSTR(sddl.as_ptr()),
            1,
            &mut descriptor,
            None,
        )
    }
    .map_err(|error| {
        RepairError::CommandFailed(format!(
            "repair execution mutex security descriptor creation failed: {error}"
        ))
    })?;
    if descriptor.is_invalid() {
        return Err(RepairError::CommandFailed(
            "repair execution mutex security descriptor is invalid".to_string(),
        ));
    }
    Ok(LocalSecurityDescriptor(descriptor))
}

#[cfg(windows)]
fn repair_mutex_owner_matches_current_token(owner: PSID) -> Result<bool, RepairError> {
    let mut token = HANDLE::default();
    unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) }.map_err(|error| {
        RepairError::CommandFailed(format!(
            "repair execution mutex token-owner lookup failed to open process token: {error}"
        ))
    })?;

    let result = (|| {
        let mut required = 0_u32;
        let _ = unsafe { GetTokenInformation(token, TokenOwner, None, 0, &mut required) };
        if required < std::mem::size_of::<TOKEN_OWNER>() as u32 {
            return Err(RepairError::CommandFailed(
                "repair execution mutex token-owner lookup returned no owner buffer".to_string(),
            ));
        }

        let word = std::mem::size_of::<usize>();
        let words = (required as usize).div_ceil(word);
        let mut buffer = vec![0_usize; words];
        unsafe {
            GetTokenInformation(
                token,
                TokenOwner,
                Some(buffer.as_mut_ptr().cast()),
                required,
                &mut required,
            )
        }
        .map_err(|error| {
            RepairError::CommandFailed(format!(
                "repair execution mutex token-owner lookup failed: {error}"
            ))
        })?;

        let token_owner = unsafe { &*buffer.as_ptr().cast::<TOKEN_OWNER>() };
        if token_owner.Owner.is_invalid() {
            return Err(RepairError::CommandFailed(
                "repair execution mutex current token has no default owner".to_string(),
            ));
        }
        Ok(unsafe { EqualSid(owner, token_owner.Owner).is_ok() })
    })();

    unsafe {
        let _ = CloseHandle(token);
    }
    result
}

#[cfg(windows)]
fn validate_repair_mutex_owner(handle: HANDLE) -> Result<(), RepairError> {
    let mut owner = PSID::default();
    let mut descriptor = PSECURITY_DESCRIPTOR::default();
    let status = unsafe {
        GetSecurityInfo(
            handle,
            SE_KERNEL_OBJECT,
            OWNER_SECURITY_INFORMATION,
            Some(&mut owner),
            None,
            None,
            None,
            Some(&mut descriptor),
        )
    };
    if status.0 != 0 {
        return Err(RepairError::CommandFailed(format!(
            "repair execution mutex owner validation failed with status {}",
            status.0
        )));
    }
    let _descriptor = LocalSecurityDescriptor(descriptor);
    if owner.is_invalid() {
        return Err(RepairError::CommandFailed(
            "repair execution mutex has no trusted owner".to_string(),
        ));
    }
    let trusted = repair_mutex_owner_matches_current_token(owner)?
        || unsafe {
            IsWellKnownSid(owner, WinBuiltinAdministratorsSid).as_bool()
                || IsWellKnownSid(owner, WinLocalSystemSid).as_bool()
        };
    if !trusted {
        return Err(RepairError::CommandFailed(
            "repair execution mutex owner does not match the current token, SYSTEM, or built-in Administrators"
                .to_string(),
        ));
    }
    Ok(())
}

#[cfg(windows)]
impl WindowsRepairExecutionMutex {
    pub(crate) fn acquire() -> Result<Self, RepairError> {
        let name = wide(REPAIR_MUTEX_NAME);
        let descriptor = repair_mutex_security_descriptor()?;
        let attributes = SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: descriptor.0 .0,
            bInheritHandle: false.into(),
        };
        let handle = unsafe { CreateMutexW(Some(&attributes), false, PCWSTR(name.as_ptr())) }
            .map_err(|error| {
                RepairError::CommandFailed(format!(
                    "repair execution mutex creation failed: {error}"
                ))
            })?;
        if let Err(error) = validate_repair_mutex_owner(handle) {
            unsafe {
                let _ = CloseHandle(handle);
            }
            return Err(error);
        }
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

#[cfg(test)]
mod mutex_contract_tests {
    use super::{REPAIR_MUTEX_NAME, REPAIR_MUTEX_SDDL};

    #[test]
    fn servicing_mutex_contract_is_machine_wide_and_privileged() {
        assert_eq!(
            REPAIR_MUTEX_NAME,
            "Global\\THETECHGUY.NeoDriver.RepairExecutor.v1"
        );
        assert_eq!(REPAIR_MUTEX_SDDL, "D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GA;;;OW)");
    }

    #[cfg(windows)]
    #[test]
    fn servicing_mutex_security_descriptor_is_accepted_by_windows() {
        let descriptor = super::repair_mutex_security_descriptor()
            .expect("restricted global mutex security descriptor must be valid");
        assert!(!descriptor.0.is_invalid());
    }
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
            dism: path_text(&dism)?,
            sfc: path_text(&sfc)?,
        })
    }

    fn capture(&self, program: &str, args: &[&str]) -> BoundedCommandEvidence {
        run_bounded_command(
            program,
            args,
            Duration::from_secs(REPAIR_COMMAND_TIMEOUT_SECONDS),
        )
    }

    fn capture_trusted(&self, command: TrustedCommand) -> BoundedCommandEvidence {
        let program = match command.program {
            TrustedProgram::Dism => &self.dism,
            TrustedProgram::Sfc => &self.sfc,
        };
        let args: Vec<&str> = command.args.iter().map(String::as_str).collect();
        self.capture(program, &args)
    }

    fn feature_info(&self, feature: SupportedWindowsFeature) -> WindowsFeatureObservation {
        feature_observation(
            feature,
            self.capture_trusted(feature_inspection_command(feature)),
        )
    }
}

#[cfg(windows)]
impl RepairHost for WindowsRepairHost {
    fn observe_component_store(&self) -> Result<ComponentStoreObservation, RepairError> {
        Ok(component_store_observation(
            self.capture_trusted(component_store_inspection_command()),
        ))
    }

    fn observe_system_files(&self) -> Result<SystemFileObservation, RepairError> {
        Ok(system_file_observation(
            self.capture_trusted(system_files_inspection_command()),
        ))
    }

    fn observe_feature(
        &self,
        feature: SupportedWindowsFeature,
    ) -> Result<WindowsFeatureObservation, RepairError> {
        Ok(self.feature_info(feature))
    }

    fn execute(&self, operation: RepairOperation) -> Result<BoundedCommandEvidence, RepairError> {
        Ok(self.capture_trusted(operation_command(operation)))
    }
}

#[cfg(windows)]
fn truncate_capture(value: &mut String) -> bool {
    if value.len() <= crate::model::MAX_REPAIR_EVIDENCE_BYTES {
        return false;
    }
    let mut end = crate::model::MAX_REPAIR_EVIDENCE_BYTES;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    value.truncate(end);
    true
}

#[cfg(windows)]
fn append_capture_detail(target: &mut String, detail: &str) -> bool {
    if !target.is_empty() {
        target.push('\n');
    }
    target.push_str(detail);
    truncate_capture(target)
}

#[cfg(windows)]
fn drain_pipe<R: Read>(mut reader: R) -> (String, bool, Option<String>) {
    let mut retained = Vec::with_capacity(8192);
    let mut buffer = [0u8; 8192];
    let mut truncated = false;
    loop {
        match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(count) => {
                let remaining =
                    crate::model::MAX_REPAIR_EVIDENCE_BYTES.saturating_sub(retained.len());
                let keep = remaining.min(count);
                retained.extend_from_slice(&buffer[..keep]);
                truncated |= keep < count;
            }
            Err(error) => {
                return (
                    String::from_utf8_lossy(&retained).into_owned(),
                    true,
                    Some(format!("servicing evidence pipe read failed: {error}")),
                );
            }
        }
    }
    (
        String::from_utf8_lossy(&retained).into_owned(),
        truncated,
        None,
    )
}

#[cfg(windows)]
fn finish_reader_bounded(
    handle: std::thread::JoinHandle<(String, bool, Option<String>)>,
    label: &str,
) -> (String, bool, Option<String>) {
    let deadline = Instant::now() + Duration::from_secs(2);
    while !handle.is_finished() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    if !handle.is_finished() {
        drop(handle);
        return (
            String::new(),
            true,
            Some(format!(
                "servicing {label} reader did not close within the bounded drain window"
            )),
        );
    }
    handle.join().unwrap_or_else(|_| {
        (
            String::new(),
            true,
            Some(format!("servicing {label} reader panicked")),
        )
    })
}

#[cfg(windows)]
fn run_bounded_command(program: &str, args: &[&str], timeout: Duration) -> BoundedCommandEvidence {
    let mut child = match Command::new(program)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(error) => {
            return BoundedCommandEvidence::from_command(CommandEvidence {
                program: program.to_string(),
                args: args.iter().map(|arg| (*arg).to_string()).collect(),
                exit_code: None,
                stdout: String::new(),
                stderr: error.to_string(),
                start_error: Some(error.to_string()),
            });
        }
    };

    let stdout = match child.stdout.take() {
        Some(stdout) => stdout,
        None => {
            let _ = child.kill();
            let _ = child.wait();
            let mut evidence = BoundedCommandEvidence::from_command(CommandEvidence {
                program: program.to_string(),
                args: args.iter().map(|arg| (*arg).to_string()).collect(),
                exit_code: None,
                stdout: String::new(),
                stderr:
                    "trusted Windows command stdout capture was unavailable after process start"
                        .to_string(),
                start_error: None,
            });
            evidence.timed_out = false;
            return evidence;
        }
    };
    let stderr = match child.stderr.take() {
        Some(stderr) => stderr,
        None => {
            let _ = child.kill();
            let _ = child.wait();
            let mut evidence = BoundedCommandEvidence::from_command(CommandEvidence {
                program: program.to_string(),
                args: args.iter().map(|arg| (*arg).to_string()).collect(),
                exit_code: None,
                stdout: String::new(),
                stderr:
                    "trusted Windows command stderr capture was unavailable after process start"
                        .to_string(),
                start_error: None,
            });
            evidence.timed_out = false;
            return evidence;
        }
    };

    let stdout_reader = std::thread::spawn(move || drain_pipe(stdout));
    let stderr_reader = std::thread::spawn(move || drain_pipe(stderr));
    let deadline = Instant::now() + timeout;
    let mut timed_out = false;
    let mut process_ended = false;
    let mut runtime_detail: Option<String> = None;

    let exit_code = loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                process_ended = true;
                break status.code();
            }
            Ok(None) => {
                let now = Instant::now();
                if now >= deadline {
                    timed_out = true;
                    match child.kill() {
                        Ok(()) => match child.wait() {
                            Ok(status) => {
                                process_ended = true;
                                break status.code();
                            }
                            Err(error) => {
                                runtime_detail = Some(format!(
                                    "timed-out servicing process wait failed: {error}"
                                ));
                                break None;
                            }
                        },
                        Err(error) => match child.try_wait() {
                            Ok(Some(status)) => {
                                process_ended = true;
                                runtime_detail = Some(format!(
                                    "servicing deadline was reached as the process exited; termination returned: {error}"
                                ));
                                break status.code();
                            }
                            _ => {
                                runtime_detail = Some(format!(
                                    "timed-out servicing process termination failed: {error}"
                                ));
                                break None;
                            }
                        },
                    }
                }
                std::thread::sleep(
                    (deadline - now).min(Duration::from_millis(REPAIR_COMMAND_POLL_MILLISECONDS)),
                );
            }
            Err(error) => {
                runtime_detail = Some(format!(
                    "trusted Windows command status polling failed: {error}"
                ));
                if child.kill().is_ok() {
                    if let Ok(status) = child.wait() {
                        process_ended = true;
                        break status.code();
                    }
                }
                break None;
            }
        }
    };

    let (stdout, stdout_truncated, stdout_error, mut stderr, mut stderr_truncated, stderr_error) =
        if process_ended {
            let (stdout, stdout_truncated, stdout_error) =
                finish_reader_bounded(stdout_reader, "stdout");
            let (stderr, stderr_truncated, stderr_error) =
                finish_reader_bounded(stderr_reader, "stderr");
            (
                stdout,
                stdout_truncated,
                stdout_error,
                stderr,
                stderr_truncated,
                stderr_error,
            )
        } else {
            drop(stdout_reader);
            drop(stderr_reader);
            (
                String::new(),
                true,
                Some("servicing stdout could not be safely joined because process termination was unproven".to_string()),
                String::new(),
                true,
                Some("servicing stderr could not be safely joined because process termination was unproven".to_string()),
            )
        };

    if timed_out {
        stderr_truncated |= append_capture_detail(
            &mut stderr,
            &format!(
                "Phase 21 servicing command timed out after {} seconds.",
                timeout.as_secs()
            ),
        );
    }
    for detail in [runtime_detail, stdout_error, stderr_error]
        .into_iter()
        .flatten()
    {
        stderr_truncated |= append_capture_detail(&mut stderr, &detail);
    }

    let mut evidence = BoundedCommandEvidence::from_command(CommandEvidence {
        program: program.to_string(),
        args: args.iter().map(|arg| (*arg).to_string()).collect(),
        exit_code,
        stdout,
        stderr,
        start_error: None,
    });
    evidence.timed_out = timed_out;
    evidence.stdout_truncated |= stdout_truncated;
    evidence.stderr_truncated |= stderr_truncated;
    evidence
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

#[cfg(all(test, windows))]
mod command_timeout_tests {
    use super::*;

    #[test]
    #[ignore = "child fixture for bounded servicing runner"]
    fn timeout_child_fixture() {
        std::thread::sleep(Duration::from_secs(5));
    }

    #[test]
    fn servicing_process_is_killed_at_the_bounded_deadline() {
        let executable = std::env::current_exe().expect("current test executable");
        let program = executable.to_string_lossy().into_owned();
        let evidence = run_bounded_command(
            &program,
            &[
                "--exact",
                "host::command_timeout_tests::timeout_child_fixture",
                "--ignored",
            ],
            Duration::from_millis(100),
        );
        assert!(evidence.timed_out);
        assert!(evidence.start_error.is_none());
        assert!(!evidence.succeeded());
    }
}

#[cfg(all(test, windows))]
mod command_pipe_tests {
    use super::*;

    #[test]
    #[ignore = "child fixture for bounded servicing output drain"]
    fn large_output_child_fixture() {
        let payload = vec![b'x'; crate::model::MAX_REPAIR_EVIDENCE_BYTES * 2];
        std::io::stdout()
            .write_all(&payload)
            .expect("write large child fixture output");
    }

    #[test]
    fn servicing_runner_drains_output_while_process_is_running() {
        let executable = std::env::current_exe().expect("current test executable");
        let program = executable.to_string_lossy().into_owned();
        let evidence = run_bounded_command(
            &program,
            &[
                "--exact",
                "host::command_pipe_tests::large_output_child_fixture",
                "--ignored",
                "--nocapture",
            ],
            Duration::from_secs(5),
        );
        assert!(!evidence.timed_out);
        assert!(evidence.stdout_truncated);
        assert!(evidence.exit_code == Some(0));
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
                timed_out: false,
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
