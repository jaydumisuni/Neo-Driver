use crate::{RuntimeExecutorError, RuntimeHost, RuntimeInvocation, RuntimeProcessResult};
use neo_catalogue::RuntimeInstallerKind;
use neo_runtime::RuntimeInventory;
use neo_runtime_probe::scan_current_runtime_inventory;
use sha2::{Digest, Sha256};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom};
use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::process::Command;
use windows::core::{Error as WinError, PCWSTR};
use windows::Win32::Foundation::{
    CloseHandle, HANDLE, WAIT_ABANDONED, WAIT_OBJECT_0, WAIT_TIMEOUT,
};
use windows::Win32::System::SystemInformation::GetWindowsDirectoryW;
use windows::Win32::System::Threading::{CreateMutexW, ReleaseMutex, WaitForSingleObject};

const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
const FILE_SHARE_READ: u32 = 0x0000_0001;
const RUNTIME_MUTEX_NAME: &str = "Local\\THETECHGUY.NeoDriver.RuntimeExecutor.v1";
const RUNTIME_MUTEX_TIMEOUT_MS: u32 = 300_000;

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct WindowsRuntimeHost;

impl RuntimeHost for WindowsRuntimeHost {
    fn inventory(&self) -> Result<RuntimeInventory, RuntimeExecutorError> {
        scan_current_runtime_inventory()
            .map(|report| report.inventory)
            .map_err(|error| RuntimeExecutorError::Host(error.to_string()))
    }

    fn execute(
        &self,
        invocation: &RuntimeInvocation,
    ) -> Result<RuntimeProcessResult, RuntimeExecutorError> {
        invocation.validate()?;
        let _runtime_lock = RuntimeExecutionMutex::acquire()?;
        let mut payload = open_locked_payload(&invocation.payload)?;
        let observed = sha256_locked_file(&mut payload)?;
        if observed != invocation.expected_sha256.as_str() {
            return Ok(RuntimeProcessResult::start_failed(format!(
                "locked staged payload hash mismatch: expected {}, observed {observed}",
                invocation.expected_sha256
            )));
        }

        let mut command = match invocation.installer {
            RuntimeInstallerKind::Exe => {
                let mut command = Command::new(&invocation.payload);
                command.args(&invocation.arguments);
                command
            }
            RuntimeInstallerKind::Msi => {
                let mut command = Command::new(trusted_msiexec_path()?);
                command
                    .arg("/i")
                    .arg(&invocation.payload)
                    .arg("/qn")
                    .arg("/norestart")
                    .args(&invocation.arguments);
                command
            }
        };

        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(error) => {
                return Ok(RuntimeProcessResult::start_failed(format!(
                    "runtime process creation failed: {error}"
                )))
            }
        };

        match child.wait() {
            Ok(status) => match status.code() {
                Some(code) => Ok(RuntimeProcessResult::exited(
                    code,
                    format!("runtime process exited with code {code}"),
                )),
                None => Ok(RuntimeProcessResult::started_without_exit(
                    "runtime process exited without an observable numeric exit code",
                )),
            },
            Err(error) => Ok(RuntimeProcessResult::started_without_exit(format!(
                "runtime process started but wait/status observation failed: {error}"
            ))),
        }
    }
}

fn open_locked_payload(path: &Path) -> Result<File, RuntimeExecutorError> {
    let file = OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)
        .map_err(|error| RuntimeExecutorError::Host(format!("payload open failed: {error}")))?;
    let metadata = file
        .metadata()
        .map_err(|error| RuntimeExecutorError::Host(format!("payload metadata failed: {error}")))?;
    if !metadata.is_file() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err(RuntimeExecutorError::Host(
            "staged payload is not a normal non-reparse file".to_string(),
        ));
    }
    Ok(file)
}

fn sha256_locked_file(file: &mut File) -> Result<String, RuntimeExecutorError> {
    file.seek(SeekFrom::Start(0))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    file.seek(SeekFrom::Start(0))?;
    let digest = hasher.finalize();
    let mut encoded = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        write!(&mut encoded, "{byte:02x}").expect("writing to String cannot fail");
    }
    Ok(encoded)
}

fn trusted_msiexec_path() -> Result<PathBuf, RuntimeExecutorError> {
    let windows = trusted_windows_directory()?;
    let path = windows.join("System32").join("msiexec.exe");
    let metadata = std::fs::metadata(&path)
        .map_err(|error| RuntimeExecutorError::Host(format!("msiexec unavailable: {error}")))?;
    if !metadata.is_file() {
        return Err(RuntimeExecutorError::Host(format!(
            "trusted msiexec path is not a file: {}",
            path.display()
        )));
    }
    Ok(path)
}

fn trusted_windows_directory() -> Result<PathBuf, RuntimeExecutorError> {
    let mut buffer = vec![0u16; 260];
    loop {
        let length = unsafe { GetWindowsDirectoryW(Some(&mut buffer)) } as usize;
        if length == 0 {
            return Err(RuntimeExecutorError::Host(
                WinError::from_thread().to_string(),
            ));
        }
        if length < buffer.len() {
            return Ok(PathBuf::from(String::from_utf16_lossy(&buffer[..length])));
        }
        buffer.resize(length + 1, 0);
    }
}

struct RuntimeExecutionMutex {
    handle: HANDLE,
    acquired: bool,
}

impl RuntimeExecutionMutex {
    fn acquire() -> Result<Self, RuntimeExecutorError> {
        let mut name = RUNTIME_MUTEX_NAME.encode_utf16().collect::<Vec<_>>();
        name.push(0);
        let handle =
            unsafe { CreateMutexW(None, false, PCWSTR(name.as_ptr())) }.map_err(|error| {
                RuntimeExecutorError::Host(format!("mutex creation failed: {error}"))
            })?;
        let wait = unsafe { WaitForSingleObject(handle, RUNTIME_MUTEX_TIMEOUT_MS) };
        if wait == WAIT_OBJECT_0 || wait == WAIT_ABANDONED {
            Ok(Self {
                handle,
                acquired: true,
            })
        } else {
            unsafe {
                let _ = CloseHandle(handle);
            }
            if wait == WAIT_TIMEOUT {
                return Err(RuntimeExecutorError::Host(format!(
                    "runtime executor mutex wait timed out after {RUNTIME_MUTEX_TIMEOUT_MS} ms"
                )));
            }
            Err(RuntimeExecutorError::Host(format!(
                "runtime executor mutex wait failed with status {wait:?}"
            )))
        }
    }
}

impl Drop for RuntimeExecutionMutex {
    fn drop(&mut self) {
        unsafe {
            if self.acquired {
                let _ = ReleaseMutex(self.handle);
            }
            let _ = CloseHandle(self.handle);
        }
    }
}
