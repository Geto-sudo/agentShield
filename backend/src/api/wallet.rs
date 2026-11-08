use axum::extract::{Path, State};
use axum::Json;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::service;
use crate::state::AppState;
use crate::store;

#[derive(Debug, serde::Deserialize)]
pub struct TopupRequest {
    pub amount_cents: i64,
}

pub async fn get_wallet(
    State(state): State<AppState>,
    Path(agent_id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    let (wallet, quote) = service::wallet_quote(&state.db, agent_id)?;
    let ledger = state
        .db
        .with_conn(|c| store::wallet::ledger(c, agent_id, 25))
        .map_err(ApiError::from)?;

    Ok(Json(
        json!({ "wallet": wallet, "quote": quote, "ledger": ledger }),
    ))
}

pub async fn ledger(
    State(state): State<AppState>,
    Path(agent_id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    let entries = state
        .db
        .with_conn(|c| store::wallet::ledger(c, agent_id, 100))
        .map_err(ApiError::from)?;

    Ok(Json(json!({ "entries": entries })))
}

pub async fn quote(
    State(state): State<AppState>,
    Path(agent_id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    let (wallet, quote) = service::wallet_quote(&state.db, agent_id)?;
    Ok(Json(json!({ "wallet": wallet, "quote": quote })))
}

/// Creates a PayPal Sandbox checkout to top up the simulated wallet.
pub async fn topup(
    State(state): State<AppState>,
    Path(agent_id): Path<Uuid>,
    Json(req): Json<TopupRequest>,
) -> ApiResult<Json<Value>> {
    let order = service::create_topup_order(
        &state.db,
        &state.config,
        state.paypal.as_ref(),
        agent_id,
        req.amount_cents,
    )
    .await?;

    Ok(Json(json!({
        "order": order,
        "notice": "Sandbox top-up: capture the order to credit the simulated wallet.",
    })))
}

pub async fn reserve(State(state): State<AppState>) -> ApiResult<Json<Value>> {
    let balance = state
        .db
        .with_conn(store::wallet::reserve_balance)
        .map_err(ApiError::from)?;
    let ledger = state
        .db
        .with_conn(|c| store::wallet::reserve_ledger(c, 25))
        .map_err(ApiError::from)?;

    Ok(Json(json!({ "balance_cents": balance, "ledger": ledger })))
}
