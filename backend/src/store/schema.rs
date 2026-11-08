use rusqlite::Connection;

/// The schema is compiled into the binary from `migrations/0001_init.sql`:
/// a single source of truth, with no runtime dependency on an external file.
pub const SCHEMA_SQL: &str = include_str!("../../migrations/0001_init.sql");

pub fn apply(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(SCHEMA_SQL)?;
    // Single shared reserve row: create it if it does not exist.
    let exists: i64 = conn.query_row("SELECT COUNT(*) FROM reserve", [], |r| r.get(0))?;
    if exists == 0 {
        conn.execute(
            "INSERT INTO reserve (id, balance_cents, currency, updated_at) VALUES (1, ?1, 'EUR', ?2)",
            rusqlite::params![250_000_i64, chrono::Utc::now().to_rfc3339()],
        )?;
    }
    Ok(())
}
