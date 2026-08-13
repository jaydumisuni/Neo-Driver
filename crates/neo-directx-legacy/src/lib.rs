//! Read-only DirectX End-User Runtimes (June 2010) framework-component evidence.
//!
//! This crate models the documented side-by-side legacy DirectX component set
//! separately from modern DirectX/GPU capability. It never downloads, installs,
//! registers, repairs, or deletes DLLs.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[cfg(windows)]
use windows::core::Error as WinError;
#[cfg(windows)]
use windows::Win32::System::SystemInformation::GetWindowsDirectoryW;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LegacyDirectXState {
    Installed,
    Partial,
    Missing,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowsArchitecture {
    X86,
    X64,
    Arm64,
}

impl WindowsArchitecture {
    pub fn parse(value: &str) -> Result<Self, LegacyDirectXError> {
        match value.trim().to_ascii_lowercase().as_str() {
            "x86" | "i386" | "i686" => Ok(Self::X86),
            "x64" | "amd64" | "x86_64" => Ok(Self::X64),
            "arm64" | "aarch64" => Ok(Self::Arm64),
            other => Err(LegacyDirectXError::UnsupportedArchitecture(
                other.to_string(),
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchitectureReport {
    pub architecture: WindowsArchitecture,
    pub directory: PathBuf,
    pub expected_files: usize,
    pub present_files: usize,
    #[serde(default)]
    pub missing_files: Vec<String>,
}

impl ArchitectureReport {
    pub fn complete(&self) -> bool {
        self.expected_files > 0 && self.present_files == self.expected_files
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LegacyDirectXReport {
    pub state: LegacyDirectXState,
    pub source: String,
    pub expected_files: usize,
    pub present_files: usize,
    #[serde(default)]
    pub architectures: Vec<ArchitectureReport>,
    #[serde(default)]
    pub warnings: Vec<String>,
}

impl LegacyDirectXReport {
    pub fn certified_complete(&self) -> bool {
        self.state == LegacyDirectXState::Installed
            && self.expected_files > 0
            && self.present_files == self.expected_files
            && self.architectures.iter().all(ArchitectureReport::complete)
    }
}

/// Scan the current Windows installation using the trusted Windows directory
/// returned by `GetWindowsDirectoryW`. Process-controlled `%SystemRoot%` state
/// is intentionally not used as authority.
pub fn scan_current(architecture: WindowsArchitecture) -> LegacyDirectXReport {
    #[cfg(windows)]
    {
        let system_root = match trusted_windows_directory() {
            Ok(value) => value,
            Err(error) => {
                return unknown(&format!(
                    "GetWindowsDirectoryW could not establish the Windows directory: {error}"
                ));
            }
        };
        scan_at(&system_root, architecture, usize::BITS as u8)
    }

    #[cfg(not(windows))]
    {
        let _ = architecture;
        unknown("Current-system legacy DirectX scanning is Windows-only.")
    }
}

#[cfg(windows)]
fn trusted_windows_directory() -> Result<PathBuf, String> {
    let mut buffer = vec![0u16; 260];
    loop {
        let length = unsafe { GetWindowsDirectoryW(Some(&mut buffer)) } as usize;
        if length == 0 {
            return Err(WinError::from_thread().to_string());
        }
        if length < buffer.len() {
            return Ok(PathBuf::from(String::from_utf16_lossy(&buffer[..length])));
        }
        buffer.resize(length + 1, 0);
    }
}

/// Deterministic filesystem predicate used by the live scanner and tests.
pub fn scan_at(
    system_root: &Path,
    architecture: WindowsArchitecture,
    process_bits: u8,
) -> LegacyDirectXReport {
    match architecture {
        WindowsArchitecture::Arm64 => unknown(
            "The June 2010 x86/x64 legacy component layout is not yet frozen for Windows on ARM; Neo reports Unknown rather than projecting an x64 filesystem rule onto ARM64.",
        ),
        WindowsArchitecture::X64 if process_bits != 64 => unknown(
            "A 64-bit Neo process is required to prove native x64 System32 plus x86 SysWOW64 legacy DirectX completeness without WOW64 redirection ambiguity.",
        ),
        WindowsArchitecture::X86 => scan_directories(&[(
            WindowsArchitecture::X86,
            system_root.join("System32"),
        )]),
        WindowsArchitecture::X64 => scan_directories(&[
            (WindowsArchitecture::X64, system_root.join("System32")),
            (WindowsArchitecture::X86, system_root.join("SysWOW64")),
        ]),
    }
}

fn scan_directories(directories: &[(WindowsArchitecture, PathBuf)]) -> LegacyDirectXReport {
    if directories.iter().any(|(_, directory)| !directory.is_dir()) {
        return unknown(
            "One or more required Windows system directories are unavailable; Neo cannot distinguish missing legacy components from an inaccessible layout.",
        );
    }

    let names = expected_component_files();
    let mut architecture_reports = Vec::new();
    let mut total_present = 0usize;
    let total_expected = names.len() * directories.len();

    for (architecture, directory) in directories {
        let mut missing_files = Vec::new();
        let mut present_files = 0usize;
        for name in &names {
            if directory.join(name).is_file() {
                present_files += 1;
            } else {
                missing_files.push(name.clone());
            }
        }
        total_present += present_files;
        architecture_reports.push(ArchitectureReport {
            architecture: *architecture,
            directory: directory.clone(),
            expected_files: names.len(),
            present_files,
            missing_files,
        });
    }

    let state = if total_present == total_expected {
        LegacyDirectXState::Installed
    } else if total_present == 0 {
        LegacyDirectXState::Missing
    } else {
        LegacyDirectXState::Partial
    };

    LegacyDirectXReport {
        state,
        source: "Microsoft legacy DirectX framework component set (June 2010 ranges)".to_string(),
        expected_files: total_expected,
        present_files: total_present,
        architectures: architecture_reports,
        warnings: Vec::new(),
    }
}

fn unknown(detail: &str) -> LegacyDirectXReport {
    LegacyDirectXReport {
        state: LegacyDirectXState::Unknown,
        source: "Microsoft legacy DirectX framework component set (June 2010 ranges)".to_string(),
        expected_files: 0,
        present_files: 0,
        architectures: Vec::new(),
        warnings: vec![detail.to_string()],
    }
}

/// Canonical side-by-side legacy component filenames documented by Microsoft's
/// current GDK framework-package guidance.
pub fn expected_component_files() -> Vec<String> {
    let mut names = Vec::new();

    push_numbered(&mut names, "D3DCompiler_", 33, 43, ".dll");
    push_numbered(&mut names, "D3DCSX_", 42, 43, ".dll");

    names.push("D3DX10.dll".to_string());
    push_numbered(&mut names, "D3DX10_", 33, 43, ".dll");
    push_numbered(&mut names, "D3DX11_", 42, 43, ".dll");
    push_numbered(&mut names, "D3DX9_", 24, 43, ".dll");

    push_numbered(&mut names, "X3DAudio1_", 0, 7, ".dll");
    push_numbered(&mut names, "XACTEngine2_", 0, 9, ".dll");
    push_numbered(&mut names, "XACTEngine3_", 0, 7, ".dll");
    push_numbered(&mut names, "XAPOFX1_", 0, 5, ".dll");
    push_numbered(&mut names, "XAudio2_", 0, 7, ".dll");
    push_numbered(&mut names, "XInput1_", 1, 3, ".dll");

    names
}

fn push_numbered(names: &mut Vec<String>, prefix: &str, first: u8, last: u8, suffix: &str) {
    names.extend((first..=last).map(|number| format!("{prefix}{number}{suffix}")));
}

#[derive(Debug, Error)]
pub enum LegacyDirectXError {
    #[error("unsupported Windows architecture: {0}")]
    UnsupportedArchitecture(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "neo-directx-legacy-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn populate(directory: &Path, names: &[String]) {
        fs::create_dir_all(directory).unwrap();
        for name in names {
            fs::write(directory.join(name), b"fixture").unwrap();
        }
    }

    #[test]
    fn canonical_component_set_has_no_duplicates_and_expected_extent() {
        let names = expected_component_files();
        let unique = names.iter().collect::<std::collections::BTreeSet<_>>();
        assert_eq!(names.len(), unique.len());
        assert_eq!(names.len(), 90);
        for required in [
            "D3DCompiler_43.dll",
            "D3DCSX_43.dll",
            "D3DX9_43.dll",
            "D3DX10_43.dll",
            "D3DX11_43.dll",
            "X3DAudio1_7.dll",
            "XACTEngine3_7.dll",
            "XAPOFX1_5.dll",
            "XAudio2_7.dll",
            "XInput1_3.dll",
        ] {
            assert!(names.iter().any(|name| name == required));
        }
    }

    #[test]
    fn x64_requires_complete_native_and_x86_component_sets() {
        let root = temp_root("complete-x64");
        let names = expected_component_files();
        populate(&root.join("System32"), &names);
        populate(&root.join("SysWOW64"), &names);

        let report = scan_at(&root, WindowsArchitecture::X64, 64);
        assert_eq!(report.state, LegacyDirectXState::Installed);
        assert!(report.certified_complete());
        assert_eq!(report.expected_files, names.len() * 2);
        assert_eq!(report.present_files, report.expected_files);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn one_missing_component_is_partial_not_installed() {
        let root = temp_root("partial-x64");
        let mut names = expected_component_files();
        populate(&root.join("System32"), &names);
        let missing = names.pop().unwrap();
        populate(&root.join("SysWOW64"), &names);

        let report = scan_at(&root, WindowsArchitecture::X64, 64);
        assert_eq!(report.state, LegacyDirectXState::Partial);
        assert!(!report.certified_complete());
        assert!(report
            .architectures
            .iter()
            .any(|item| item.missing_files.iter().any(|name| name == &missing)));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn empty_existing_directories_are_missing() {
        let root = temp_root("missing-x64");
        fs::create_dir_all(root.join("System32")).unwrap();
        fs::create_dir_all(root.join("SysWOW64")).unwrap();

        let report = scan_at(&root, WindowsArchitecture::X64, 64);
        assert_eq!(report.state, LegacyDirectXState::Missing);
        assert_eq!(report.present_files, 0);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn inaccessible_layout_fails_closed() {
        let root = temp_root("unknown-layout");
        let report = scan_at(&root, WindowsArchitecture::X64, 64);
        assert_eq!(report.state, LegacyDirectXState::Unknown);
        assert!(!report.warnings.is_empty());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn x64_host_with_32_bit_process_fails_closed() {
        let root = temp_root("wow64-ambiguous");
        fs::create_dir_all(root.join("System32")).unwrap();
        fs::create_dir_all(root.join("SysWOW64")).unwrap();
        let report = scan_at(&root, WindowsArchitecture::X64, 32);
        assert_eq!(report.state, LegacyDirectXState::Unknown);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn arm64_stays_unknown_until_layout_is_proven() {
        let root = temp_root("arm64");
        let report = scan_at(&root, WindowsArchitecture::Arm64, 64);
        assert_eq!(report.state, LegacyDirectXState::Unknown);
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn live_windows_scan_classifies_without_mutation() {
        let architecture = WindowsArchitecture::parse(std::env::consts::ARCH).unwrap();
        let report = scan_current(architecture);
        assert!(!report.source.trim().is_empty());
        assert!(matches!(
            report.state,
            LegacyDirectXState::Installed
                | LegacyDirectXState::Partial
                | LegacyDirectXState::Missing
                | LegacyDirectXState::Unknown
        ));
    }
}
