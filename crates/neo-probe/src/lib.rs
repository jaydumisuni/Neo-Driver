//! Read-only Neo system probes.
//!
//! Phase 1 deliberately exposes only observation. Mutation belongs to later
//! transaction-gated crates after baseline capture, authority, rollback, and
//! verification contracts are implemented.

use neo_core::{EvidenceItem, MachineProfile, OsIdentity, SecurityState};
use serde::{Deserialize, Serialize};
use std::process::Command;
use thiserror::Error;

pub trait CommandRunner {
    fn run(&self, program: &str, args: &[&str]) -> Result<CommandEvidence, ProbeError>;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandEvidence {
    pub program: String,
    pub args: Vec<String>,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub start_error: Option<String>,
}

impl CommandEvidence {
    pub fn succeeded(&self) -> bool {
        self.start_error.is_none() && self.exit_code == Some(0)
    }

    pub fn failed_to_start(program: &str, args: &[&str], error: &ProbeError) -> Self {
        Self {
            program: program.to_string(),
            args: args.iter().map(|arg| (*arg).to_string()).collect(),
            exit_code: None,
            stdout: String::new(),
            stderr: error.to_string(),
            start_error: Some(error.to_string()),
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SystemCommandRunner;

impl CommandRunner for SystemCommandRunner {
    fn run(&self, program: &str, args: &[&str]) -> Result<CommandEvidence, ProbeError> {
        let output = Command::new(program)
            .args(args)
            .output()
            .map_err(|source| ProbeError::CommandStart {
                program: program.to_string(),
                detail: source.to_string(),
            })?;

        Ok(CommandEvidence {
            program: program.to_string(),
            args: args.iter().map(|arg| (*arg).to_string()).collect(),
            exit_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            start_error: None,
        })
    }
}

pub trait ReadOnlyProbe {
    fn scan(&self) -> Result<ProbeReport, ProbeError>;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProbeReport {
    pub profile: MachineProfile,
    pub command_evidence: Vec<CommandEvidence>,
}

pub struct WindowsProbe<R> {
    runner: R,
}

impl<R> WindowsProbe<R> {
    pub fn new(runner: R) -> Self {
        Self { runner }
    }
}

impl<R: CommandRunner> WindowsProbe<R> {
    fn capture(&self, program: &str, args: &[&str]) -> CommandEvidence {
        match self.runner.run(program, args) {
            Ok(evidence) => evidence,
            Err(error) => CommandEvidence::failed_to_start(program, args, &error),
        }
    }

    fn query_current_version(&self) -> CommandEvidence {
        self.capture(
            "reg.exe",
            &[
                "query",
                r"HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion",
            ],
        )
    }

    fn query_boot_entry(&self) -> CommandEvidence {
        self.capture("bcdedit.exe", &["/enum", "{current}"])
    }

    fn query_secure_boot(&self) -> CommandEvidence {
        self.capture(
            "reg.exe",
            &[
                "query",
                r"HKLM\SYSTEM\CurrentControlSet\Control\SecureBoot\State",
                "/v",
                "UEFISecureBootEnabled",
            ],
        )
    }

    fn query_memory_integrity(&self) -> CommandEvidence {
        self.capture(
            "reg.exe",
            &[
                "query",
                r"HKLM\SYSTEM\CurrentControlSet\Control\DeviceGuard\Scenarios\HypervisorEnforcedCodeIntegrity",
                "/v",
                "Enabled",
            ],
        )
    }

    fn query_component_reboot_pending(&self) -> CommandEvidence {
        self.capture(
            "reg.exe",
            &[
                "query",
                r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Component Based Servicing\RebootPending",
            ],
        )
    }

    fn query_windows_update_reboot_pending(&self) -> CommandEvidence {
        self.capture(
            "reg.exe",
            &[
                "query",
                r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\WindowsUpdate\Auto Update\RebootRequired",
            ],
        )
    }

    fn query_pending_file_rename(&self) -> CommandEvidence {
        self.capture(
            "reg.exe",
            &[
                "query",
                r"HKLM\SYSTEM\CurrentControlSet\Control\Session Manager",
                "/v",
                "PendingFileRenameOperations",
            ],
        )
    }

    fn query_driver_devices(&self) -> CommandEvidence {
        self.capture(
            "pnputil.exe",
            &["/enum-devices", "/connected", "/deviceids"],
        )
    }

    fn query_problem_devices(&self) -> CommandEvidence {
        self.capture("pnputil.exe", &["/enum-devices", "/problem", "/deviceids"])
    }

    fn query_driver_store(&self) -> CommandEvidence {
        self.capture("pnputil.exe", &["/enum-drivers"])
    }

    fn command_warning(label: &str, evidence: &CommandEvidence) -> Option<String> {
        if evidence.succeeded() {
            None
        } else {
            Some(format!(
                "{label} probe did not complete successfully (exit={:?}, start_error={}); evidence retained",
                evidence.exit_code,
                evidence.start_error.as_deref().unwrap_or("none")
            ))
        }
    }
}

impl<R: CommandRunner> ReadOnlyProbe for WindowsProbe<R> {
    fn scan(&self) -> Result<ProbeReport, ProbeError> {
        let current_version = self.query_current_version();
        let boot_entry = self.query_boot_entry();
        let secure_boot = self.query_secure_boot();
        let memory_integrity = self.query_memory_integrity();
        let component_reboot = self.query_component_reboot_pending();
        let update_reboot = self.query_windows_update_reboot_pending();
        let pending_rename = self.query_pending_file_rename();
        let devices = self.query_driver_devices();
        let problem_devices = self.query_problem_devices();
        let driver_store = self.query_driver_store();

        let os = parse_windows_current_version(&current_version.stdout);
        let mut security = parse_bcd_security_state(&boot_entry.stdout);
        if boot_entry.succeeded() {
            // Microsoft documents TESTSIGNING as not set by default; an omitted
            // element in a successfully enumerated current boot entry therefore
            // means the persistent option is off rather than unknown.
            security.test_signing = Some(security.test_signing.unwrap_or(false));
            security.no_integrity_checks = Some(security.no_integrity_checks.unwrap_or(false));
        }
        security.secure_boot = parse_reg_dword_bool(&secure_boot.stdout, "UEFISecureBootEnabled");
        security.memory_integrity = parse_reg_dword_bool(&memory_integrity.stdout, "Enabled");
        security.pending_reboot =
            determine_pending_reboot(&component_reboot, &update_reboot, &pending_rename);

        let mut profile = MachineProfile {
            os,
            security,
            evidence: vec![
                evidence_status("windows.current_version_probe", &current_version),
                evidence_status("windows.boot_entry_probe", &boot_entry),
                evidence_presence("windows.secure_boot_probe", &secure_boot),
                evidence_presence("windows.memory_integrity_probe", &memory_integrity),
                evidence_presence("windows.cbs_reboot_probe", &component_reboot),
                evidence_presence("windows.update_reboot_probe", &update_reboot),
                evidence_presence("windows.pending_rename_probe", &pending_rename),
                evidence_status("windows.connected_devices_probe", &devices),
                evidence_status("windows.problem_devices_probe", &problem_devices),
                evidence_status("windows.driver_store_probe", &driver_store),
            ],
            warnings: Vec::new(),
        };

        for (label, evidence) in [
            ("Windows identity", &current_version),
            ("BCD security", &boot_entry),
            ("Connected devices", &devices),
            ("Problem devices", &problem_devices),
            ("Driver Store", &driver_store),
        ] {
            if let Some(warning) = Self::command_warning(label, evidence) {
                profile.warnings.push(warning);
            }
        }

        for (label, evidence) in [
            ("Secure Boot", &secure_boot),
            ("Memory Integrity", &memory_integrity),
            ("Component reboot", &component_reboot),
            ("Windows Update reboot", &update_reboot),
            ("Pending file rename", &pending_rename),
        ] {
            if let Some(warning) = expected_absence_warning(label, evidence) {
                profile.warnings.push(warning);
            }
        }

        Ok(ProbeReport {
            profile,
            command_evidence: vec![
                current_version,
                boot_entry,
                secure_boot,
                memory_integrity,
                component_reboot,
                update_reboot,
                pending_rename,
                devices,
                problem_devices,
                driver_store,
            ],
        })
    }
}

fn evidence_status(key: &str, evidence: &CommandEvidence) -> EvidenceItem {
    EvidenceItem::new(
        key,
        if evidence.succeeded() { "ok" } else { "failed" },
        evidence.program.clone(),
    )
}

fn evidence_presence(key: &str, evidence: &CommandEvidence) -> EvidenceItem {
    let status = if evidence.succeeded() {
        "present"
    } else if evidence.start_error.is_none() && evidence.exit_code == Some(1) {
        "absent"
    } else {
        "failed"
    };
    EvidenceItem::new(key, status, evidence.program.clone())
}

fn expected_absence_warning(label: &str, evidence: &CommandEvidence) -> Option<String> {
    if evidence.succeeded() || (evidence.start_error.is_none() && evidence.exit_code == Some(1)) {
        None
    } else {
        Some(format!(
            "{label} probe could not establish presence/absence (exit={:?}, start_error={}); evidence retained",
            evidence.exit_code,
            evidence.start_error.as_deref().unwrap_or("none")
        ))
    }
}

pub fn parse_windows_current_version(output: &str) -> OsIdentity {
    OsIdentity {
        product_name: parse_reg_value(output, "ProductName"),
        display_version: parse_reg_value(output, "DisplayVersion"),
        build_number: parse_reg_value(output, "CurrentBuildNumber")
            .or_else(|| parse_reg_value(output, "CurrentBuild")),
        update_build_revision: parse_reg_value(output, "UBR"),
        installation_type: parse_reg_value(output, "InstallationType"),
        architecture: native_windows_architecture(),
    }
}

pub fn parse_bcd_security_state(output: &str) -> SecurityState {
    SecurityState {
        test_signing: parse_bcd_bool(output, "testsigning"),
        no_integrity_checks: parse_bcd_bool(output, "nointegritychecks"),
        ..SecurityState::default()
    }
}

fn native_windows_architecture() -> Option<String> {
    std::env::var("PROCESSOR_ARCHITEW6432")
        .or_else(|_| std::env::var("PROCESSOR_ARCHITECTURE"))
        .ok()
}

fn parse_reg_value(output: &str, key: &str) -> Option<String> {
    output.lines().find_map(|line| {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return None;
        }

        let mut parts = trimmed.split_whitespace();
        let name = parts.next()?;
        if !name.eq_ignore_ascii_case(key) {
            return None;
        }

        let _reg_type = parts.next()?;
        let value = parts.collect::<Vec<_>>().join(" ");
        (!value.is_empty()).then_some(value)
    })
}

fn parse_reg_dword_bool(output: &str, key: &str) -> Option<bool> {
    let raw = parse_reg_value(output, key)?;
    let raw = raw.trim();
    let value = if let Some(hex) = raw.strip_prefix("0x").or_else(|| raw.strip_prefix("0X")) {
        u64::from_str_radix(hex, 16).ok()?
    } else {
        raw.parse::<u64>().ok()?
    };
    Some(value != 0)
}

fn parse_bcd_bool(output: &str, key: &str) -> Option<bool> {
    output.lines().find_map(|line| {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return None;
        }

        let mut parts = trimmed.split_whitespace();
        let name = parts.next()?;
        if !name.eq_ignore_ascii_case(key) {
            return None;
        }

        let value = parts.collect::<Vec<_>>().join(" ").to_ascii_lowercase();
        match value.as_str() {
            "yes" | "on" | "true" | "1" => Some(true),
            "no" | "off" | "false" | "0" => Some(false),
            _ => None,
        }
    })
}

fn determine_pending_reboot(
    component_reboot: &CommandEvidence,
    update_reboot: &CommandEvidence,
    pending_rename: &CommandEvidence,
) -> Option<bool> {
    if component_reboot.start_error.is_some()
        || update_reboot.start_error.is_some()
        || pending_rename.start_error.is_some()
    {
        return None;
    }

    Some(
        component_reboot.succeeded()
            || update_reboot.succeeded()
            || parse_reg_value(&pending_rename.stdout, "PendingFileRenameOperations").is_some(),
    )
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ProbeError {
    #[error("failed to start '{program}': {detail}")]
    CommandStart { program: String, detail: String },
    #[error("Neo system scan is currently supported on Windows only")]
    UnsupportedPlatform,
}

pub fn scan_current_machine() -> Result<ProbeReport, ProbeError> {
    #[cfg(target_os = "windows")]
    {
        WindowsProbe::new(SystemCommandRunner).scan()
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(ProbeError::UnsupportedPlatform)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::Mutex;

    struct FakeRunner {
        results: Mutex<VecDeque<Result<CommandEvidence, ProbeError>>>,
    }

    impl FakeRunner {
        fn new(results: Vec<Result<CommandEvidence, ProbeError>>) -> Self {
            Self {
                results: Mutex::new(results.into()),
            }
        }
    }

    impl CommandRunner for FakeRunner {
        fn run(&self, _program: &str, _args: &[&str]) -> Result<CommandEvidence, ProbeError> {
            self.results
                .lock()
                .expect("fake runner lock")
                .pop_front()
                .expect("fixture command evidence")
        }
    }

    fn evidence(program: &str, stdout: &str) -> Result<CommandEvidence, ProbeError> {
        Ok(CommandEvidence {
            program: program.to_string(),
            args: vec![],
            exit_code: Some(0),
            stdout: stdout.to_string(),
            stderr: String::new(),
            start_error: None,
        })
    }

    fn absent(program: &str) -> Result<CommandEvidence, ProbeError> {
        Ok(CommandEvidence {
            program: program.to_string(),
            args: vec![],
            exit_code: Some(1),
            stdout: String::new(),
            stderr: "not found".to_string(),
            start_error: None,
        })
    }

    #[test]
    fn parses_windows_current_version_registry_output() {
        let output = r#"
HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Windows NT\CurrentVersion
    ProductName    REG_SZ    Windows 11 Pro
    DisplayVersion    REG_SZ    24H2
    CurrentBuildNumber    REG_SZ    26100
    UBR    REG_DWORD    0x1234
    InstallationType    REG_SZ    Client
"#;

        let parsed = parse_windows_current_version(output);
        assert_eq!(parsed.product_name.as_deref(), Some("Windows 11 Pro"));
        assert_eq!(parsed.display_version.as_deref(), Some("24H2"));
        assert_eq!(parsed.build_number.as_deref(), Some("26100"));
        assert_eq!(parsed.update_build_revision.as_deref(), Some("0x1234"));
        assert_eq!(parsed.installation_type.as_deref(), Some("Client"));
    }

    #[test]
    fn parses_bcd_security_flags_without_conflating_them() {
        let output = r#"
Windows Boot Loader
-------------------
identifier              {current}
testsigning             Yes
nointegritychecks       No
"#;

        let parsed = parse_bcd_security_state(output);
        assert_eq!(parsed.test_signing, Some(true));
        assert_eq!(parsed.no_integrity_checks, Some(false));
    }

    #[test]
    fn successful_bcd_scan_without_optional_flags_defaults_them_off() {
        let runner = FakeRunner::new(vec![
            evidence("reg.exe", "ProductName REG_SZ Windows 11 Pro"),
            evidence("bcdedit.exe", "identifier {current}"),
            absent("reg.exe"),
            absent("reg.exe"),
            absent("reg.exe"),
            absent("reg.exe"),
            absent("reg.exe"),
            evidence("pnputil.exe", "device evidence"),
            evidence("pnputil.exe", "problem evidence"),
            evidence("pnputil.exe", "driver evidence"),
        ]);

        let report = WindowsProbe::new(runner).scan().expect("scan");
        assert_eq!(report.profile.security.test_signing, Some(false));
        assert_eq!(report.profile.security.no_integrity_checks, Some(false));
    }

    #[test]
    fn parses_registry_dword_boolean() {
        let output = "UEFISecureBootEnabled    REG_DWORD    0x1";
        assert_eq!(
            parse_reg_dword_bool(output, "UEFISecureBootEnabled"),
            Some(true)
        );

        let output = "Enabled    REG_DWORD    0x0";
        assert_eq!(parse_reg_dword_bool(output, "Enabled"), Some(false));
    }

    #[test]
    fn pending_reboot_is_true_when_any_supported_indicator_exists() {
        let present = evidence("reg.exe", "HKEY_LOCAL_MACHINE\\...\\RebootPending").unwrap();
        let missing = absent("reg.exe").unwrap();
        assert_eq!(
            determine_pending_reboot(&present, &missing, &missing),
            Some(true)
        );
    }

    #[test]
    fn pending_reboot_is_unknown_when_a_probe_cannot_start() {
        let error = ProbeError::CommandStart {
            program: "reg.exe".to_string(),
            detail: "fixture".to_string(),
        };
        let failed = CommandEvidence::failed_to_start("reg.exe", &["query"], &error);
        let missing = absent("reg.exe").unwrap();
        assert_eq!(determine_pending_reboot(&failed, &missing, &missing), None);
    }

    #[test]
    fn scan_retains_evidence_from_all_read_only_lanes() {
        let runner = FakeRunner::new(vec![
            evidence(
                "reg.exe",
                "ProductName    REG_SZ    Windows 11 Pro\nCurrentBuildNumber REG_SZ 26100",
            ),
            evidence("bcdedit.exe", "testsigning No\nnointegritychecks No"),
            evidence("reg.exe", "UEFISecureBootEnabled REG_DWORD 0x1"),
            evidence("reg.exe", "Enabled REG_DWORD 0x1"),
            absent("reg.exe"),
            absent("reg.exe"),
            absent("reg.exe"),
            evidence("pnputil.exe", "Instance ID: PCI\\VEN_8086&DEV_0000"),
            evidence("pnputil.exe", "No problem devices"),
            evidence("pnputil.exe", "Published Name: oem1.inf"),
        ]);

        let report = WindowsProbe::new(runner).scan().expect("scan");
        assert_eq!(report.command_evidence.len(), 10);
        assert_eq!(report.profile.security.test_signing, Some(false));
        assert_eq!(report.profile.security.secure_boot, Some(true));
        assert_eq!(report.profile.security.memory_integrity, Some(true));
        assert_eq!(report.profile.security.pending_reboot, Some(false));
        assert_eq!(report.profile.evidence.len(), 10);
    }

    #[test]
    fn one_failed_command_start_does_not_abort_other_probe_lanes() {
        let start_error = Err(ProbeError::CommandStart {
            program: "bcdedit.exe".to_string(),
            detail: "fixture missing".to_string(),
        });
        let runner = FakeRunner::new(vec![
            evidence("reg.exe", "ProductName REG_SZ Windows 11 Pro"),
            start_error,
            evidence("reg.exe", "UEFISecureBootEnabled REG_DWORD 0x1"),
            evidence("reg.exe", "Enabled REG_DWORD 0x1"),
            absent("reg.exe"),
            absent("reg.exe"),
            absent("reg.exe"),
            evidence("pnputil.exe", "device evidence"),
            evidence("pnputil.exe", "problem evidence"),
            evidence("pnputil.exe", "driver evidence"),
        ]);

        let report = WindowsProbe::new(runner).scan().expect("scan continues");
        assert_eq!(report.command_evidence.len(), 10);
        assert!(report.command_evidence[1].start_error.is_some());
        assert!(report
            .profile
            .warnings
            .iter()
            .any(|warning| warning.contains("BCD security")));
    }
}
