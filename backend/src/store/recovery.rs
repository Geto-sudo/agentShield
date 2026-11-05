use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Row};
use uuid::Uuid;

use crate::domain::money::Currency;
use crate::domain::recovery::{Incident, IncidentKind, IncidentStatus, Payout, PayoutStatus};
use crate::store::{
    json_to_text, opt_ts_from_text, opt_ts_to_text, text_to_json, ts_from_text, ts_to_text, uid,
};

const INCIDENT_COLUMNS: &str = "id, agent_id, mandate_id, action_id, kind, status, description, \
     amount_cents, currency, overseen_by, verdict, payout_id, created_at, resolved_at";

fn row_to_incident(row: &Row<'_>) -> rusqlite::Result<Incident> {
    let verdict_text: Option<String> = row.get(10)?;
    Ok(Incident {
        id: uid(&row.get::<_, String>(0)?),
        agent_id: uid(&row.get::<_, String>(1)?),
        mandate_id: uid(&row.get::<_, String>(2)?),
        action_id: row.get::<_, Option<String>>(3)?.map(|s| uid(&s)),
        kind: IncidentKind::parse(&row.get::<_, String>(4)?),
        status: IncidentStatus::parse(&row.get::<_, String>(5)?),
        description: row.get(6)?,
        amount_cents: row.get(7)?,
        currency: row.get::<_, String>(8)?.parse().unwrap_or(Currency::Eur),
        overseen_by: row.get(9)?,
        verdict: verdict_text.map(|t| text_to_json(&t)),
        payout_id: row.get::<_, Option<String>>(11)?.map(|s| uid(&s)),
        created_at: ts_from_text(&row.get::<_, String>(12)?),
        resolved_at: opt_ts_from_text(row.get(13)?),
    })
}

pub fn insert_incident(conn: &Connection, i: &Incident) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO incidents (id, agent_id, mandate_id, action_id, kind, status, description,
                                amount_cents, currency, overseen_by, verdict, payout_id,
                                created_at, resolved_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
        params![
            i.id.to_string(),
            i.agent_id.to_string(),
            i.mandate_id.to_string(),
            i.action_id.map(|u| u.to_string()),
            i.kind.as_str(),
            i.status.as_str(),
            i.description,
            i.amount_cents,
            i.currency.as_str(),
            i.overseen_by,
            i.verdict.as_ref().map(json_to_text),
            i.payout_id.map(|u| u.to_string()),
            ts_to_text(i.created_at),
            opt_ts_to_text(i.resolved_at),
        ],
    )?;
    Ok(())
}

pub fn get_incident(conn: &Connection, id: Uuid) -> rusqlite::Result<Option<Incident>> {
    let sql = format!("SELECT {INCIDENT_COLUMNS} FROM incidents WHERE id = ?1");
    match conn.query_row(&sql, params![id.to_string()], row_to_incident) {
        Ok(i) => Ok(Some(i)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

pub fn list_incidents(
    conn: &Connection,
    status: Option<IncidentStatus>,
    limit: i64,
) -> rusqlite::Result<Vec<Incident>> {
    let sql = format!(
        "SELECT {INCIDENT_COLUMNS} FROM incidents WHERE (?1 IS NULL OR status = ?1)
         ORDER BY created_at DESC LIMIT ?2"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(
        params![status.map(|s| s.as_str().to_string()), limit.clamp(1, 500)],
        row_to_incident,
    )?;
    rows.collect()
}

/// Cherche un sinistre encore ouvert, éventuellement pour un agent donné.
pub fn latest_open_incident(
    conn: &Connection,
    agent_id: Option<Uuid>,
) -> rusqlite::Result<Option<Incident>> {
    let sql = format!(
        "SELECT {INCIDENT_COLUMNS} FROM incidents
         WHERE status IN ('open','under_review') AND (?1 IS NULL OR agent_id = ?1)
         ORDER BY created_at ASC LIMIT 1"
    );
    match conn.query_row(
        &sql,
        params![agent_id.map(|a| a.to_string())],
        row_to_incident,
    ) {
        Ok(i) => Ok(Some(i)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

pub fn update_incident(
    conn: &Connection,
    id: Uuid,
    status: IncidentStatus,
    verdict: Option<&serde_json::Value>,
    payout_id: Option<Uuid>,
    resolved_at: Option<DateTime<Utc>>,
) -> rusqlite::Result<usize> {
    conn.execute(
        "UPDATE incidents SET status = ?1, verdict = COALESCE(?2, verdict),
                payout_id = COALESCE(?3, payout_id), resolved_at = ?4
         WHERE id = ?5",
        params![
            status.as_str(),
            verdict.map(json_to_text),
            payout_id.map(|u| u.to_string()),
            opt_ts_to_text(resolved_at),
            id.to_string()
        ],
    )
}

pub fn count_open_incidents(conn: &Connection, agent_id: Uuid) -> rusqlite::Result<i64> {
    conn.query_row(
        "SELECT COUNT(*) FROM incidents WHERE agent_id = ?1 AND status IN ('open','under_review')",
        params![agent_id.to_string()],
        |r| r.get(0),
    )
}

pub fn count_incidents(conn: &Connection, agent_id: Uuid) -> rusqlite::Result<i64> {
    conn.query_row(
        "SELECT COUNT(*) FROM incidents WHERE agent_id = ?1",
        params![agent_id.to_string()],
        |r| r.get(0),
    )
}

// --------------------------------------------------------------------------
// Payouts Sandbox
// --------------------------------------------------------------------------

const PAYOUT_COLUMNS: &str = "id, incident_id, paypal_batch_id, amount_cents, currency, status, \
     mode, recipient_email, note, created_at, completed_at";

fn row_to_payout(row: &Row<'_>) -> rusqlite::Result<Payout> {
    Ok(Payout {
        id: uid(&row.get::<_, String>(0)?),
        incident_id: row.get::<_, Option<String>>(1)?.map(|s| uid(&s)),
        paypal_batch_id: row.get(2)?,
        amount_cents: row.get(3)?,
        currency: row.get::<_, String>(4)?.parse().unwrap_or(Currency::Eur),
        status: PayoutStatus::parse(&row.get::<_, String>(5)?),
        mode: row.get(6)?,
        recipient_email: row.get(7)?,
        note: row.get(8)?,
        created_at: ts_from_text(&row.get::<_, String>(9)?),
        completed_at: opt_ts_from_text(row.get(10)?),
    })
}

pub fn insert_payout(conn: &Connection, p: &Payout) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO payouts (id, incident_id, paypal_batch_id, amount_cents, currency, status,
                              mode, recipient_email, note, created_at, completed_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
        params![
            p.id.to_string(),
            p.incident_id.map(|u| u.to_string()),
            p.paypal_batch_id,
            p.amount_cents,
            p.currency.as_str(),
            p.status.as_str(),
            p.mode,
            p.recipient_email,
            p.note,
            ts_to_text(p.created_at),
            opt_ts_to_text(p.completed_at),
        ],
    )?;
    Ok(())
}

pub fn get_payout(conn: &Connection, id: Uuid) -> rusqlite::Result<Option<Payout>> {
    let sql = format!("SELECT {PAYOUT_COLUMNS} FROM payouts WHERE id = ?1");
    match conn.query_row(&sql, params![id.to_string()], row_to_payout) {
        Ok(p) => Ok(Some(p)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

pub fn list_payouts(conn: &Connection, limit: i64) -> rusqlite::Result<Vec<Payout>> {
    let sql = format!("SELECT {PAYOUT_COLUMNS} FROM payouts ORDER BY created_at DESC LIMIT ?1");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![limit.clamp(1, 500)], row_to_payout)?;
    rows.collect()
}
