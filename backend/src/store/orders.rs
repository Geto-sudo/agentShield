use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Row};
use uuid::Uuid;

use crate::domain::money::Currency;
use crate::domain::order::{OrderPurpose, OrderStatus, SandboxOrder};
use crate::store::{opt_ts_from_text, opt_ts_to_text, ts_from_text, ts_to_text, uid};

const COLUMNS: &str = "id, action_id, purpose, paypal_order_id, status, amount_cents, currency, \
     description, approve_url, capture_id, mode, failure_reason, created_at, captured_at";

fn row_to_order(row: &Row<'_>) -> rusqlite::Result<SandboxOrder> {
    Ok(SandboxOrder {
        id: uid(&row.get::<_, String>(0)?),
        action_id: row.get::<_, Option<String>>(1)?.map(|s| uid(&s)),
        purpose: OrderPurpose::parse(&row.get::<_, String>(2)?),
        paypal_order_id: row.get(3)?,
        status: OrderStatus::parse(&row.get::<_, String>(4)?),
        amount_cents: row.get(5)?,
        currency: row.get::<_, String>(6)?.parse().unwrap_or(Currency::Eur),
        description: row.get(7)?,
        approve_url: row.get(8)?,
        capture_id: row.get(9)?,
        mode: row.get(10)?,
        failure_reason: row.get(11)?,
        created_at: ts_from_text(&row.get::<_, String>(12)?),
        captured_at: opt_ts_from_text(row.get(13)?),
    })
}

pub fn insert(conn: &Connection, o: &SandboxOrder) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO orders (id, action_id, purpose, paypal_order_id, status, amount_cents,
                             currency, description, approve_url, capture_id, mode, failure_reason,
                             created_at, captured_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
        params![
            o.id.to_string(),
            o.action_id.map(|u| u.to_string()),
            o.purpose.as_str(),
            o.paypal_order_id,
            o.status.as_str(),
            o.amount_cents,
            o.currency.as_str(),
            o.description,
            o.approve_url,
            o.capture_id,
            o.mode,
            o.failure_reason,
            ts_to_text(o.created_at),
            opt_ts_to_text(o.captured_at),
        ],
    )?;
    Ok(())
}

pub fn get(conn: &Connection, id: Uuid) -> rusqlite::Result<Option<SandboxOrder>> {
    let sql = format!("SELECT {COLUMNS} FROM orders WHERE id = ?1");
    match conn.query_row(&sql, params![id.to_string()], row_to_order) {
        Ok(o) => Ok(Some(o)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

pub fn get_by_paypal_id(
    conn: &Connection,
    paypal_order_id: &str,
) -> rusqlite::Result<Option<SandboxOrder>> {
    let sql = format!("SELECT {COLUMNS} FROM orders WHERE paypal_order_id = ?1");
    match conn.query_row(&sql, params![paypal_order_id], row_to_order) {
        Ok(o) => Ok(Some(o)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

pub fn get_by_action(conn: &Connection, action_id: Uuid) -> rusqlite::Result<Option<SandboxOrder>> {
    let sql = format!(
        "SELECT {COLUMNS} FROM orders WHERE action_id = ?1 ORDER BY created_at DESC LIMIT 1"
    );
    match conn.query_row(&sql, params![action_id.to_string()], row_to_order) {
        Ok(o) => Ok(Some(o)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

pub fn list(conn: &Connection, limit: i64) -> rusqlite::Result<Vec<SandboxOrder>> {
    let sql = format!("SELECT {COLUMNS} FROM orders ORDER BY created_at DESC LIMIT ?1");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![limit.clamp(1, 500)], row_to_order)?;
    rows.collect()
}

pub fn mark_captured(
    conn: &Connection,
    id: Uuid,
    capture_id: Option<&str>,
    now: DateTime<Utc>,
) -> rusqlite::Result<usize> {
    conn.execute(
        "UPDATE orders SET status = 'captured', capture_id = ?1, captured_at = ?2 WHERE id = ?3",
        params![capture_id, ts_to_text(now), id.to_string()],
    )
}

pub fn mark_failed(conn: &Connection, id: Uuid, reason: &str) -> rusqlite::Result<usize> {
    conn.execute(
        "UPDATE orders SET status = 'failed', failure_reason = ?1 WHERE id = ?2",
        params![reason, id.to_string()],
    )
}

pub fn count_captured(conn: &Connection) -> rusqlite::Result<i64> {
    conn.query_row(
        "SELECT COUNT(*) FROM orders WHERE status = 'captured'",
        [],
        |r| r.get(0),
    )
}
