//! Read-only Windows runtime evidence collection for Neo Driver.
//!
//! This crate is an adapter between the existing `neo-probe` command-evidence
//! boundary and the pure `neo-runtime` assessment model. It does not install,
//! repair, download, enable, disable, or otherwise mutate Windows state.

use neo_directx_legacy::{
    scan_current as scan_legacy_directx, LegacyDirectXReport, LegacyDirectXState,
    WindowsArchitecture as LegacyDirectXArchitecture,
};
use neo_probe::{
    scan_current_machine, CommandEvidence, CommandRunner, ProbeError, SystemCommandRunner,
};
use neo_runtime::{
    RuntimeComponent, RuntimeError, RuntimeInventory, RuntimeObservation, RuntimeState,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

const WEBVIEW2_PRODUCT_GUID: &str = "{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeProbeReport {
    pub inventory: RuntimeInventory,
    pub command_evidence: Vec<CommandEvidence>,
    #[serde(default)]
    pub warnings: Vec<String>,
}

pub struct WindowsRuntimeProbe<R> {
    runner: R,
}

impl<R> WindowsRuntimeProbe<R> {
    pub fn new(runner: R) -> Self {
        Self { runner }
    }
}

impl<R: CommandRunner> WindowsRuntimeProbe<R> {
    fn capture(&self, program: &str, args: &[&str]) -> CommandEvidence {
        match self.runner.run(program, args) {
            Ok(evidence) => evidence,
            Err(error) => CommandEvidence::failed_to_start(program, args, &error),
        }
    }

    pub fn scan_for_host(
        &self,
        windows_build: u32,
        architecture: &str,
    ) -> Result<RuntimeProbeReport, RuntimeProbeError> {
        let canonical_architecture = canonical_architecture(architecture)
            .ok_or_else(|| RuntimeProbeError::UnsupportedArchitecture(architecture.to_string()))?;
        let directx_architecture = LegacyDirectXArchitecture::parse(canonical_architecture)
            .map_err(|_| RuntimeProbeError::UnsupportedArchitecture(architecture.to_string()))?;
        let legacy_directx = scan_legacy_directx(directx_architecture);

        let vc_x86_native = self.capture(
            "reg.exe",
            &[
                "query",
                r"HKLM\SOFTWARE\Microsoft\VisualStudio\14.0\VC\Runtimes\x86",
                "/v",
                "Version",
            ],
        );
        let vc_x86_wow = self.capture(
            "reg.exe",
            &[
                "query",
                r"HKLM\SOFTWARE\Wow6432Node\Microsoft\VisualStudio\14.0\VC\Runtimes\x86",
                "/v",
                "Version",
            ],
        );
        let vc_x64_native = self.capture(
            "reg.exe",
            &[
                "query",
                r"HKLM\SOFTWARE\Microsoft\VisualStudio\14.0\VC\Runtimes\x64",
                "/v",
                "Version",
            ],
        );
        let vc_x64_wow = self.capture(
            "reg.exe",
            &[
                "query",
                r"HKLM\SOFTWARE\Wow6432Node\Microsoft\VisualStudio\14.0\VC\Runtimes\x64",
                "/v",
                "Version",
            ],
        );
        let netfx4 = self.capture(
            "reg.exe",
            &[
                "query",
                r"HKLM\SOFTWARE\Microsoft\NET Framework Setup\NDP\v4\Full",
                "/v",
                "Release",
            ],
        );
        let dotnet = self.capture("dotnet.exe", &["--list-runtimes"]);

        let webview_machine_key = if canonical_architecture == "x86" {
            format!(r"HKLM\SOFTWARE\Microsoft\EdgeUpdate\Clients\{WEBVIEW2_PRODUCT_GUID}")
        } else {
            format!(
                r"HKLM\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{WEBVIEW2_PRODUCT_GUID}"
            )
        };
        let webview_user_key =
            format!(r"HKCU\Software\Microsoft\EdgeUpdate\Clients\{WEBVIEW2_PRODUCT_GUID}");
        let webview_machine = self.capture(
            "reg.exe",
            &["query", webview_machine_key.as_str(), "/v", "pv"],
        );
        let webview_user =
            self.capture("reg.exe", &["query", webview_user_key.as_str(), "/v", "pv"]);

        let netfx3 = self.capture(
            "dism.exe",
            &[
                "/Online",
                "/Get-FeatureInfo",
                "/FeatureName:NetFx3",
                "/English",
            ],
        );
        let directplay = self.capture(
            "dism.exe",
            &[
                "/Online",
                "/Get-FeatureInfo",
                "/FeatureName:DirectPlay",
                "/English",
            ],
        );

        // `py -0p` is a listing operation retained for legacy-launcher
        // compatibility by the current Python Install Manager. Do not replace
        // it with a bare `python` or `py` launch: current Python can install a
        // runtime on demand when none is present.
        let python_list = self.capture("py.exe", &["-0p"]);
        let python_path = self.capture("where.exe", &["python.exe"]);
        let py_path = self.capture("where.exe", &["py.exe"]);
        let pip_path = self.capture("where.exe", &["pip.exe"]);

        let observations = vec![
            classify_registry_runtime(
                RuntimeComponent::VcRedist2015PlusX86,
                "Microsoft Visual C++ v14 x86 registry",
                "Version",
                &[&vc_x86_native, &vc_x86_wow],
            ),
            if canonical_architecture == "x86" {
                unknown_observation(
                    RuntimeComponent::VcRedist2015PlusX64,
                    "neo-runtime-probe:vc-x64-host-policy-pending",
                    "The current Phase 6 profile law is x64-oriented; Neo does not invent x64-runtime applicability on a 32-bit Windows host.",
                )
            } else {
                classify_registry_runtime(
                    RuntimeComponent::VcRedist2015PlusX64,
                    "Microsoft Visual C++ v14 x64 registry",
                    "Version",
                    &[&vc_x64_native, &vc_x64_wow],
                )
            },
            classify_legacy_directx(&legacy_directx),
            classify_windows_feature(RuntimeComponent::DotNetFramework35, "NetFx3", &netfx3),
            classify_netfx4(&netfx4),
            classify_dotnet_runtime(RuntimeComponent::DotNetRuntime, "Microsoft.NETCore.App", &dotnet),
            classify_dotnet_runtime(
                RuntimeComponent::DotNetDesktopRuntime,
                "Microsoft.WindowsDesktop.App",
                &dotnet,
            ),
            classify_python(&python_list, &python_path, &py_path, &pip_path),
            classify_webview2(&[&webview_machine, &webview_user]),
            unknown_observation(
                RuntimeComponent::XnaFramework40Refresh,
                "neo-runtime-probe:xna-predicate-pending",
                "No verified XNA installation predicate is frozen yet; Neo reports Unknown rather than guessing.",
            ),
            unknown_observation(
                RuntimeComponent::OpenAl,
                "neo-runtime-probe:openal-predicate-pending",
                "No verified OpenAL installation predicate is frozen yet; Neo reports Unknown rather than guessing.",
            ),
            unknown_observation(
                RuntimeComponent::Physx,
                "neo-runtime-probe:physx-predicate-pending",
                "No verified PhysX installation predicate is frozen yet; Neo reports Unknown rather than guessing.",
            ),
            unknown_observation(
                RuntimeComponent::PhysxLegacy,
                "neo-runtime-probe:physx-legacy-predicate-pending",
                "No verified PhysX Legacy installation predicate is frozen yet; Neo reports Unknown rather than guessing.",
            ),
            classify_windows_feature(RuntimeComponent::DirectPlay, "DirectPlay", &directplay),
        ];

        let inventory = RuntimeInventory {
            windows_build,
            architecture: canonical_architecture.to_string(),
            observations,
        };
        inventory.validate()?;

        let command_evidence = vec![
            vc_x86_native,
            vc_x86_wow,
            vc_x64_native,
            vc_x64_wow,
            netfx4,
            dotnet,
            webview_machine,
            webview_user,
            netfx3,
            directplay,
            python_list,
            python_path,
            py_path,
            pip_path,
        ];
        let mut warnings: Vec<String> = command_evidence
            .iter()
            .filter(|item| !item.succeeded() && !known_absent(item))
            .map(|item| {
                format!(
                    "runtime probe {} {:?} could not establish a normal result (exit={:?}, start_error={}); raw evidence retained",
                    item.program,
                    item.args,
                    item.exit_code,
                    item.start_error.as_deref().unwrap_or("none")
                )
            })
            .collect();
        warnings.extend(legacy_directx.warnings.iter().map(|warning| {
            format!("legacy DirectX probe could not certify completeness: {warning}")
        }));

        Ok(RuntimeProbeReport {
            inventory,
            command_evidence,
            warnings,
        })
    }
}

pub fn scan_current_runtime_inventory() -> Result<RuntimeProbeReport, RuntimeProbeError> {
    let host = scan_current_machine()?;
    let build = host
        .profile
        .os
        .build_number
        .as_deref()
        .and_then(|value| value.trim().parse::<u32>().ok())
        .ok_or(RuntimeProbeError::MissingWindowsBuild)?;
    let architecture = host
        .profile
        .os
        .architecture
        .as_deref()
        .ok_or(RuntimeProbeError::MissingArchitecture)?;
    WindowsRuntimeProbe::new(SystemCommandRunner).scan_for_host(build, architecture)
}

fn classify_legacy_directx(report: &LegacyDirectXReport) -> RuntimeObservation {
    let state = match report.state {
        LegacyDirectXState::Installed => RuntimeState::Installed,
        LegacyDirectXState::Partial => RuntimeState::Partial,
        LegacyDirectXState::Missing => RuntimeState::Missing,
        LegacyDirectXState::Unknown => RuntimeState::Unknown,
    };
    let mut details = vec![
        format!("expected_files={}", report.expected_files),
        format!("present_files={}", report.present_files),
    ];
    for architecture in &report.architectures {
        details.push(format!(
            "architecture={:?};present={}/{};missing={}",
            architecture.architecture,
            architecture.present_files,
            architecture.expected_files,
            architecture.missing_files.join(",")
        ));
    }
    details.extend(report.warnings.iter().cloned());
    RuntimeObservation {
        component: RuntimeComponent::DirectXLegacyJune2010,
        state,
        detected_version: (state == RuntimeState::Installed)
            .then_some("June 2010 legacy framework component set".to_string()),
        source: report.source.clone(),
        details,
    }
}

fn classify_registry_runtime(
    component: RuntimeComponent,
    source: &str,
    value_name: &str,
    evidence: &[&CommandEvidence],
) -> RuntimeObservation {
    let versions: Vec<String> = evidence
        .iter()
        .filter(|item| item.succeeded())
        .filter_map(|item| parse_reg_value(&item.stdout, value_name))
        .filter(|value| !value.trim().is_empty())
        .collect();
    if !versions.is_empty() {
        return observation(component, RuntimeState::Installed, source, versions, None);
    }
    if evidence.iter().any(|item| item.succeeded()) {
        return observation(
            component,
            RuntimeState::Broken,
            source,
            vec![],
            Some("Runtime registry key exists but the documented Version value is absent."),
        );
    }
    if evidence.iter().all(|item| known_absent(item)) {
        return observation(component, RuntimeState::Missing, source, vec![], None);
    }
    unknown_observation(
        component,
        source,
        "Registry evidence could not prove installed or absent state.",
    )
}

fn classify_netfx4(evidence: &CommandEvidence) -> RuntimeObservation {
    let source = "Microsoft .NET Framework v4 Full Release registry";
    if evidence.succeeded() {
        if let Some(release) = parse_reg_value(&evidence.stdout, "Release") {
            if parse_reg_number(&release).is_some() {
                return observation(
                    RuntimeComponent::DotNetFramework4,
                    RuntimeState::Installed,
                    source,
                    vec![format!("Release={release}")],
                    None,
                );
            }
        }
        return observation(
            RuntimeComponent::DotNetFramework4,
            RuntimeState::Broken,
            source,
            vec![],
            Some(".NET Framework v4 Full key exists but a valid Release DWORD was not recovered."),
        );
    }
    if known_absent(evidence) {
        return observation(
            RuntimeComponent::DotNetFramework4,
            RuntimeState::Missing,
            source,
            vec![],
            None,
        );
    }
    unknown_observation(
        RuntimeComponent::DotNetFramework4,
        source,
        ".NET Framework registry evidence could not prove installed or absent state.",
    )
}

fn classify_dotnet_runtime(
    component: RuntimeComponent,
    family: &str,
    evidence: &CommandEvidence,
) -> RuntimeObservation {
    let source = "dotnet --list-runtimes";
    if !evidence.succeeded() {
        return unknown_observation(
            component,
            source,
            "The dotnet host was unavailable or failed; Neo cannot infer that all .NET runtimes are absent.",
        );
    }
    let versions: Vec<String> = evidence
        .stdout
        .lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let name = parts.next()?;
            let version = parts.next()?;
            name.eq_ignore_ascii_case(family)
                .then_some(version.to_string())
        })
        .collect();
    if versions.is_empty() {
        observation(component, RuntimeState::Missing, source, vec![], None)
    } else {
        observation(component, RuntimeState::Installed, source, versions, None)
    }
}

fn classify_webview2(evidence: &[&CommandEvidence]) -> RuntimeObservation {
    let source = "Microsoft WebView2 EdgeUpdate pv registry";
    let versions: Vec<String> = evidence
        .iter()
        .filter(|item| item.succeeded())
        .filter_map(|item| parse_reg_value(&item.stdout, "pv"))
        .filter(|value| {
            let trimmed = value.trim();
            !trimmed.is_empty() && trimmed != "0.0.0.0"
        })
        .collect();
    if !versions.is_empty() {
        return observation(
            RuntimeComponent::WebView2,
            RuntimeState::Installed,
            source,
            versions,
            None,
        );
    }
    if evidence.iter().all(|item| known_absent(item)) {
        return observation(
            RuntimeComponent::WebView2,
            RuntimeState::Missing,
            source,
            vec![],
            None,
        );
    }
    if evidence.iter().any(|item| item.succeeded()) {
        return observation(
            RuntimeComponent::WebView2,
            RuntimeState::Broken,
            source,
            vec![],
            Some("A WebView2 client key was found without a usable non-zero pv value."),
        );
    }
    unknown_observation(
        RuntimeComponent::WebView2,
        source,
        "WebView2 registry evidence could not prove installed or absent state.",
    )
}

fn classify_windows_feature(
    component: RuntimeComponent,
    feature_name: &str,
    evidence: &CommandEvidence,
) -> RuntimeObservation {
    let source = format!("DISM /Get-FeatureInfo {feature_name} /English");
    if !evidence.succeeded() {
        return unknown_observation(
            component,
            &source,
            "DISM feature query did not complete successfully; Neo does not infer absence.",
        );
    }
    match parse_dism_feature_state(&evidence.stdout) {
        Some(FeatureState::Enabled) => observation(
            component,
            RuntimeState::Installed,
            &source,
            vec!["State=Enabled".to_string()],
            None,
        ),
        Some(FeatureState::Disabled) => observation(
            component,
            RuntimeState::Missing,
            &source,
            vec!["State=Disabled".to_string()],
            None,
        ),
        Some(FeatureState::PayloadRemoved) => observation(
            component,
            RuntimeState::Missing,
            &source,
            vec!["State=DisabledWithPayloadRemoved".to_string()],
            Some("Windows feature payload is not currently enabled/present."),
        ),
        None => unknown_observation(
            component,
            &source,
            "DISM succeeded but Neo could not parse a recognized feature State value.",
        ),
    }
}

fn classify_python(
    python_list: &CommandEvidence,
    python_path: &CommandEvidence,
    py_path: &CommandEvidence,
    pip_path: &CommandEvidence,
) -> RuntimeObservation {
    let source = "Python launcher/install-manager listing plus PATH discovery";
    let runtime_paths: Vec<String> = python_list
        .stdout
        .lines()
        .map(str::trim)
        .filter(|line| line.to_ascii_lowercase().contains("python.exe"))
        .map(ToOwned::to_owned)
        .collect();
    let python_on_path = python_path.succeeded() && !python_path.stdout.trim().is_empty();
    let py_on_path = py_path.succeeded() && !py_path.stdout.trim().is_empty();
    let pip_on_path = pip_path.succeeded() && !pip_path.stdout.trim().is_empty();

    if python_list.succeeded() && !runtime_paths.is_empty() {
        let mut details = runtime_paths;
        details.push(format!("python_on_path={python_on_path}"));
        details.push(format!("py_on_path={py_on_path}"));
        details.push(format!("pip_on_path={pip_on_path}"));
        let state = if python_on_path && py_on_path {
            RuntimeState::Installed
        } else {
            RuntimeState::Partial
        };
        return observation(RuntimeComponent::Python, state, source, details, None);
    }

    if python_on_path || py_on_path || pip_on_path {
        return observation(
            RuntimeComponent::Python,
            RuntimeState::Partial,
            source,
            vec![
                format!("python_on_path={python_on_path}"),
                format!("py_on_path={py_on_path}"),
                format!("pip_on_path={pip_on_path}"),
            ],
            Some("Python-related commands exist, but registered runtime enumeration is incomplete or unavailable."),
        );
    }

    unknown_observation(
        RuntimeComponent::Python,
        source,
        "No global Python command evidence was recovered, but Neo does not treat PATH absence as proof that no Python runtime exists elsewhere.",
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FeatureState {
    Enabled,
    Disabled,
    PayloadRemoved,
}

fn parse_dism_feature_state(output: &str) -> Option<FeatureState> {
    output.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        if !name.trim().eq_ignore_ascii_case("State") {
            return None;
        }
        let normalized = value.trim().to_ascii_lowercase().replace([' ', '-'], "");
        match normalized.as_str() {
            "enabled" | "enablepending" => Some(FeatureState::Enabled),
            "disabled" | "disablepending" => Some(FeatureState::Disabled),
            "disabledwithpayloadremoved" | "removed" => Some(FeatureState::PayloadRemoved),
            _ => None,
        }
    })
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

fn parse_reg_number(raw: &str) -> Option<u64> {
    let value = raw.trim();
    if let Some(hex) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        u64::from_str_radix(hex, 16).ok()
    } else {
        value.parse::<u64>().ok()
    }
}

fn observation(
    component: RuntimeComponent,
    state: RuntimeState,
    source: &str,
    details: Vec<String>,
    warning: Option<&str>,
) -> RuntimeObservation {
    let mut details = details;
    if let Some(warning) = warning {
        details.push(warning.to_string());
    }
    let detected_version = details.iter().find_map(|item| {
        if item.starts_with("State=") || item.contains("_on_path=") {
            None
        } else if let Some(value) = item.strip_prefix("Release=") {
            Some(value.to_string())
        } else if item.starts_with("Runtime registry") {
            None
        } else {
            Some(item.clone())
        }
    });
    RuntimeObservation {
        component,
        state,
        detected_version,
        source: source.to_string(),
        details,
    }
}

fn unknown_observation(
    component: RuntimeComponent,
    source: &str,
    detail: &str,
) -> RuntimeObservation {
    RuntimeObservation {
        component,
        state: RuntimeState::Unknown,
        detected_version: None,
        source: source.to_string(),
        details: vec![detail.to_string()],
    }
}

fn known_absent(evidence: &CommandEvidence) -> bool {
    evidence.start_error.is_none() && evidence.exit_code == Some(1)
}

fn canonical_architecture(value: &str) -> Option<&'static str> {
    match value.trim().to_ascii_lowercase().as_str() {
        "x64" | "amd64" | "x86_64" => Some("x64"),
        "x86" | "i386" | "i686" => Some("x86"),
        "arm64" | "aarch64" => Some("arm64"),
        _ => None,
    }
}

#[derive(Debug, Error)]
pub enum RuntimeProbeError {
    #[error(transparent)]
    Probe(#[from] ProbeError),
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
    #[error("Windows build evidence was not available from the base Neo probe")]
    MissingWindowsBuild,
    #[error("Windows architecture evidence was not available from the base Neo probe")]
    MissingArchitecture,
    #[error("unsupported Windows architecture for runtime probing: {0}")]
    UnsupportedArchitecture(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn success(stdout: &str) -> CommandEvidence {
        CommandEvidence {
            program: "fixture".to_string(),
            args: vec![],
            exit_code: Some(0),
            stdout: stdout.to_string(),
            stderr: String::new(),
            start_error: None,
        }
    }

    fn absent() -> CommandEvidence {
        CommandEvidence {
            program: "fixture".to_string(),
            args: vec![],
            exit_code: Some(1),
            stdout: String::new(),
            stderr: String::new(),
            start_error: None,
        }
    }

    #[test]
    fn documented_webview2_guid_is_frozen() {
        assert_eq!(
            WEBVIEW2_PRODUCT_GUID,
            "{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}"
        );
    }

    #[test]
    fn vc_registry_version_is_authoritative_installed_evidence() {
        let item = success("    Version    REG_SZ    v14.44.35211.0\n");
        let missing = absent();
        let result = classify_registry_runtime(
            RuntimeComponent::VcRedist2015PlusX64,
            "fixture",
            "Version",
            &[&item, &missing],
        );
        assert_eq!(result.state, RuntimeState::Installed);
        assert!(result.details.iter().any(|value| value.contains("14.44")));
    }

    #[test]
    fn vc_registry_known_absence_is_missing() {
        let a = absent();
        let b = absent();
        let result = classify_registry_runtime(
            RuntimeComponent::VcRedist2015PlusX86,
            "fixture",
            "Version",
            &[&a, &b],
        );
        assert_eq!(result.state, RuntimeState::Missing);
    }

    #[test]
    fn dism_feature_parser_is_strict_and_read_only() {
        assert_eq!(
            parse_dism_feature_state("Feature Name : NetFx3\nState : Enabled\n"),
            Some(FeatureState::Enabled)
        );
        assert_eq!(
            parse_dism_feature_state(
                "Feature Name : NetFx3\nState : Disabled with Payload Removed\n"
            ),
            Some(FeatureState::PayloadRemoved)
        );
        assert_eq!(parse_dism_feature_state("State : Surprise\n"), None);
    }

    #[test]
    fn dotnet_successful_listing_proves_family_presence_or_absence() {
        let evidence = success(
            "Microsoft.NETCore.App 8.0.19 [C:\\dotnet\\shared\\Microsoft.NETCore.App]\nMicrosoft.WindowsDesktop.App 8.0.19 [C:\\dotnet\\shared\\Microsoft.WindowsDesktop.App]\n",
        );
        assert_eq!(
            classify_dotnet_runtime(
                RuntimeComponent::DotNetRuntime,
                "Microsoft.NETCore.App",
                &evidence
            )
            .state,
            RuntimeState::Installed
        );
        assert_eq!(
            classify_dotnet_runtime(
                RuntimeComponent::DotNetDesktopRuntime,
                "Missing.Family",
                &evidence
            )
            .state,
            RuntimeState::Missing
        );
    }

    #[test]
    fn webview_zero_version_is_not_installed() {
        let zero = success("    pv    REG_SZ    0.0.0.0\n");
        let missing = absent();
        let result = classify_webview2(&[&zero, &missing]);
        assert_eq!(result.state, RuntimeState::Broken);
    }

    #[test]
    fn python_path_gap_is_partial_not_a_second_install_trigger() {
        let list = success(" -V:3.14 * C:\\Users\\neo\\Python314\\python.exe\n");
        let python = absent();
        let py = success("C:\\WindowsApps\\py.exe\n");
        let pip = absent();
        let result = classify_python(&list, &python, &py, &pip);
        assert_eq!(result.state, RuntimeState::Partial);
        assert!(result
            .details
            .iter()
            .any(|value| value == "python_on_path=false"));
    }

    #[test]
    fn no_python_path_evidence_is_unknown_not_missing() {
        let no_list = absent();
        let no_python = absent();
        let no_py = absent();
        let no_pip = absent();
        let result = classify_python(&no_list, &no_python, &no_py, &no_pip);
        assert_eq!(result.state, RuntimeState::Unknown);
    }
}
