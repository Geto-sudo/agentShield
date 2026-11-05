use rusqlite::{params, Connection, Row};
use uuid::Uuid;

use crate::domain::mandate::{Mandate, MandateStatus};
use crate::domain::money::Currency;
use crate::store::{text_to_vec, ts_from_text, ts_to_text, uid, vec_to_text};

const COLUMNS: &str = "id, agent_id, owner_name, intent, category, currency, max_total_cents, \
     per_txn_max_cents, auto_approve_max_cents, escalate_until_cents, allowed_merchants, \
     blocked_merchants, allowed_countries, max_delivery_days, valid_from, valid_until, status, created_at";

fn row_to_mandate(row: &Row<'_>) -> rusqlite::Result<Mandate> {
    Ok(Mandate {
        id: uid(&row.get::<_, String>(0)?),
        agent_id: uid(&row.get::<_, String>(1)?),
        owner_name: row.get(2)?,
        intent: row.get(3)?,
        category: row.get(4)?,
        currency: row.get::<_, String>(5)?.parse().unwrap_or(Currency::Eur),
        max_total_cents: row.get(6)?,
        per_txn_max_cents: row.get(7)?,
        auto_approve_max_cents: row.get(8)?,
        escalate_until_cents: row.get(9)?,
        allowed_merchants: text_to_vec(&row.get::<_, String>(10)?),
        blocked_merchants: text_to_vec(&row.get::<_, String>(11)?),
        allowed_countries: text_to_vec(&row.get::<_, String>(12)?),
        max_delivery_days: row.get::<_, i64>(13)? as u32,
        valid_from: ts_from_text(&row.get::<_, String>(14)?),
        valid_until: ts_from_text(&row.get::<_, String>(15)?),
        status: MandateStatus::parse(&row.get::<_, String>(16)?),
        created_at: ts_from_text(&row.get::<_, String>(17)?),
    })
}

pub fn insert(conn: &Connection, m: &Mandate) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO mandates (
            id, agent_id, owner_name, intent, category, currency, max_total_cents,
            per_txn_max_cents, auto_approve_max_cents, escalate_until_cents,
            allowed_merchants, blocked_merchants, allowed_countries, max_delivery_days,
            valid_from, valid_until, status, created_at
         ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18)",
        params![
            m.id.to_string(),
            m.agent_id.to_string(),
            m.owner_name,
            m.intent,
            m.category,
            m.currency.as_str(),
            m.max_total_cents,
            m.per_txn_max_cents,
            m.auto_approve_max_cents,
            m.escalate_until_cents,
            vec_to_text(&m.allowed_merchants),
            vec_to_text(&m.blocked_merchants),
            vec_to_text(&m.allowed_countries),
            m.max_delivery_days as i64,
            ts_to_text(m.valid_from),
            ts_to_text(m.valid_until),
            m.status.as_str(),
            ts_to_text(m.created_at),
        ],
    )?;
    Ok(())
}

pub fn get(conn: &Connection, id: Uuid) -> rusqlite::Result<Option<Mandate>> {
    let sql = format!("SELECT {COLUMNS} FROM mandates WHERE id = ?1");
    match conn.query_row(&sql, params![id.to_string()], row_to_mandate) {
        Ok(m) => Ok(Some(m)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

pub fn list(
    conn: &Connection,
    agent_id: Option<Uuid>,
    limit: i64,
) -> rusqlite::Result<Vec<Mandate>> {
    let limit = limit.clamp(1, 500);
    match agent_id {
        Some(agent) => {
            let sql = format!(
                "SELECT {COLUMNS} FROM mandates WHERE agent_id = ?1 ORDER BY created_at DESC LIMIT ?2"
            );
            let mut stmt = conn.prepare(&sql)?;
            let rows = stmt.query_map(params![agent.to_string(), limit], row_to_mandate)?;
            rows.collect()
        }
        None => {
            let sql = format!("SELECT {COLUMNS} FROM mandates ORDER BY created_at DESC LIMIT ?1");
            let mut stmt = conn.prepare(&sql)?;
            let rows = stmt.query_map(params![limit], row_to_mandate)?;
            rows.collect()
        }
    }
}

pub fn update_status(
    conn: &Connection,
    id: Uuid,
    status: MandateStatus,
) -> rusqlite::Result<usize> {
    conn.execute(
        "UPDATE mandates SET status = ?1 WHERE id = ?2",
        params![status.as_str(), id.to_string()],
    )
}

/// Somme des dépenses déjà approuvées pour ce mandat.
pub fn committed_cents(conn: &Connection, mandate_id: Uuid) -> rusqlite::Result<i64> {
    conn.query_row(
        "SELECT COALESCE(SUM(amount_cents), 0) FROM actions
         WHERE mandate_id = ?1 AND decision IN ('APPROVE','ESCALATE')",
        params![mandate_id.to_string()],
        |r| r.get(0),
    )
}
