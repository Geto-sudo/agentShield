use axum::extract::{Path, State};
use axum::Json;
use serde_json::{json, Value};

use crate::error::{ApiError, ApiResult};
use crate::service;
use crate::state::AppState;
use crate::store;

pub async fn list_orders(State(state): State<AppState>) -> ApiResult<Json<Value>> {
    let orders = state
        .db
        .with_conn(|c| store::orders::list(c, 50))
        .map_err(ApiError::from)?;

    Ok(Json(json!({
        "orders": orders,
        "mode": state.config.paypal_mode(),
    })))
}

/// `POST /api/paypal/orders/{order_id}/capture`
pub async fn capture(
    State(state): State<AppState>,
    Path(order_id): Path<String>,
) -> ApiResult<Json<Value>> {
    let outcome = service::capture_order(&state.db, state.paypal.as_ref(), &order_id).await?;

    Ok(Json(json!({
        "order": outcome.order,
        "wallet": outcome.wallet,
        "action": outcome.action,
        "audit": outcome.audit,
        "notice": "Capture Sandbox : le wallet simulé a été mis à jour, aucun argent réel n'a bougé.",
    })))
}
