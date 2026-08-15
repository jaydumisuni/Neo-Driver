//! Phase 14 read-only Windows AppX inventory adapter for Neo Driver.
//!
//! The adapter executes only two fixed Microsoft-supported inventory commands:
//! current-user `Get-AppxPackage` and online `Get-AppxProvisionedPackage`.
//! Catalogue identities are never interpolated into command text. Neo enumerates
//! first and matches identities in Rust. No package mutation authority exists here.

use neo_debloat::{
    DebloatCatalogue, DebloatError, DebloatEvidence, DebloatObservation, ObservedPresence,
};
use neo_probe::{CommandEvidence, CommandRunner, SystemCommandRunner};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

const INSTALLED_SCRIPT: &str = "$ErrorActionPreference='Stop'; $items=@(Get-AppxPackage -PackageTypeFilter Bundle,Framework,Main,Resource,Optional | Select-Object Name,@{Name='Version';Expression={$_.Version.ToString()}}); ConvertTo-Json -InputObject $items -Compress -Depth 3";
const PROVISIONED_SCRIPT: &str = "$ErrorActionPreference='Stop'; $items=@(Get-AppxProvisionedPackage -Online | Select-Object DisplayName,PackageName); ConvertTo-Json -InputObject $items -Compress -Depth 3";
const POWERSHELL_ARGS_PREFIX: [&str; 5] =
    ["-NoLogo", "-NoProfile", "-NonInteractive", "-Command", ""];

#[derive(Debug, Error)]
pub enum DebloatProbeError {
    #[error(transparent)]
    Debloat(#[from] DebloatError),
    #[error("Neo live debloat inventory is currently supported on Windows only")]
    UnsupportedPlatform,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebloatProbeReport {
    pub evidence: DebloatEvidence,
    pub command_evidence: Vec<CommandEvidence>,
    #[serde(default)]
    pub warnings: Vec<String>,
    pub machine_changes: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct InstalledPackageRecord {
    #[serde(rename = "Name")]
    name: Option<String>,
    #[serde(rename = "Version")]
    version: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct ProvisionedPackageRecord {
    #[serde(rename = "DisplayName")]
    display_name: Option<String>,
}

pub struct WindowsDebloatProbe<R> {
    runner: R,
}

impl<R> WindowsDebloatProbe<R> {
    pub fn new(runner: R) -> Self {
        Self { runner }
    }
}

impl<R: CommandRunner> WindowsDebloatProbe<R> {
    fn capture_script(&self, script: &'static str) -> CommandEvidence {
        let mut args = POWERSHELL_ARGS_PREFIX;
        args[4] = script;
        match self.runner.run("powershell.exe", &args) {
            Ok(evidence) => evidence,
            Err(error) => CommandEvidence::failed_to_start("powershell.exe", &args, &error),
        }
    }

    pub fn scan(
        &self,
        catalogue: &DebloatCatalogue,
    ) -> Result<DebloatProbeReport, DebloatProbeError> {
        let installed_command = self.capture_script(INSTALLED_SCRIPT);
        let provisioned_command = self.capture_script(PROVISIONED_SCRIPT);
        let mut warnings = Vec::new();

        let installed_records = parse_records::<InstalledPackageRecord>(
            "current-user AppX inventory",
            &installed_command,
            &mut warnings,
        );
        let provisioned_records = parse_records::<ProvisionedPackageRecord>(
            "provisioned AppX inventory",
            &provisioned_command,
            &mut warnings,
        );

        let installed_index = installed_records
            .as_ref()
            .map(|records| index_installed(records, &mut warnings));
        let provisioned_index = provisioned_records
            .as_ref()
            .map(|records| index_provisioned(records));

        let observations = catalogue
            .items()
            .iter()
            .map(|definition| {
                let key = canonical(&definition.package_id);
                let installed = match &installed_index {
                    Some(index) if index.contains_key(&key) => ObservedPresence::Present,
                    Some(_) => ObservedPresence::Absent,
                    None => ObservedPresence::Unavailable,
                };
                let provisioned = match &provisioned_index {
                    Some(index) if index.contains(&key) => ObservedPresence::Present,
                    Some(_) => ObservedPresence::Absent,
                    None => ObservedPresence::Unavailable,
                };
                let version = installed_index
                    .as_ref()
                    .and_then(|index| index.get(&key))
                    .and_then(unique_version);

                DebloatObservation {
                    package_id: definition.package_id.clone(),
                    installed,
                    provisioned,
                    version,
                    source: "neo-debloat-probe:fixed-windows-appx-inventory".to_string(),
                }
            })
            .collect();

        Ok(DebloatProbeReport {
            evidence: DebloatEvidence::new(observations)?,
            command_evidence: vec![installed_command, provisioned_command],
            warnings,
            machine_changes: false,
        })
    }
}

fn parse_records<T: for<'de> Deserialize<'de>>(
    label: &str,
    evidence: &CommandEvidence,
    warnings: &mut Vec<String>,
) -> Option<Vec<T>> {
    if !evidence.succeeded() {
        warnings.push(format!(
            "{label} could not be established (exit={:?}, start_error={}); state remains Unavailable",
            evidence.exit_code,
            evidence.start_error.as_deref().unwrap_or("none")
        ));
        return None;
    }

    match serde_json::from_str::<Vec<T>>(evidence.stdout.trim()) {
        Ok(records) => Some(records),
        Err(error) => {
            warnings.push(format!(
                "{label} returned malformed JSON ({error}); state remains Unavailable"
            ));
            None
        }
    }
}

fn index_installed(
    records: &[InstalledPackageRecord],
    warnings: &mut Vec<String>,
) -> BTreeMap<String, BTreeSet<String>> {
    let mut index = BTreeMap::<String, BTreeSet<String>>::new();
    for record in records {
        let Some(name) = record
            .name
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            warnings.push("current-user AppX record missing Name; record ignored".to_string());
            continue;
        };
        let versions = index.entry(canonical(name)).or_default();
        if let Some(version) = record
            .version
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            versions.insert(version.to_string());
        }
    }
    index
}

fn index_provisioned(records: &[ProvisionedPackageRecord]) -> BTreeSet<String> {
    records
        .iter()
        .filter_map(|record| record.display_name.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(canonical)
        .collect()
}

fn unique_version(versions: &BTreeSet<String>) -> Option<String> {
    (versions.len() == 1)
        .then(|| versions.iter().next().cloned())
        .flatten()
}

fn canonical(value: &str) -> String {
    value.to_ascii_lowercase()
}

pub fn scan_current_debloat_evidence(
    catalogue: &DebloatCatalogue,
) -> Result<DebloatProbeReport, DebloatProbeError> {
    #[cfg(target_os = "windows")]
    {
        WindowsDebloatProbe::new(SystemCommandRunner).scan(catalogue)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = catalogue;
        Err(DebloatProbeError::UnsupportedPlatform)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use neo_probe::ProbeError;
    use std::cell::RefCell;

    struct FakeRunner {
        calls: RefCell<Vec<(String, Vec<String>)>>,
        installed: CommandEvidence,
        provisioned: CommandEvidence,
    }

    impl CommandRunner for FakeRunner {
        fn run(&self, program: &str, args: &[&str]) -> Result<CommandEvidence, ProbeError> {
            self.calls.borrow_mut().push((
                program.to_string(),
                args.iter().map(|value| (*value).to_string()).collect(),
            ));
            if args.last() == Some(&PROVISIONED_SCRIPT) {
                Ok(self.provisioned.clone())
            } else {
                Ok(self.installed.clone())
            }
        }
    }

    fn command(stdout: &str, exit_code: i32) -> CommandEvidence {
        CommandEvidence {
            program: "powershell.exe".to_string(),
            args: Vec::new(),
            exit_code: Some(exit_code),
            stdout: stdout.to_string(),
            stderr: String::new(),
            start_error: None,
        }
    }

    fn catalogue() -> DebloatCatalogue {
        serde_json::from_str(
            r#"{"items":[{"id":"appx.contoso.optional","package_id":"Contoso.Optional","title":"Fixture","category":"Fixture","description":"Fixture package","class":"safe_optional","scope":"current_user_and_provisioned","risk":"low","recommendation":"optional_component","verdict":"certified","selected_by_default":true,"restore":{"kind":"store","store_id":"9CONTOSO1"},"side_effects":[],"preserve_in_profiles":[]}]}"#,
        )
        .expect("fixture catalogue must validate")
    }

    #[test]
    fn fixed_inventory_is_matched_in_rust_without_package_interpolation() {
        let runner = FakeRunner {
            calls: RefCell::new(Vec::new()),
            installed: command(r#"[{"Name":"CONTOSO.OPTIONAL","Version":"1.2.3.4"}]"#, 0),
            provisioned: command(r#"[{"DisplayName":"contoso.optional"}]"#, 0),
        };
        let report = WindowsDebloatProbe::new(runner)
            .scan(&catalogue())
            .expect("scan must succeed");
        let observation = &report.evidence.observations()[0];
        assert_eq!(observation.installed, ObservedPresence::Present);
        assert_eq!(observation.provisioned, ObservedPresence::Present);
        assert_eq!(observation.version.as_deref(), Some("1.2.3.4"));
        assert!(!report.machine_changes);

        for evidence in &report.command_evidence {
            assert!(!evidence
                .args
                .iter()
                .any(|arg| arg.contains("Contoso.Optional")));
        }
    }

    #[test]
    fn failed_inventory_never_becomes_false_absence() {
        let runner = FakeRunner {
            calls: RefCell::new(Vec::new()),
            installed: command("[]", 0),
            provisioned: command("access denied", 1),
        };
        let report = WindowsDebloatProbe::new(runner)
            .scan(&catalogue())
            .expect("scan must normalize query failure");
        let observation = &report.evidence.observations()[0];
        assert_eq!(observation.installed, ObservedPresence::Absent);
        assert_eq!(observation.provisioned, ObservedPresence::Unavailable);
        assert!(!report.warnings.is_empty());
    }

    #[test]
    fn malformed_success_output_remains_unavailable() {
        let runner = FakeRunner {
            calls: RefCell::new(Vec::new()),
            installed: command("not-json", 0),
            provisioned: command("[]", 0),
        };
        let report = WindowsDebloatProbe::new(runner)
            .scan(&catalogue())
            .expect("scan must fail closed on malformed evidence");
        let observation = &report.evidence.observations()[0];
        assert_eq!(observation.installed, ObservedPresence::Unavailable);
        assert_eq!(observation.provisioned, ObservedPresence::Absent);
    }
}
