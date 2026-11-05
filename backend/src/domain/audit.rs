use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Une entrée du journal d'audit, chaînée par hash et signée.
///
/// `hash = sha256(prev_hash || seq || ts || kind || subject || payload)`
/// `signature = Ed25519(hash)` avec la clé d'audit du processus.
/// Toute modification d'une entrée casse la chaîne, et une réécriture complète
/// ne passe pas `verify` sans la clé privée.
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
    /// Clé publique Ed25519 (base64) qui a signé cette entrée ; `None` pour une
    /// entrée écrite avant l'introduction de la signature.
    #[serde(default)]
    pub pubkey: Option<String>,
    /// Signature Ed25519 du `hash` (base64, 64 octets).
    #[serde(default)]
    pub signature: Option<String>,
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

/// Identité de la clé qui signe les entrées d'audit de ce processus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditSigningInfo {
    pub algorithm: String,
    /// Clé publique Ed25519 en base64 : vérifiable hors ligne, sans secret.
    pub public_key: String,
    /// `env` (clé fournie par l'environnement) ou `ephemeral` (générée au
    /// démarrage, propre à ce processus).
    pub key_source: String,
}
