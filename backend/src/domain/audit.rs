use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Une entrée du journal d'audit, chaînée par hash.
///
/// `hash = sha256(prev_hash || seq || ts || kind || subject || payload)`
/// Toute modification d'une entrée casse la chaîne et `verify` le détecte.
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
    /// `seq` de la première entrée invalide, si la chaîne est rompue.
    pub broken_at_seq: Option<i64>,
    pub message: String,
}
