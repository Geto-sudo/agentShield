use std::sync::OnceLock;

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine as _;
use chrono::Utc;
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use rusqlite::{params, Connection};
use sha2::{Digest, Sha256};

use crate::domain::audit::{AuditEntry, AuditSigningInfo, ChainStatus};
use crate::store::{json_to_text, text_to_json, ts_from_text};

pub const GENESIS: &str = "GENESIS";

/// Variable d'environnement qui porte la graine de signature (base64, 32 octets).
pub const SIGNING_KEY_ENV: &str = "AGENTSHIELD_AUDIT_SIGNING_KEY";

/// Algorithme de signature des entrées d'audit.
pub const SIGNING_ALGORITHM: &str = "ed25519";

/// Taille d'une graine Ed25519 (le secret ; la clé publique en dérive).
const SEED_LEN: usize = 32;

/// `hash = sha256(prev_hash | seq | ts | kind | subject | payload)`
pub fn compute_hash(
    prev_hash: &str,
    seq: i64,
    ts: &str,
    kind: &str,
    subject: &str,
    payload: &str,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(prev_hash.as_bytes());
    hasher.update(b"|");
    hasher.update(seq.to_string().as_bytes());
    hasher.update(b"|");
    hasher.update(ts.as_bytes());
    hasher.update(b"|");
    hasher.update(kind.as_bytes());
    hasher.update(b"|");
    hasher.update(subject.as_bytes());
    hasher.update(b"|");
    hasher.update(payload.as_bytes());
    hex::encode(hasher.finalize())
}

// ---------------------------------------------------------------------------
// Clé de signature
// ---------------------------------------------------------------------------

/// La clé Ed25519 du processus : elle signe chaque entrée et vérifie toute la
/// chaîne. La graine privée ne quitte jamais la mémoire et n'est jamais loggée.
struct AuditSigner {
    signing: SigningKey,
    verifying: VerifyingKey,
    pubkey_b64: String,
    /// `env` si `AGENTSHIELD_AUDIT_SIGNING_KEY` était fournie, `ephemeral` sinon.
    source: &'static str,
}

impl AuditSigner {
    fn from_seed(seed: &[u8], source: &'static str) -> Result<Self, String> {
        let bytes: [u8; SEED_LEN] = seed.try_into().map_err(|_| {
            format!("{SIGNING_KEY_ENV} doit contenir une graine Ed25519 de {SEED_LEN} octets")
        })?;
        let signing = SigningKey::from_bytes(&bytes);
        let verifying = signing.verifying_key();
        let pubkey_b64 = BASE64.encode(verifying.to_bytes());
        Ok(Self {
            signing,
            verifying,
            pubkey_b64,
            source,
        })
    }

    fn from_env_base64(raw: &str) -> Result<Self, String> {
        let decoded = BASE64.decode(raw.trim()).map_err(|_| {
            format!("{SIGNING_KEY_ENV} n'est pas du base64 standard valide (graine de 32 octets attendue)")
        })?;
        Self::from_seed(&decoded, "env")
    }

    fn ephemeral() -> Self {
        let mut seed = [0u8; SEED_LEN];
        if let Err(e) = getrandom::fill(&mut seed) {
            // Sans entropie système, signer serait un mensonge : on refuse de démarrer.
            panic!("entropie système indisponible pour la clé d'audit éphémère : {e}");
        }
        Self::from_seed(&seed, "ephemeral")
            .expect("une graine de 32 octets est toujours valide")
    }

    fn info(&self) -> AuditSigningInfo {
        AuditSigningInfo {
            algorithm: SIGNING_ALGORITHM.to_string(),
            public_key: self.pubkey_b64.clone(),
            key_source: self.source.to_string(),
        }
    }
}

static AUDIT_SIGNER: OnceLock<AuditSigner> = OnceLock::new();

/// Clé d'audit du processus. Initialisée au démarrage par `init_signing_key`,
/// ou à la première utilisation (tests, usages directs de `store`).
fn signer() -> &'static AuditSigner {
    AUDIT_SIGNER.get_or_init(|| {
        let signer = AuditSigner::ephemeral();
        tracing::warn!(
            public_key = %signer.pubkey_b64,
            "Aucune clé d'audit configurée : clé Ed25519 éphémère générée. Elle signe les \
             nouvelles entrées et ne vérifiera PAS les entrées écrites par un processus \
             précédent. Définir {SIGNING_KEY_ENV} pour une clé stable."
        );
        signer
    })
}

/// Installe la clé d'audit du processus.
///
/// * `configured` = contenu de [`SIGNING_KEY_ENV`] s'il est présent ;
/// * sinon une clé éphémère est générée et sa clé **publique** est journalisée
///   (même esprit que le mode `mock` de PayPal : tout reste utilisable sans
///   configuration). La graine privée n'est jamais loggée.
pub fn init_signing_key(configured: Option<&str>) -> Result<AuditSigningInfo, String> {
    match configured {
        Some(raw) => {
            let candidate = AuditSigner::from_env_base64(raw)?;
            let candidate_pubkey = candidate.pubkey_b64.clone();
            if AUDIT_SIGNER.set(candidate).is_err() && signer().pubkey_b64 != candidate_pubkey {
                return Err(
                    "une autre clé d'audit est déjà installée dans ce processus".to_string(),
                );
            }
            let info = signer().info();
            tracing::info!(
                public_key = %info.public_key,
                "Audit : signature {SIGNING_ALGORITHM} (clé fournie par {SIGNING_KEY_ENV})."
            );
            Ok(info)
        }
        None => Ok(signer().info()),
    }
}

/// Clé publique et origine de la clé, pour `/api/health` et `/api/audit/pubkey`.
pub fn signing_info() -> AuditSigningInfo {
    signer().info()
}

fn sign_hash(hash: &str) -> String {
    let signature: Signature = signer().signing.sign(hash.as_bytes());
    BASE64.encode(signature.to_bytes())
}

fn check_signature(
    expected: &AuditSigner,
    hash: &str,
    pubkey: Option<&str>,
    signature: Option<&str>,
) -> Result<(), String> {
    let (Some(pubkey), Some(signature)) = (pubkey, signature) else {
        return Err(
            "entrée non signée : la preuve n'est pas attestée par la clé d'audit.".to_string(),
        );
    };
    if pubkey != expected.pubkey_b64 {
        return Err(
            "signature produite par une autre clé que la clé d'audit attendue.".to_string(),
        );
    }
    let raw = BASE64
        .decode(signature)
        .map_err(|_| "signature illisible (base64 invalide).".to_string())?;
    let signature = Signature::from_slice(&raw)
        .map_err(|_| "signature de taille invalide.".to_string())?;
    expected
        .verifying
        .verify_strict(hash.as_bytes(), &signature)
        .map_err(|_| {
            "signature invalide : l'entrée a été modifiée ou signée par une autre clé."
                .to_string()
        })
}

// ---------------------------------------------------------------------------
// Écriture et lecture
// ---------------------------------------------------------------------------

/// Ajoute une entrée en fin de chaîne, signée par la clé du processus.
/// À appeler dans la même transaction que l'écriture métier, pour que la preuve
/// ne puisse pas exister sans l'effet.
///
/// Renvoie `(seq, hash, prev_hash)`.
pub fn append(
    conn: &Connection,
    kind: &str,
    subject: &str,
    payload: &serde_json::Value,
) -> rusqlite::Result<(i64, String, String)> {
    let head = conn.query_row(
        "SELECT seq, hash FROM audit_log ORDER BY seq DESC LIMIT 1",
        [],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
    );

    let (seq, prev_hash) = match head {
        Ok((last_seq, last_hash)) => (last_seq + 1, last_hash),
        Err(rusqlite::Error::QueryReturnedNoRows) => (1, GENESIS.to_string()),
        Err(e) => return Err(e),
    };

    let ts = Utc::now().to_rfc3339();
    let payload_text = json_to_text(payload);
    let hash = compute_hash(&prev_hash, seq, &ts, kind, subject, &payload_text);
    let signature = sign_hash(&hash);
    let pubkey = signer().pubkey_b64.clone();

    conn.execute(
        "INSERT INTO audit_log
            (seq, ts, kind, subject, payload, prev_hash, hash, signature, pubkey)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            seq,
            ts,
            kind,
            subject,
            payload_text,
            prev_hash,
            hash,
            signature,
            pubkey
        ],
    )?;

    Ok((seq, hash, prev_hash))
}

pub fn list(conn: &Connection, limit: i64) -> rusqlite::Result<Vec<AuditEntry>> {
    let mut stmt = conn.prepare(
        "SELECT id, seq, ts, kind, subject, payload, prev_hash, hash, pubkey, signature
         FROM audit_log ORDER BY seq DESC LIMIT ?1",
    )?;
    let rows = stmt.query_map(params![limit], |row| {
        Ok(AuditEntry {
            id: row.get(0)?,
            seq: row.get(1)?,
            ts: ts_from_text(&row.get::<_, String>(2)?),
            kind: row.get(3)?,
            subject: row.get(4)?,
            payload: text_to_json(&row.get::<_, String>(5)?),
            prev_hash: row.get(6)?,
            hash: row.get(7)?,
            pubkey: row.get(8)?,
            signature: row.get(9)?,
        })
    })?;
    rows.collect()
}

pub fn head_hash(conn: &Connection) -> rusqlite::Result<String> {
    match conn.query_row(
        "SELECT hash FROM audit_log ORDER BY seq DESC LIMIT 1",
        [],
        |row| row.get::<_, String>(0),
    ) {
        Ok(h) => Ok(h),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(GENESIS.to_string()),
        Err(e) => Err(e),
    }
}

/// Recalcule toute la chaîne et vérifie la signature Ed25519 de chaque entrée.
/// Renvoie la première rupture éventuelle (hash ou signature).
// TODO(ancrage externe) : la signature rend la réécriture impossible sans la clé
// privée, mais `verify` reste ancré dans cette base. Supprimer les dernières
// entrées (troncature) ou restaurer une base antérieure complète reste
// indétectable sans publication du `head_hash` hors de la base (journal
// append-only, horodatage tiers, transparency log). Volontairement hors
// périmètre du prototype.
pub fn verify(conn: &Connection) -> rusqlite::Result<ChainStatus> {
    let expected = signer();
    let mut stmt = conn.prepare(
        "SELECT seq, ts, kind, subject, payload, prev_hash, hash, pubkey, signature
         FROM audit_log ORDER BY seq ASC",
    )?;
    let mut rows = stmt.query([])?;

    let mut expected_prev = GENESIS.to_string();
    let mut expected_seq = 1_i64;
    let mut count = 0_i64;
    let mut last_hash = GENESIS.to_string();

    while let Some(row) = rows.next()? {
        let seq: i64 = row.get(0)?;
        let ts: String = row.get(1)?;
        let kind: String = row.get(2)?;
        let subject: String = row.get(3)?;
        let payload: String = row.get(4)?;
        let prev_hash: String = row.get(5)?;
        let hash: String = row.get(6)?;
        let pubkey: Option<String> = row.get(7)?;
        let signature: Option<String> = row.get(8)?;

        let recomputed = compute_hash(&prev_hash, seq, &ts, &kind, &subject, &payload);

        if prev_hash != expected_prev || seq != expected_seq || recomputed != hash {
            return Ok(broken(
                seq,
                count,
                last_hash,
                "la preuve ne correspond plus à son contenu.",
            ));
        }

        if let Err(reason) =
            check_signature(expected, &hash, pubkey.as_deref(), signature.as_deref())
        {
            return Ok(broken(seq, count, last_hash, reason));
        }

        expected_prev = hash.clone();
        expected_seq += 1;
        count += 1;
        last_hash = hash;
    }

    Ok(ChainStatus {
        valid: true,
        entries: count,
        head_hash: last_hash,
        checked_at: Utc::now(),
        broken_at_seq: None,
        message: format!("{count} entrées vérifiées, chaîne intacte et signatures valides."),
    })
}

fn broken(
    seq: i64,
    entries: i64,
    head_hash: String,
    reason: impl Into<String>,
) -> ChainStatus {
    ChainStatus {
        valid: false,
        entries,
        head_hash,
        checked_at: Utc::now(),
        broken_at_seq: Some(seq),
        message: format!("Chaîne rompue à la séquence {seq} : {}", reason.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Db;
    use serde_json::json;

    #[test]
    fn chain_is_valid_after_appends() {
        let db = Db::open_memory().unwrap();
        db.with_tx(|tx| {
            append(tx, "mandate.created", "m1", &json!({"a": 1}))?;
            append(tx, "action.proposed", "a1", &json!({"b": 2}))?;
            append(tx, "decision.refused", "a1", &json!({"c": 3}))?;
            Ok(())
        })
        .unwrap();

        let status = db.with_conn(verify).unwrap();
        assert!(status.valid, "{:?}", status);
        assert_eq!(status.entries, 3);
        assert_eq!(status.broken_at_seq, None);
    }

    #[test]
    fn tampering_breaks_the_chain() {
        let db = Db::open_memory().unwrap();
        db.with_tx(|tx| {
            append(tx, "mandate.created", "m1", &json!({"a": 1}))?;
            append(tx, "action.proposed", "a1", &json!({"b": 2}))?;
            Ok(())
        })
        .unwrap();

        db.with_conn(|c| {
            c.execute(
                "UPDATE audit_log SET payload = '{\"b\":99}' WHERE seq = 2",
                [],
            )
            .map(|_| ())
        })
        .unwrap();

        let status = db.with_conn(verify).unwrap();
        assert!(!status.valid);
        assert_eq!(status.broken_at_seq, Some(2));
    }

    #[test]
    fn every_entry_is_signed_with_the_process_key() {
        let db = Db::open_memory().unwrap();
        db.with_tx(|tx| {
            append(tx, "mandate.created", "m1", &json!({"a": 1}))?;
            Ok(())
        })
        .unwrap();

        let entries = db.with_conn(|c| list(c, 10)).unwrap();
        let entry = &entries[0];
        assert_eq!(entry.pubkey.as_deref(), Some(signer().pubkey_b64.as_str()));
        assert!(entry.signature.is_some(), "signature attendue");

        // La signature stockée est bien celle du hash stocké.
        check_signature(
            signer(),
            &entry.hash,
            entry.pubkey.as_deref(),
            entry.signature.as_deref(),
        )
        .expect("signature valide");
    }

    /// Le cas visé par la signature : un attaquant qui a un accès écriture à
    /// SQLite remplace une signature par celle d'une autre clé.
    #[test]
    fn signature_falsified_is_detected() {
        let db = Db::open_memory().unwrap();
        db.with_tx(|tx| {
            append(tx, "mandate.created", "m1", &json!({"a": 1}))?;
            append(tx, "action.proposed", "a1", &json!({"b": 2}))?;
            Ok(())
        })
        .unwrap();

        let forged = {
            let key = SigningKey::from_bytes(&[7u8; SEED_LEN]);
            let hash = db
                .with_conn(|c| {
                    c.query_row("SELECT hash FROM audit_log WHERE seq = 2", [], |r| {
                        r.get::<_, String>(0)
                    })
                })
                .unwrap();
            BASE64.encode(key.sign(hash.as_bytes()).to_bytes())
        };

        db.with_conn(|c| {
            c.execute(
                "UPDATE audit_log SET signature = ?1 WHERE seq = 2",
                params![forged],
            )
            .map(|_| ())
        })
        .unwrap();

        let status = db.with_conn(verify).unwrap();
        assert!(!status.valid, "{:?}", status);
        assert_eq!(status.broken_at_seq, Some(2));
        assert!(
            status.message.contains("signature"),
            "message attendu sur la signature : {}",
            status.message
        );
    }

    /// Attaque complète : contenu réécrit ET chaîne re-hachée de bout en bout,
    /// signatures laissées intactes. Sans la clé privée, `verify` doit refuser.
    #[test]
    fn rehashed_chain_still_fails_without_the_private_key() {
        let db = Db::open_memory().unwrap();
        db.with_tx(|tx| {
            append(tx, "mandate.created", "m1", &json!({"a": 1}))?;
            append(tx, "action.proposed", "a1", &json!({"b": 2}))?;
            append(tx, "action.refused", "a1", &json!({"c": 3}))?;
            Ok(())
        })
        .unwrap();

        db.with_tx(|tx| {
            let (prev2, ts2, kind2, subject2): (String, String, String, String) = tx
                .query_row(
                    "SELECT prev_hash, ts, kind, subject FROM audit_log WHERE seq = 2",
                    [],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
                )
                .unwrap();
            let payload2 = json!({ "b": 99 }).to_string();
            let hash2 = compute_hash(&prev2, 2, &ts2, &kind2, &subject2, &payload2);
            tx.execute(
                "UPDATE audit_log SET payload = ?1, hash = ?2 WHERE seq = 2",
                params![payload2, hash2],
            )?;

            let (ts3, kind3, subject3, payload3): (String, String, String, String) = tx
                .query_row(
                    "SELECT ts, kind, subject, payload FROM audit_log WHERE seq = 3",
                    [],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
                )
                .unwrap();
            let hash3 = compute_hash(&hash2, 3, &ts3, &kind3, &subject3, &payload3);
            tx.execute(
                "UPDATE audit_log SET prev_hash = ?1, hash = ?2 WHERE seq = 3",
                params![hash2, hash3],
            )?;
            Ok(())
        })
        .unwrap();

        let status = db.with_conn(verify).unwrap();
        assert!(!status.valid, "{:?}", status);
        assert_eq!(
            status.broken_at_seq,
            Some(2),
            "la chaîne re-hachée est cohérente, seule la signature la trahit : {}",
            status.message
        );
    }

    /// Une entrée écrite avant la signature (ou dont la signature a été
    /// supprimée) ne doit jamais passer pour valide.
    #[test]
    fn unsigned_entry_is_reported_as_broken() {
        let db = Db::open_memory().unwrap();
        db.with_tx(|tx| {
            append(tx, "mandate.created", "m1", &json!({"a": 1}))?;
            append(tx, "action.proposed", "a1", &json!({"b": 2}))?;
            Ok(())
        })
        .unwrap();

        db.with_conn(|c| {
            c.execute("UPDATE audit_log SET signature = NULL WHERE seq = 1", [])
                .map(|_| ())
        })
        .unwrap();

        let status = db.with_conn(verify).unwrap();
        assert!(!status.valid, "{:?}", status);
        assert_eq!(status.broken_at_seq, Some(1));
        assert!(
            status.message.contains("non signée"),
            "message attendu sur l'absence de signature : {}",
            status.message
        );
    }

    #[test]
    fn signing_key_is_loaded_from_base64_seed() {
        let seed = BASE64.encode([3u8; SEED_LEN]);
        let first = AuditSigner::from_env_base64(&seed).unwrap();
        let second = AuditSigner::from_env_base64(&format!(" {} ", seed)).unwrap();

        assert_eq!(first.source, "env");
        assert_eq!(first.pubkey_b64, second.pubkey_b64, "même graine, même clé");

        // Graine de mauvaise taille et base64 invalide : refus explicite.
        assert!(AuditSigner::from_env_base64(&BASE64.encode([1u8; 8])).is_err());
        assert!(AuditSigner::from_env_base64("pas du base64 !").is_err());
    }

    /// La clé publique exposée par l'API est bien celle qui a signé les entrées.
    #[test]
    fn signing_info_matches_the_signer() {
        let info = signing_info();
        assert_eq!(info.algorithm, SIGNING_ALGORITHM);
        assert_eq!(info.public_key, signer().pubkey_b64);
        assert!(matches!(info.key_source.as_str(), "env" | "ephemeral"));
    }
}
