use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalStatus {
    #[default]
    Pending,
    Confirmed,
    Denied,
    Expired,
}

impl ApprovalStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            ApprovalStatus::Pending => "pending",
            ApprovalStatus::Confirmed => "confirmed",
            ApprovalStatus::Denied => "denied",
            ApprovalStatus::Expired => "expired",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "confirmed" => ApprovalStatus::Confirmed,
            "denied" => ApprovalStatus::Denied,
            "expired" => ApprovalStatus::Expired,
            _ => ApprovalStatus::Pending,
        }
    }
}

/// Une escalade en attente de décision humaine. Elle expire : une action non
/// confirmée ne devient jamais approuvée par défaut.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Approval {
    pub id: Uuid,
    pub action_id: Uuid,
    pub status: ApprovalStatus,
    pub requested_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub decided_at: Option<DateTime<Utc>>,
    pub decided_by: Option<String>,
    pub note: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl Approval {
    pub fn is_usable(&self, now: DateTime<Utc>) -> bool {
        self.status == ApprovalStatus::Confirmed && now <= self.expires_at
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct DecideApprovalRequest {
    #[serde(default)]
    pub decided_by: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
}
