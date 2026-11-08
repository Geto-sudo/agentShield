use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// An audit log entry linked to the previous entry by a hash.
///
/// `hash = sha256(prev_hash || seq || ts || kind || subject || payload)`
/// Any modification to an entry breaks the chain, which `verify` detects.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: i64,
    pub seq: i64,
    pub ts: DateTime<Utc>,
    pub kind: String,
    pub subject: String,
    pub payload: serde_json::Value,
    pub prev_hash: String,
    pub hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainStatus {
    pub valid: bool,
    pub entries: i64,
    pub head_hash: String,
    pub checked_at: DateTime<Utc>,
    /// Sequence number of the first invalid entry, if the chain is broken.
    pub broken_at_seq: Option<i64>,
    pub message: String,
}
