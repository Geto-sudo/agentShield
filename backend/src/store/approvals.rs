use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Row};
use uuid::Uuid;

use crate::domain::approval::{Approval, ApprovalStatus};
use crate::store::{opt_ts_from_text, opt_ts_to_text, ts_from_text, ts_to_text, uid};

const COLUMNS: &str =
    "id, action_id, status, requested_at, expires_at, decided_at, decided_by, note, created_at";

fn row_to_approval(row: &Row<'_>) -> rusqlite::Result<Approval> {
    Ok(Approval {
        id: uid(&row.get::<_, String>(0)?),
        action_id: uid(&row.get::<_, String>(1)?),
        status: ApprovalStatus::parse(&row.get::<_, String>(2)?),
        requested_at: ts_from_text(&row.get::<_, String>(3)?),
        expires_at: ts_from_text(&row.get::<_, String>(4)?),
        decided_at: opt_ts_from_text(row.get(5)?),
        decided_by: row.get(6)?,
        note: row.get(7)?,
        created_at: ts_from_text(&row.get::<_, String>(8)?),
    })
}

pub fn insert(conn: &Connection, a: &Approval) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO approvals (id, action_id, status, requested_at, expires_at, decided_at,
                                decided_by, note, created_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
        params![
            a.id.to_string(),
            a.action_id.to_string(),
            a.status.as_str(),
            ts_to_text(a.requested_at),
            ts_to_text(a.expires_at),
            opt_ts_to_text(a.decided_at),
            a.decided_by,
            a.note,
            ts_to_text(a.created_at),
        ],
    )?;
    Ok(())
}

pub fn get(conn: &Connection, id: Uuid) -> rusqlite::Result<Option<Approval>> {
    let sql = format!("SELECT {COLUMNS} FROM approvals WHERE id = ?1");
    match conn.query_row(&sql, params![id.to_string()], row_to_approval) {
        Ok(a) => Ok(Some(a)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

pub fn get_by_action(conn: &Connection, action_id: Uuid) -> rusqlite::Result<Option<Approval>> {
    let sql = format!(
        "SELECT {COLUMNS} FROM approvals WHERE action_id = ?1 ORDER BY created_at DESC LIMIT 1"
    );
    match conn.query_row(&sql, params![action_id.to_string()], row_to_approval) {
        Ok(a) => Ok(Some(a)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

pub fn list(
    conn: &Connection,
    status: Option<ApprovalStatus>,
    limit: i64,
) -> rusqlite::Result<Vec<Approval>> {
    let limit = limit.clamp(1, 500);
    let sql = format!(
        "SELECT {COLUMNS} FROM approvals WHERE (?1 IS NULL OR status = ?1) ORDER BY created_at DESC LIMIT ?2"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(
        params![status.map(|s| s.as_str().to_string()), limit],
        row_to_approval,
    )?;
    rows.collect()
}

pub fn decide(
    conn: &Connection,
    id: Uuid,
    status: ApprovalStatus,
    decided_by: Option<&str>,
    note: Option<&str>,
    now: DateTime<Utc>,
) -> rusqlite::Result<usize> {
    conn.execute(
        "UPDATE approvals SET status = ?1, decided_at = ?2, decided_by = ?3, note = ?4
         WHERE id = ?5 AND status = 'pending'",
        params![
            status.as_str(),
            ts_to_text(now),
            decided_by,
            note,
            id.to_string()
        ],
    )
}

/// Marque comme expirées les escalades dont la fenêtre est passée.
pub fn expire_stale(conn: &Connection, now: DateTime<Utc>) -> rusqlite::Result<usize> {
    conn.execute(
        "UPDATE approvals SET status = 'expired', decided_at = ?1
         WHERE status = 'pending' AND expires_at < ?2",
        params![ts_to_text(now), ts_to_text(now)],
    )
}

pub fn pending_count(conn: &Connection) -> rusqlite::Result<i64> {
    conn.query_row(
        "SELECT COUNT(*) FROM approvals WHERE status = 'pending'",
        [],
        |r| r.get(0),
    )
}
