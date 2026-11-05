//! Types métier d'AgentShield.
//!
//! Règle de base du prototype : **l'argent est toujours un entier de centimes**.
//! Aucun `f64` ne participe à une décision financière.

pub mod action;
pub mod agent;
pub mod approval;
pub mod audit;
pub mod decision;
pub mod mandate;
pub mod money;
pub mod order;
pub mod recovery;
pub mod wallet;

pub use action::{ActionKind, ActionRecord, ProposedAction};
pub use agent::{Agent, AgentStatus};
pub use approval::{Approval, ApprovalStatus, DecideApprovalRequest};
pub use audit::{AuditEntry, ChainStatus};
pub use decision::{
    CheckSeverity, Decision, GuardrailVerdict, RiskAssessment, RiskBand, RiskFactor, RuleCheck,
    Thresholds,
};
pub use mandate::{CreateMandateRequest, Mandate, MandateStatus};
pub use money::{format_cents, parse_currency, Currency};
pub use order::{OrderPurpose, OrderStatus, SandboxOrder};
pub use recovery::{
    ExpertVerdict, Incident, IncidentKind, IncidentStatus, Payout, PayoutStatus, RecoveryOutcome,
};
pub use wallet::{LedgerEntry, PriceLine, PriceQuote, Wallet};
