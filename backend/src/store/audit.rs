use chrono::Utc;
use rusqlite::{params, Connection};
use sha2::{Digest, Sha256};

use crate::domain::audit::{AuditEntry, ChainStatus};
use crate::store::{json_to_text, text_to_json, ts_from_text};

pub const GENESIS: &str = "GENESIS";

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

/// Ajoute une entrée en fin de chaîne. À appeler dans la même transaction que
/// l'écriture métier, pour que la preuve ne puisse pas exister sans l'effet.
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

    conn.execute(
        "INSERT INTO audit_log (seq, ts, kind, subject, payload, prev_hash, hash)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![seq, ts, kind, subject, payload_text, prev_hash, hash],
    )?;

    Ok((seq, hash, prev_hash))
}

pub fn list(conn: &Connection, limit: i64) -> rusqlite::Result<Vec<AuditEntry>> {
    let mut stmt = conn.prepare(
        "SELECT id, seq, ts, kind, subject, payload, prev_hash, hash
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

/// Recalcule toute la chaîne et renvoie la première rupture éventuelle.
pub fn verify(conn: &Connection) -> rusqlite::Result<ChainStatus> {
    let mut stmt = conn.prepare(
        "SELECT seq, ts, kind, subject, payload, prev_hash, hash
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

        let recomputed = compute_hash(&prev_hash, seq, &ts, &kind, &subject, &payload);

        if prev_hash != expected_prev || seq != expected_seq || recomputed != hash {
            return Ok(ChainStatus {
                valid: false,
                entries: count,
                head_hash: last_hash,
                checked_at: Utc::now(),
                broken_at_seq: Some(seq),
                message: format!(
                    "Chaîne rompue à la séquence {seq} : la preuve ne correspond plus à son contenu."
                ),
            });
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
        message: format!("{count} entrées vérifiées, chaîne intacte."),
    })
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
}
