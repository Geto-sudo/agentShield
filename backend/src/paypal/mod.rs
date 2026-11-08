//! PayPal Sandbox integration.
//!
//! Two implementations of the same trait:
//! * [`live::SandboxGateway`] — real calls to `api-m.sandbox.paypal.com`;
//! * [`mock::MockGateway`] — simulated responses when no credentials are
//!   configured, allowing the demo to run offline.
//!
//! The selected mode is always included in the HTTP response.

pub mod live;
pub mod mock;

use async_trait::async_trait;

use crate::domain::money::Currency;

#[derive(Debug, Clone)]
pub struct CreateOrderRequest {
    pub amount_cents: i64,
    pub currency: Currency,
    pub description: String,
    /// Internal reference (action ID) sent to PayPal.
    pub reference_id: String,
    pub return_url: String,
    pub cancel_url: String,
}

#[derive(Debug, Clone)]
pub struct CreatedOrder {
    pub id: String,
    pub status: String,
    pub approve_url: Option<String>,
    pub raw: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct CapturedOrder {
    pub id: String,
    pub status: String,
    pub capture_id: Option<String>,
    pub amount_cents: Option<i64>,
    pub currency: Option<Currency>,
    pub raw: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct PayoutRequest {
    pub amount_cents: i64,
    pub currency: Currency,
    pub recipient_email: String,
    pub note: String,
    pub sender_batch_id: String,
}

#[derive(Debug, Clone)]
pub struct CreatedPayout {
    pub batch_id: String,
    pub status: String,
    pub raw: serde_json::Value,
}

#[derive(Debug, thiserror::Error)]
pub enum PayPalError {
    #[error("Incomplete PayPal configuration: {0}")]
    Config(String),

    #[error("PayPal transport error: {0}")]
    Transport(String),

    #[error("PayPal returned {status}: {message}")]
    Api { status: u16, message: String },

    #[error("Unexpected PayPal response: {0}")]
    Decode(String),
}

#[async_trait]
pub trait PayPalGateway: Send + Sync {
    /// Returns `mock` or `live-sandbox`.
    fn mode(&self) -> &'static str;

    /// Returns `true` when real network requests are made.
    fn is_live(&self) -> bool {
        self.mode() != "mock"
    }

    async fn create_order(&self, req: CreateOrderRequest) -> Result<CreatedOrder, PayPalError>;

    async fn capture_order(&self, order_id: &str) -> Result<CapturedOrder, PayPalError>;

    async fn create_payout(&self, req: PayoutRequest) -> Result<CreatedPayout, PayPalError>;
}
