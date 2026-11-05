use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::domain::action::ProposedAction;
use crate::domain::decision::Decision;
use crate::error::{ApiError, ApiResult};
use crate::service::{self, ProposalOutcome};
use crate::state::AppState;
use crate::store;

#[derive(Debug, Deserialize)]
pub struct ActionQuery {
    pub agent_id: Option<Uuid>,
    pub mandate_id: Option<Uuid>,
    pub decision: Option<String>,
    pub limit: Option<i64>,
}

/// `POST /api/actions` — le geste central de la démo.
pub async fn propose(
    State(state): State<AppState>,
    Json(payload): Json<ProposedAction>,
) -> ApiResult<Json<ProposalOutcome>> {
    let outcome = service::propose(&state.db, &state.config, payload)?;
    Ok(Json(outcome))
}

pub async fn list(
    State(state): State<AppState>,
    Query(query): Query<ActionQuery>,
) -> ApiResult<Json<Value>> {
    let filter = store::actions::ActionFilter {
        agent_id: query.agent_id,
        mandate_id: query.mandate_id,
        decision: query.decision.as_deref().and_then(Decision::parse_opt),
        limit: query.limit.unwrap_or(50),
    };

    let actions = state
        .db
        .with_conn(|c| store::actions::list(c, &filter))
        .map_err(ApiError::from)?;

    Ok(Json(json!({ "actions": actions, "count": actions.len() })))
}

pub async fn get_one(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    let action = state
        .db
        .with_conn(|c| store::actions::get(c, id))
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::NotFound(format!("action {id}")))?;

    let mandate = state
        .db
        .with_conn(|c| store::mandates::get(c, action.mandate_id))
        .map_err(ApiError::from)?;

    let approval = state
        .db
        .with_conn(|c| store::approvals::get_by_action(c, id))
        .map_err(ApiError::from)?;

    let order = state
        .db
        .with_conn(|c| store::orders::get_by_action(c, id))
        .map_err(ApiError::from)?;

    Ok(Json(json!({
        "action": action,
        "mandate": mandate,
        "approval": approval,
        "order": order,
    })))
}

/// `POST /api/actions/{id}/paypal/order` — refusé si l'action n'est pas approuvée.
pub async fn create_paypal_order(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    let (order, action) =
        service::create_order_for_action(&state.db, &state.config, state.paypal.as_ref(), id)
            .await?;

    Ok(Json(json!({
        "order": order,
        "action": action,
        "mode": state.config.paypal_mode(),
        "notice": "Commande PayPal Sandbox : aucun argent réel n'est transféré.",
    })))
}
