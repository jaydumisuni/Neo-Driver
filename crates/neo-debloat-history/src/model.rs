use crate::DebloatHistoryError;
use neo_debloat_plan::{ExactPackageDependency, ExactPackageIdentity};
use neo_transaction::{TransactionCheckpoint, TransactionPlan, TransactionStage};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt::Write as _;

pub const DEBLOAT_REMOVAL_RECEIPT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryRestoreRoute {
    package_full_name: String,
    package_family_name: String,
    dependency_full_names: Vec<String>,
}

impl HistoryRestoreRoute {
    pub(crate) fn new(
        package_full_name: String,
        package_family_name: String,
        dependency_full_names: Vec<String>,
    ) -> Self {
        Self {
            package_full_name,
            package_family_name,
            dependency_full_names,
        }
    }

    pub fn package_full_name(&self) -> &str {
        &self.package_full_name
    }

    pub fn package_family_name(&self) -> &str {
        &self.package_family_name
    }

    pub fn dependency_full_names(&self) -> &[String] {
        &self.dependency_full_names
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DebloatRemovalReceipt {
    schema_version: u32,
    receipt_id: String,
    source_transaction_id: String,
    source_transaction_fingerprint: String,
    source_mission_id: String,
    debloat_id: String,
    package_id: String,
    main: ExactPackageIdentity,
    dependencies: Vec<ExactPackageDependency>,
    restore: HistoryRestoreRoute,
    source_checkpoint: TransactionCheckpoint,
    receipt_fingerprint: String,
}

#[derive(Debug, Deserialize)]
struct DebloatRemovalReceiptWire {
    schema_version: u32,
    receipt_id: String,
    source_transaction_id: String,
    source_transaction_fingerprint: String,
    source_mission_id: String,
    debloat_id: String,
    package_id: String,
    main: ExactPackageIdentity,
    dependencies: Vec<ExactPackageDependency>,
    restore: HistoryRestoreRoute,
    source_checkpoint: TransactionCheckpoint,
    receipt_fingerprint: String,
}

#[derive(Serialize)]
struct ReceiptFingerprintMaterial<'a> {
    schema_version: u32,
    receipt_id: &'a str,
    source_transaction_id: &'a str,
    source_transaction_fingerprint: &'a str,
    source_mission_id: &'a str,
    debloat_id: &'a str,
    package_id: &'a str,
    main: &'a ExactPackageIdentity,
    dependencies: &'a [ExactPackageDependency],
    restore: &'a HistoryRestoreRoute,
    source_checkpoint: &'a TransactionCheckpoint,
}

impl TryFrom<DebloatRemovalReceiptWire> for DebloatRemovalReceipt {
    type Error = DebloatHistoryError;

    fn try_from(value: DebloatRemovalReceiptWire) -> Result<Self, Self::Error> {
        let receipt = Self {
            schema_version: value.schema_version,
            receipt_id: value.receipt_id,
            source_transaction_id: value.source_transaction_id,
            source_transaction_fingerprint: value.source_transaction_fingerprint,
            source_mission_id: value.source_mission_id,
            debloat_id: value.debloat_id,
            package_id: value.package_id,
            main: value.main,
            dependencies: value.dependencies,
            restore: value.restore,
            source_checkpoint: value.source_checkpoint,
            receipt_fingerprint: value.receipt_fingerprint,
        };
        receipt.validate()?;
        Ok(receipt)
    }
}

impl<'de> Deserialize<'de> for DebloatRemovalReceipt {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = DebloatRemovalReceiptWire::deserialize(deserializer)?;
        Self::try_from(wire).map_err(serde::de::Error::custom)
    }
}

impl DebloatRemovalReceipt {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        source_transaction_id: String,
        source_transaction_fingerprint: String,
        source_mission_id: String,
        debloat_id: String,
        package_id: String,
        main: ExactPackageIdentity,
        dependencies: Vec<ExactPackageDependency>,
        restore: HistoryRestoreRoute,
        source_checkpoint: TransactionCheckpoint,
    ) -> Result<Self, DebloatHistoryError> {
        let receipt_id = format!("{source_transaction_id}:phase17-removal-receipt");
        let mut receipt = Self {
            schema_version: DEBLOAT_REMOVAL_RECEIPT_SCHEMA_VERSION,
            receipt_id,
            source_transaction_id,
            source_transaction_fingerprint,
            source_mission_id,
            debloat_id,
            package_id,
            main,
            dependencies,
            restore,
            source_checkpoint,
            receipt_fingerprint: String::new(),
        };
        receipt.receipt_fingerprint = receipt.compute_fingerprint()?;
        receipt.validate()?;
        Ok(receipt)
    }

    pub fn from_json_str(input: &str) -> Result<Self, DebloatHistoryError> {
        Ok(serde_json::from_str(input)?)
    }

    pub fn to_json_pretty(&self) -> Result<String, DebloatHistoryError> {
        self.validate()?;
        Ok(serde_json::to_string_pretty(self)?)
    }

    pub fn validate(&self) -> Result<(), DebloatHistoryError> {
        if self.schema_version != DEBLOAT_REMOVAL_RECEIPT_SCHEMA_VERSION {
            return Err(DebloatHistoryError::InvalidReceipt(format!(
                "unsupported schema version {}",
                self.schema_version
            )));
        }
        for (label, value) in [
            ("receipt id", self.receipt_id.as_str()),
            ("source transaction id", self.source_transaction_id.as_str()),
            (
                "source transaction fingerprint",
                self.source_transaction_fingerprint.as_str(),
            ),
            ("source mission id", self.source_mission_id.as_str()),
            ("debloat id", self.debloat_id.as_str()),
            ("package id", self.package_id.as_str()),
            ("receipt fingerprint", self.receipt_fingerprint.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(DebloatHistoryError::InvalidReceipt(format!(
                    "{label} must not be empty"
                )));
            }
        }
        self.main.validate()?;
        for dependency in &self.dependencies {
            dependency.validate()?;
        }
        if self.source_checkpoint.stage() != TransactionStage::Complete {
            return Err(DebloatHistoryError::InvalidReceipt(
                "source checkpoint is not Complete".to_string(),
            ));
        }
        let source_plan = self.source_checkpoint.plan();
        if source_plan.transaction_id() != self.source_transaction_id
            || source_plan.mission_id() != self.source_mission_id
            || self.source_checkpoint.plan_fingerprint() != self.source_transaction_fingerprint
            || source_plan.fingerprint()? != self.source_transaction_fingerprint
        {
            return Err(DebloatHistoryError::InvalidReceipt(
                "source transaction/checkpoint identity continuity failed".to_string(),
            ));
        }
        if source_plan.actions().len() != 1
            || source_plan.actions()[0].action.id != self.debloat_id
            || source_plan.actions()[0].action.kind != neo_core::ActionKind::Debloat
        {
            return Err(DebloatHistoryError::InvalidReceipt(
                "source checkpoint does not contain exactly the completed Debloat action"
                    .to_string(),
            ));
        }
        if self.receipt_id
            != format!("{}:phase17-removal-receipt", self.source_transaction_id)
        {
            return Err(DebloatHistoryError::InvalidReceipt(
                "receipt id does not bind to source transaction id".to_string(),
            ));
        }
        if !self
            .restore
            .package_full_name
            .eq_ignore_ascii_case(&self.main.full_name)
            || !self
                .restore
                .package_family_name
                .eq_ignore_ascii_case(&self.main.family_name)
            || self.restore.dependency_full_names.len() != self.dependencies.len()
            || !self
                .restore
                .dependency_full_names
                .iter()
                .zip(&self.dependencies)
                .all(|(left, right)| left.eq_ignore_ascii_case(&right.full_name))
        {
            return Err(DebloatHistoryError::InvalidReceipt(
                "restore route does not match captured main/dependency identities".to_string(),
            ));
        }
        if self.main.dependencies != self.dependencies {
            return Err(DebloatHistoryError::InvalidReceipt(
                "captured main dependency list differs from receipt dependencies".to_string(),
            ));
        }
        validate_checkpoint_baseline(self)?;
        let expected = self.compute_fingerprint()?;
        if expected != self.receipt_fingerprint {
            return Err(DebloatHistoryError::InvalidReceipt(
                "receipt fingerprint mismatch".to_string(),
            ));
        }
        Ok(())
    }

    fn compute_fingerprint(&self) -> Result<String, DebloatHistoryError> {
        let material = ReceiptFingerprintMaterial {
            schema_version: self.schema_version,
            receipt_id: &self.receipt_id,
            source_transaction_id: &self.source_transaction_id,
            source_transaction_fingerprint: &self.source_transaction_fingerprint,
            source_mission_id: &self.source_mission_id,
            debloat_id: &self.debloat_id,
            package_id: &self.package_id,
            main: &self.main,
            dependencies: &self.dependencies,
            restore: &self.restore,
            source_checkpoint: &self.source_checkpoint,
        };
        let bytes = serde_json::to_vec(&material)?;
        let digest = Sha256::digest(bytes);
        let mut encoded = String::with_capacity(digest.len() * 2);
        for byte in digest {
            write!(&mut encoded, "{byte:02x}").expect("writing to String cannot fail");
        }
        Ok(encoded)
    }

    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }

    pub fn receipt_id(&self) -> &str {
        &self.receipt_id
    }

    pub fn source_transaction_id(&self) -> &str {
        &self.source_transaction_id
    }

    pub fn source_transaction_fingerprint(&self) -> &str {
        &self.source_transaction_fingerprint
    }

    pub fn source_mission_id(&self) -> &str {
        &self.source_mission_id
    }

    pub fn debloat_id(&self) -> &str {
        &self.debloat_id
    }

    pub fn package_id(&self) -> &str {
        &self.package_id
    }

    pub fn main(&self) -> &ExactPackageIdentity {
        &self.main
    }

    pub fn dependencies(&self) -> &[ExactPackageDependency] {
        &self.dependencies
    }

    pub fn restore(&self) -> &HistoryRestoreRoute {
        &self.restore
    }

    pub fn source_checkpoint(&self) -> &TransactionCheckpoint {
        &self.source_checkpoint
    }

    pub fn receipt_fingerprint(&self) -> &str {
        &self.receipt_fingerprint
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DebloatRestorePreparedStep {
    debloat_id: String,
    package_id: String,
    main: ExactPackageIdentity,
    dependencies: Vec<ExactPackageDependency>,
    restore: HistoryRestoreRoute,
}

impl DebloatRestorePreparedStep {
    pub(crate) fn new(receipt: &DebloatRemovalReceipt) -> Self {
        Self {
            debloat_id: receipt.debloat_id.clone(),
            package_id: receipt.package_id.clone(),
            main: receipt.main.clone(),
            dependencies: receipt.dependencies.clone(),
            restore: receipt.restore.clone(),
        }
    }

    pub fn debloat_id(&self) -> &str {
        &self.debloat_id
    }

    pub fn package_id(&self) -> &str {
        &self.package_id
    }

    pub fn main(&self) -> &ExactPackageIdentity {
        &self.main
    }

    pub fn dependencies(&self) -> &[ExactPackageDependency] {
        &self.dependencies
    }

    pub fn restore(&self) -> &HistoryRestoreRoute {
        &self.restore
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DebloatRestorePreparedTransaction {
    receipt_fingerprint: String,
    step: DebloatRestorePreparedStep,
    transaction: TransactionPlan,
    checkpoint: TransactionCheckpoint,
    machine_changes: bool,
}

impl DebloatRestorePreparedTransaction {
    pub(crate) fn new(
        receipt: &DebloatRemovalReceipt,
        transaction: TransactionPlan,
        checkpoint: TransactionCheckpoint,
    ) -> Self {
        Self {
            receipt_fingerprint: receipt.receipt_fingerprint.clone(),
            step: DebloatRestorePreparedStep::new(receipt),
            transaction,
            checkpoint,
            machine_changes: false,
        }
    }

    pub fn receipt_fingerprint(&self) -> &str {
        &self.receipt_fingerprint
    }

    pub fn step(&self) -> &DebloatRestorePreparedStep {
        &self.step
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
}

fn validate_checkpoint_baseline(receipt: &DebloatRemovalReceipt) -> Result<(), DebloatHistoryError> {
    let baseline = receipt
        .source_checkpoint
        .baseline()
        .ok_or_else(|| DebloatHistoryError::InvalidReceipt("source baseline is missing".to_string()))?;
    let main_target = appx_target(&receipt.main.full_name);
    let main_value = baseline.get(&main_target).ok_or_else(|| {
        DebloatHistoryError::InvalidReceipt("source main baseline target is missing".to_string())
    })?;
    let neo_transaction::CapturedValue::Present(main_json) = main_value else {
        return Err(DebloatHistoryError::InvalidReceipt(
            "source main baseline was not Present".to_string(),
        ));
    };
    let main: ExactPackageIdentity = serde_json::from_str(main_json)?;
    if main != receipt.main {
        return Err(DebloatHistoryError::InvalidReceipt(
            "source main baseline differs from receipt identity".to_string(),
        ));
    }
    for dependency in &receipt.dependencies {
        let target = appx_target(&dependency.full_name);
        let value = baseline.get(&target).ok_or_else(|| {
            DebloatHistoryError::InvalidReceipt(format!(
                "source dependency baseline target {} is missing",
                dependency.full_name
            ))
        })?;
        let neo_transaction::CapturedValue::Present(json) = value else {
            return Err(DebloatHistoryError::InvalidReceipt(format!(
                "source dependency baseline {} was not Present",
                dependency.full_name
            )));
        };
        let parsed: ExactPackageDependency = serde_json::from_str(json)?;
        if parsed != *dependency {
            return Err(DebloatHistoryError::InvalidReceipt(format!(
                "source dependency baseline {} differs from receipt identity",
                dependency.full_name
            )));
        }
    }
    Ok(())
}

pub(crate) fn appx_target(full_name: &str) -> neo_transaction::StateTarget {
    neo_transaction::StateTarget {
        kind: neo_transaction::StateTargetKind::AppxPackage,
        key: format!("current_user:{full_name}"),
    }
}
