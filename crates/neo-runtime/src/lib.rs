//! Deterministic runtime and gaming readiness assessment for Neo Driver.
//!
//! Phase 6 is deliberately read-only at this boundary. It consumes normalized
//! runtime evidence plus the existing Neo package catalogue and produces a
//! reviewable, individually selectable plan. It does not download or install
//! runtimes, change Windows features, or advance transactions.

use neo_catalogue::{
    Catalogue, PackageKind, PackageManifest, RebootRequirement as CatalogueReboot,
};
use neo_core::{
    ActionKind, EvidenceItem, EvidenceVerdict, PlannedAction, RebootRequirement,
    RecommendationState, RiskLevel,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeComponent {
    #[serde(rename = "vc_redist_2015_plus_x86")]
    VcRedist2015PlusX86,
    #[serde(rename = "vc_redist_2015_plus_x64")]
    VcRedist2015PlusX64,
    #[serde(rename = "directx_legacy_june_2010")]
    DirectXLegacyJune2010,
    #[serde(rename = "dotnet_framework_35")]
    DotNetFramework35,
    #[serde(rename = "dotnet_framework_4")]
    DotNetFramework4,
    #[serde(rename = "dotnet_runtime")]
    DotNetRuntime,
    #[serde(rename = "dotnet_desktop_runtime")]
    DotNetDesktopRuntime,
    Python,
    #[serde(rename = "webview2")]
    WebView2,
    #[serde(rename = "xna_framework_40_refresh")]
    XnaFramework40Refresh,
    #[serde(rename = "openal")]
    OpenAl,
    Physx,
    PhysxLegacy,
    #[serde(rename = "directplay")]
    DirectPlay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeState {
    Installed,
    Missing,
    Broken,
    Partial,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeProfile {
    FreshWindows,
    Gaming,
    Technician,
    Developer,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeObservation {
    pub component: RuntimeComponent,
    pub state: RuntimeState,
    #[serde(default)]
    pub detected_version: Option<String>,
    pub source: String,
    #[serde(default)]
    pub details: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeInventory {
    pub windows_build: u32,
    pub architecture: String,
    #[serde(default)]
    pub observations: Vec<RuntimeObservation>,
}

impl RuntimeInventory {
    pub fn validate(&self) -> Result<(), RuntimeError> {
        if self.windows_build == 0 {
            return Err(RuntimeError::InvalidWindowsBuild);
        }
        if canonical_arch(&self.architecture).is_none() {
            return Err(RuntimeError::UnsupportedArchitecture(
                self.architecture.clone(),
            ));
        }
        let mut components = BTreeSet::new();
        for observation in &self.observations {
            if observation.source.trim().is_empty() {
                return Err(RuntimeError::MissingObservationSource(
                    observation.component,
                ));
            }
            if !components.insert(observation.component) {
                return Err(RuntimeError::DuplicateObservation(observation.component));
            }
        }
        Ok(())
    }

    pub fn from_json_str(input: &str) -> Result<Self, RuntimeError> {
        let inventory: Self = serde_json::from_str(input)?;
        inventory.validate()?;
        Ok(inventory)
    }

    pub fn read_json(path: impl AsRef<Path>) -> Result<Self, RuntimeError> {
        Self::from_json_str(&std::fs::read_to_string(path)?)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimePackageBinding {
    pub component: RuntimeComponent,
    pub package_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct RuntimePolicy {
    #[serde(default)]
    pub bindings: Vec<RuntimePackageBinding>,
}

impl RuntimePolicy {
    pub fn validate(&self, catalogue: &Catalogue) -> Result<(), RuntimeError> {
        catalogue
            .validate()
            .map_err(|error| RuntimeError::Catalogue(error.to_string()))?;
        let packages: BTreeMap<&str, &PackageManifest> = catalogue
            .packages
            .iter()
            .map(|package| (package.package_id.as_str(), package))
            .collect();
        let mut exact = BTreeSet::new();
        for binding in &self.bindings {
            if binding.package_id.trim().is_empty() {
                return Err(RuntimeError::EmptyPackageId(binding.component));
            }
            if !exact.insert((binding.component, binding.package_id.as_str())) {
                return Err(RuntimeError::DuplicateBinding {
                    component: binding.component,
                    package_id: binding.package_id.clone(),
                });
            }
            let package = packages
                .get(binding.package_id.as_str())
                .ok_or_else(|| RuntimeError::UnknownPackage(binding.package_id.clone()))?;
            if package.kind != PackageKind::Runtime {
                return Err(RuntimeError::BindingTargetsNonRuntime {
                    component: binding.component,
                    package_id: binding.package_id.clone(),
                });
            }
        }
        Ok(())
    }

    pub fn from_json_str(input: &str, catalogue: &Catalogue) -> Result<Self, RuntimeError> {
        let policy: Self = serde_json::from_str(input)?;
        policy.validate(catalogue)?;
        Ok(policy)
    }

    pub fn read_json(path: impl AsRef<Path>, catalogue: &Catalogue) -> Result<Self, RuntimeError> {
        Self::from_json_str(&std::fs::read_to_string(path)?, catalogue)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeRecommendation {
    pub component: RuntimeComponent,
    pub state: RuntimeState,
    pub baseline_for_profile: bool,
    pub user_selectable: bool,
    pub recommendation: RecommendationState,
    pub verdict: EvidenceVerdict,
    #[serde(default)]
    pub package_id: Option<String>,
    #[serde(default)]
    pub action: Option<PlannedAction>,
    pub rationale: String,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeAssessment {
    pub profile: RuntimeProfile,
    pub ready: bool,
    pub recommendations: Vec<RuntimeRecommendation>,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
struct Requirement {
    component: RuntimeComponent,
    baseline: bool,
}

pub fn assess_runtime_profile(
    profile: RuntimeProfile,
    inventory: &RuntimeInventory,
    catalogue: &Catalogue,
    policy: &RuntimePolicy,
) -> Result<RuntimeAssessment, RuntimeError> {
    inventory.validate()?;
    policy.validate(catalogue)?;

    let observations: BTreeMap<RuntimeComponent, &RuntimeObservation> = inventory
        .observations
        .iter()
        .map(|observation| (observation.component, observation))
        .collect();

    let mut recommendations = Vec::new();
    let mut ready = true;
    for &requirement in requirements(profile) {
        let observation = observations.get(&requirement.component).copied();
        let state = observation.map_or(RuntimeState::Unknown, |item| item.state);
        let candidates = compatible_packages(requirement.component, inventory, catalogue, policy);

        let mut warnings = Vec::new();
        let chosen = match candidates.as_slice() {
            [single] => Some(*single),
            [] => {
                if !matches!(state, RuntimeState::Installed) {
                    warnings.push(
                        "No compatible runtime package is bound in the validated policy."
                            .to_string(),
                    );
                }
                None
            }
            _ => {
                warnings.push(format!(
                    "{} compatible packages are bound; Neo will not guess between them.",
                    candidates.len()
                ));
                None
            }
        };

        let (recommendation, verdict, rationale, action) = match state {
            RuntimeState::Installed => (
                RecommendationState::Healthy,
                EvidenceVerdict::Certified,
                "Detected runtime satisfies the normalized installed-state predicate.".to_string(),
                None,
            ),
            RuntimeState::Missing => build_change_recommendation(
                requirement,
                observation,
                chosen,
                ActionKind::RuntimeInstall,
                RecommendationState::Recommended,
            ),
            RuntimeState::Broken | RuntimeState::Partial => build_change_recommendation(
                requirement,
                observation,
                chosen,
                ActionKind::RuntimeRepair,
                RecommendationState::Repair,
            ),
            RuntimeState::Unknown => (
                RecommendationState::Unknown,
                EvidenceVerdict::Investigate,
                "Runtime state is not proven; Neo refuses to convert unknown evidence into an install recommendation.".to_string(),
                None,
            ),
        };

        if requirement.baseline && !matches!(state, RuntimeState::Installed) {
            ready = false;
        }

        recommendations.push(RuntimeRecommendation {
            component: requirement.component,
            state,
            baseline_for_profile: requirement.baseline,
            user_selectable: true,
            recommendation,
            verdict,
            package_id: chosen.map(|package| package.package_id.clone()),
            action,
            rationale,
            warnings,
        });
    }

    Ok(RuntimeAssessment {
        profile,
        ready,
        recommendations,
        warnings: Vec::new(),
    })
}

fn build_change_recommendation(
    requirement: Requirement,
    observation: Option<&RuntimeObservation>,
    package: Option<&PackageManifest>,
    kind: ActionKind,
    recommendation: RecommendationState,
) -> (
    RecommendationState,
    EvidenceVerdict,
    String,
    Option<PlannedAction>,
) {
    let Some(package) = package else {
        return (
            if requirement.baseline {
                recommendation
            } else {
                RecommendationState::OptionalComponent
            },
            EvidenceVerdict::Investigate,
            "A change may be appropriate, but no single compatible validated package is proven."
                .to_string(),
            None,
        );
    };

    let mut evidence = vec![
        EvidenceItem::new(
            "runtime_component",
            component_key(requirement.component),
            "neo-runtime",
        ),
        EvidenceItem::new("package_id", &package.package_id, "neo-catalogue"),
        EvidenceItem::new(
            "package_sha256",
            &package.provenance.sha256,
            "neo-catalogue",
        ),
        EvidenceItem::new(
            "package_source",
            &package.provenance.source_name,
            "neo-catalogue",
        ),
    ];
    if let Some(observation) = observation {
        evidence.push(EvidenceItem::new(
            "runtime_state",
            format!("{:?}", observation.state).to_ascii_lowercase(),
            &observation.source,
        ));
        if let Some(version) = &observation.detected_version {
            evidence.push(EvidenceItem::new(
                "detected_version",
                version,
                &observation.source,
            ));
        }
    }

    let selected_by_default = requirement.baseline;
    let rationale = if requirement.baseline {
        "Profile baseline is not healthy; one compatible validated package is available. The item may be preselected but remains individually deselectable and requires confirmation.".to_string()
    } else {
        "Optional profile component is not healthy; one compatible validated package is available and remains unselected until the user chooses it.".to_string()
    };

    let action = PlannedAction {
        id: format!("runtime.{}", component_key(requirement.component)),
        title: component_label(requirement.component).to_string(),
        kind,
        risk: RiskLevel::Normal,
        recommendation: if requirement.baseline {
            recommendation
        } else {
            RecommendationState::OptionalComponent
        },
        verdict: EvidenceVerdict::Certified,
        rationale: rationale.clone(),
        selected_by_default,
        requires_confirmation: true,
        requires_admin: true,
        reboot: map_reboot(package.reboot),
        rollback_available: false,
        evidence,
        warnings: vec![
            "Phase 6 assessment does not execute this action; runtime execution remains behind a later bounded executor gate.".to_string(),
        ],
    };

    (
        action.recommendation,
        EvidenceVerdict::Certified,
        rationale,
        Some(action),
    )
}

fn compatible_packages<'a>(
    component: RuntimeComponent,
    inventory: &RuntimeInventory,
    catalogue: &'a Catalogue,
    policy: &RuntimePolicy,
) -> Vec<&'a PackageManifest> {
    policy
        .bindings
        .iter()
        .filter(|binding| binding.component == component)
        .filter_map(|binding| {
            catalogue
                .packages
                .iter()
                .find(|package| package.package_id == binding.package_id)
        })
        .filter(|package| package_applies(package, inventory))
        .collect()
}

fn package_applies(package: &PackageManifest, inventory: &RuntimeInventory) -> bool {
    let Some(host_arch) = canonical_arch(&inventory.architecture) else {
        return false;
    };
    let architecture_ok = package
        .windows
        .architectures
        .iter()
        .filter_map(|value| canonical_arch(value))
        .any(|candidate| candidate == host_arch);
    if !architecture_ok {
        return false;
    }
    if package
        .windows
        .minimum_build
        .is_some_and(|minimum| inventory.windows_build < minimum)
    {
        return false;
    }
    if package
        .windows
        .maximum_build
        .is_some_and(|maximum| inventory.windows_build > maximum)
    {
        return false;
    }
    true
}

fn requirements(profile: RuntimeProfile) -> &'static [Requirement] {
    use RuntimeComponent::*;
    match profile {
        RuntimeProfile::FreshWindows => &[
            Requirement {
                component: VcRedist2015PlusX86,
                baseline: true,
            },
            Requirement {
                component: VcRedist2015PlusX64,
                baseline: true,
            },
            Requirement {
                component: DirectXLegacyJune2010,
                baseline: true,
            },
            Requirement {
                component: WebView2,
                baseline: false,
            },
        ],
        RuntimeProfile::Gaming => &[
            Requirement {
                component: VcRedist2015PlusX86,
                baseline: true,
            },
            Requirement {
                component: VcRedist2015PlusX64,
                baseline: true,
            },
            Requirement {
                component: DirectXLegacyJune2010,
                baseline: true,
            },
            Requirement {
                component: XnaFramework40Refresh,
                baseline: false,
            },
            Requirement {
                component: OpenAl,
                baseline: false,
            },
            Requirement {
                component: Physx,
                baseline: false,
            },
            Requirement {
                component: PhysxLegacy,
                baseline: false,
            },
            Requirement {
                component: DotNetFramework35,
                baseline: false,
            },
            Requirement {
                component: DirectPlay,
                baseline: false,
            },
        ],
        RuntimeProfile::Technician => &[
            Requirement {
                component: VcRedist2015PlusX86,
                baseline: true,
            },
            Requirement {
                component: VcRedist2015PlusX64,
                baseline: true,
            },
            Requirement {
                component: DotNetFramework4,
                baseline: false,
            },
            Requirement {
                component: Python,
                baseline: false,
            },
        ],
        RuntimeProfile::Developer => &[
            Requirement {
                component: VcRedist2015PlusX86,
                baseline: true,
            },
            Requirement {
                component: VcRedist2015PlusX64,
                baseline: true,
            },
            Requirement {
                component: DotNetRuntime,
                baseline: false,
            },
            Requirement {
                component: DotNetDesktopRuntime,
                baseline: false,
            },
            Requirement {
                component: Python,
                baseline: false,
            },
        ],
    }
}

fn map_reboot(value: CatalogueReboot) -> RebootRequirement {
    match value {
        CatalogueReboot::None => RebootRequirement::None,
        CatalogueReboot::Recommended => RebootRequirement::Possible,
        CatalogueReboot::Required => RebootRequirement::Required,
    }
}

fn canonical_arch(value: &str) -> Option<&'static str> {
    match value.trim().to_ascii_lowercase().as_str() {
        "x64" | "amd64" | "x86_64" => Some("x64"),
        "x86" | "i386" | "i686" => Some("x86"),
        "arm64" | "aarch64" => Some("arm64"),
        _ => None,
    }
}

pub fn component_key(component: RuntimeComponent) -> &'static str {
    use RuntimeComponent::*;
    match component {
        VcRedist2015PlusX86 => "vc_redist_2015_plus_x86",
        VcRedist2015PlusX64 => "vc_redist_2015_plus_x64",
        DirectXLegacyJune2010 => "directx_legacy_june_2010",
        DotNetFramework35 => "dotnet_framework_35",
        DotNetFramework4 => "dotnet_framework_4",
        DotNetRuntime => "dotnet_runtime",
        DotNetDesktopRuntime => "dotnet_desktop_runtime",
        Python => "python",
        WebView2 => "webview2",
        XnaFramework40Refresh => "xna_framework_40_refresh",
        OpenAl => "openal",
        Physx => "physx",
        PhysxLegacy => "physx_legacy",
        DirectPlay => "directplay",
    }
}

pub fn component_label(component: RuntimeComponent) -> &'static str {
    use RuntimeComponent::*;
    match component {
        VcRedist2015PlusX86 => "Visual C++ 2015+ x86",
        VcRedist2015PlusX64 => "Visual C++ 2015+ x64",
        DirectXLegacyJune2010 => "DirectX Legacy (June 2010)",
        DotNetFramework35 => ".NET Framework 3.5",
        DotNetFramework4 => ".NET Framework 4.x",
        DotNetRuntime => ".NET Runtime",
        DotNetDesktopRuntime => ".NET Desktop Runtime",
        Python => "Python",
        WebView2 => "Microsoft Edge WebView2 Runtime",
        XnaFramework40Refresh => "XNA Framework 4.0 Refresh",
        OpenAl => "OpenAL",
        Physx => "NVIDIA PhysX",
        PhysxLegacy => "NVIDIA PhysX Legacy",
        DirectPlay => "DirectPlay",
    }
}

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("Windows build must be greater than zero")]
    InvalidWindowsBuild,
    #[error("unsupported architecture: {0}")]
    UnsupportedArchitecture(String),
    #[error("observation source missing for {0:?}")]
    MissingObservationSource(RuntimeComponent),
    #[error("duplicate runtime observation: {0:?}")]
    DuplicateObservation(RuntimeComponent),
    #[error("empty package id for runtime binding {0:?}")]
    EmptyPackageId(RuntimeComponent),
    #[error("duplicate runtime binding for {component:?}: {package_id}")]
    DuplicateBinding {
        component: RuntimeComponent,
        package_id: String,
    },
    #[error("runtime policy references unknown package: {0}")]
    UnknownPackage(String),
    #[error("runtime binding {component:?} targets non-runtime package {package_id}")]
    BindingTargetsNonRuntime {
        component: RuntimeComponent,
        package_id: String,
    },
    #[error("catalogue validation failed: {0}")]
    Catalogue(String),
    #[error("runtime JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("runtime I/O error: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use neo_catalogue::{
        Provenance, RedistributionPolicy, SecurityRequirements, WindowsApplicability,
    };

    fn package(id: &str) -> PackageManifest {
        PackageManifest {
            package_id: id.to_string(),
            name: id.to_string(),
            vendor: "fixture".to_string(),
            version: "1.0".to_string(),
            kind: PackageKind::Runtime,
            provenance: Provenance {
                source_name: "fixture".to_string(),
                source_url: None,
                sha256: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                    .to_string(),
                redistribution: RedistributionPolicy::Unknown,
            },
            windows: WindowsApplicability {
                architectures: vec!["x64".to_string()],
                minimum_build: Some(19041),
                maximum_build: None,
            },
            driver_artifacts: vec![],
            dependencies: vec![],
            conflicts: vec![],
            security: SecurityRequirements::default(),
            reboot: CatalogueReboot::None,
        }
    }

    fn inventory(state: RuntimeState) -> RuntimeInventory {
        RuntimeInventory {
            windows_build: 26100,
            architecture: "x64".to_string(),
            observations: vec![
                RuntimeObservation {
                    component: RuntimeComponent::VcRedist2015PlusX86,
                    state,
                    detected_version: None,
                    source: "fixture".to_string(),
                    details: vec![],
                },
                RuntimeObservation {
                    component: RuntimeComponent::VcRedist2015PlusX64,
                    state: RuntimeState::Installed,
                    detected_version: Some("14.x".to_string()),
                    source: "fixture".to_string(),
                    details: vec![],
                },
                RuntimeObservation {
                    component: RuntimeComponent::DirectXLegacyJune2010,
                    state: RuntimeState::Installed,
                    detected_version: None,
                    source: "fixture".to_string(),
                    details: vec![],
                },
            ],
        }
    }

    #[test]
    fn serialized_component_names_match_canonical_keys() {
        let components = [
            RuntimeComponent::VcRedist2015PlusX86,
            RuntimeComponent::VcRedist2015PlusX64,
            RuntimeComponent::DirectXLegacyJune2010,
            RuntimeComponent::DotNetFramework35,
            RuntimeComponent::DotNetFramework4,
            RuntimeComponent::DotNetRuntime,
            RuntimeComponent::DotNetDesktopRuntime,
            RuntimeComponent::Python,
            RuntimeComponent::WebView2,
            RuntimeComponent::XnaFramework40Refresh,
            RuntimeComponent::OpenAl,
            RuntimeComponent::Physx,
            RuntimeComponent::PhysxLegacy,
            RuntimeComponent::DirectPlay,
        ];
        for component in components {
            let encoded = serde_json::to_string(&component).unwrap();
            assert_eq!(encoded, format!("\"{}\"", component_key(component)));
            let decoded: RuntimeComponent = serde_json::from_str(&encoded).unwrap();
            assert_eq!(decoded, component);
        }
    }

    #[test]
    fn baseline_missing_is_preselected_but_confirmed_and_selectable() {
        let catalogue = Catalogue {
            packages: vec![package("runtime.vc.x86")],
        };
        let policy = RuntimePolicy {
            bindings: vec![RuntimePackageBinding {
                component: RuntimeComponent::VcRedist2015PlusX86,
                package_id: "runtime.vc.x86".to_string(),
            }],
        };
        let assessment = assess_runtime_profile(
            RuntimeProfile::FreshWindows,
            &inventory(RuntimeState::Missing),
            &catalogue,
            &policy,
        )
        .unwrap();
        let item = assessment
            .recommendations
            .iter()
            .find(|item| item.component == RuntimeComponent::VcRedist2015PlusX86)
            .unwrap();
        let action = item.action.as_ref().unwrap();
        assert!(item.user_selectable);
        assert!(action.selected_by_default);
        assert!(action.requires_confirmation);
        assert!(action.validate().is_ok());
    }

    #[test]
    fn optional_missing_is_never_preselected() {
        let catalogue = Catalogue {
            packages: vec![package("runtime.python")],
        };
        let policy = RuntimePolicy {
            bindings: vec![RuntimePackageBinding {
                component: RuntimeComponent::Python,
                package_id: "runtime.python".to_string(),
            }],
        };
        let assessment = assess_runtime_profile(
            RuntimeProfile::Technician,
            &RuntimeInventory {
                windows_build: 26100,
                architecture: "x64".to_string(),
                observations: vec![RuntimeObservation {
                    component: RuntimeComponent::Python,
                    state: RuntimeState::Missing,
                    detected_version: None,
                    source: "fixture".to_string(),
                    details: vec![],
                }],
            },
            &catalogue,
            &policy,
        )
        .unwrap();
        let python = assessment
            .recommendations
            .iter()
            .find(|item| item.component == RuntimeComponent::Python)
            .unwrap();
        assert!(!python.action.as_ref().unwrap().selected_by_default);
    }

    #[test]
    fn unknown_state_never_becomes_install_authority() {
        let catalogue = Catalogue {
            packages: vec![package("runtime.vc.x86")],
        };
        let policy = RuntimePolicy {
            bindings: vec![RuntimePackageBinding {
                component: RuntimeComponent::VcRedist2015PlusX86,
                package_id: "runtime.vc.x86".to_string(),
            }],
        };
        let assessment = assess_runtime_profile(
            RuntimeProfile::FreshWindows,
            &inventory(RuntimeState::Unknown),
            &catalogue,
            &policy,
        )
        .unwrap();
        let item = &assessment.recommendations[0];
        assert_eq!(item.verdict, EvidenceVerdict::Investigate);
        assert!(item.action.is_none());
    }

    #[test]
    fn ambiguous_packages_fail_closed() {
        let catalogue = Catalogue {
            packages: vec![package("runtime.vc.x86.a"), package("runtime.vc.x86.b")],
        };
        let policy = RuntimePolicy {
            bindings: vec![
                RuntimePackageBinding {
                    component: RuntimeComponent::VcRedist2015PlusX86,
                    package_id: "runtime.vc.x86.a".to_string(),
                },
                RuntimePackageBinding {
                    component: RuntimeComponent::VcRedist2015PlusX86,
                    package_id: "runtime.vc.x86.b".to_string(),
                },
            ],
        };
        let assessment = assess_runtime_profile(
            RuntimeProfile::FreshWindows,
            &inventory(RuntimeState::Missing),
            &catalogue,
            &policy,
        )
        .unwrap();
        let item = &assessment.recommendations[0];
        assert_eq!(item.verdict, EvidenceVerdict::Investigate);
        assert!(item.action.is_none());
    }

    #[test]
    fn non_runtime_binding_is_rejected() {
        let mut invalid = package("not.runtime");
        invalid.kind = PackageKind::Application;
        let catalogue = Catalogue {
            packages: vec![invalid],
        };
        let policy = RuntimePolicy {
            bindings: vec![RuntimePackageBinding {
                component: RuntimeComponent::Python,
                package_id: "not.runtime".to_string(),
            }],
        };
        assert!(matches!(
            policy.validate(&catalogue),
            Err(RuntimeError::BindingTargetsNonRuntime { .. })
        ));
    }
}
