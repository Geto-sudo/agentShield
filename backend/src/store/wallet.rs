use chrono::Utc;
use rusqlite::{params, Connection, Row};
use uuid::Uuid;

use crate::domain::wallet::{LedgerEntry, Wallet};
use crate::store::{ts_from_text, ts_to_text, uid};

const COLUMNS: &str = "agent_id, balance_cents, premium_cents, bonus_malus_bps, \
     reserve_contribution_cents, spent_this_period_cents, updated_at";

fn row_to_wallet(row: &Row<'_>) -> rusqlite::Result<Wallet> {
    Ok(Wallet {
        agent_id: uid(&row.get::<_, String>(0)?),
        balance_cents: row.get(1)?,
        premium_cents: row.get(2)?,
        bonus_malus_bps: row.get(3)?,
        reserve_contribution_cents: row.get(4)?,
        spent_this_period_cents: row.get(5)?,
        updated_at: ts_from_text(&row.get::<_, String>(6)?),
    })
}

/// Creates the wallet if it does not exist, with a base premium and demo balance.
pub fn ensure(
    conn: &Connection,
    agent_id: Uuid,
    premium_cents: i64,
    initial_balance_cents: i64,
) -> rusqlite::Result<Wallet> {
    let existing: Option<Wallet> = match conn.query_row(
        &format!("SELECT {COLUMNS} FROM wallets WHERE agent_id = ?1"),
        params![agent_id.to_string()],
        row_to_wallet,
    ) {
        Ok(w) => Some(w),
        Err(rusqlite::Error::QueryReturnedNoRows) => None,
        Err(e) => return Err(e),
    };

    if let Some(w) = existing {
        return Ok(w);
    }

    let wallet = Wallet {
        agent_id,
        balance_cents: initial_balance_cents,
        premium_cents,
        bonus_malus_bps: 0,
        reserve_contribution_cents: premium_cents / 10,
        spent_this_period_cents: 0,
        updated_at: Utc::now(),
    };

    conn.execute(
        "INSERT INTO wallets (agent_id, balance_cents, premium_cents, bonus_malus_bps,
                              reserve_contribution_cents, spent_this_period_cents, updated_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7)",
        params![
            agent_id.to_string(),
            wallet.balance_cents,
            wallet.premium_cents,
            wallet.bonus_malus_bps,
            wallet.reserve_contribution_cents,
            wallet.spent_this_period_cents,
            ts_to_text(wallet.updated_at),
        ],
    )?;

    if initial_balance_cents > 0 {
        insert_ledger(
            conn,
            agent_id,
            "credit",
            initial_balance_cents,
            initial_balance_cents,
            "Initial demonstration balance",
            None,
        )?;
    }

    Ok(wallet)
}

pub fn get(conn: &Connection, agent_id: Uuid) -> rusqlite::Result<Option<Wallet>> {
    match conn.query_row(
        &format!("SELECT {COLUMNS} FROM wallets WHERE agent_id = ?1"),
        params![agent_id.to_string()],
        row_to_wallet,
    ) {
        Ok(w) => Ok(Some(w)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

/// Applies a balance movement and writes the corresponding ledger entry.
pub fn move_balance(
    conn: &Connection,
    agent_id: Uuid,
    delta_cents: i64,
    kind: &str,
    label: &str,
    reference: Option<&str>,
) -> rusqlite::Result<Wallet> {
    let wallet = get(conn, agent_id)?.unwrap_or(Wallet {
        agent_id,
        balance_cents: 0,
        premium_cents: 1_200,
        bonus_malus_bps: 0,
        reserve_contribution_cents: 120,
        spent_this_period_cents: 0,
        updated_at: Utc::now(),
    });

    let new_balance = wallet.balance_cents + delta_cents;
    let new_spent = if delta_cents < 0 {
        wallet.spent_this_period_cents + delta_cents.abs()
    } else {
        wallet.spent_this_period_cents
    };

    conn.execute(
        "UPDATE wallets SET balance_cents = ?1, spent_this_period_cents = ?2, updated_at = ?3
         WHERE agent_id = ?4",
        params![
            new_balance,
            new_spent,
            ts_to_text(Utc::now()),
            agent_id.to_string()
        ],
    )?;

    insert_ledger(
        conn,
        agent_id,
        kind,
        delta_cents,
        new_balance,
        label,
        reference,
    )?;

    Ok(Wallet {
        balance_cents: new_balance,
        spent_this_period_cents: new_spent,
        updated_at: Utc::now(),
        ..wallet
    })
}

fn insert_ledger(
    conn: &Connection,
    agent_id: Uuid,
    kind: &str,
    amount_cents: i64,
    balance_after: i64,
    label: &str,
    reference: Option<&str>,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO ledger (agent_id, kind, amount_cents, balance_after_cents, label, reference, created_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7)",
        params![
            agent_id.to_string(),
            kind,
            amount_cents,
            balance_after,
            label,
            reference,
            ts_to_text(Utc::now())
        ],
    )?;
    Ok(())
}

pub fn set_bonus_malus(conn: &Connection, agent_id: Uuid, bps: i32) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE wallets SET bonus_malus_bps = ?1, updated_at = ?2 WHERE agent_id = ?3",
        params![bps, ts_to_text(Utc::now()), agent_id.to_string()],
    )?;
    Ok(())
}

pub fn ledger(conn: &Connection, agent_id: Uuid, limit: i64) -> rusqlite::Result<Vec<LedgerEntry>> {
    let mut stmt = conn.prepare(
        "SELECT id, agent_id, kind, amount_cents, balance_after_cents, label, reference, created_at
         FROM ledger WHERE agent_id = ?1 ORDER BY id DESC LIMIT ?2",
    )?;
    let rows = stmt.query_map(params![agent_id.to_string(), limit.clamp(1, 200)], |row| {
        Ok(LedgerEntry {
            id: row.get(0)?,
            agent_id: uid(&row.get::<_, String>(1)?),
            kind: row.get(2)?,
            amount_cents: row.get(3)?,
            balance_after_cents: row.get(4)?,
            label: row.get(5)?,
            reference: row.get(6)?,
            created_at: ts_from_text(&row.get::<_, String>(7)?),
        })
    })?;
    rows.collect()
}
// --------------------------------------------------------------------------
// Local shared reserve
// --------------------------------------------------------------------------

pub fn reserve_balance(conn: &Connection) -> rusqlite::Result<i64> {
    conn.query_row("SELECT balance_cents FROM reserve WHERE id = 1", [], |r| {
        r.get(0)
    })
}

/// Applies a movement to the shared reserve and records it in the ledger.
pub fn reserve_move(
    conn: &Connection,
    delta_cents: i64,
    kind: &str,
    label: &str,
    reference: Option<&str>,
) -> rusqlite::Result<i64> {
    let current = reserve_balance(conn)?;
    let new_balance = (current + delta_cents).max(0);

    conn.execute(
        "UPDATE reserve SET balance_cents = ?1, updated_at = ?2 WHERE id = 1",
        params![new_balance, ts_to_text(Utc::now())],
    )?;

    conn.execute(
        "INSERT INTO reserve_ledger (kind, amount_cents, balance_after_cents, label, reference, created_at)
         VALUES (?1,?2,?3,?4,?5,?6)",
        params![kind, delta_cents, new_balance, label, reference, ts_to_text(Utc::now())],
    )?;

    Ok(new_balance)
}

pub fn reserve_ledger(conn: &Connection, limit: i64) -> rusqlite::Result<Vec<serde_json::Value>> {
    let mut stmt = conn.prepare(
        "SELECT id, kind, amount_cents, balance_after_cents, label, reference, created_at
         FROM reserve_ledger ORDER BY id DESC LIMIT ?1",
    )?;
    let rows = stmt.query_map(params![limit.clamp(1, 200)], |row| {
        Ok(serde_json::json!({
            "id": row.get::<_, i64>(0)?,
            "kind": row.get::<_, String>(1)?,
            "amount_cents": row.get::<_, i64>(2)?,
            "balance_after_cents": row.get::<_, i64>(3)?,
            "label": row.get::<_, String>(4)?,
            "reference": row.get::<_, Option<String>>(5)?,
            "created_at": row.get::<_, String>(6)?,
        }))
    })?;
    rows.collect()
}
