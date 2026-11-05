use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::money::Currency;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum OrderPurpose {
    /// Paiement d'une action approuvée par le garde-fou.
    #[default]
    Purchase,
    /// Rechargement du wallet simulé.
    Topup,
}

impl OrderPurpose {
    pub const fn as_str(self) -> &'static str {
        match self {
            OrderPurpose::Purchase => "purchase",
            OrderPurpose::Topup => "topup",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "topup" => OrderPurpose::Topup,
            _ => OrderPurpose::Purchase,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum OrderStatus {
    #[default]
    Created,
    Approved,
    Captured,
    Failed,
}

impl OrderStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            OrderStatus::Created => "created",
            OrderStatus::Approved => "approved",
            OrderStatus::Captured => "captured",
            OrderStatus::Failed => "failed",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "approved" | "APPROVED" => OrderStatus::Approved,
            "captured" | "COMPLETED" => OrderStatus::Captured,
            "failed" => OrderStatus::Failed,
            _ => OrderStatus::Created,
        }
    }
}

/// Commande PayPal Sandbox créée par AgentShield. Une commande ne peut exister
/// que pour une action approuvée (ou une escalade confirmée).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxOrder {
    pub id: Uuid,
    pub action_id: Option<Uuid>,
    pub purpose: OrderPurpose,
    pub paypal_order_id: String,
    pub status: OrderStatus,
    pub amount_cents: i64,
    pub currency: Currency,
    pub description: String,
    pub approve_url: Option<String>,
    pub capture_id: Option<String>,
    /// `mock` ou `live-sandbox`.
    pub mode: String,
    pub failure_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub captured_at: Option<DateTime<Utc>>,
}
