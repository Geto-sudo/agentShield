//! Couche HTTP : uniquement de la traduction requête <-> service.
//!
//! Ce service est **API-only** : aucune interface n'est servie ici. Le front est
//! développé séparément et consomme les routes `/api/*` (CORS ouvert).

pub mod actions;
pub mod agents;
pub mod approvals;
pub mod audit;
pub mod mandates;
pub mod paypal;
pub mod recovery;
pub mod wallet;

use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::json;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::error::{ApiError, ApiResult};
use crate::service;
use crate::state::AppState;

/// Routeur complet de l'API JSON.
pub fn router(state: AppState) -> Router {
    let api = Router::new()
        .route("/health", get(health))
        .route("/dashboard", get(dashboard))
        // Agents
        .route("/agents", get(agents::list).post(agents::create))
        // Mandats
        .route("/mandates", get(mandates::list).post(mandates::create))
        .route("/mandates/{id}", get(mandates::get_one))
        // Actions
        .route("/actions", get(actions::list).post(actions::propose))
        .route("/actions/{id}", get(actions::get_one))
        .route(
            "/actions/{id}/paypal/order",
            post(actions::create_paypal_order),
        )
        // Approbations
        .route("/approvals", get(approvals::list))
        .route("/approvals/{id}/confirm", post(approvals::confirm))
        .route("/approvals/{id}/deny", post(approvals::deny))
        // Wallet
        .route("/wallet/reserve", get(wallet::reserve))
        .route("/wallet/{agent_id}", get(wallet::get_wallet))
        .route("/wallet/{agent_id}/ledger", get(wallet::ledger))
        .route("/wallet/{agent_id}/quote", get(wallet::quote))
        .route("/wallet/{agent_id}/topup", post(wallet::topup))
        // PayPal Sandbox
        .route("/paypal/orders", get(paypal::list_orders))
        .route("/paypal/orders/{order_id}/capture", post(paypal::capture))
        // Audit
        .route("/audit", get(audit::list))
        .route("/audit/verify", get(audit::verify).post(audit::verify))
        // Récupération / sinistres
        .route("/incidents", get(recovery::list_incidents))
        .route("/payouts", get(recovery::list_payouts))
        .route("/recovery/simulate", post(recovery::simulate));

    Router::new()
        .nest("/api", api)
        .route("/", get(root))
        .fallback(not_found)
        .layer(TraceLayer::new_for_http())
        // CORS ouvert : le front tourne sur un autre port (Vite, Next, etc.).
        .layer(CorsLayer::permissive())
        .with_state(state)
}

/// Point d'entrée humain : décrit le service et les routes disponibles.
async fn root(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(json!({
        "service": "agentshield-backend",
        "version": env!("CARGO_PKG_VERSION"),
        "kind": "API JSON seule — le front est externe",
        "base_path": "/api",
        "cors": "permissif (développement)",
        "modes": {
            "paypal": state.config.paypal_mode(),
            "expert": state.config.expert_mode(),
        },
        "endpoints": [
            "GET  /api/health",
            "GET  /api/dashboard",
            "GET  /api/agents · POST /api/agents",
            "GET  /api/mandates · POST /api/mandates · GET /api/mandates/{id}",
            "POST /api/actions · GET /api/actions · GET /api/actions/{id}",
            "POST /api/actions/{id}/paypal/order",
            "GET  /api/approvals · POST /api/approvals/{id}/confirm · POST /api/approvals/{id}/deny",
            "GET  /api/wallet/{agent_id} · /ledger · /quote · POST /api/wallet/{agent_id}/topup",
            "GET  /api/wallet/reserve",
            "GET  /api/paypal/orders · POST /api/paypal/orders/{order_id}/capture",
            "GET  /api/audit · GET|POST /api/audit/verify",
            "GET  /api/incidents · GET /api/payouts · POST /api/recovery/simulate"
        ],
        "notice": "Prototype Sandbox — données simulées, aucun argent réel.",
    }))
}

async fn not_found() -> ApiError {
    ApiError::NotFound("route inconnue (voir GET / pour la liste)".to_string())
}

async fn health(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "service": "agentshield-backend",
        "version": env!("CARGO_PKG_VERSION"),
        "time": chrono::Utc::now(),
        "modes": {
            "paypal": state.config.paypal_mode(),
            "expert": state.config.expert_mode(),
        },
        "notice": "Prototype Sandbox — données simulées, aucun argent réel.",
    }))
}

async fn dashboard(State(state): State<AppState>) -> ApiResult<Json<serde_json::Value>> {
    let value = service::dashboard(&state.db, &state.config)?;
    Ok(Json(value))
}
