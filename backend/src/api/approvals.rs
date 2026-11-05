use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::domain::approval::DecideApprovalRequest;
use crate::error::{ApiError, ApiResult};
use crate::service;
use crate::state::AppState;
use crate::store;

#[derive(Debug, Deserialize)]
pub struct ApprovalQuery {
    pub status: Option<String>,
    pub limit: Option<i64>,
}

pub async fn list(
    State(state): State<AppState>,
    Query(query): Query<ApprovalQuery>,
) -> ApiResult<Json<Value>> {
    let status = query
        .status
        .as_deref()
        .map(crate::domain::approval::ApprovalStatus::parse);

    // On purge d'abord les escalades expirées : une approbation ne survit pas
    // à sa fenêtre de confirmation.
    state
        .db
        .with_conn(|c| store::approvals::expire_stale(c, chrono::Utc::now()))
        .map_err(ApiError::from)?;

    let approvals = state
        .db
        .with_conn(|c| store::approvals::list(c, status, query.limit.unwrap_or(50)))
        .map_err(ApiError::from)?;

    Ok(Json(
        json!({ "approvals": approvals, "count": approvals.len() }),
    ))
}

/// `POST /api/approvals/{id}/confirm` — la confirmation humaine explicite.
pub async fn confirm(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<DecideApprovalRequest>,
) -> ApiResult<Json<Value>> {
    let (approval, proof) = service::decide_approval(
        &state.db,
        id,
        true,
        req.decided_by.as_deref(),
        req.note.as_deref(),
    )?;

    Ok(Json(json!({
        "approval": approval,
        "audit": proof,
        "notice": "Le propriétaire a confirmé explicitement. Un checkout PayPal peut désormais être créé.",
    })))
}

/// `POST /api/approvals/{id}/deny`
pub async fn deny(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<DecideApprovalRequest>,
) -> ApiResult<Json<Value>> {
    let (approval, proof) = service::decide_approval(
        &state.db,
        id,
        false,
        req.decided_by.as_deref(),
        req.note.as_deref(),
    )?;

    Ok(Json(json!({
        "approval": approval,
        "audit": proof,
        "notice": "Le propriétaire a refusé l'escalade. Aucun checkout ne sera créé.",
    })))
}
