use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::domain::mandate::{CreateMandateRequest, Mandate};
use crate::error::{ApiError, ApiResult};
use crate::service;
use crate::state::AppState;
use crate::store;

#[derive(Debug, Deserialize)]
pub struct MandateQuery {
    pub agent_id: Option<Uuid>,
    pub limit: Option<i64>,
}

pub async fn list(
    State(state): State<AppState>,
    Query(query): Query<MandateQuery>,
) -> ApiResult<Json<Value>> {
    let limit = query.limit.unwrap_or(50);
    let mandates: Vec<Mandate> = state
        .db
        .with_conn(|c| store::mandates::list(c, query.agent_id, limit))
        .map_err(ApiError::from)?;

    Ok(Json(json!({ "mandates": mandates })))
}

pub async fn create(
    State(state): State<AppState>,
    Json(req): Json<CreateMandateRequest>,
) -> ApiResult<Json<Value>> {
    let (mandate, proof) = service::create_mandate(&state.db, &state.config, req)?;
    Ok(Json(json!({
        "mandate": mandate,
        "audit": proof,
        "thresholds": {
            "auto_approve_max_cents": mandate.auto_approve_max_cents,
            "escalate_until_cents": mandate.escalate_until_cents,
        }
    })))
}

pub async fn get_one(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    let mandate = state
        .db
        .with_conn(|c| store::mandates::get(c, id))
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::NotFound(format!("mandate {id}")))?;

    let committed = state
        .db
        .with_conn(|c| store::mandates::committed_cents(c, id))
        .map_err(ApiError::from)?;

    Ok(Json(json!({
        "mandate": mandate,
        "committed_cents": committed,
        "remaining_cents": (mandate.max_total_cents - committed).max(0),
    })))
}
