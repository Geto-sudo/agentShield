use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::money::Currency;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IncidentKind {
    /// Une dépense a contourné le garde-fou (« action rogue »).
    RogueAction,
    DuplicateCharge,
    RefundRequest,
}

impl IncidentKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            IncidentKind::RogueAction => "rogue_action",
            IncidentKind::DuplicateCharge => "duplicate_charge",
            IncidentKind::RefundRequest => "refund_request",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "duplicate_charge" => IncidentKind::DuplicateCharge,
            "refund_request" => IncidentKind::RefundRequest,
            _ => IncidentKind::RogueAction,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IncidentStatus {
    Open,
    UnderReview,
    Resolved,
    Rejected,
}

impl IncidentStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            IncidentStatus::Open => "open",
            IncidentStatus::UnderReview => "under_review",
            IncidentStatus::Resolved => "resolved",
            IncidentStatus::Rejected => "rejected",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "under_review" => IncidentStatus::UnderReview,
            "resolved" => IncidentStatus::Resolved,
            "rejected" => IncidentStatus::Rejected,
            _ => IncidentStatus::Open,
        }
    }
}

/// Un sinistre simulé : la trace d'une dépense qui n'aurait pas dû passer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Incident {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub mandate_id: Uuid,
    pub action_id: Option<Uuid>,
    pub kind: IncidentKind,
    pub status: IncidentStatus,
    pub description: String,
    pub amount_cents: i64,
    pub currency: Currency,
    /// Qui a constaté le sinistre (`system`, `owner`, `expert`).
    pub overseen_by: String,
    pub verdict: Option<serde_json::Value>,
    pub payout_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PayoutStatus {
    Created,
    Pending,
    Success,
    Failed,
}

impl PayoutStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            PayoutStatus::Created => "created",
            PayoutStatus::Pending => "pending",
            PayoutStatus::Success => "success",
            PayoutStatus::Failed => "failed",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "pending" => PayoutStatus::Pending,
            "success" | "completed" => PayoutStatus::Success,
            "failed" | "denied" => PayoutStatus::Failed,
            _ => PayoutStatus::Created,
        }
    }
}

/// Payout PayPal Sandbox : l'indemnisation simulée de la réserve commune.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payout {
    pub id: Uuid,
    pub incident_id: Option<Uuid>,
    pub paypal_batch_id: String,
    pub amount_cents: i64,
    pub currency: Currency,
    pub status: PayoutStatus,
    /// `mock` ou `live-sandbox`.
    pub mode: String,
    pub recipient_email: String,
    pub note: String,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Verdict structuré de l'expert. Le LLM ne fixe jamais un montant : il
/// qualifie l'écart entre le mandat et le log, et propose une résolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpertVerdict {
    /// `UPHOLD` (le refus tient), `FLAG` (violation confirmée, à indemniser),
    /// `INCONCLUSIVE` (preuves insuffisantes).
    pub verdict: String,
    /// Entre 0.0 et 1.0.
    pub confidence: f64,
    pub summary_fr: String,
    pub cited_rules: Vec<String>,
    pub recommended_resolution: String,
    /// `openai:<modèle>` ou `deterministic-fallback`.
    pub source: String,
    pub latency_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryOutcome {
    pub incident: Incident,
    pub verdict: ExpertVerdict,
    pub payout: Option<Payout>,
    pub reserve_balance_cents: i64,
    pub steps: Vec<String>,
}
