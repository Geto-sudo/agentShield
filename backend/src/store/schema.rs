use rusqlite::Connection;

/// Le schéma est compilé dans le binaire depuis `migrations/0001_init.sql` :
/// une seule source de vérité, aucune dépendance à un fichier au runtime.
pub const SCHEMA_SQL: &str = include_str!("../../migrations/0001_init.sql");

pub fn apply(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(SCHEMA_SQL)?;

    // `CREATE TABLE IF NOT EXISTS` ne touche pas une table déjà présente : les
    // bases créées avant la signature Ed25519 n'ont pas ces colonnes. On les
    // ajoute de façon idempotente (les entrées existantes restent non signées
    // et seront signalées par `audit::verify`).
    ensure_column(conn, "audit_log", "signature", "TEXT")?;
    ensure_column(conn, "audit_log", "pubkey", "TEXT")?;

    // Ligne unique de réserve commune : créée si absente.
    let exists: i64 = conn.query_row("SELECT COUNT(*) FROM reserve", [], |r| r.get(0))?;
    if exists == 0 {
        conn.execute(
            "INSERT INTO reserve (id, balance_cents, currency, updated_at) VALUES (1, ?1, 'EUR', ?2)",
            rusqlite::params![250_000_i64, chrono::Utc::now().to_rfc3339()],
        )?;
    }
    Ok(())
}

/// `ALTER TABLE … ADD COLUMN` conditionnel : no-op si la colonne existe déjà.
/// Les identifiants sont des constantes du code, jamais des entrées utilisateur.
fn ensure_column(
    conn: &Connection,
    table: &str,
    column: &str,
    declaration: &str,
) -> rusqlite::Result<()> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;
    let mut present = false;
    for name in rows {
        if name?.eq_ignore_ascii_case(column) {
            present = true;
            break;
        }
    }
    drop(stmt);

    if !present {
        conn.execute_batch(&format!(
            "ALTER TABLE {table} ADD COLUMN {column} {declaration}"
        ))?;
        tracing::info!(table, column, "Migration : colonne ajoutée");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_adds_signature_columns_to_an_existing_audit_log() {
        let conn = Connection::open_in_memory().unwrap();
        // Table « historique », sans signature : c'est l'état des bases créées
        // avant l'introduction de la signature Ed25519.
        conn.execute_batch(
            "CREATE TABLE audit_log (
                id        INTEGER PRIMARY KEY AUTOINCREMENT,
                seq       INTEGER NOT NULL UNIQUE,
                ts        TEXT NOT NULL,
                kind      TEXT NOT NULL,
                subject   TEXT NOT NULL,
                payload   TEXT NOT NULL,
                prev_hash TEXT NOT NULL,
                hash      TEXT NOT NULL
            );
            INSERT INTO audit_log (seq, ts, kind, subject, payload, prev_hash, hash)
            VALUES (1, '2026-01-01T00:00:00Z', 'agent.created', 'a1', '{}', 'GENESIS', 'abc');",
        )
        .unwrap();

        apply(&conn).unwrap();
        apply(&conn).unwrap(); // idempotent

        let columns: Vec<String> = {
            let mut stmt = conn.prepare("PRAGMA table_info(audit_log)").unwrap();
            let rows = stmt.query_map([], |row| row.get::<_, String>(1)).unwrap();
            rows.map(|r| r.unwrap()).collect()
        };
        assert!(columns.iter().any(|c| c == "signature"), "{columns:?}");
        assert!(columns.iter().any(|c| c == "pubkey"), "{columns:?}");

        // L'entrée historique est conservée, simplement non signée.
        let (count, signature): (i64, Option<String>) = conn
            .query_row(
                "SELECT COUNT(*), MAX(signature) FROM audit_log",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(count, 1);
        assert_eq!(signature, None);
    }
}
