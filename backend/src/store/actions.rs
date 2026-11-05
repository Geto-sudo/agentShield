use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Row};
use uuid::Uuid;

use crate::domain::action::{ActionKind, ActionRecord};
use crate::domain::decision::{Decision, RiskBand, RuleCheck};
use crate::domain::money::Currency;
use crate::store::{json_to_text, opt_ts_to_text, text_to_json, ts_from_text, ts_to_text, uid};

const COLUMNS: &str = "id, mandate_id, agent_id, kind, merchant, merchant_country, amount_cents, \
     currency, category, delivery_days, description, idempotency_key, raw_log, decision, \
     risk_score, risk_band, checks, reasons, explanation, approval_id, order_id, settled, \
     audit_hash, created_at";

fn row_to_action(row: &Row<'_>) -> rusqlite::Result<ActionRecord> {
    let checks: Vec<RuleCheck> =
        serde_json::from_str(&row.get::<_, String>(16)?).unwrap_or_default();
    let reasons: Vec<String> = serde_json::from_str(&row.get::<_, String>(17)?).unwrap_or_default();

    Ok(ActionRecord {
        id: uid(&row.get::<_, String>(0)?),
        mandate_id: uid(&row.get::<_, String>(1)?),
        agent_id: uid(&row.get::<_, String>(2)?),
        kind: ActionKind::parse(&row.get::<_, String>(3)?),
        merchant: row.get(4)?,
        merchant_country: row.get(5)?,
        amount_cents: row.get(6)?,
        currency: row.get::<_, String>(7)?.parse().unwrap_or(Currency::Eur),
        category: row.get(8)?,
        delivery_days: row.get::<_, Option<i64>>(9)?.map(|d| d as u32),
        description: row.get(10)?,
        idempotency_key: row.get(11)?,
        raw_log: text_to_json(&row.get::<_, String>(12)?),
        decision: Decision::parse(&row.get::<_, String>(13)?),
        risk_score: row.get::<_, i64>(14)? as u8,
        risk_band: RiskBand::parse(&row.get::<_, String>(15)?),
        checks,
        reasons,
        explanation: row.get(18)?,
        approval_id: row.get::<_, Option<String>>(19)?.map(|s| uid(&s)),
        order_id: row.get::<_, Option<String>>(20)?.map(|s| uid(&s)),
        settled: row.get::<_, i64>(21)? != 0,
        audit_hash: row.get(22)?,
        created_at: ts_from_text(&row.get::<_, String>(23)?),
    })
}

pub fn insert(conn: &Connection, a: &ActionRecord) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO actions (
            id, mandate_id, agent_id, kind, merchant, merchant_country, amount_cents, currency,
            category, delivery_days, description, idempotency_key, occurred_at, metadata, raw_log,
            decision, risk_score, risk_band, checks, reasons, explanation, approval_id, order_id,
            settled, audit_hash, created_at
         ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25,?26)",
        params![
            a.id.to_string(),
            a.mandate_id.to_string(),
            a.agent_id.to_string(),
            a.kind.as_str(),
            a.merchant,
            a.merchant_country,
            a.amount_cents,
            a.currency.as_str(),
            a.category,
            a.delivery_days.map(|d| d as i64),
            a.description,
            a.idempotency_key,
            opt_ts_to_text(a.raw_log.get("occurred_at").and_then(|v| v.as_str()).and_then(|s| s.parse::<DateTime<Utc>>().ok())),
            a.raw_log.get("metadata").map(json_to_text),
            json_to_text(&a.raw_log),
            a.decision.as_str(),
            a.risk_score as i64,
            a.risk_band.as_str(),
            serde_json::to_string(&a.checks).unwrap_or_else(|_| "[]".into()),
            serde_json::to_string(&a.reasons).unwrap_or_else(|_| "[]".into()),
            a.explanation,
            a.approval_id.map(|u| u.to_string()),
            a.order_id.map(|u| u.to_string()),
            if a.settled { 1 } else { 0 },
            a.audit_hash,
            ts_to_text(a.created_at),
        ],
    )?;
    Ok(())
}

pub fn get(conn: &Connection, id: Uuid) -> rusqlite::Result<Option<ActionRecord>> {
    let sql = format!("SELECT {COLUMNS} FROM actions WHERE id = ?1");
    match conn.query_row(&sql, params![id.to_string()], row_to_action) {
        Ok(a) => Ok(Some(a)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

pub struct ActionFilter {
    pub agent_id: Option<Uuid>,
    pub mandate_id: Option<Uuid>,
    pub decision: Option<Decision>,
    pub limit: i64,
}

impl Default for ActionFilter {
    fn default() -> Self {
        Self {
            agent_id: None,
            mandate_id: None,
            decision: None,
            limit: 50,
        }
    }
}

pub fn list(conn: &Connection, filter: &ActionFilter) -> rusqlite::Result<Vec<ActionRecord>> {
    let limit = filter.limit.clamp(1, 500);
    let mut sql = format!("SELECT {COLUMNS} FROM actions WHERE 1 = 1");
    let mut args: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(agent) = filter.agent_id {
        sql.push_str(" AND agent_id = ?");
        args.push(Box::new(agent.to_string()));
    }
    if let Some(mandate) = filter.mandate_id {
        sql.push_str(" AND mandate_id = ?");
        args.push(Box::new(mandate.to_string()));
    }
    if let Some(decision) = filter.decision {
        sql.push_str(" AND decision = ?");
        args.push(Box::new(decision.as_str().to_string()));
    }
    sql.push_str(" ORDER BY created_at DESC LIMIT ?");
    args.push(Box::new(limit));

    let mut stmt = conn.prepare(&sql)?;
    let refs: Vec<&dyn rusqlite::ToSql> = args.iter().map(|b| b.as_ref()).collect();
    let rows = stmt.query_map(refs.as_slice(), row_to_action)?;
    rows.collect()
}

pub fn find_by_idempotency_key(
    conn: &Connection,
    mandate_id: Uuid,
    key: &str,
) -> rusqlite::Result<Option<ActionRecord>> {
    let sql = format!(
        "SELECT {COLUMNS} FROM actions WHERE mandate_id = ?1 AND idempotency_key = ?2 ORDER BY created_at DESC LIMIT 1"
    );
    match conn.query_row(&sql, params![mandate_id.to_string(), key], row_to_action) {
        Ok(a) => Ok(Some(a)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

pub fn recent_count(
    conn: &Connection,
    agent_id: Uuid,
    since: DateTime<Utc>,
) -> rusqlite::Result<i64> {
    conn.query_row(
        "SELECT COUNT(*) FROM actions WHERE agent_id = ?1 AND created_at >= ?2",
        params![agent_id.to_string(), ts_to_text(since)],
        |r| r.get(0),
    )
}

pub fn seen_merchants(conn: &Connection, agent_id: Uuid) -> rusqlite::Result<Vec<String>> {
    let mut stmt = conn
        .prepare("SELECT DISTINCT lower(merchant) FROM actions WHERE agent_id = ?1 ORDER BY 1")?;
    let rows = stmt.query_map(params![agent_id.to_string()], |r| r.get::<_, String>(0))?;
    rows.collect()
}

pub fn attach_order(conn: &Connection, action_id: Uuid, order_id: Uuid) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE actions SET order_id = ?1 WHERE id = ?2",
        params![order_id.to_string(), action_id.to_string()],
    )?;
    Ok(())
}

pub fn mark_settled(conn: &Connection, action_id: Uuid) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE actions SET settled = 1 WHERE id = ?1",
        params![action_id.to_string()],
    )?;
    Ok(())
}

pub fn counts_by_decision(conn: &Connection) -> rusqlite::Result<(i64, i64, i64)> {
    let mut approved = 0;
    let mut escalated = 0;
    let mut refused = 0;
    let mut stmt = conn.prepare("SELECT decision, COUNT(*) FROM actions GROUP BY decision")?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?;
    for row in rows {
        let (decision, count) = row?;
        match decision.as_str() {
            "APPROVE" => approved = count,
            "ESCALATE" => escalated = count,
            _ => refused = count,
        }
    }
    Ok((approved, escalated, refused))
}

pub fn settled_total_cents(conn: &Connection) -> rusqlite::Result<i64> {
    conn.query_row(
        "SELECT COALESCE(SUM(amount_cents),0) FROM actions WHERE settled = 1",
        [],
        |r| r.get(0),
    )
}

pub fn weekly_refusals(conn: &Connection, since: DateTime<Utc>) -> rusqlite::Result<i64> {
    conn.query_row(
        "SELECT COUNT(*) FROM actions WHERE decision = 'REFUSE' AND created_at >= ?1",
        params![ts_to_text(since)],
        |r| r.get(0),
    )
}
