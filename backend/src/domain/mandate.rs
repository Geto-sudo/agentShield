use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::money::Currency;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MandateStatus {
    #[default]
    Active,
    Revoked,
    Expired,
}

impl MandateStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            MandateStatus::Active => "active",
            MandateStatus::Revoked => "revoked",
            MandateStatus::Expired => "expired",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "revoked" => MandateStatus::Revoked,
            "expired" => MandateStatus::Expired,
            _ => MandateStatus::Active,
        }
    }
}

/// The mandate is the only source of authority. It is defined by the human
/// before the agent proposes any action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mandate {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub owner_name: String,
    /// Free-form description provided by the owner and displayed in the audit log.
    pub intent: String,
    /// Authorized purchase category (case-insensitive comparison).
    pub category: String,
    pub currency: Currency,
    /// Total spending limit over the mandate's lifetime.
    pub max_total_cents: i64,
    /// Maximum amount allowed per transaction.
    pub per_txn_max_cents: i64,
    /// At or below this amount, the deterministic engine approves the action.
    pub auto_approve_max_cents: i64,
    /// Above `auto_approve_max_cents` and up to this limit: escalation.
    /// Above this limit: rejection.
    pub escalate_until_cents: i64,
    /// Merchant allowlist. Empty means any merchant not on the blocklist.
    pub allowed_merchants: Vec<String>,
    pub blocked_merchants: Vec<String>,
    /// Country allowlist (`FR`, `EU`, etc.). Empty means no country restriction.
    pub allowed_countries: Vec<String>,
    /// Maximum accepted delivery time in days. `0` means no restriction.
    pub max_delivery_days: u32,
    pub valid_from: DateTime<Utc>,
    pub valid_until: DateTime<Utc>,
    pub status: MandateStatus,
    pub created_at: DateTime<Utc>,
}

impl Mandate {
    pub fn is_active_at(&self, now: DateTime<Utc>) -> bool {
        self.status == MandateStatus::Active && now >= self.valid_from && now <= self.valid_until
    }

    /// Returns the effective thresholds, capped by the mandate's limits.
    pub fn thresholds(&self) -> super::decision::Thresholds {
        super::decision::Thresholds {
            auto_approve_max_cents: self
                .auto_approve_max_cents
                .min(self.per_txn_max_cents)
                .min(self.escalate_until_cents),
            escalate_until_cents: self.escalate_until_cents.min(self.per_txn_max_cents),
        }
    }
}

/// Request body for `POST /api/mandates`.
/// All policy fields are optional and default to values derived from the total spending limit.
#[derive(Debug, Clone, Deserialize)]
pub struct CreateMandateRequest {
    pub agent_id: Uuid,
    pub owner_name: String,
    pub intent: String,
    pub category: String,
    pub max_total_cents: i64,
    #[serde(default)]
    pub currency: Option<Currency>,
    #[serde(default)]
    pub per_txn_max_cents: Option<i64>,
    #[serde(default)]
    pub auto_approve_max_cents: Option<i64>,
    #[serde(default)]
    pub escalate_until_cents: Option<i64>,
    #[serde(default)]
    pub allowed_merchants: Option<Vec<String>>,
    #[serde(default)]
    pub blocked_merchants: Option<Vec<String>>,
    #[serde(default)]
    pub allowed_countries: Option<Vec<String>>,
    #[serde(default)]
    pub max_delivery_days: Option<u32>,
    #[serde(default)]
    pub valid_days: Option<i64>,
}
