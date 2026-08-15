#!/usr/bin/env python3
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]


def write(rel: str, content: str) -> None:
    path = ROOT / rel
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content.rstrip() + "\n", encoding="utf-8")


# Workspace membership and the two WinRT namespaces Phase 15 actually uses.
workspace_path = ROOT / "Cargo.toml"
workspace = workspace_path.read_text(encoding="utf-8")
workspace = workspace.replace(
    '    "crates/neo-debloat-probe",\n    "crates/neo-cli",',
    '    "crates/neo-debloat-probe",\n    "crates/neo-debloat-plan",\n    "crates/neo-cli",',
)
workspace = workspace.replace(
    '    "Win32_Devices_DeviceAndDriverInstallation",',
    '    "ApplicationModel",\n    "Management_Deployment",\n    "Win32_Devices_DeviceAndDriverInstallation",',
)
workspace_path.write_text(workspace, encoding="utf-8")

write("crates/neo-debloat-plan/Cargo.toml", r'''
[package]
name = "neo-debloat-plan"
version.workspace = true
edition.workspace = true
authors.workspace = true
repository.workspace = true

[dependencies]
neo-core = { path = "../neo-core" }
neo-debloat = { path = "../neo-debloat" }
neo-debloat-probe = { path = "../neo-debloat-probe" }
neo-transaction = { path = "../neo-transaction" }
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true

[target.'cfg(windows)'.dependencies]
windows.workspace = true
''')

write("crates/neo-debloat-plan/src/error.rs", r'''
use neo_debloat::DebloatError;
use neo_debloat_probe::DebloatProbeError;
use neo_transaction::TransactionError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DebloatPlanError {
    #[error(transparent)]
    Debloat(#[from] DebloatError),
    #[error(transparent)]
    Probe(#[from] DebloatProbeError),
    #[error(transparent)]
    Transaction(#[from] TransactionError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("Neo Phase 15 exact AppX planning is currently supported on Windows only")]
    UnsupportedPlatform,
    #[error("invalid Phase 15 request: {0}")]
    InvalidRequest(String),
    #[error("Phase 15 accepts exactly one selected debloat item per prepared transaction")]
    BatchNotSupported,
    #[error("selected item is not a Phase 13 removal candidate: {0}")]
    NotRemovalCandidate(String),
    #[error("Phase 15 mutation planning supports current-user scope only: {0}")]
    UnsupportedScope(String),
    #[error("declared restore metadata is not executable Phase 15 rollback authority: {0}")]
    RestoreNotReady(String),
    #[error("native AppX inventory failure: {0}")]
    NativeInventory(String),
    #[error("Phase 14 presence and native exact identity evidence disagree: {0}")]
    InventoryDrift(String),
    #[error("missing exact AppX identity: {0}")]
    MissingExactIdentity(String),
    #[error("ambiguous exact AppX identity: {0}")]
    AmbiguousExactIdentity(String),
    #[error("unsupported AppX package kind for controlled removal planning: {0}")]
    UnsafePackageKind(String),
}
''')

write("crates/neo-debloat-plan/src/model.rs", r'''
use crate::DebloatPlanError;
use neo_debloat::DebloatAssessment;
use neo_transaction::{TransactionCheckpoint, TransactionPlan};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExactPackageDependency {
    pub name: String,
    pub full_name: String,
    pub family_name: String,
}

impl ExactPackageDependency {
    pub fn validate(&self) -> Result<(), DebloatPlanError> {
        require_text("dependency name", &self.name)?;
        require_text("dependency full name", &self.full_name)?;
        require_text("dependency family name", &self.family_name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExactPackageIdentity {
    pub name: String,
    pub full_name: String,
    pub family_name: String,
    pub is_framework: bool,
    pub is_resource: bool,
    pub is_bundle: bool,
    pub is_optional: bool,
    #[serde(default)]
    pub dependencies: Vec<ExactPackageDependency>,
}

impl ExactPackageIdentity {
    pub fn validate(&self) -> Result<(), DebloatPlanError> {
        require_text("package name", &self.name)?;
        require_text("package full name", &self.full_name)?;
        require_text("package family name", &self.family_name)?;
        let mut dependency_full_names = BTreeSet::new();
        for dependency in &self.dependencies {
            dependency.validate()?;
            if !dependency_full_names.insert(canonical(&dependency.full_name)) {
                return Err(DebloatPlanError::AmbiguousExactIdentity(format!(
                    "duplicate dependency full name {} on {}",
                    dependency.full_name, self.full_name
                )));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExactAppxInventory {
    pub current_user: Vec<ExactPackageIdentity>,
    pub provisioned: Vec<ExactPackageIdentity>,
    pub source: String,
    pub machine_changes: bool,
}

impl ExactAppxInventory {
    pub fn new(
        current_user: Vec<ExactPackageIdentity>,
        provisioned: Vec<ExactPackageIdentity>,
        source: impl Into<String>,
    ) -> Result<Self, DebloatPlanError> {
        let inventory = Self {
            current_user,
            provisioned,
            source: source.into(),
            machine_changes: false,
        };
        inventory.validate()?;
        Ok(inventory)
    }

    pub fn validate(&self) -> Result<(), DebloatPlanError> {
        require_text("native AppX inventory source", &self.source)?;
        if self.machine_changes {
            return Err(DebloatPlanError::InvalidRequest(
                "exact AppX inventory cannot claim machine changes".to_string(),
            ));
        }
        validate_unique_full_names("current-user", &self.current_user)?;
        validate_unique_full_names("provisioned", &self.provisioned)
    }

    pub(crate) fn current_by_name(&self, package_name: &str) -> Vec<&ExactPackageIdentity> {
        let key = canonical(package_name);
        self.current_user
            .iter()
            .filter(|package| canonical(&package.name) == key)
            .collect()
    }

    pub(crate) fn provisioned_by_name(&self, package_name: &str) -> Vec<&ExactPackageIdentity> {
        let key = canonical(package_name);
        self.provisioned
            .iter()
            .filter(|package| canonical(&package.name) == key)
            .collect()
    }

    pub(crate) fn provisioned_exact(
        &self,
        full_name: &str,
        family_name: &str,
    ) -> Vec<&ExactPackageIdentity> {
        let full_key = canonical(full_name);
        let family_key = canonical(family_name);
        self.provisioned
            .iter()
            .filter(|package| {
                canonical(&package.full_name) == full_key
                    && canonical(&package.family_name) == family_key
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DebloatRestoreRoute {
    RegisterByFullNameFromProvisioned {
        package_full_name: String,
        package_family_name: String,
        dependency_full_names: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DebloatPreparedStep {
    pub debloat_id: String,
    pub package_id: String,
    pub package_full_name: String,
    pub package_family_name: String,
    pub dependency_full_names: Vec<String>,
    pub restore: DebloatRestoreRoute,
}

#[derive(Debug, Clone, Serialize)]
pub struct DebloatPreparedTransaction {
    pub assessment: DebloatAssessment,
    pub steps: Vec<DebloatPreparedStep>,
    pub transaction: TransactionPlan,
    pub checkpoint: TransactionCheckpoint,
    pub machine_changes: bool,
}

impl DebloatPreparedTransaction {
    pub fn plan_fingerprint(&self) -> &str {
        self.checkpoint.plan_fingerprint()
    }
}

fn validate_unique_full_names(
    label: &str,
    packages: &[ExactPackageIdentity],
) -> Result<(), DebloatPlanError> {
    let mut full_names = BTreeSet::new();
    for package in packages {
        package.validate()?;
        if !full_names.insert(canonical(&package.full_name)) {
            return Err(DebloatPlanError::AmbiguousExactIdentity(format!(
                "duplicate {label} package full name {}",
                package.full_name
            )));
        }
    }
    Ok(())
}

fn require_text(label: &str, value: &str) -> Result<(), DebloatPlanError> {
    if value.trim().is_empty() {
        return Err(DebloatPlanError::InvalidRequest(format!(
            "{label} must not be empty"
        )));
    }
    Ok(())
}

pub(crate) fn canonical(value: &str) -> String {
    value.to_ascii_lowercase()
}
''')

write("crates/neo-debloat-plan/src/plan.rs", r'''
use crate::model::{
    canonical, DebloatPreparedStep, DebloatPreparedTransaction, DebloatRestoreRoute,
    ExactAppxInventory, ExactPackageDependency, ExactPackageIdentity,
};
use crate::DebloatPlanError;
use neo_core::{ActionKind, EvidenceItem, PlannedAction, RebootRequirement};
use neo_debloat::{
    assess_debloat, DebloatCatalogue, DebloatDisposition, DebloatEvidence, DebloatProfile,
    DebloatScope, ObservedPresence, RestoreMethod,
};
use neo_transaction::{
    CapturedState, CapturedValue, RollbackPlan, StateTarget, StateTargetKind, TransactionAction,
    TransactionCheckpoint, TransactionPlan, VerificationExpectation, VerificationPredicate,
};

pub fn prepare_debloat_transaction_from_evidence(
    catalogue: &DebloatCatalogue,
    evidence: &DebloatEvidence,
    inventory: &ExactAppxInventory,
    profile: DebloatProfile,
    selected_ids: &[String],
    mission_id: impl Into<String>,
) -> Result<DebloatPreparedTransaction, DebloatPlanError> {
    inventory.validate()?;
    if selected_ids.len() != 1 {
        return Err(DebloatPlanError::BatchNotSupported);
    }
    let mission_id = mission_id.into();
    if mission_id.trim().is_empty() {
        return Err(DebloatPlanError::InvalidRequest(
            "mission id must not be empty".to_string(),
        ));
    }

    let assessment = assess_debloat(catalogue, evidence, profile, selected_ids)?;
    let assessed = assessment
        .items
        .first()
        .ok_or(DebloatPlanError::BatchNotSupported)?;
    if assessed.disposition != DebloatDisposition::RemovalCandidate {
        return Err(DebloatPlanError::NotRemovalCandidate(assessed.id.clone()));
    }
    if assessed.scope != DebloatScope::CurrentUser {
        return Err(DebloatPlanError::UnsupportedScope(assessed.id.clone()));
    }
    if assessed.installed != ObservedPresence::Present {
        return Err(DebloatPlanError::InventoryDrift(format!(
            "{} is not present for the current user",
            assessed.package_id
        )));
    }

    let definition = catalogue
        .get(&assessed.id)
        .ok_or_else(|| DebloatPlanError::NotRemovalCandidate(assessed.id.clone()))?;
    if !matches!(definition.restore, RestoreMethod::ProvisionedImage) {
        return Err(DebloatPlanError::RestoreNotReady(format!(
            "{} requires a matching provisioned staged identity, not {:?}",
            definition.id, definition.restore
        )));
    }
    if assessed.provisioned != ObservedPresence::Present {
        return Err(DebloatPlanError::RestoreNotReady(format!(
            "{} has no proven provisioned identity",
            definition.package_id
        )));
    }

    let current_matches = inventory.current_by_name(&definition.package_id);
    let current = exact_one(
        current_matches,
        &format!("current-user {}", definition.package_id),
    )?;
    ensure_removal_kind(current)?;

    let provisioned_matches = inventory.provisioned_by_name(&definition.package_id);
    let provisioned = exact_one(
        provisioned_matches,
        &format!("provisioned {}", definition.package_id),
    )?;
    ensure_removal_kind(provisioned)?;
    if canonical(&current.full_name) != canonical(&provisioned.full_name)
        || canonical(&current.family_name) != canonical(&provisioned.family_name)
    {
        return Err(DebloatPlanError::InventoryDrift(format!(
            "current/provisioned exact identity mismatch for {}",
            definition.package_id
        )));
    }

    for dependency in &current.dependencies {
        ensure_dependency_restore_ready(inventory, dependency)?;
    }

    let main_target = appx_target(&current.full_name);
    let mut snapshot_targets = vec![main_target.clone()];
    let mut baseline_states = vec![CapturedState {
        target: main_target.clone(),
        value: CapturedValue::Present(serde_json::to_string(current)?),
    }];
    let mut rollback_verification = vec![VerificationPredicate {
        id: format!("rollback:{}:main", definition.id),
        target: main_target.clone(),
        expectation: VerificationExpectation::MatchesBaseline,
        required: true,
    }];

    let mut dependency_full_names = Vec::with_capacity(current.dependencies.len());
    for (index, dependency) in current.dependencies.iter().enumerate() {
        let target = appx_target(&dependency.full_name);
        snapshot_targets.push(target.clone());
        baseline_states.push(CapturedState {
            target: target.clone(),
            value: CapturedValue::Present(serde_json::to_string(dependency)?),
        });
        rollback_verification.push(VerificationPredicate {
            id: format!("rollback:{}:dependency:{index}", definition.id),
            target,
            expectation: VerificationExpectation::MatchesBaseline,
            required: true,
        });
        dependency_full_names.push(dependency.full_name.clone());
    }

    let action = TransactionAction {
        action: PlannedAction {
            id: definition.id.clone(),
            title: format!("Remove {} for current user", definition.title),
            kind: ActionKind::Debloat,
            risk: definition.risk,
            recommendation: definition.recommendation,
            verdict: definition.verdict,
            rationale: "Prepare controlled current-user AppX removal only after exact package/dependency identity capture and deterministic provisioned-image re-registration readiness proof.".to_string(),
            selected_by_default: false,
            requires_confirmation: true,
            requires_admin: false,
            reboot: RebootRequirement::None,
            rollback_available: true,
            evidence: vec![
                EvidenceItem::new(
                    "package_id",
                    definition.package_id.clone(),
                    "Phase 13 certified debloat definition",
                ),
                EvidenceItem::new(
                    "package_full_name",
                    current.full_name.clone(),
                    "Phase 15 native PackageManager current-user inventory",
                ),
                EvidenceItem::new(
                    "package_family_name",
                    current.family_name.clone(),
                    "Phase 15 native PackageManager identity",
                ),
                EvidenceItem::new(
                    "provisioned_restore_identity",
                    provisioned.full_name.clone(),
                    "Phase 15 native PackageManager provisioned inventory",
                ),
                EvidenceItem::new(
                    "dependency_count",
                    current.dependencies.len().to_string(),
                    "Phase 15 native Package.Dependencies inventory",
                ),
            ],
            warnings: vec![
                "Phase 15 prepares evidence and transaction state only; no AppX mutation capability is issued.".to_string(),
                "Windows may remove unneeded dependency registrations with the main package; every direct dependency is captured as a rollback target and must have a matching provisioned staged identity.".to_string(),
            ],
        },
        snapshot_targets: snapshot_targets.clone(),
        postconditions: vec![VerificationPredicate {
            id: format!("verify:{}:main-absent", definition.id),
            target: main_target,
            expectation: VerificationExpectation::Absent,
            required: true,
        }],
        rollback: RollbackPlan::Reversible {
            restore_targets: snapshot_targets,
            verification: rollback_verification,
        },
    };

    let transaction = TransactionPlan::new(
        format!("{mission_id}:phase15-debloat-current-user"),
        1,
        mission_id,
        vec![action],
    )?;
    let mut checkpoint = TransactionCheckpoint::new(transaction.clone())?;
    checkpoint.capture_baseline(baseline_states)?;

    let step = DebloatPreparedStep {
        debloat_id: definition.id.clone(),
        package_id: definition.package_id.clone(),
        package_full_name: current.full_name.clone(),
        package_family_name: current.family_name.clone(),
        dependency_full_names: dependency_full_names.clone(),
        restore: DebloatRestoreRoute::RegisterByFullNameFromProvisioned {
            package_full_name: current.full_name.clone(),
            package_family_name: current.family_name.clone(),
            dependency_full_names,
        },
    };

    Ok(DebloatPreparedTransaction {
        assessment,
        steps: vec![step],
        transaction,
        checkpoint,
        machine_changes: false,
    })
}

fn exact_one<'a>(
    matches: Vec<&'a ExactPackageIdentity>,
    label: &str,
) -> Result<&'a ExactPackageIdentity, DebloatPlanError> {
    match matches.as_slice() {
        [] => Err(DebloatPlanError::MissingExactIdentity(label.to_string())),
        [package] => Ok(*package),
        _ => Err(DebloatPlanError::AmbiguousExactIdentity(label.to_string())),
    }
}

fn ensure_removal_kind(package: &ExactPackageIdentity) -> Result<(), DebloatPlanError> {
    if package.is_framework || package.is_resource {
        return Err(DebloatPlanError::UnsafePackageKind(package.full_name.clone()));
    }
    Ok(())
}

fn ensure_dependency_restore_ready(
    inventory: &ExactAppxInventory,
    dependency: &ExactPackageDependency,
) -> Result<(), DebloatPlanError> {
    let matches = inventory.provisioned_exact(&dependency.full_name, &dependency.family_name);
    match matches.as_slice() {
        [_] => Ok(()),
        [] => Err(DebloatPlanError::RestoreNotReady(format!(
            "dependency {} is not present as the exact provisioned staged identity",
            dependency.full_name
        ))),
        _ => Err(DebloatPlanError::AmbiguousExactIdentity(format!(
            "dependency {} has multiple provisioned exact identities",
            dependency.full_name
        ))),
    }
}

fn appx_target(full_name: &str) -> StateTarget {
    StateTarget {
        kind: StateTargetKind::AppxPackage,
        key: format!("current_user:{full_name}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ExactPackageDependency, ExactPackageIdentity};
    use neo_transaction::TransactionStage;

    fn catalogue(scope: &str, restore: &str) -> DebloatCatalogue {
        serde_json::from_str(&format!(
            r#"{{"items":[{{"id":"appx.contoso.phase15","package_id":"Contoso.Phase15","title":"Contoso Phase15","category":"Fixture","description":"Synthetic Phase 15 package","class":"safe_optional","scope":"{scope}","risk":"low","recommendation":"optional_component","verdict":"certified","selected_by_default":false,"restore":{restore},"side_effects":[],"preserve_in_profiles":[]}}]}}"#
        ))
        .expect("catalogue must validate")
    }

    fn evidence(installed: &str, provisioned: &str) -> DebloatEvidence {
        serde_json::from_str(&format!(
            r#"{{"observations":[{{"package_id":"Contoso.Phase15","installed":"{installed}","provisioned":"{provisioned}","version":"1.2.3.4","source":"phase15-test"}}]}}"#
        ))
        .expect("evidence must validate")
    }

    fn identity() -> ExactPackageIdentity {
        ExactPackageIdentity {
            name: "Contoso.Phase15".to_string(),
            full_name: "Contoso.Phase15_1.2.3.4_x64__contoso".to_string(),
            family_name: "Contoso.Phase15_contoso".to_string(),
            is_framework: false,
            is_resource: false,
            is_bundle: false,
            is_optional: false,
            dependencies: vec![ExactPackageDependency {
                name: "Contoso.Framework".to_string(),
                full_name: "Contoso.Framework_1.0.0.0_x64__contoso".to_string(),
                family_name: "Contoso.Framework_contoso".to_string(),
            }],
        }
    }

    fn inventory() -> ExactAppxInventory {
        let current = identity();
        let dependency = ExactPackageIdentity {
            name: "Contoso.Framework".to_string(),
            full_name: current.dependencies[0].full_name.clone(),
            family_name: current.dependencies[0].family_name.clone(),
            is_framework: true,
            is_resource: false,
            is_bundle: false,
            is_optional: false,
            dependencies: Vec::new(),
        };
        ExactAppxInventory::new(
            vec![current.clone(), dependency.clone()],
            vec![current, dependency],
            "phase15-test-native",
        )
        .expect("inventory must validate")
    }

    #[test]
    fn prepares_exact_current_user_transaction_without_mutation_authority() {
        let prepared = prepare_debloat_transaction_from_evidence(
            &catalogue("current_user", r#"{"kind":"provisioned_image"}"#),
            &evidence("present", "present"),
            &inventory(),
            DebloatProfile::SafeCleanup,
            &["appx.contoso.phase15".to_string()],
            "mission-phase15",
        )
        .expect("transaction readiness should be proven");

        assert!(!prepared.machine_changes);
        assert_eq!(prepared.steps.len(), 1);
        assert_eq!(prepared.transaction.actions().len(), 1);
        assert_eq!(prepared.checkpoint.stage(), TransactionStage::BaselineCaptured);
        assert_eq!(
            prepared.checkpoint.plan_fingerprint(),
            prepared.transaction.fingerprint().expect("fingerprint must compute")
        );
        assert_eq!(
            prepared.transaction.actions()[0].action.kind,
            ActionKind::Debloat
        );
        assert_eq!(prepared.transaction.actions()[0].snapshot_targets.len(), 2);
    }

    #[test]
    fn store_metadata_is_not_treated_as_executable_rollback() {
        let error = prepare_debloat_transaction_from_evidence(
            &catalogue("current_user", r#"{"kind":"store","store_id":"9CONTOSO15"}"#),
            &evidence("present", "present"),
            &inventory(),
            DebloatProfile::SafeCleanup,
            &["appx.contoso.phase15".to_string()],
            "mission-phase15",
        )
        .expect_err("Store metadata is not deterministic local rollback authority");
        assert!(matches!(error, DebloatPlanError::RestoreNotReady(_)));
    }

    #[test]
    fn combined_or_provisioned_scope_stays_blocked() {
        let error = prepare_debloat_transaction_from_evidence(
            &catalogue(
                "current_user_and_provisioned",
                r#"{"kind":"provisioned_image"}"#,
            ),
            &evidence("present", "present"),
            &inventory(),
            DebloatProfile::SafeCleanup,
            &["appx.contoso.phase15".to_string()],
            "mission-phase15",
        )
        .expect_err("provisioning mutation remains separately gated");
        assert!(matches!(error, DebloatPlanError::UnsupportedScope(_)));
    }

    #[test]
    fn exact_identity_drift_fails_closed() {
        let mut drifted = inventory();
        drifted.current_user[0].full_name =
            "Contoso.Phase15_9.9.9.9_x64__contoso".to_string();
        let error = prepare_debloat_transaction_from_evidence(
            &catalogue("current_user", r#"{"kind":"provisioned_image"}"#),
            &evidence("present", "present"),
            &drifted,
            DebloatProfile::SafeCleanup,
            &["appx.contoso.phase15".to_string()],
            "mission-phase15",
        )
        .expect_err("drift must block authority preparation");
        assert!(matches!(error, DebloatPlanError::InventoryDrift(_)));
    }

    #[test]
    fn missing_dependency_restore_identity_fails_closed() {
        let mut missing = inventory();
        missing
            .provisioned
            .retain(|package| package.name != "Contoso.Framework");
        let error = prepare_debloat_transaction_from_evidence(
            &catalogue("current_user", r#"{"kind":"provisioned_image"}"#),
            &evidence("present", "present"),
            &missing,
            DebloatProfile::SafeCleanup,
            &["appx.contoso.phase15".to_string()],
            "mission-phase15",
        )
        .expect_err("dependency restore gap must block planning");
        assert!(matches!(error, DebloatPlanError::RestoreNotReady(_)));
    }

    #[test]
    fn batch_preparation_is_deliberately_blocked() {
        let error = prepare_debloat_transaction_from_evidence(
            &catalogue("current_user", r#"{"kind":"provisioned_image"}"#),
            &evidence("present", "present"),
            &inventory(),
            DebloatProfile::SafeCleanup,
            &[
                "appx.contoso.phase15".to_string(),
                "appx.contoso.phase15.other".to_string(),
            ],
            "mission-phase15",
        )
        .expect_err("Phase 15 is single-item only");
        assert!(matches!(error, DebloatPlanError::BatchNotSupported));
    }
}
''')

write("crates/neo-debloat-plan/src/windows.rs", r'''
use crate::{DebloatPlanError, ExactAppxInventory, ExactPackageDependency, ExactPackageIdentity};
use windows::ApplicationModel::Package;
use windows::Management::Deployment::PackageManager;
use windows::core::HSTRING;

pub(crate) fn scan_native_inventory() -> Result<ExactAppxInventory, DebloatPlanError> {
    let manager = PackageManager::new().map_err(native_error("create PackageManager"))?;

    let mut current_user = manager
        .FindPackagesByUserSecurityId(&HSTRING::new())
        .map_err(native_error("enumerate current-user packages"))?
        .into_iter()
        .map(|package| package_identity(&package))
        .collect::<Result<Vec<_>, _>>()?;

    let mut provisioned = manager
        .FindProvisionedPackages()
        .map_err(native_error("enumerate provisioned packages"))?
        .into_iter()
        .map(|package| package_identity(&package))
        .collect::<Result<Vec<_>, _>>()?;

    current_user.sort_by(|left, right| {
        left.full_name
            .to_ascii_lowercase()
            .cmp(&right.full_name.to_ascii_lowercase())
    });
    provisioned.sort_by(|left, right| {
        left.full_name
            .to_ascii_lowercase()
            .cmp(&right.full_name.to_ascii_lowercase())
    });

    ExactAppxInventory::new(
        current_user,
        provisioned,
        "neo-debloat-plan:Windows.Management.Deployment.PackageManager",
    )
}

fn package_identity(package: &Package) -> Result<ExactPackageIdentity, DebloatPlanError> {
    let id = package.Id().map_err(native_error("read Package.Id"))?;
    let mut dependencies = package
        .Dependencies()
        .map_err(native_error("read Package.Dependencies"))?
        .into_iter()
        .map(|dependency| {
            let dependency_id = dependency
                .Id()
                .map_err(native_error("read dependency Package.Id"))?;
            Ok(ExactPackageDependency {
                name: dependency_id
                    .Name()
                    .map_err(native_error("read dependency name"))?
                    .to_string_lossy(),
                full_name: dependency_id
                    .FullName()
                    .map_err(native_error("read dependency full name"))?
                    .to_string_lossy(),
                family_name: dependency_id
                    .FamilyName()
                    .map_err(native_error("read dependency family name"))?
                    .to_string_lossy(),
            })
        })
        .collect::<Result<Vec<_>, DebloatPlanError>>()?;
    dependencies.sort_by(|left, right| {
        left.full_name
            .to_ascii_lowercase()
            .cmp(&right.full_name.to_ascii_lowercase())
    });

    Ok(ExactPackageIdentity {
        name: id
            .Name()
            .map_err(native_error("read package name"))?
            .to_string_lossy(),
        full_name: id
            .FullName()
            .map_err(native_error("read package full name"))?
            .to_string_lossy(),
        family_name: id
            .FamilyName()
            .map_err(native_error("read package family name"))?
            .to_string_lossy(),
        is_framework: package
            .IsFramework()
            .map_err(native_error("read package framework flag"))?,
        is_resource: package
            .IsResourcePackage()
            .map_err(native_error("read package resource flag"))?,
        is_bundle: package
            .IsBundle()
            .map_err(native_error("read package bundle flag"))?,
        is_optional: package
            .IsOptional()
            .map_err(native_error("read package optional flag"))?,
        dependencies,
    })
}

fn native_error(
    operation: &'static str,
) -> impl FnOnce(windows::core::Error) -> DebloatPlanError {
    move |error| DebloatPlanError::NativeInventory(format!("{operation}: {error}"))
}
''')

write("crates/neo-debloat-plan/src/lib.rs", r'''
//! Phase 15 exact AppX mutation-plan and rollback-readiness boundary.
//!
//! Phase 15 is still read-only. It composes the proven Phase 14 presence evidence with a native
//! PackageManager exact-identity inventory, then prepares one current-user Debloat transaction
//! only when the exact package and every direct dependency have matching provisioned staged
//! identities suitable for a future RegisterPackageByFullName rollback path. No removal,
//! registration, deprovisioning, provisioning, capability issuance, CLI write command, plugin,
//! or MCP/RPC debloat authority exists in this crate.

mod error;
mod model;
mod plan;
#[cfg(target_os = "windows")]
mod windows;

pub use error::DebloatPlanError;
pub use model::{
    DebloatPreparedStep, DebloatPreparedTransaction, DebloatRestoreRoute, ExactAppxInventory,
    ExactPackageDependency, ExactPackageIdentity,
};
pub use plan::prepare_debloat_transaction_from_evidence;

use neo_debloat::{DebloatCatalogue, DebloatProfile};

pub fn scan_windows_exact_appx_inventory() -> Result<ExactAppxInventory, DebloatPlanError> {
    #[cfg(target_os = "windows")]
    {
        windows::scan_native_inventory()
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(DebloatPlanError::UnsupportedPlatform)
    }
}

pub fn prepare_windows_debloat_transaction(
    catalogue: &DebloatCatalogue,
    profile: DebloatProfile,
    selected_ids: &[String],
    mission_id: impl Into<String>,
) -> Result<DebloatPreparedTransaction, DebloatPlanError> {
    #[cfg(target_os = "windows")]
    {
        let phase14 = neo_debloat_probe::scan_current_debloat_evidence(catalogue)?;
        let exact = windows::scan_native_inventory()?;
        prepare_debloat_transaction_from_evidence(
            catalogue,
            &phase14.evidence,
            &exact,
            profile,
            selected_ids,
            mission_id,
        )
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (catalogue, profile, selected_ids, mission_id.into());
        Err(DebloatPlanError::UnsupportedPlatform)
    }
}
''')

write("crates/neo-debloat-plan/src/bin/neo-debloat-prepare.rs", r'''
use neo_debloat::{DebloatCatalogue, DebloatEvidence, DebloatProfile};
use neo_debloat_plan::{prepare_debloat_transaction_from_evidence, ExactAppxInventory};
use std::env;
use std::fs;
use std::str::FromStr;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = env::args().collect::<Vec<_>>();
    if args.len() < 8 {
        return Err("usage: neo-debloat-prepare <catalogue.json> <evidence.json> <exact-inventory.json> <profile> <selected-id> <mission-id> <--json>".into());
    }
    let catalogue: DebloatCatalogue = serde_json::from_str(&fs::read_to_string(&args[1])?)?;
    let evidence: DebloatEvidence = serde_json::from_str(&fs::read_to_string(&args[2])?)?;
    let inventory: ExactAppxInventory = serde_json::from_str(&fs::read_to_string(&args[3])?)?;
    let profile = DebloatProfile::from_str(&args[4])?;
    let selected = vec![args[5].clone()];
    let prepared = prepare_debloat_transaction_from_evidence(
        &catalogue,
        &evidence,
        &inventory,
        profile,
        &selected,
        args[6].clone(),
    )?;
    if args[7] == "--json" {
        println!("{}", serde_json::to_string_pretty(&prepared)?);
    } else {
        println!("Prepared transaction: {}", prepared.transaction.transaction_id());
        println!("Plan fingerprint: {}", prepared.plan_fingerprint());
        println!("Machine changes: none");
    }
    Ok(())
}
''')

write("crates/neo-debloat-plan/tests/live_read_only.rs", r'''
#![cfg(target_os = "windows")]

use neo_debloat_plan::scan_windows_exact_appx_inventory;
use std::fs;
use std::path::{Path, PathBuf};

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("fixtures")
        .join("debloat")
}

fn directory_snapshot(path: &Path) -> Vec<(String, Vec<u8>)> {
    let mut entries = fs::read_dir(path)
        .expect("fixture directory must exist")
        .map(|entry| {
            let entry = entry.expect("fixture entry must be readable");
            let name = entry.file_name().to_string_lossy().into_owned();
            let bytes = fs::read(entry.path()).expect("fixture file must be readable");
            (name, bytes)
        })
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| left.0.cmp(&right.0));
    entries
}

#[test]
fn native_exact_appx_identity_scan_is_read_only_to_fixture_state() {
    let fixtures = fixture_dir();
    let before = directory_snapshot(&fixtures);
    let inventory = scan_windows_exact_appx_inventory().expect("native exact inventory must execute");
    assert!(!inventory.machine_changes);
    assert!(inventory
        .current_user
        .iter()
        .all(|package| !package.name.trim().is_empty()
            && !package.full_name.trim().is_empty()
            && !package.family_name.trim().is_empty()));
    assert!(inventory
        .provisioned
        .iter()
        .all(|package| !package.name.trim().is_empty()
            && !package.full_name.trim().is_empty()
            && !package.family_name.trim().is_empty()));
    let after = directory_snapshot(&fixtures);
    assert_eq!(
        before, after,
        "native exact AppX inventory changed fixture state"
    );
}
''')

write("fixtures/debloat/phase15_catalogue.json", r'''
{
  "items": [
    {
      "id": "appx.contoso.phase15",
      "package_id": "Contoso.Phase15",
      "title": "Contoso Phase15 Optional",
      "category": "Synthetic Optional Apps",
      "description": "Synthetic current-user package used only to prove Phase 15 exact transaction readiness.",
      "class": "safe_optional",
      "scope": "current_user",
      "risk": "low",
      "recommendation": "optional_component",
      "verdict": "certified",
      "selected_by_default": false,
      "restore": {
        "kind": "provisioned_image"
      },
      "side_effects": [],
      "preserve_in_profiles": []
    }
  ]
}
''')

write("fixtures/debloat/phase15_evidence.json", r'''
{
  "observations": [
    {
      "package_id": "Contoso.Phase15",
      "installed": "present",
      "provisioned": "present",
      "version": "1.2.3.4",
      "source": "phase15 synthetic Phase 14 evidence"
    }
  ]
}
''')

write("fixtures/debloat/phase15_inventory.json", r'''
{
  "current_user": [
    {
      "name": "Contoso.Phase15",
      "full_name": "Contoso.Phase15_1.2.3.4_x64__contoso",
      "family_name": "Contoso.Phase15_contoso",
      "is_framework": false,
      "is_resource": false,
      "is_bundle": false,
      "is_optional": false,
      "dependencies": [
        {
          "name": "Contoso.Framework",
          "full_name": "Contoso.Framework_1.0.0.0_x64__contoso",
          "family_name": "Contoso.Framework_contoso"
        }
      ]
    },
    {
      "name": "Contoso.Framework",
      "full_name": "Contoso.Framework_1.0.0.0_x64__contoso",
      "family_name": "Contoso.Framework_contoso",
      "is_framework": true,
      "is_resource": false,
      "is_bundle": false,
      "is_optional": false,
      "dependencies": []
    }
  ],
  "provisioned": [
    {
      "name": "Contoso.Phase15",
      "full_name": "Contoso.Phase15_1.2.3.4_x64__contoso",
      "family_name": "Contoso.Phase15_contoso",
      "is_framework": false,
      "is_resource": false,
      "is_bundle": false,
      "is_optional": false,
      "dependencies": [
        {
          "name": "Contoso.Framework",
          "full_name": "Contoso.Framework_1.0.0.0_x64__contoso",
          "family_name": "Contoso.Framework_contoso"
        }
      ]
    },
    {
      "name": "Contoso.Framework",
      "full_name": "Contoso.Framework_1.0.0.0_x64__contoso",
      "family_name": "Contoso.Framework_contoso",
      "is_framework": true,
      "is_resource": false,
      "is_bundle": false,
      "is_optional": false,
      "dependencies": []
    }
  ],
  "source": "phase15 synthetic native inventory",
  "machine_changes": false
}
''')

write("docs/decisions/0015-PHASE15-DEBLOAT-TRANSACTION-READINESS.md", r'''
# Decision 0015 — Phase 15 Debloat Transaction Readiness

**Status:** FROZEN FOR IMPLEMENTATION PROOF  
**Product domain:** third bounded child of the frozen Debloat domain  
**Authority:** read-only exact AppX identity, rollback-readiness, and transaction preparation only

## Decision

Phase 15 does not remove an AppX package. It closes the evidence gap between Phase 14 logical presence and a future controlled executor by proving the exact package identity, exact direct dependency set, deterministic local restore readiness, and Phase 4 transaction/checkpoint binding before any mutation capability can exist.

The flow is:

```text
Phase 13 certified removal candidate
        +
Phase 14 current-user/provisioned presence evidence
        +
Windows PackageManager exact identity inventory
        ↓
exact package full/family identity + direct dependencies
        ↓
matching provisioned staged identity for main + every dependency
        ↓
Phase 4 Debloat transaction + captured baseline checkpoint
        ↓
NO APPLY CAPABILITY IN PHASE 15
```

## Scope

Phase 15 intentionally prepares exactly one `CurrentUser` removal candidate at a time. `Provisioned` and `CurrentUserAndProvisioned` mutation planning remain blocked because deprovisioning is a separate administrator operation with a different restore and verification contract. Batch removal also remains blocked because selected apps can share framework/dependency rollback targets and Phase 4 correctly rejects overlapping state targets.

Only a Phase 13 `RemovalCandidate` may enter Phase 15. Protected, profile-preserved, policy-blocked, review-only, absent, uncertified, higher-risk, or unavailable-evidence items cannot be converted into a transaction plan here.

## Exact Windows identity boundary

Phase 15 adds a native read-only `Windows.Management.Deployment.PackageManager` inventory in Rust. It records for current-user and provisioned packages:

- package `Name`;
- package `FullName`;
- package `FamilyName`;
- framework/resource/bundle/optional classification flags;
- exact direct dependency names/full names/family names for current-user packages.

Catalogue IDs do not become WinRT method names, commands, scripts, paths, or executable text. Matching remains case-insensitive in Rust. Duplicate exact full names, missing identities, ambiguous package-name matches, resource/framework main candidates, or disagreement with Phase 14 presence fail closed.

## Restore readiness law

Phase 13 restore metadata is descriptive until Phase 15 proves an executable local rollback route. Phase 15 therefore accepts only `RestoreMethod::ProvisionedImage` for prepared mutation authority and additionally requires:

- the selected current-user package to have exactly one exact identity;
- an exact matching provisioned package with the same FullName and FamilyName;
- every direct dependency to have an exact matching provisioned FullName and FamilyName.

Store IDs and vendor-source metadata remain useful recovery information but are not treated as deterministic local rollback authority. Phase 15 does not perform Store/network acquisition.

This readiness contract is designed for the future native PackageManager executor: current-user removal is keyed by package FullName, while current-user re-registration can be keyed by the same staged package FullName plus dependency package FullNames. Phase 15 only proves and records those identities; it does not invoke either operation.

## Transaction law

A prepared item becomes one Phase 4 `ActionKind::Debloat` transaction action with:

- explicit confirmation required;
- the exact current-user package FullName as the main `AppxPackage` state target;
- every exact direct dependency FullName as an additional rollback state target;
- exact serialized baseline identities captured into the Phase 4 checkpoint;
- required postcondition that the main current-user package becomes absent;
- reversible rollback metadata requiring every captured target to match its baseline after restoration;
- transaction fingerprint binding all of the above.

The checkpoint stops at `BaselineCaptured`. No authorization, apply record, verification result, rollback execution, or capability issuance occurs in Phase 15.

## Proof boundary

Phase 15 proves:

- native PackageManager current-user exact identity enumeration on real Windows CI;
- native PackageManager provisioned exact identity enumeration on real Windows CI;
- exact package/dependency validation and case-insensitive matching;
- Phase 14-vs-native drift rejection;
- single-item/current-user-only authority boundary;
- resource/framework main-candidate rejection;
- Store/vendor metadata not misrepresented as executable rollback;
- exact provisioned twin required for main and every direct dependency;
- Phase 4 `ActionKind::Debloat` transaction creation;
- baseline capture and transaction fingerprint continuity;
- deterministic fixture proof on Ubuntu and Windows;
- byte-for-byte fixture-tree equality around the Windows live inventory proof;
- continued absence of AppX mutation capability.

Phase 15 does **not** prove or implement:

- `PackageManager.RemovePackageAsync` execution;
- `RegisterPackageByFullNameAsync` execution;
- deprovision/provision execution;
- Store/network restore;
- batch debloat transactions;
- all-users package mutation;
- live package mutation on CI or a donor machine;
- public CLI/GUI write actions;
- plugin dependency;
- MCP/RPC debloat capability issuance or execution.

Those remain separately gated.
''')

write("docs/PHASE15_20_LANE_REVIEW.md", r'''
# Phase 15 — 20-Lane Engineering Review

**Boundary:** exact AppX identity + rollback readiness + Phase 4 transaction preparation only  
**Mutation authority:** none

1. Phase 13 candidate law remains authoritative.
2. Phase 14 presence evidence is composed rather than bypassed.
3. Native PackageManager inventory is read-only.
4. Exact Name/FullName/FamilyName are captured.
5. Direct dependency identities are captured.
6. Exact inventory validates non-empty identities.
7. Duplicate full names fail closed.
8. Package-name ambiguity fails closed.
9. Phase 14/native evidence drift fails closed.
10. Framework/resource packages cannot be main removal candidates.
11. Exactly one selected item is allowed.
12. Only current-user scope is allowed.
13. Store/vendor metadata is not promoted to rollback authority.
14. Main package requires an exact matching provisioned staged identity.
15. Every direct dependency requires an exact matching provisioned staged identity.
16. Debloat transaction uses exact `AppxPackage` state targets and explicit confirmation.
17. Baseline checkpoint contains main and dependency identity state.
18. Fingerprint binds the exact prepared plan and rollback obligations.
19. Windows live native inventory proof is behaviorally read-only and fixture-preserving.
20. No remove/register/deprovision/provision/public-write/plugin/MCP-RPC capability exists.

All twenty lanes must pass together. A failed lane blocks Phase 15 freeze.
''')

write("tools/phase15_static_review.py", r'''
#!/usr/bin/env python3
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CRATE = ROOT / "crates" / "neo-debloat-plan"
SRC = CRATE / "src"
production = "\n".join(path.read_text(encoding="utf-8") for path in sorted(SRC.rglob("*.rs")))
workspace = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
manifest = (CRATE / "Cargo.toml").read_text(encoding="utf-8")
review = (ROOT / "docs" / "PHASE15_20_LANE_REVIEW.md").read_text(encoding="utf-8")
decision = (ROOT / "docs" / "decisions" / "0015-PHASE15-DEBLOAT-TRANSACTION-READINESS.md").read_text(encoding="utf-8")
behavior = (CRATE / "tests" / "live_read_only.rs").read_text(encoding="utf-8")
ci = (ROOT / ".github" / "workflows" / "ci.yml").read_text(encoding="utf-8")

phase15_static_step = """      - name: Phase 15 twenty-lane static review
        run: python -W error tools/phase15_static_review.py"""
phase15_live_step = """      - name: Phase 15 live Windows exact AppX identity proof
        if: runner.os == 'Windows'
        run: cargo test --locked -p neo-debloat-plan --test live_read_only"""
phase15_fixture_step = """      - name: Phase 15 transaction-readiness fixture proof
        run: cargo run --locked -p neo-debloat-plan --bin neo-debloat-prepare -- fixtures/debloat/phase15_catalogue.json fixtures/debloat/phase15_evidence.json fixtures/debloat/phase15_inventory.json safe-cleanup appx.contoso.phase15 phase15-fixture --json"""

checks = [
    ("workspace member", '"crates/neo-debloat-plan"' in workspace),
    ("bounded dependencies", all(name in manifest for name in ("neo-core", "neo-debloat", "neo-debloat-probe", "neo-transaction"))),
    ("native PackageManager read surface", all(value in production for value in ("PackageManager::new", "FindPackagesByUserSecurityId", "FindProvisionedPackages"))),
    ("exact package identity", all(value in production for value in ("pub name: String", "pub full_name: String", "pub family_name: String"))),
    ("direct dependency identity", all(value in production for value in ("Package.Dependencies", "dependency_full_names", "ExactPackageDependency"))),
    ("inventory validation", "inventory.validate()?" in production and "validate_unique_full_names" in production),
    ("duplicate exact identity rejected", "AmbiguousExactIdentity" in production and "duplicate {label} package full name" in production),
    ("package-name ambiguity rejected", "fn exact_one" in production and "AmbiguousExactIdentity(label.to_string())" in production),
    ("phase14 native drift rejected", "InventoryDrift" in production and "current/provisioned exact identity mismatch" in production),
    ("unsafe main package kinds blocked", "package.is_framework || package.is_resource" in production and "UnsafePackageKind" in production),
    ("single item only", "selected_ids.len() != 1" in production and "BatchNotSupported" in production),
    ("current user only", "assessed.scope != DebloatScope::CurrentUser" in production and "UnsupportedScope" in production),
    ("metadata not rollback authority", "RestoreMethod::ProvisionedImage" in production and "Store metadata is not deterministic local rollback authority" in production),
    ("main provisioned twin", "provisioned_by_name" in production and "canonical(&current.full_name) != canonical(&provisioned.full_name)" in production),
    ("dependency provisioned twins", "ensure_dependency_restore_ready" in production and "provisioned_exact" in production),
    ("debloat transaction binding", "kind: ActionKind::Debloat" in production and "requires_confirmation: true" in production and "StateTargetKind::AppxPackage" in production),
    ("captured baseline checkpoint", "checkpoint.capture_baseline(baseline_states)?" in production and "CapturedValue::Present" in production),
    ("fingerprint continuity", "plan_fingerprint" in production and "transaction.fingerprint()" in production),
    ("live read only behavior", "native_exact_appx_identity_scan_is_read_only_to_fixture_state" in behavior and "assert!(!inventory.machine_changes);" in behavior and "before, after," in behavior),
    (
        "negative mutation and integration boundary",
        all(value not in production for value in ("RemovePackageAsync(", "RegisterPackageByFullNameAsync(", "DeprovisionPackageForAllUsersAsync(", "ProvisionPackageForAllUsersAsync(", "MCP_TWEAK", "rpc::", "plugin"))
        and "**Mutation authority:** none" in review
        and "plugin dependency" in decision
        and phase15_static_step in ci
        and phase15_live_step in ci
        and phase15_fixture_step in ci,
    ),
]

failed = []
for index, (name, ok) in enumerate(checks, 1):
    print(f"{index:02d}. {'PASS' if ok else 'FAIL'} - {name}")
    if not ok:
        print(f"::error title=Phase 15 lane {index:02d} failed::{name}")
        failed.append(name)

if failed:
    raise SystemExit("Phase 15 static review failed: " + ", ".join(failed))

print("PHASE 15 STATIC REVIEW PASS: 20/20")
''')

ci_path = ROOT / ".github" / "workflows" / "ci.yml"
ci = ci_path.read_text(encoding="utf-8")
ci = ci.replace(
    "      - name: Phase 14 twenty-lane static review\n        run: python -W error tools/phase14_static_review.py\n",
    "      - name: Phase 14 twenty-lane static review\n        run: python -W error tools/phase14_static_review.py\n\n      - name: Phase 15 twenty-lane static review\n        run: python -W error tools/phase15_static_review.py\n",
)
ci = ci.replace(
    "      - name: Phase 14 live Windows debloat inventory proof\n        if: runner.os == 'Windows'\n        run: cargo test --locked -p neo-debloat-probe --test live_read_only\n",
    "      - name: Phase 14 live Windows debloat inventory proof\n        if: runner.os == 'Windows'\n        run: cargo test --locked -p neo-debloat-probe --test live_read_only\n\n      - name: Phase 15 live Windows exact AppX identity proof\n        if: runner.os == 'Windows'\n        run: cargo test --locked -p neo-debloat-plan --test live_read_only\n",
)
ci = ci.replace(
    "      - name: Debloat assessment fixture proof\n        run: cargo run --locked -p neo-debloat --bin neo-debloat-assess -- fixtures/debloat/catalogue.json fixtures/debloat/evidence.json gaming appx.contoso.optional,appx.contoso.gaming,appx.contoso.system --json\n",
    "      - name: Debloat assessment fixture proof\n        run: cargo run --locked -p neo-debloat --bin neo-debloat-assess -- fixtures/debloat/catalogue.json fixtures/debloat/evidence.json gaming appx.contoso.optional,appx.contoso.gaming,appx.contoso.system --json\n\n      - name: Phase 15 transaction-readiness fixture proof\n        run: cargo run --locked -p neo-debloat-plan --bin neo-debloat-prepare -- fixtures/debloat/phase15_catalogue.json fixtures/debloat/phase15_evidence.json fixtures/debloat/phase15_inventory.json safe-cleanup appx.contoso.phase15 phase15-fixture --json\n",
)
ci_path.write_text(ci, encoding="utf-8")

subprocess.run(["cargo", "fmt", "--all"], cwd=ROOT, check=True)
subprocess.run(["cargo", "generate-lockfile"], cwd=ROOT, check=True)
subprocess.run(["python", "-W", "error", "tools/phase15_static_review.py"], cwd=ROOT, check=True)

# One-shot helper removes itself and its workflow before committing the real branch surface.
(ROOT / "tools" / "phase15_apply_patch.py").unlink()
(ROOT / ".github" / "workflows" / "phase15_apply_patch.yml").unlink()
