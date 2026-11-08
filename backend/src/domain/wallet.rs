use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::money::Currency;

/// Simulated agent wallet. No real balance is handled.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wallet {
    pub agent_id: Uuid,
    pub balance_cents: i64,
    /// Simulated monthly premium (the cost of coverage).
    pub premium_cents: i64,
    /// Bonus-malus in basis points: `-1500` = -15%, `+2000` = +20%.
    pub bonus_malus_bps: i32,
    /// Portion of the premium allocated to the local shared reserve.
    pub reserve_contribution_cents: i64,
    /// Approved and settled expenses during the current period.
    pub spent_this_period_cents: i64,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerEntry {
    pub id: i64,
    pub agent_id: Uuid,
    /// `credit`, `debit`, `payout`, or `refund`.
    pub kind: String,
    pub amount_cents: i64,
    pub balance_after_cents: i64,
    pub label: String,
    pub reference: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceLine {
    pub code: String,
    pub label: String,
    pub amount_cents: i64,
    pub explanation: String,
}

/// Line-by-line breakdown of the displayed price. Nothing is hidden.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceQuote {
    pub currency: Currency,
    pub base_premium_cents: i64,
    pub risk_loading_cents: i64,
    pub bonus_malus_cents: i64,
    pub reserve_contribution_cents: i64,
    pub total_cents: i64,
    pub lines: Vec<PriceLine>,
    pub headline: String,
}
