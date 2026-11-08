//! AgentShield decision engine.
//!
//! Strict separation of responsibilities:
//! * [`guardrail`] and [`risk`] are deterministic and authoritative;
//! * [`pricing`] explains a price using fixed rules;
//! * [`expert`] may explain and qualify, but never determine an amount.

pub mod expert;
pub mod guardrail;
pub mod pricing;
pub mod risk;

/// History window injected into the engine to keep it a pure function.
#[derive(Debug, Clone, Default)]
pub struct HistoryWindow {
    /// Total amount of actions already approved or escalated under this mandate.
    pub committed_cents: i64,
    /// Number of actions proposed by this agent in the last 15 minutes.
    pub recent_actions_15min: i64,
    /// Merchants previously seen for this agent (lowercase).
    pub known_merchants: Vec<String>,
    /// An action with the same idempotency key already exists.
    pub duplicate_action: bool,
    /// Number of open incidents for this agent.
    pub open_incidents: i64,
}

impl HistoryWindow {
    pub fn merchant_is_known(&self, merchant: &str) -> bool {
        let needle = merchant.trim().to_ascii_lowercase();
        self.known_merchants.iter().any(|m| m == &needle)
    }
}
