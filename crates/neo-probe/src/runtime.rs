use super::{CommandEvidence, CommandRunner, ProbeError};
use neo_runtime::{RuntimeComponent, RuntimeInventory, RuntimeObservation, RuntimeState};
use serde::{Deserialize, Serialize};

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
    ) -> Result<RuntimeProbeReport, ProbeError> {
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
        let webview_machine_wow = self.capture(
            "reg.exe",
            &[
                "query",
                r"HKLM\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F1E7E6E8-65B8-4C64-A68C-1A2D16D5B3F7}",
                "/v",
                "pv",
            ],
        );
        let webview_machine_native = self.capture(
            "reg.exe",
            &[
                "query",
                r"HKLM\SOFTWARE\Microsoft\EdgeUpdate\Clients\{F1E7E6E8-65B8-4C64-A68C-1A2D16D5B3F7}",
                "/v",
                "pv",
            ],
        );
        let webview_user = self.capture(
            "reg.exe",
            &[
                "query",
                r"HKCU\Software\Microsoft\EdgeUpdate\Clients\{F1E7E6E8-65B8-4C64-A68C-1A2D16D5B3F7}",
                "/v",
                "pv",
            ],
        );
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
        // `py -0p` is a compatibility listing operation supported by both the
        // legacy launcher and the current Python Install Manager. We avoid a
        // bare `python`/`py` launch because the current manager may auto-install
        // a runtime when none is present.
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
            classify_registry_runtime(
                RuntimeComponent::VcRedist2015PlusX64,
                "Microsoft Visual C++ v14 x64 registry",
                "Version",
                &[&vc_x64_native, &vc_x64_wow],
            ),
            unknown_observation(
                RuntimeComponent::DirectXLegacyJune2010,
                "neo-runtime-probe:directx-legacy-predicate-pending",
                "Modern DirectX and the June 2010 side-by-side legacy package are distinct; Phase 6 does not infer legacy completeness from the OS DirectX version.",
            ),
            classify_windows_feature(RuntimeComponent::DotNetFramework35, "NetFx3", &netfx3),
            classify_netfx4(&netfx4),
            classify_dotnet_runtime(RuntimeComponent::DotNetRuntime, "Microsoft.NETCore.App", &dotnet),
            classify_dotnet_runtime(
                RuntimeComponent::DotNetDesktopRuntime,
                "Microsoft.WindowsDesktop.App",
                &dotnet,
            ),
            classify_python(&python_list, &python_path, &py_path, &pip_path),
            classify_webview2(&[
                &webview_machine_wow,
                &webview_machine_native,
                &webview_user,
            ]),
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
            architecture: architecture.to_string(),
            observations,
        };
        inventory
            .validate()
            .map_err(|error| ProbeError::RuntimeEvidence(error.to_string()))?;

        let command_evidence = vec![
            vc_x86_native,
            vc_x86_wow,
            vc_x64_native,
            vc_x64_wow,
            netfx4,
            dotnet,
            webview_machine_wow,
            webview_machine_native,
            webview_user,
            netfx3,
            directplay,
            python_list,
            python_path,
            py_path,
            pip_path,
        ];
        let warnings = command_evidence
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

        Ok(RuntimeProbeReport {
            inventory,
            command_evidence,
            warnings,
        })
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
        .filter_map(|item| super::parse_reg_value(&item.stdout, value_name))
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
        if let Some(release) = super::parse_reg_value(&evidence.stdout, "Release") {
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
        .filter_map(|item| super::parse_reg_value(&item.stdout, "pv"))
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
        let normalized = value
            .trim()
            .to_ascii_lowercase()
            .replace([' ', '-'], "");
        match normalized.as_str() {
            "enabled" | "enablepending" => Some(FeatureState::Enabled),
            "disabled" | "disablepending" => Some(FeatureState::Disabled),
            "disabledwithpayloadremoved" => Some(FeatureState::PayloadRemoved),
            _ => None,
        }
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
    let detected_version = details
        .iter()
        .find(|item| {
            !item.starts_with("State=")
                && !item.contains("_on_path=")
                && !item.starts_with("Runtime registry")
        })
        .cloned();
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
    fn python_path_gap_is_partial_not_a_second_install_trigger() {
        let list = success(" -V:3.14 * C:\\Users\\neo\\Python314\\python.exe\n");
        let python = absent();
        let py = success("C:\\WindowsApps\\py.exe\n");
        let pip = absent();
        let result = classify_python(&list, &python, &py, &pip);
        assert_eq!(result.state, RuntimeState::Partial);
        assert!(result.details.iter().any(|value| value == "python_on_path=false"));
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
