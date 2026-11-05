use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::money::Currency;

/// Portefeuille simulé d'un agent. Aucun solde réel n'est manipulé.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wallet {
    pub agent_id: Uuid,
    pub balance_cents: i64,
    /// Prime mensuelle simulée (ce que coûterait la couverture).
    pub premium_cents: i64,
    /// Bonus-malus en points de base : `-1500` = -15 %, `+2000` = +20 %.
    pub bonus_malus_bps: i32,
    /// Part de la prime versée à la réserve commune locale.
    pub reserve_contribution_cents: i64,
    /// Dépenses approuvées et réglées sur la période courante.
    pub spent_this_period_cents: i64,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerEntry {
    pub id: i64,
    pub agent_id: Uuid,
    /// `credit`, `debit`, `payout`, `refund`.
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

/// Décomposition ligne par ligne du prix affiché : rien n'est caché.
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
