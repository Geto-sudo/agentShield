//! Persistance SQLite. Une seule connexion sérialisée par un `Mutex` : c'est
//! suffisant et volontairement simple pour un prototype, et cela garantit que
//! la chaîne d'audit est toujours écrite dans l'ordre.

pub mod actions;
pub mod agents;
pub mod approvals;
pub mod audit;
pub mod mandates;
pub mod orders;
pub mod recovery;
pub mod schema;
pub mod seed;
pub mod wallet;

use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use rusqlite::Connection;
use uuid::Uuid;

#[derive(Clone)]
pub struct Db {
    conn: Arc<Mutex<Connection>>,
}

impl Db {
    pub fn open(path: &str) -> rusqlite::Result<Self> {
        if let Some(dir) = std::path::Path::new(path).parent() {
            if !dir.as_os_str().is_empty() {
                let _ = std::fs::create_dir_all(dir);
            }
        }
        let conn = Connection::open(path)?;
        // WAL améliore la lecture concurrente ; ignoré silencieusement en mémoire.
        let _ = conn.pragma_update(None, "journal_mode", "WAL");
        Self::init(conn)
    }

    pub fn open_memory() -> rusqlite::Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> rusqlite::Result<Self> {
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        schema::apply(&conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn with_conn<T>(
        &self,
        f: impl FnOnce(&Connection) -> rusqlite::Result<T>,
    ) -> rusqlite::Result<T> {
        let guard = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        f(&guard)
    }

    pub fn with_tx<T>(
        &self,
        f: impl FnOnce(&Connection) -> rusqlite::Result<T>,
    ) -> rusqlite::Result<T> {
        let guard = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let tx = guard.unchecked_transaction()?;
        let out = f(&tx)?;
        tx.commit()?;
        Ok(out)
    }
}

// ---------------------------------------------------------------------------
// Helpers de (dé)sérialisation partagés par tous les dépôts.
// ---------------------------------------------------------------------------

pub fn ts_to_text(ts: DateTime<Utc>) -> String {
    ts.to_rfc3339()
}

pub fn ts_from_text(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s)
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or_else(|_| DateTime::<Utc>::from_timestamp(0, 0).expect("epoch"))
}

pub fn opt_ts_to_text(ts: Option<DateTime<Utc>>) -> Option<String> {
    ts.map(ts_to_text)
}

pub fn opt_ts_from_text(s: Option<String>) -> Option<DateTime<Utc>> {
    s.as_deref().map(ts_from_text)
}

pub fn uid(s: &str) -> Uuid {
    Uuid::parse_str(s).unwrap_or_default()
}

pub fn vec_to_text(v: &[String]) -> String {
    serde_json::to_string(v).unwrap_or_else(|_| "[]".to_string())
}

pub fn text_to_vec(s: &str) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(s).unwrap_or_default()
}

pub fn json_to_text(v: &serde_json::Value) -> String {
    serde_json::to_string(v).unwrap_or_else(|_| "{}".to_string())
}

pub fn text_to_json(s: &str) -> serde_json::Value {
    serde_json::from_str(s).unwrap_or(serde_json::Value::Null)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrips_timestamps() {
        let now = Utc::now();
        let text = ts_to_text(now);
        assert_eq!(ts_from_text(&text), now);
    }

    #[test]
    fn roundtrips_json_vectors() {
        let v = vec!["TechStore".to_string(), "AudioLab".to_string()];
        assert_eq!(text_to_vec(&vec_to_text(&v)), v);
        assert!(text_to_vec("not json").is_empty());
    }

    #[test]
    fn opens_in_memory_database() {
        let db = Db::open_memory().expect("db memoire");
        let count: i64 = db
            .with_conn(|c| c.query_row("SELECT COUNT(*) FROM mandates", [], |r| r.get(0)))
            .expect("count");
        assert_eq!(count, 0);
    }
}
