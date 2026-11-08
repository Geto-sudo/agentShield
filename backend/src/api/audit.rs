use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::{ApiError, ApiResult};
use crate::service;
use crate::state::AppState;
use crate::store;

#[derive(Debug, Deserialize)]
pub struct AuditQuery {
    pub limit: Option<i64>,
}

pub async fn list(
    State(state): State<AppState>,
    Query(query): Query<AuditQuery>,
) -> ApiResult<Json<Value>> {
    let entries = state
        .db
        .with_conn(|c| store::audit::list(c, query.limit.unwrap_or(200)))
        .map_err(ApiError::from)?;

    Ok(Json(json!({
        "entries": entries,
        "count": entries.len(),
        "algorithm": "sha256(prev_hash | seq | ts | kind | subject | payload)",
    })))
}

/// Recalculate the entire chain: any alteration is detected.
pub async fn verify(State(state): State<AppState>) -> ApiResult<Json<Value>> {
    let status = service::chain_status(&state.db)?;
    Ok(Json(json!({ "chain": status })))
}
