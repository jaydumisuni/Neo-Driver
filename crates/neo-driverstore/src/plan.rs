use neo_catalogue::{Catalogue, PackageKind, SignatureStatus};
use neo_core::{
    ActionKind, EvidenceItem, EvidenceVerdict, PlannedAction, RebootRequirement,
    RecommendationState, RiskLevel,
};
use neo_match::{match_device, MatchContext};
use neo_transaction::{
    BaselineSnapshot, CapturedState, CapturedValue, RollbackPlan, StateTarget, StateTargetKind,
    TransactionAction, TransactionPlan, VerificationExpectation, VerificationPredicate,
};
use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

use crate::{
    model::sha256_file, DriverBindingBaseline, DriverHost, DriverInstallImpact, DriverInstallPlan,
    DriverStoreBaseline, DriverStoreError, PreparedDriverInstall, VerifiedInfSignature,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriverInstallRequest {
    pub package_root: PathBuf,
    pub package_id: String,
    pub inf_path: String,
    pub architecture: String,
    pub windows_build: u32,
    pub action_id: String,
    pub mission_id: String,
}

pub fn prepare_driver_install<H: DriverHost>(
    host: &H,
    catalogue: &Catalogue,
    request: &DriverInstallRequest,
) -> Result<PreparedDriverInstall, DriverStoreError> {
    let package_root = request.package_root.as_path();
    let package_id = request.package_id.as_str();
    let inf_path = request.inf_path.as_str();
    let architecture = request.architecture.as_str();
    let windows_build = request.windows_build;
    let action_id = request.action_id.as_str();
    let mission_id = request.mission_id.as_str();
    catalogue.validate()?;
    let actual_windows_build = host.windows_build()?;
    if actual_windows_build != windows_build {
        return Err(DriverStoreError::WindowsBuildMismatch {
            requested: windows_build,
            actual: actual_windows_build,
        });
    }
    let package = catalogue
        .packages
        .iter()
        .find(|package| package.package_id == package_id)
        .ok_or_else(|| DriverStoreError::PackageNotFound(package_id.to_string()))?;
    if package.kind != PackageKind::InfDriverBundle {
        return Err(DriverStoreError::WrongPackageKind);
    }
    let artifact = package
        .driver_artifacts
        .iter()
        .find(|artifact| artifact.inf_path.eq_ignore_ascii_case(inf_path))
        .ok_or_else(|| DriverStoreError::ArtifactNotFound {
            package_id: package_id.to_string(),
            inf_path: inf_path.to_string(),
        })?;
    if artifact.signature.status != SignatureStatus::Verified {
        return Err(DriverStoreError::UnverifiedArtifact);
    }
    let expected_signer = artifact
        .signature
        .signer
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or(DriverStoreError::MissingExpectedSigner)?;

    let (package_root, source_inf) = resolve_source_inf(package_root, inf_path)?;
    let source_inf_sha256 = sha256_file(&source_inf)?;
    let verified_signature = host.verify_inf_signature(&source_inf)?;
    verified_signature.validate()?;
    if !verified_signature
        .signer
        .trim()
        .eq_ignore_ascii_case(expected_signer.trim())
        || !catalogue_file_matches(&verified_signature.catalog_file, &artifact.catalog_files)
    {
        return Err(DriverStoreError::SignatureMismatch);
    }

    let inventory = host.inventory()?;
    inventory.validate()?;
    let windows_impacts = normalized_id_set(host.compatible_present_devices(&source_inf)?)?;
    if windows_impacts.is_empty() {
        return Err(DriverStoreError::NoSupportedPresentDevice);
    }

    let context = MatchContext {
        architecture: architecture.to_string(),
        windows_build,
    };
    let mut catalogue_impacts = BTreeSet::new();
    for device in &inventory.devices {
        let report = match_device(device, catalogue, &context)?;
        let applicable = report.candidates.iter().any(|candidate| {
            candidate.package_id == package_id
                && candidate.inf_path.eq_ignore_ascii_case(inf_path)
                && candidate.verdict != EvidenceVerdict::Rejected
        });
        if applicable {
            catalogue_impacts.insert(device.instance_id.as_str().to_ascii_lowercase());
        }
    }
    if catalogue_impacts != windows_impacts {
        return Err(DriverStoreError::CatalogueImpactMismatch);
    }

    let mut impacts = Vec::with_capacity(windows_impacts.len());
    for identity in &windows_impacts {
        let device = inventory
            .devices
            .iter()
            .find(|device| device.instance_id.as_str().eq_ignore_ascii_case(identity))
            .ok_or(DriverStoreError::ImpactDrift)?;
        let binding = device
            .active_driver
            .clone()
            .ok_or_else(|| DriverStoreError::MissingBaselineBinding(identity.clone()))?;
        let published = binding
            .published_name
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| {
                DriverStoreError::MissingBaselinePublishedInf(device.instance_id.to_string())
            })?;
        let baseline_package = host.resolve_published_package(published)?.ok_or_else(|| {
            DriverStoreError::MissingBaselinePackage(device.instance_id.to_string())
        })?;
        baseline_package.validate()?;
        impacts.push(DriverInstallImpact {
            instance_id: device.instance_id.to_string(),
            baseline: DriverBindingBaseline {
                binding,
                problem_code: device.problem_code,
            },
            baseline_package,
        });
    }
    impacts.sort_by_key(|impact| impact.instance_id.to_ascii_lowercase());

    let store_baseline = match host.find_equivalent_package(&source_inf, &artifact.catalog_files)? {
        Some(package) => {
            package.validate()?;
            DriverStoreBaseline::Existing { package }
        }
        None => DriverStoreBaseline::Absent,
    };

    let driver_plan = DriverInstallPlan {
        action_id: action_id.to_string(),
        mission_id: mission_id.to_string(),
        package_id: package_id.to_string(),
        inf_path: normalize_inf_path(inf_path)?,
        package_root,
        source_inf,
        source_inf_sha256,
        architecture: architecture.to_string(),
        windows_build,
        expected_signature: VerifiedInfSignature {
            catalog_file: verified_signature.catalog_file,
            signer: verified_signature.signer,
            signer_version: verified_signature.signer_version,
        },
        store_baseline,
        impacts,
    };
    driver_plan.validate()?;
    let transaction_plan = transaction_contract(&driver_plan)?;
    let baseline = baseline_contract(&driver_plan, &transaction_plan)?;

    Ok(PreparedDriverInstall {
        driver_plan,
        transaction_plan,
        baseline,
    })
}

pub(crate) fn transaction_contract(
    driver_plan: &DriverInstallPlan,
) -> Result<TransactionPlan, DriverStoreError> {
    driver_plan.validate()?;
    let driver_fingerprint = driver_plan.fingerprint()?;
    let store_target = store_target(&driver_fingerprint);
    let mut snapshot_targets = vec![store_target.clone()];
    let mut rollback_verification = vec![VerificationPredicate {
        id: format!("rollback.store.{driver_fingerprint}"),
        target: store_target,
        expectation: VerificationExpectation::MatchesBaseline,
        required: true,
    }];
    for impact in &driver_plan.impacts {
        let target = binding_target(&impact.instance_id);
        snapshot_targets.push(target.clone());
        rollback_verification.push(VerificationPredicate {
            id: format!("rollback.binding.{}", impact.instance_id),
            target,
            expectation: VerificationExpectation::MatchesBaseline,
            required: true,
        });
    }
    let postconditions = vec![VerificationPredicate {
        id: format!("driver.policy.{driver_fingerprint}"),
        target: policy_target(&driver_fingerprint),
        expectation: VerificationExpectation::Equals("satisfied".to_string()),
        required: true,
    }];
    let recommendation = if driver_plan
        .impacts
        .iter()
        .any(|impact| impact.baseline.problem_code.is_some_and(|code| code != 0))
    {
        RecommendationState::Repair
    } else {
        RecommendationState::Recommended
    };
    let action = PlannedAction {
        id: driver_plan.action_id.clone(),
        title: format!("Install approved driver package {}", driver_plan.package_id),
        kind: ActionKind::DriverInstall,
        risk: RiskLevel::Normal,
        recommendation,
        verdict: EvidenceVerdict::Certified,
        rationale: "The exact source INF is catalogue-approved, re-verified by Windows, and its present-device blast radius and rollback baseline are captured before authority.".to_string(),
        selected_by_default: false,
        requires_confirmation: true,
        requires_admin: true,
        reboot: RebootRequirement::Possible,
        rollback_available: true,
        evidence: vec![
            EvidenceItem::new("driver.plan_fingerprint", driver_fingerprint.clone(), "neo-driverstore"),
            EvidenceItem::new("driver.source_inf_sha256", driver_plan.source_inf_sha256.clone(), "neo-driverstore"),
            EvidenceItem::new("driver.signer", driver_plan.expected_signature.signer.clone(), "SetupVerifyInfFileW"),
            EvidenceItem::new("driver.impact_count", driver_plan.impacts.len().to_string(), "Windows exact-INF compatibility list"),
        ],
        warnings: vec![
            "Forward installation preserves Windows best-match policy; no force-install flag is authorized.".to_string(),
            "Specific-device installation is reserved exclusively for restoring captured rollback state.".to_string(),
        ],
    };
    let transaction_action = TransactionAction {
        action,
        snapshot_targets: snapshot_targets.clone(),
        postconditions,
        rollback: RollbackPlan::Reversible {
            restore_targets: snapshot_targets,
            verification: rollback_verification,
        },
    };
    Ok(TransactionPlan::new(
        format!("driver-install-{}", &driver_fingerprint[..16]),
        1,
        driver_plan.mission_id.clone(),
        vec![transaction_action],
    )?)
}

pub(crate) fn baseline_contract(
    driver_plan: &DriverInstallPlan,
    transaction_plan: &TransactionPlan,
) -> Result<BaselineSnapshot, DriverStoreError> {
    let fingerprint = driver_plan.fingerprint()?;
    let mut states = vec![CapturedState {
        target: store_target(&fingerprint),
        value: match &driver_plan.store_baseline {
            DriverStoreBaseline::Existing { package } => {
                CapturedValue::Present(serde_json::to_string(package)?)
            }
            DriverStoreBaseline::Absent => CapturedValue::Absent,
        },
    }];
    for impact in &driver_plan.impacts {
        states.push(CapturedState {
            target: binding_target(&impact.instance_id),
            value: CapturedValue::Present(serde_json::to_string(&impact.baseline)?),
        });
    }
    Ok(BaselineSnapshot::for_plan(transaction_plan, states)?)
}

pub(crate) fn store_target(fingerprint: &str) -> StateTarget {
    StateTarget {
        kind: StateTargetKind::Other,
        key: format!("driver-store-package:{fingerprint}"),
    }
}

pub(crate) fn binding_target(instance_id: &str) -> StateTarget {
    StateTarget {
        kind: StateTargetKind::DriverBinding,
        key: instance_id.to_string(),
    }
}

pub(crate) fn policy_target(fingerprint: &str) -> StateTarget {
    StateTarget {
        kind: StateTargetKind::Other,
        key: format!("driver-install-policy:{fingerprint}"),
    }
}

fn resolve_source_inf(
    package_root: &Path,
    inf_path: &str,
) -> Result<(PathBuf, PathBuf), DriverStoreError> {
    let normalized = normalize_inf_path(inf_path)?;
    let relative = Path::new(&normalized);
    if relative.is_absolute()
        || relative.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(DriverStoreError::UnsafeInfPath);
    }
    let root = std::fs::canonicalize(package_root)?;
    let source = std::fs::canonicalize(root.join(relative))?;
    if !source.starts_with(&root)
        || source
            .extension()
            .and_then(|value| value.to_str())
            .is_none_or(|value| !value.eq_ignore_ascii_case("inf"))
    {
        return Err(DriverStoreError::UnsafeInfPath);
    }
    Ok((root, source))
}

fn normalize_inf_path(value: &str) -> Result<String, DriverStoreError> {
    let normalized = value.replace('\\', "/");
    if normalized.trim().is_empty() {
        return Err(DriverStoreError::UnsafeInfPath);
    }
    Ok(normalized)
}

pub(crate) fn signature_matches(
    actual: &VerifiedInfSignature,
    expected: &VerifiedInfSignature,
) -> bool {
    actual
        .signer
        .trim()
        .eq_ignore_ascii_case(expected.signer.trim())
        && file_name(&actual.catalog_file).eq_ignore_ascii_case(&file_name(&expected.catalog_file))
}

fn catalogue_file_matches(actual: &str, expected: &[String]) -> bool {
    let actual_name = file_name(actual);
    expected
        .iter()
        .any(|value| file_name(value).eq_ignore_ascii_case(&actual_name))
}

fn file_name(value: &str) -> String {
    Path::new(value)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(value)
        .to_string()
}

pub(crate) fn normalized_id_set(values: Vec<String>) -> Result<BTreeSet<String>, DriverStoreError> {
    let mut set = BTreeSet::new();
    for value in values {
        if value.trim().is_empty() {
            return Err(DriverStoreError::EmptyField("compatible instance_id"));
        }
        let identity = value.to_ascii_lowercase();
        if !set.insert(identity) {
            return Err(DriverStoreError::DuplicateImpact(value));
        }
    }
    Ok(set)
}
