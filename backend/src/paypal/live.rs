use std::time::{Duration, Instant};

use async_trait::async_trait;
use serde_json::{json, Value};

use crate::domain::money::{cents_to_paypal_amount, Currency};

use super::{
    CapturedOrder, CreateOrderRequest, CreatedOrder, CreatedPayout, PayPalError, PayPalGateway,
    PayoutRequest,
};

/// Client for the real PayPal Sandbox API (`api-m.sandbox.paypal.com`).
/// No real money is handled: only Business/Personal Sandbox accounts
/// created for the hackathon are used.
pub struct SandboxGateway {
    http: reqwest::Client,
    base_url: String,
    client_id: String,
    client_secret: String,
    token: std::sync::Mutex<Option<(String, Instant)>>,
}

impl SandboxGateway {
    pub fn new(
        base_url: String,
        client_id: String,
        client_secret: String,
    ) -> Result<Self, PayPalError> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(20))
            .build()
            .map_err(|e| PayPalError::Config(format!("HTTP client: {e}")))?;

        Ok(Self {
            http,
            base_url: base_url.trim_end_matches('/').to_string(),
            client_id,
            client_secret,
            token: std::sync::Mutex::new(None),
        })
    }

    fn cached_token(&self) -> Option<String> {
        let guard = self.token.lock().unwrap_or_else(|e| e.into_inner());

        guard.as_ref().and_then(|(token, obtained)| {
            // PayPal tokens last approximately 9 hours; refresh them earlier as a precaution.
            if obtained.elapsed() < Duration::from_secs(3_000) {
                Some(token.clone())
            } else {
                None
            }
        })
    }

    fn store_token(&self, token: String) {
        let mut guard = self.token.lock().unwrap_or_else(|e| e.into_inner());
        *guard = Some((token, Instant::now()));
    }

    async fn access_token(&self) -> Result<String, PayPalError> {
        if let Some(token) = self.cached_token() {
            return Ok(token);
        }

        let response = self
            .http
            .post(format!("{}/v1/oauth2/token", self.base_url))
            .basic_auth(&self.client_id, Some(&self.client_secret))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body("grant_type=client_credentials")
            .send()
            .await
            .map_err(|e| PayPalError::Transport(e.to_string()))?;

        let status = response.status();
        let text = response.text().await.unwrap_or_default();

        if !status.is_success() {
            return Err(PayPalError::Api {
                status: status.as_u16(),
                message: shorten(&text),
            });
        }

        let parsed: Value = serde_json::from_str(&text)
            .map_err(|e| PayPalError::Decode(format!("OAuth token: {e}")))?;

        let token = parsed
            .get("access_token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| PayPalError::Decode("OAuth token is missing".into()))?
            .to_string();

        self.store_token(token.clone());

        Ok(token)
    }

    async fn post(&self, path: &str, body: Value) -> Result<Value, PayPalError> {
        let token = self.access_token().await?;

        let response = self
            .http
            .post(format!("{}{path}", self.base_url))
            .bearer_auth(token)
            .header("Content-Type", "application/json")
            .header("Prefer", "return=representation")
            .json(&body)
            .send()
            .await
            .map_err(|e| PayPalError::Transport(e.to_string()))?;

        let status = response.status();
        let text = response.text().await.unwrap_or_default();

        if !status.is_success() {
            return Err(PayPalError::Api {
                status: status.as_u16(),
                message: shorten(&text),
            });
        }

        serde_json::from_str(&text)
            .map_err(|e| PayPalError::Decode(format!("{e}: {}", shorten(&text))))
    }
}

#[async_trait]
impl PayPalGateway for SandboxGateway {
    fn mode(&self) -> &'static str {
        "live-sandbox"
    }

    async fn create_order(&self, req: CreateOrderRequest) -> Result<CreatedOrder, PayPalError> {
        let body = json!({
            "intent": "CAPTURE",
            "purchase_units": [{
                "reference_id": req.reference_id,
                "description": truncate(&req.description, 127),
                "amount": {
                    "currency_code": req.currency.paypal_code(),
                    "value": cents_to_paypal_amount(req.amount_cents),
                }
            }],
            "application_context": {
                "brand_name": "AgentShield Sandbox",
                "user_action": "PAY_NOW",
                "shipping_preference": "NO_SHIPPING",
                "return_url": req.return_url,
                "cancel_url": req.cancel_url,
            }
        });

        let value = self.post("/v2/checkout/orders", body).await?;

        let approve_url = value
            .get("links")
            .and_then(|l| l.as_array())
            .and_then(|links| {
                links.iter().find_map(|link| {
                    let rel = link.get("rel").and_then(|r| r.as_str()).unwrap_or_default();

                    if rel == "approve" || rel == "payer-action" {
                        link.get("href")
                            .and_then(|h| h.as_str())
                            .map(str::to_string)
                    } else {
                        None
                    }
                })
            });

        Ok(CreatedOrder {
            id: value
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            status: value
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("CREATED")
                .to_string(),
            approve_url,
            raw: value,
        })
    }

    async fn capture_order(&self, order_id: &str) -> Result<CapturedOrder, PayPalError> {
        let value = self
            .post(
                &format!("/v2/checkout/orders/{order_id}/capture"),
                json!({}),
            )
            .await?;

        let capture = value
            .get("purchase_units")
            .and_then(|p| p.as_array())
            .and_then(|units| units.first())
            .and_then(|unit| unit.get("payments"))
            .and_then(|p| p.get("captures"))
            .and_then(|c| c.as_array())
            .and_then(|captures| captures.first());

        let capture_id = capture
            .and_then(|c| c.get("id"))
            .and_then(|v| v.as_str())
            .map(str::to_string);

        let amount_cents = capture
            .and_then(|c| c.get("amount"))
            .and_then(|a| a.get("value"))
            .and_then(|v| v.as_str())
            .and_then(crate::domain::money::paypal_amount_to_cents);

        let currency = capture
            .and_then(|c| c.get("amount"))
            .and_then(|a| a.get("currency_code"))
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<Currency>().ok());

        Ok(CapturedOrder {
            id: value
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or(order_id)
                .to_string(),
            status: value
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("COMPLETED")
                .to_string(),
            capture_id,
            amount_cents,
            currency,
            raw: value,
        })
    }

    async fn create_payout(&self, req: PayoutRequest) -> Result<CreatedPayout, PayPalError> {
        let body = json!({
            "sender_batch_header": {
                "sender_batch_id": req.sender_batch_id,
                "email_subject": "AgentShield Sandbox - dispute resolution",
                "email_message": "Demonstration payment issued by AgentShield (Sandbox prototype). No real money is involved.",
            },
            "items": [{
                "recipient_type": "EMAIL",
                "receiver": req.recipient_email,
                "note": truncate(&req.note, 255),
                "sender_item_id": req.sender_batch_id,
                "amount": {
                    "currency": req.currency.paypal_code(),
                    "value": cents_to_paypal_amount(req.amount_cents),
                }
            }]
        });

        let value = self.post("/v1/payments/payouts", body).await?;
        let header = value.get("batch_header").cloned().unwrap_or(Value::Null);

        Ok(CreatedPayout {
            batch_id: header
                .get("payout_batch_id")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            status: header
                .get("batch_status")
                .and_then(|v| v.as_str())
                .unwrap_or("PENDING")
                .to_string(),
            raw: value,
        })
    }
}

fn shorten(text: &str) -> String {
    let flat = text.replace('\n', " ");

    if flat.chars().count() <= 400 {
        flat
    } else {
        flat.chars().take(400).collect::<String>() + "…"
    }
}

fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        text.to_string()
    } else {
        text.chars().take(max.saturating_sub(1)).collect::<String>() + "…"
    }
}
