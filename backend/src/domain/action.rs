use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::decision::{Decision, RiskBand, RuleCheck};
use super::money::Currency;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ActionKind {
    #[default]
    Purchase,
    Subscription,
    Payout,
    Refund,
}

impl ActionKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            ActionKind::Purchase => "purchase",
            ActionKind::Subscription => "subscription",
            ActionKind::Payout => "payout",
            ActionKind::Refund => "refund",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "subscription" => ActionKind::Subscription,
            "payout" => ActionKind::Payout,
            "refund" => ActionKind::Refund,
            _ => ActionKind::Purchase,
        }
    }

    /// Seul un achat (ou un abonnement) peut consommer un mandat de dépense.
    pub const fn is_spend(self) -> bool {
        matches!(self, ActionKind::Purchase | ActionKind::Subscription)
    }
}

/// Le « log JSON » qu'un agent envoie quand il veut dépenser.
/// C'est la seule entrée du garde-fou.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposedAction {
    pub mandate_id: Uuid,
    #[serde(default)]
    pub agent_id: Option<Uuid>,
    #[serde(default)]
    pub kind: ActionKind,
    pub merchant: String,
    #[serde(default)]
    pub merchant_country: Option<String>,
    pub amount_cents: i64,
    #[serde(default)]
    pub currency: Option<Currency>,
    pub category: String,
    #[serde(default)]
    pub delivery_days: Option<u32>,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub idempotency_key: Option<String>,
    #[serde(default)]
    pub occurred_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

impl ProposedAction {
    pub fn currency_or_default(&self) -> Currency {
        self.currency.unwrap_or_default()
    }
}

/// L'action telle qu'elle est persistée, avec la preuve de décision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionRecord {
    pub id: Uuid,
    pub mandate_id: Uuid,
    pub agent_id: Uuid,
    pub kind: ActionKind,
    pub merchant: String,
    pub merchant_country: Option<String>,
    pub amount_cents: i64,
    pub currency: Currency,
    pub category: String,
    pub delivery_days: Option<u32>,
    pub description: String,
    pub idempotency_key: Option<String>,
    pub raw_log: serde_json::Value,
    pub decision: Decision,
    pub risk_score: u8,
    pub risk_band: RiskBand,
    pub checks: Vec<RuleCheck>,
    pub reasons: Vec<String>,
    pub explanation: String,
    pub approval_id: Option<Uuid>,
    pub order_id: Option<Uuid>,
    pub settled: bool,
    pub audit_hash: String,
    pub created_at: DateTime<Utc>,
}
