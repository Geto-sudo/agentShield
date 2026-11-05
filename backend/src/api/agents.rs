use axum::extract::State;
use axum::Json;
use serde_json::{json, Value};

use crate::domain::agent::{Agent, CreateAgentRequest};
use crate::error::ApiResult;
use crate::service::{self, AuditProof};
use crate::state::AppState;
use crate::store;

pub async fn list(State(state): State<AppState>) -> ApiResult<Json<Value>> {
    let agents: Vec<Agent> = state
        .db
        .with_conn(store::agents::list)
        .map_err(crate::error::ApiError::from)?;

    let mut enriched = Vec::with_capacity(agents.len());
    for agent in agents {
        let wallet = state
            .db
            .with_conn(|c| store::wallet::get(c, agent.id))
            .map_err(crate::error::ApiError::from)?;
        enriched.push(json!({ "agent": agent, "wallet": wallet }));
    }

    Ok(Json(json!({ "agents": enriched })))
}

pub async fn create(
    State(state): State<AppState>,
    Json(req): Json<CreateAgentRequest>,
) -> ApiResult<Json<Value>> {
    let (agent, proof): (Agent, AuditProof) = service::create_agent(
        &state.db,
        &req.name,
        req.owner.as_deref(),
        req.purpose.as_deref(),
    )?;

    // Un agent sans wallet ne peut pas recevoir de rechargement : on en crée un
    // avec un solde de démonstration.
    state
        .db
        .with_tx(|tx| store::wallet::ensure(tx, agent.id, 1_200, 5_000))
        .map_err(crate::error::ApiError::from)?;

    Ok(Json(json!({ "agent": agent, "audit": proof })))
}
