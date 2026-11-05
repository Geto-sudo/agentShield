use rusqlite::{params, Connection, Row};
use uuid::Uuid;

use crate::domain::agent::{Agent, AgentStatus};
use crate::store::{ts_from_text, ts_to_text, uid};

const COLUMNS: &str = "id, name, owner, purpose, status, created_at";

fn row_to_agent(row: &Row<'_>) -> rusqlite::Result<Agent> {
    Ok(Agent {
        id: uid(&row.get::<_, String>(0)?),
        name: row.get(1)?,
        owner: row.get(2)?,
        purpose: row.get(3)?,
        status: AgentStatus::parse(&row.get::<_, String>(4)?),
        created_at: ts_from_text(&row.get::<_, String>(5)?),
    })
}

pub fn insert(conn: &Connection, agent: &Agent) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO agents (id, name, owner, purpose, status, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            agent.id.to_string(),
            agent.name,
            agent.owner,
            agent.purpose,
            agent.status.as_str(),
            ts_to_text(agent.created_at),
        ],
    )?;
    Ok(())
}

pub fn get(conn: &Connection, id: Uuid) -> rusqlite::Result<Option<Agent>> {
    let sql = format!("SELECT {COLUMNS} FROM agents WHERE id = ?1");
    match conn.query_row(&sql, params![id.to_string()], row_to_agent) {
        Ok(a) => Ok(Some(a)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

pub fn get_or_err(conn: &Connection, id: Uuid) -> rusqlite::Result<Option<Agent>> {
    get(conn, id)
}

pub fn list(conn: &Connection) -> rusqlite::Result<Vec<Agent>> {
    let sql = format!("SELECT {COLUMNS} FROM agents ORDER BY created_at ASC");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], row_to_agent)?;
    rows.collect()
}

pub fn count(conn: &Connection) -> rusqlite::Result<i64> {
    conn.query_row("SELECT COUNT(*) FROM agents", [], |r| r.get(0))
}
