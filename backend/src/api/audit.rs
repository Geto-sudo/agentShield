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
        "signature_algorithm": store::audit::SIGNING_ALGORITHM,
    })))
}

/// Recalcule toute la chaîne : n'importe quelle altération est détectée, et
/// chaque entrée doit porter une signature Ed25519 valide.
pub async fn verify(State(state): State<AppState>) -> ApiResult<Json<Value>> {
    let status = service::chain_status(&state.db)?;
    Ok(Json(json!({ "chain": status })))
}

/// `GET /api/audit/pubkey` — clé publique qui atteste les entrées.
///
/// Permet à un tiers de conserver la clé hors ligne et de revérifier plus tard
/// que les entrées n'ont pas été re-signées.
pub async fn pubkey() -> Json<Value> {
    let audit = store::audit::signing_info();
    let notice = if audit.key_source == "ephemeral" {
        "Clé éphémère générée au démarrage : elle ne vérifie que les entrées écrites \
         par ce processus. Définir AGENTSHIELD_AUDIT_SIGNING_KEY pour une clé stable."
    } else {
        "Clé stable fournie par AGENTSHIELD_AUDIT_SIGNING_KEY."
    };

    Json(json!({ "audit": audit, "notice": notice }))
}
