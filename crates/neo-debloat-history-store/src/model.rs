use crate::DebloatHistoryStoreError;
use neo_debloat_history::DebloatRemovalReceipt;
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;
use std::path::PathBuf;

pub const DEBLOAT_HISTORY_STORE_SCHEMA_VERSION: u32 = 1;
pub const MAX_HISTORY_RECORD_BYTES: u64 = 4 * 1024 * 1024;
pub(crate) const RECORD_FILE_NAME: &str = "receipt.json";
pub(crate) const STAGING_DIRECTORY_NAME: &str = ".staging";
pub(crate) const STAGING_MARKER_NAME: &str = ".neo-history-staging.json";

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct DebloatHistoryRecordId(String);

impl DebloatHistoryRecordId {
    pub fn new(value: impl Into<String>) -> Result<Self, DebloatHistoryStoreError> {
        let value = value.into();
        if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(DebloatHistoryStoreError::InvalidRecordId(value));
        }
        Ok(Self(value.to_ascii_lowercase()))
    }

    pub fn from_receipt(receipt: &DebloatRemovalReceipt) -> Result<Self, DebloatHistoryStoreError> {
        receipt.validate()?;
        Self::new(receipt.receipt_fingerprint())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DebloatHistoryRecordId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for DebloatHistoryRecordId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HistoryRecordDisposition {
    Recorded,
    AlreadyPresent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryRecordWriteReceipt {
    pub record_id: DebloatHistoryRecordId,
    pub disposition: HistoryRecordDisposition,
    pub path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredDebloatRemovalSummary {
    pub record_id: DebloatHistoryRecordId,
    pub receipt_id: String,
    pub source_transaction_id: String,
    pub source_mission_id: String,
    pub debloat_id: String,
    pub package_id: String,
    pub package_full_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredDebloatRemovalReceipt {
    record_id: DebloatHistoryRecordId,
    receipt: DebloatRemovalReceipt,
}

impl StoredDebloatRemovalReceipt {
    pub(crate) fn new(
        record_id: DebloatHistoryRecordId,
        receipt: DebloatRemovalReceipt,
    ) -> Self {
        Self { record_id, receipt }
    }

    pub fn record_id(&self) -> &DebloatHistoryRecordId {
        &self.record_id
    }

    pub fn receipt(&self) -> &DebloatRemovalReceipt {
        &self.receipt
    }

    pub fn summary(&self) -> StoredDebloatRemovalSummary {
        StoredDebloatRemovalSummary {
            record_id: self.record_id.clone(),
            receipt_id: self.receipt.receipt_id().to_string(),
            source_transaction_id: self.receipt.source_transaction_id().to_string(),
            source_mission_id: self.receipt.source_mission_id().to_string(),
            debloat_id: self.receipt.debloat_id().to_string(),
            package_id: self.receipt.package_id().to_string(),
            package_full_name: self.receipt.main().full_name.clone(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct StoredReceiptEnvelope {
    pub schema_version: u32,
    pub record_id: DebloatHistoryRecordId,
    pub receipt: DebloatRemovalReceipt,
}

impl StoredReceiptEnvelope {
    pub fn new(receipt: DebloatRemovalReceipt) -> Result<Self, DebloatHistoryStoreError> {
        let record_id = DebloatHistoryRecordId::from_receipt(&receipt)?;
        Ok(Self {
            schema_version: DEBLOAT_HISTORY_STORE_SCHEMA_VERSION,
            record_id,
            receipt,
        })
    }

    pub fn validate(
        self,
        expected_record_id: &DebloatHistoryRecordId,
    ) -> Result<StoredDebloatRemovalReceipt, DebloatHistoryStoreError> {
        if self.schema_version != DEBLOAT_HISTORY_STORE_SCHEMA_VERSION {
            return Err(DebloatHistoryStoreError::InvalidRecord(format!(
                "unsupported store schema version {}",
                self.schema_version
            )));
        }
        self.receipt.validate()?;
        let receipt_record_id = DebloatHistoryRecordId::from_receipt(&self.receipt)?;
        if &self.record_id != expected_record_id || receipt_record_id != self.record_id {
            return Err(DebloatHistoryStoreError::InvalidRecord(
                "directory, envelope, and receipt record identities differ".to_string(),
            ));
        }
        Ok(StoredDebloatRemovalReceipt::new(
            self.record_id,
            self.receipt,
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct StagingMarker {
    pub schema_version: u32,
    pub staging_name: String,
    pub record_id: DebloatHistoryRecordId,
}
