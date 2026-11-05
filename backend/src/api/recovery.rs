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
    /// Sinistre précis à instruire.
    #[serde(default)]
    pub incident_id: Option<Uuid>,
    /// Crée d'abord un nouveau scénario « action rogue » pour cet agent.
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
        "notice": "Payouts Sandbox de démonstration : aucune couverture réelle.",
    })))
}

/// `POST /api/recovery/simulate`
///
/// Instruit un sinistre : le garde-fou est rejoué sur le log, l'expert IA rend un
/// verdict structuré, puis un Payout Sandbox peut être émis depuis la réserve
/// commune locale. Le montant du payout est **calculé par le code**, jamais par
/// le modèle.
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
        "notice": "Simulation de récupération : ce n'est pas un produit d'assurance.",
    })))
}
