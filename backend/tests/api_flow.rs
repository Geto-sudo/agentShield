//! HTTP integration tests covering the complete flow:
//! mandate -> action -> decision -> PayPal checkout -> capture.
//! All tests run in memory, without network access or real money.

use agentshield::config::{Config, OpenAiConfig, PayPalConfig};
use agentshield::state::AppState;
use agentshield::{api, build_state};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use serde_json::{json, Value};
use tower::ServiceExt;

const BIG_BUFFER: usize = 2 * 1024 * 1024;

fn test_state() -> AppState {
    let config = Config {
        addr: "127.0.0.1:0".to_string(),
        db_path: ":memory:".to_string(),
        seed_demo: false,
        default_auto_approve_max_cents: 4_000,
        approval_ttl_minutes: 30,
        public_base_url: "http://localhost:8080".to_string(),
        owner_payout_email: "sandbox-owner@example.com".to_string(),
        openai: OpenAiConfig {
            api_key: None,
            model: "gpt-4o-mini".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            timeout_secs: 5,
        },
        paypal: PayPalConfig {
            client_id: None,
            client_secret: None,
            base_url: "https://api-m.sandbox.paypal.com".to_string(),
            force_mock: true,
        },
    };

    build_state(config).expect("test application state")
}

struct Harness {
    app: Router,
}

impl Harness {
    fn new() -> Self {
        Self {
            app: api::router(test_state()),
        }
    }

    async fn get(&self, uri: &str) -> (StatusCode, Value) {
        let request = Request::builder()
            .method("GET")
            .uri(uri)
            .body(Body::empty())
            .expect("GET request");

        self.send(request).await
    }

    async fn post(&self, uri: &str, body: Value) -> (StatusCode, Value) {
        let request = Request::builder()
            .method("POST")
            .uri(uri)
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .expect("POST request");

        self.send(request).await
    }

    async fn send(&self, request: Request<Body>) -> (StatusCode, Value) {
        let response = self
            .app
            .clone()
            .oneshot(request)
            .await
            .expect("router response");

        let status = response.status();

        let bytes = axum::body::to_bytes(response.into_body(), BIG_BUFFER)
            .await
            .expect("response body");

        let value: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);

        (status, value)
    }

    /// Creates a demo agent and mandate and returns their identifiers.
    async fn setup(&self) -> (String, String) {
        let (status, agent) = self
            .post(
                "/api/agents",
                json!({
                    "name": "Atlas Shopper",
                    "owner": "Camille",
                    "purpose": "shopping"
                }),
            )
            .await;

        assert_eq!(status, StatusCode::OK, "agent creation: {agent}");

        let agent_id = agent["agent"]["id"].as_str().expect("agent ID").to_string();

        let (status, mandate) = self
            .post(
                "/api/mandates",
                json!({
                    "agent_id": agent_id,
                    "owner_name": "Camille",
                    "intent": "New headphones, maximum 50 EUR, delivery within 5 days.",
                    "category": "electronics",
                    "max_total_cents": 20_000,
                    "per_txn_max_cents": 5_000,
                    "auto_approve_max_cents": 4_000,
                    "escalate_until_cents": 5_000,
                    "allowed_countries": ["FR"],
                    "max_delivery_days": 5,
                    "valid_days": 30
                }),
            )
            .await;

        assert_eq!(status, StatusCode::OK, "mandate creation: {mandate}");

        let mandate_id = mandate["mandate"]["id"]
            .as_str()
            .expect("mandate ID")
            .to_string();

        (agent_id, mandate_id)
    }

    async fn propose(
        &self,
        mandate_id: &str,
        amount_cents: i64,
        country: &str,
    ) -> (StatusCode, Value) {
        self.post(
            "/api/actions",
            json!({
                "mandate_id": mandate_id,
                "merchant": "TechStore SAS",
                "merchant_country": country,
                "amount_cents": amount_cents,
                "currency": "EUR",
                "category": "electronics",
                "delivery_days": 4,
                "description": "Wireless headphones",
                "kind": "purchase"
            }),
        )
        .await
    }
}

#[tokio::test]
async fn approves_and_settles_a_small_purchase() {
    let h = Harness::new();
    let (agent_id, mandate_id) = h.setup().await;

    // Read the initial wallet balance before the purchase.
    let (status, wallet_before) = h.get(&format!("/api/wallet/{agent_id}")).await;

    assert_eq!(status, StatusCode::OK, "{wallet_before}");

    let initial_balance = wallet_before["wallet"]["balance_cents"]
        .as_i64()
        .expect("initial wallet balance");

    // Propose a purchase below the automatic approval threshold.
    let (status, body) = h.propose(&mandate_id, 3_800, "FR").await;

    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["decision"], "APPROVE");

    let action_id = body["action"]["id"]
        .as_str()
        .expect("action ID")
        .to_string();

    let audit_hash = body["audit"]["hash"]
        .as_str()
        .expect("audit hash")
        .to_string();

    assert_eq!(
        audit_hash.len(),
        64,
        "audit hash must contain 64 characters"
    );

    // Create a PayPal checkout order in mock mode.
    let (status, order) = h
        .post(&format!("/api/actions/{action_id}/paypal/order"), json!({}))
        .await;

    assert_eq!(status, StatusCode::OK, "{order}");
    assert_eq!(order["order"]["status"], "created");
    assert_eq!(order["order"]["mode"], "mock");

    let paypal_order = order["order"]["paypal_order_id"]
        .as_str()
        .expect("PayPal order ID")
        .to_string();

    // Capture the mock PayPal order.
    let (status, capture) = h
        .post(
            &format!("/api/paypal/orders/{paypal_order}/capture"),
            json!({}),
        )
        .await;

    assert_eq!(status, StatusCode::OK, "{capture}");
    assert_eq!(capture["order"]["status"], "captured");

    // Verify that the wallet was debited.
    let (status, wallet_after) = h.get(&format!("/api/wallet/{agent_id}")).await;

    assert_eq!(status, StatusCode::OK, "{wallet_after}");

    let final_balance = wallet_after["wallet"]["balance_cents"]
        .as_i64()
        .expect("final wallet balance");

    assert_eq!(
        final_balance,
        initial_balance - 3_800,
        "wallet should be debited after capture"
    );
}

#[tokio::test]
async fn escalation_requires_explicit_human_confirmation() {
    let h = Harness::new();
    let (_, mandate_id) = h.setup().await;

    let (status, body) = h.propose(&mandate_id, 4_500, "FR").await;

    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["decision"], "ESCALATE");

    let action_id = body["action"]["id"]
        .as_str()
        .expect("action ID")
        .to_string();

    let approval_id = body["approval"]["id"]
        .as_str()
        .expect("approval ID")
        .to_string();

    assert_eq!(body["approval"]["status"], "pending");

    // Checkout must be forbidden before human confirmation.
    let (status, _) = h
        .post(&format!("/api/actions/{action_id}/paypal/order"), json!({}))
        .await;

    assert_eq!(status, StatusCode::FORBIDDEN);

    // The owner explicitly confirms the expense.
    let (status, confirmed) = h
        .post(
            &format!("/api/approvals/{approval_id}/confirm"),
            json!({ "decided_by": "Camille" }),
        )
        .await;

    assert_eq!(status, StatusCode::OK, "{confirmed}");
    assert_eq!(confirmed["approval"]["status"], "confirmed");

    // A confirmed approval must remain committed to the mandate budget.
    let (status, mandate) = h.get(&format!("/api/mandates/{mandate_id}")).await;

    assert_eq!(status, StatusCode::OK, "{mandate}");
    assert_eq!(mandate["committed_cents"], 4_500);
    assert_eq!(mandate["remaining_cents"], 15_500);

    // Checkout is allowed only after confirmation.
    let (status, order) = h
        .post(&format!("/api/actions/{action_id}/paypal/order"), json!({}))
        .await;

    assert_eq!(status, StatusCode::OK, "{order}");
    assert_eq!(order["order"]["amount_cents"], 4_500);
}

#[tokio::test]
async fn denied_approval_releases_committed_budget() {
    let h = Harness::new();
    let (_, mandate_id) = h.setup().await;

    // Propose an expense requiring human approval.
    let (status, body) = h.propose(&mandate_id, 4_500, "FR").await;

    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["decision"], "ESCALATE");

    let approval_id = body["approval"]["id"]
        .as_str()
        .expect("approval ID")
        .to_string();

    // The budget is reserved while approval is pending.
    let (status, mandate) = h.get(&format!("/api/mandates/{mandate_id}")).await;

    assert_eq!(status, StatusCode::OK, "{mandate}");
    assert_eq!(mandate["committed_cents"], 4_500);

    // The owner denies the expense.
    let (status, denied) = h
        .post(
            &format!("/api/approvals/{approval_id}/deny"),
            json!({ "decided_by": "Camille" }),
        )
        .await;

    assert_eq!(status, StatusCode::OK, "{denied}");
    assert_eq!(denied["approval"]["status"], "denied");

    // The committed budget must be released.
    let (status, mandate) = h.get(&format!("/api/mandates/{mandate_id}")).await;

    assert_eq!(status, StatusCode::OK, "{mandate}");
    assert_eq!(mandate["committed_cents"], 0);
    assert_eq!(mandate["remaining_cents"], 20_000);
}

#[tokio::test]
async fn refusal_never_produces_a_checkout() {
    let h = Harness::new();
    let (_, mandate_id) = h.setup().await;

    // Propose an expense exceeding the per-transaction limit.
    let (status, body) = h.propose(&mandate_id, 6_200, "FR").await;

    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["decision"], "REFUSE");

    // Verify that the response contains at least one refusal reason.
    let reasons = body["reasons"]
        .as_array()
        .expect("reasons should be an array");

    assert!(
        !reasons.is_empty(),
        "a refused action should contain at least one reason"
    );

    let action_id = body["action"]["id"].as_str().expect("action ID");

    // A refused action must never produce a PayPal checkout.
    let (status, denied) = h
        .post(&format!("/api/actions/{action_id}/paypal/order"), json!({}))
        .await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(denied["error"]["code"], "guardrail_denied");
}

#[tokio::test]
async fn refuses_unknown_country_and_keeps_the_reason() {
    let h = Harness::new();
    let (_, mandate_id) = h.setup().await;

    let (status, body) = h.propose(&mandate_id, 3_000, "BR").await;

    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["decision"], "REFUSE");

    let codes: Vec<String> = body["checks"]
        .as_array()
        .expect("checks should be an array")
        .iter()
        .filter(|check| check["passed"] == false)
        .map(|check| check["code"].as_str().expect("rule code").to_string())
        .collect();

    assert!(
        codes.contains(&"COUNTRY_ALLOWED".to_string()),
        "expected COUNTRY_ALLOWED to fail, got: {codes:?}"
    );
}

#[tokio::test]
async fn audit_chain_stays_valid_across_the_whole_flow() {
    let h = Harness::new();
    let (_, mandate_id) = h.setup().await;

    h.propose(&mandate_id, 3_800, "FR").await;
    h.propose(&mandate_id, 4_500, "FR").await;
    h.propose(&mandate_id, 9_000, "FR").await;

    let (status, body) = h.get("/api/audit/verify").await;

    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["chain"]["valid"], true);

    let entries = body["chain"]["entries"]
        .as_i64()
        .expect("audit entry count");

    assert!(
        entries >= 5,
        "expected at least 5 audit entries, got {entries}"
    );
}

#[tokio::test]
async fn recovery_pays_the_overage_from_the_common_reserve() {
    let h = Harness::new();
    let (agent_id, _) = h.setup().await;

    let (status, before) = h.get("/api/wallet/reserve").await;

    assert_eq!(status, StatusCode::OK, "{before}");

    let reserve_before = before["balance_cents"]
        .as_i64()
        .expect("initial reserve balance");

    let (status, body) = h
        .post("/api/recovery/simulate", json!({ "agent_id": agent_id }))
        .await;

    assert_eq!(status, StatusCode::OK, "{body}");

    let verdict = &body["outcome"]["verdict"];

    assert_eq!(verdict["verdict"], "FLAG");
    assert_eq!(verdict["recommended_resolution"], "PAYOUT_OVERAGE");

    let payout = &body["outcome"]["payout"];

    assert!(payout["paypal_batch_id"]
        .as_str()
        .expect("PayPal batch ID")
        .starts_with("MOCK-PAYOUT-"));

    let paid = payout["amount_cents"].as_i64().expect("payout amount");

    assert!(paid > 0, "payout amount must be positive");

    let reserve_after = body["outcome"]["reserve_balance_cents"]
        .as_i64()
        .expect("final reserve balance");

    assert_eq!(
        reserve_after,
        reserve_before - paid,
        "the common reserve must fund the payout"
    );
}

#[tokio::test]
async fn health_reports_sandbox_modes() {
    let h = Harness::new();

    let (status, body) = h.get("/api/health").await;

    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["modes"]["paypal"], "mock");
    assert_eq!(body["modes"]["expert"], "deterministic-fallback");
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn unknown_mandate_is_a_404() {
    let h = Harness::new();

    let (status, body) = h
        .post(
            "/api/actions",
            json!({
                "mandate_id": "00000000-0000-0000-0000-000000000000",
                "merchant": "X",
                "amount_cents": 100,
                "currency": "EUR",
                "category": "electronics"
            }),
        )
        .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"]["code"], "not_found");
}
