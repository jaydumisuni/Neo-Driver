//! Phase 19 trusted persistent Debloat history-store boundary.
//!
//! This crate persists only Phase 17 receipts derived from completed Phase 16 execution into the
//! Builder/portable-rooted NeoData tree. Selection is by a typed store record id; no caller path or
//! arbitrary receipt JSON becomes trusted history authority. Restore preparation still performs the
//! Phase 17 fresh inventory checks, and this crate does not construct or issue the opaque Phase 18
//! restore-executor capability.

mod error;
mod model;
mod store;

pub use error::DebloatHistoryStoreError;
pub use model::{
    DebloatHistoryRecordId, HistoryRecordDisposition, HistoryRecordWriteReceipt,
    StoredDebloatRemovalReceipt, StoredDebloatRemovalSummary, DEBLOAT_HISTORY_STORE_SCHEMA_VERSION,
    MAX_HISTORY_RECORD_BYTES,
};
pub use store::DebloatHistoryStore;

#[cfg(test)]
mod tests;
