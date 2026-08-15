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
    pub(crate) assessment: DebloatAssessment,
    pub(crate) steps: Vec<DebloatPreparedStep>,
    pub(crate) transaction: TransactionPlan,
    pub(crate) checkpoint: TransactionCheckpoint,
    pub(crate) machine_changes: bool,
}

impl DebloatPreparedTransaction {
    pub fn assessment(&self) -> &DebloatAssessment {
        &self.assessment
    }

    pub fn steps(&self) -> &[DebloatPreparedStep] {
        &self.steps
    }

    pub fn transaction(&self) -> &TransactionPlan {
        &self.transaction
    }

    pub fn checkpoint(&self) -> &TransactionCheckpoint {
        &self.checkpoint
    }

    pub fn machine_changes(&self) -> bool {
        self.machine_changes
    }

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
