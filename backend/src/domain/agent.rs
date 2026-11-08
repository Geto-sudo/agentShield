use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    #[default]
    Active,
    Paused,
}

impl AgentStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            AgentStatus::Active => "active",
            AgentStatus::Paused => "paused",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "paused" => AgentStatus::Paused,
            _ => AgentStatus::Active,
        }
    }
}

/// A simulated AI agent assigned to a human owner.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: Uuid,
    pub name: String,
    pub owner: String,
    pub purpose: String,
    pub status: AgentStatus,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateAgentRequest {
    pub name: String,
    #[serde(default)]
    pub owner: Option<String>,
    #[serde(default)]
    pub purpose: Option<String>,
}
