use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::domain::recovery::IncidentStatus;
use crate::error::{ApiError, ApiResult};
use crate::service;
use crate::state::AppState;
use crate::store;

#[derive(Debug, Deserialize)]
pub struct IncidentQuery {
    pub status: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Default, Deserialize)]
pub struct SimulateRequest {
    /// Specific incident to investigate.
    #[serde(default)]
    pub incident_id: Option<Uuid>,
    /// First creates a new "rogue action" scenario for this agent.
    #[serde(default)]
    pub agent_id: Option<Uuid>,
}

pub async fn list_incidents(
    State(state): State<AppState>,
    Query(query): Query<IncidentQuery>,
) -> ApiResult<Json<Value>> {
    let status = query.status.as_deref().map(IncidentStatus::parse);
    let incidents = state
        .db
        .with_conn(|c| store::recovery::list_incidents(c, status, query.limit.unwrap_or(50)))
        .map_err(ApiError::from)?;

    Ok(Json(
        json!({ "incidents": incidents, "count": incidents.len() }),
    ))
}

pub async fn list_payouts(State(state): State<AppState>) -> ApiResult<Json<Value>> {
    let payouts = state
        .db
        .with_conn(|c| store::recovery::list_payouts(c, 50))
        .map_err(ApiError::from)?;

    Ok(Json(json!({
        "payouts": payouts,
        "mode": state.config.paypal_mode(),
        "notice": "Demo Sandbox payouts: no real insurance coverage is provided.",
    })))
}

/// `POST /api/recovery/simulate`
///
/// Investigates an incident: the guardrail is replayed against the log,
/// the AI expert produces a structured verdict, and a Sandbox payout may
/// be issued from the local shared reserve.
/// The payout amount is **calculated by the code**, never by the model.
pub async fn simulate(
    State(state): State<AppState>,
    Json(req): Json<SimulateRequest>,
) -> ApiResult<Json<Value>> {
    let incident_id = match (req.agent_id, req.incident_id) {
        (Some(agent_id), _) => {
            let incident = service::spawn_rogue_scenario(&state.db, &state.config, agent_id)?;
            Some(incident.id)
        }
        (None, Some(id)) => Some(id),
        (None, None) => None,
    };

    let outcome = service::run_recovery(
        &state.db,
        &state.config,
        &state.expert,
        state.paypal.as_ref(),
        incident_id,
    )
    .await?;

    Ok(Json(json!({
        "outcome": outcome,
        "notice": "Recovery simulation: this is not an insurance product.",
    })))
}
