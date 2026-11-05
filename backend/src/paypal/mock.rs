use async_trait::async_trait;
use serde_json::json;

use super::{
    CapturedOrder, CreateOrderRequest, CreatedOrder, CreatedPayout, PayPalError, PayPalGateway,
    PayoutRequest,
};

/// Passerelle simulée : aucune requête réseau, des identifiants réalistes
/// (`MOCK-…`) pour que l'interface et les tests restent lisibles.
#[derive(Debug, Default, Clone)]
pub struct MockGateway;

impl MockGateway {
    pub fn new() -> Self {
        Self
    }

    fn short_id(prefix: &str) -> String {
        let raw = uuid::Uuid::new_v4().simple().to_string().to_uppercase();
        format!("{prefix}{}", &raw[..17])
    }
}

#[async_trait]
impl PayPalGateway for MockGateway {
    fn mode(&self) -> &'static str {
        "mock"
    }

    fn is_live(&self) -> bool {
        false
    }

    async fn create_order(&self, req: CreateOrderRequest) -> Result<CreatedOrder, PayPalError> {
        if req.amount_cents <= 0 {
            return Err(PayPalError::Api {
                status: 422,
                message: "Le montant d'une commande doit être positif.".into(),
            });
        }
        let id = Self::short_id("MOCK-ORDER-");
        Ok(CreatedOrder {
            approve_url: Some(format!(
                "https://www.sandbox.paypal.com/checkoutnow?token={id}"
            )),
            status: "CREATED".to_string(),
            raw: json!({
                "id": id,
                "status": "CREATED",
                "intent": "CAPTURE",
                "simulated": true,
                "purchase_units": [{
                    "reference_id": req.reference_id,
                    "description": req.description,
                    "amount": {
                        "currency_code": req.currency.as_str(),
                        "value": crate::domain::money::cents_to_paypal_amount(req.amount_cents),
                    }
                }]
            }),
            id,
        })
    }

    async fn capture_order(&self, order_id: &str) -> Result<CapturedOrder, PayPalError> {
        if !order_id.starts_with("MOCK-ORDER-") {
            return Err(PayPalError::Api {
                status: 404,
                message: format!("Commande inconnue : {order_id}"),
            });
        }
        let capture_id = Self::short_id("MOCK-CAPTURE-");
        Ok(CapturedOrder {
            id: order_id.to_string(),
            status: "COMPLETED".to_string(),
            capture_id: Some(capture_id.clone()),
            amount_cents: None,
            currency: None,
            raw: json!({
                "id": order_id,
                "status": "COMPLETED",
                "simulated": true,
                "purchase_units": [{
                    "payments": {
                        "captures": [{ "id": capture_id, "status": "COMPLETED" }]
                    }
                }]
            }),
        })
    }

    async fn create_payout(&self, req: PayoutRequest) -> Result<CreatedPayout, PayPalError> {
        if req.amount_cents <= 0 {
            return Err(PayPalError::Api {
                status: 422,
                message: "Le montant d'un payout doit être positif.".into(),
            });
        }
        let batch_id = Self::short_id("MOCK-PAYOUT-");
        Ok(CreatedPayout {
            status: "PENDING".to_string(),
            raw: json!({
                "batch_header": {
                    "payout_batch_id": batch_id,
                    "batch_status": "PENDING",
                    "sender_batch_id": req.sender_batch_id,
                    "simulated": true,
                },
                "items": [{ "receiver": req.recipient_email, "note": req.note }]
            }),
            batch_id,
        })
    }
}
