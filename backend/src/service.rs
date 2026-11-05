//! Couche service : toute la logique métier, indépendante du transport HTTP.
//!
//! Les handlers axum ne font que désérialiser, appeler un service et sérialiser.
//! Le seeder de démonstration utilise exactement les mêmes fonctions : la démo
//! ne peut pas produire un état impossible à atteindre par l'API.

use std::sync::Arc;

use chrono::{Duration, Utc};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::config::Config;
use crate::domain::action::{ActionKind, ActionRecord, ProposedAction};
use crate::domain::agent::{Agent, AgentStatus};
use crate::domain::approval::{Approval, ApprovalStatus};
use crate::domain::audit::ChainStatus;
use crate::domain::decision::{Decision, GuardrailVerdict};
use crate::domain::mandate::{CreateMandateRequest, Mandate, MandateStatus};
use crate::domain::money::{format_cents, Currency};
use crate::domain::order::{OrderPurpose, OrderStatus, SandboxOrder};
use crate::domain::recovery::{
    Incident, IncidentKind, IncidentStatus, Payout, PayoutStatus, RecoveryOutcome,
};
use crate::domain::wallet::{PriceQuote, Wallet};
use crate::engine::expert::{ExpertInput, ExpertService, RESOLUTION_PAYOUT, RESOLUTION_REJECT};
use crate::engine::{guardrail, pricing, HistoryWindow};
use crate::error::{ApiError, ApiResult};
use crate::paypal::{CreateOrderRequest, PayPalGateway, PayoutRequest};
use crate::store::{self, Db};

// ---------------------------------------------------------------------------
// Preuve d'audit renvoyée avec chaque écriture
// ---------------------------------------------------------------------------

/// Preuve renvoyée avec chaque écriture : position dans la chaîne + hash.
/// `POST /api/audit/verify` recalcule la chaîne entière à la demande.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AuditProof {
    pub seq: i64,
    pub hash: String,
    pub chain_valid: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ProposalOutcome {
    pub action: ActionRecord,
    pub mandate: Mandate,
    pub agent: Agent,
    pub decision: Decision,
    pub risk: crate::domain::decision::RiskAssessment,
    pub checks: Vec<crate::domain::decision::RuleCheck>,
    pub reasons: Vec<String>,
    pub explanation: String,
    pub thresholds: crate::domain::decision::Thresholds,
    pub approval: Option<Approval>,
    pub audit: AuditProof,
    pub next_steps: Vec<String>,
}

// ---------------------------------------------------------------------------
// Agents
// ---------------------------------------------------------------------------

pub fn create_agent(
    db: &Db,
    name: &str,
    owner: Option<&str>,
    purpose: Option<&str>,
) -> ApiResult<(Agent, AuditProof)> {
    let name = name.trim();
    if name.is_empty() {
        return Err(ApiError::bad_request("Le nom de l'agent est obligatoire."));
    }

    let agent = Agent {
        id: Uuid::new_v4(),
        name: name.to_string(),
        owner: owner.unwrap_or("Propriétaire de démonstration").to_string(),
        purpose: purpose
            .unwrap_or("Agent de démonstration AgentShield")
            .to_string(),
        status: AgentStatus::Active,
        created_at: Utc::now(),
    };

    let proof = db
        .with_tx(|tx| {
            store::agents::insert(tx, &agent)?;
            let (seq, hash, _prev) = store::audit::append(
                tx,
                "agent.created",
                &agent.id.to_string(),
                &json!({ "name": agent.name, "owner": agent.owner }),
            )?;
            Ok(AuditProof {
                seq,
                hash,
                chain_valid: true,
            })
        })
        .map_err(ApiError::from)?;

    Ok((agent, proof))
}

// ---------------------------------------------------------------------------
// Mandats
// ---------------------------------------------------------------------------

pub fn create_mandate(
    db: &Db,
    config: &Config,
    req: CreateMandateRequest,
) -> ApiResult<(Mandate, AuditProof)> {
    if req.max_total_cents <= 0 {
        return Err(ApiError::bad_request(
            "max_total_cents doit être un entier positif (en centimes).",
        ));
    }
    if req.intent.trim().is_empty() {
        return Err(ApiError::bad_request(
            "L'intention du mandat est obligatoire.",
        ));
    }
    if req.category.trim().is_empty() {
        return Err(ApiError::bad_request(
            "La catégorie du mandat est obligatoire.",
        ));
    }

    let agent = db
        .with_conn(|c| store::agents::get(c, req.agent_id))
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::NotFound(format!("agent {}", req.agent_id)))?;

    let now = Utc::now();
    let currency = req.currency.unwrap_or(Currency::Eur);
    let max_total = req.max_total_cents;

    let per_txn = req
        .per_txn_max_cents
        .unwrap_or(max_total)
        .clamp(1, max_total);

    let auto_approve = req
        .auto_approve_max_cents
        .unwrap_or(config.default_auto_approve_max_cents)
        .clamp(0, per_txn);

    let escalate_until = req
        .escalate_until_cents
        .unwrap_or(per_txn)
        .clamp(auto_approve, per_txn);

    let valid_days = req.valid_days.unwrap_or(30).clamp(1, 365);

    let mandate = Mandate {
        id: Uuid::new_v4(),
        agent_id: agent.id,
        owner_name: req.owner_name.trim().to_string(),
        intent: req.intent.trim().to_string(),
        category: req.category.trim().to_string(),
        currency,
        max_total_cents: max_total,
        per_txn_max_cents: per_txn,
        auto_approve_max_cents: auto_approve,
        escalate_until_cents: escalate_until,
        allowed_merchants: normalize_list(req.allowed_merchants),
        blocked_merchants: normalize_list(req.blocked_merchants),
        allowed_countries: normalize_list(req.allowed_countries),
        max_delivery_days: req.max_delivery_days.unwrap_or(7).min(90),
        valid_from: now,
        valid_until: now + Duration::days(valid_days),
        status: MandateStatus::Active,
        created_at: now,
    };

    let proof = db
        .with_tx(|tx| {
            store::mandates::insert(tx, &mandate)?;
            let (seq, hash, _prev) = store::audit::append(
                tx,
                "mandate.created",
                &mandate.id.to_string(),
                &json!({
                    "agent_id": mandate.agent_id,
                    "intent": mandate.intent,
                    "max_total_cents": mandate.max_total_cents,
                    "per_txn_max_cents": mandate.per_txn_max_cents,
                    "auto_approve_max_cents": mandate.auto_approve_max_cents,
                    "escalate_until_cents": mandate.escalate_until_cents,
                    "currency": mandate.currency.as_str(),
                }),
            )?;
            Ok(AuditProof {
                seq,
                hash,
                chain_valid: true,
            })
        })
        .map_err(ApiError::from)?;

    Ok((mandate, proof))
}

fn normalize_list(list: Option<Vec<String>>) -> Vec<String> {
    list.unwrap_or_default()
        .into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

// ---------------------------------------------------------------------------
// Cœur : proposition d'action -> décision
// ---------------------------------------------------------------------------

pub fn propose(db: &Db, config: &Config, proposed: ProposedAction) -> ApiResult<ProposalOutcome> {
    let now = Utc::now();

    let mandate = db
        .with_conn(|c| store::mandates::get(c, proposed.mandate_id))
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::NotFound(format!("mandat {}", proposed.mandate_id)))?;

    let agent_id = proposed.agent_id.unwrap_or(mandate.agent_id);
    let agent = db
        .with_conn(|c| store::agents::get(c, agent_id))
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::NotFound(format!("agent {agent_id}")))?;

    let raw_log = serde_json::to_value(&proposed).unwrap_or_else(|_| json!({}));
    let action_id = Uuid::new_v4();
    let currency = proposed.currency.unwrap_or(mandate.currency);

    // Read the budget/idempotency history and persist the decision while the
    // same SQLite lock and transaction are held. Otherwise concurrent requests
    // could both observe the same remaining budget and both be approved.
    let (action, verdict, approval, proof) = db
        .with_tx(|tx| {
            let history = build_history(tx, &mandate, agent_id, &proposed, now)?;
            let verdict = guardrail::evaluate(&proposed, &mandate, &history, now);
            let approval_id = (verdict.decision == Decision::Escalate)
                .then(Uuid::new_v4);

            let action = ActionRecord {
                id: action_id,
                mandate_id: mandate.id,
                agent_id,
                kind: proposed.kind,
                merchant: proposed.merchant.trim().to_string(),
                merchant_country: proposed.merchant_country.clone(),
                amount_cents: proposed.amount_cents,
                currency,
                category: proposed.category.trim().to_string(),
                delivery_days: proposed.delivery_days,
                description: proposed.description.trim().to_string(),
                idempotency_key: proposed
                    .idempotency_key
                    .as_ref()
                    .map(|k| k.trim().to_string())
                    .filter(|k| !k.is_empty()),
                raw_log: raw_log.clone(),
                decision: verdict.decision,
                risk_score: verdict.risk.score,
                risk_band: verdict.risk.band,
                checks: verdict.checks.clone(),
                reasons: verdict.reasons.clone(),
                explanation: verdict.explanation.clone(),
                approval_id,
                order_id: None,
                settled: false,
                audit_hash: String::new(),
                created_at: now,
            };

            let approval = approval_id.map(|id| Approval {
                id,
                action_id,
                status: ApprovalStatus::Pending,
                requested_at: now,
                expires_at: now + Duration::minutes(config.approval_ttl_minutes.max(1)),
                decided_at: None,
                decided_by: None,
                note: None,
                created_at: now,
            });

            if let Some(a) = &approval {
                store::approvals::insert(tx, a)?;
            }

            let (seq, hash, _prev) = store::audit::append(
                tx,
                match verdict.decision {
                    Decision::Approve => "action.approved",
                    Decision::Escalate => "action.escalated",
                    Decision::Refuse => "action.refused",
                },
                &action_id.to_string(),
                &json!({
                    "mandate_id": mandate.id,
                    "agent_id": agent_id,
                    "amount_cents": action.amount_cents,
                    "currency": action.currency.as_str(),
                    "merchant": action.merchant,
                    "decision": verdict.decision.as_str(),
                    "risk_score": verdict.risk.score,
                    "violated_rules": verdict.violated_rules(),
                    "thresholds": {
                        "auto_approve_max_cents": verdict.thresholds.auto_approve_max_cents,
                        "escalate_until_cents": verdict.thresholds.escalate_until_cents,
                    },
                    "raw_log": raw_log,
                }),
            )?;

            let proof = AuditProof {
                seq,
                hash: hash.clone(),
                chain_valid: true,
            };

            let mut stored = action.clone();
            stored.audit_hash = hash;
            store::actions::insert(tx, &stored)?;

            Ok((stored, verdict, approval, proof))
        })
        .map_err(ApiError::from)?;
    let approval_id = approval.as_ref().map(|a| a.id);

    Ok(ProposalOutcome {
        action,
        mandate,
        agent,
        decision: verdict.decision,
        risk: verdict.risk,
        checks: verdict.checks,
        reasons: verdict.reasons,
        explanation: verdict.explanation,
        thresholds: verdict.thresholds,
        approval,
        audit: proof,
        next_steps: next_steps(verdict.decision, action_id, approval_id),
    })
}

fn next_steps(decision: Decision, action_id: Uuid, approval_id: Option<Uuid>) -> Vec<String> {
    match decision {
        Decision::Approve => vec![
            format!(
                "POST /api/actions/{action_id}/paypal/order — créer le checkout PayPal Sandbox."
            ),
            "Seule une action approuvée peut produire une commande PayPal.".to_string(),
        ],
        Decision::Escalate => vec![
            format!(
                "POST /api/approvals/{}/confirm — le propriétaire confirme explicitement.",
                approval_id.unwrap_or_default()
            ),
            format!(
                "POST /api/approvals/{}/deny — ou il refuse.",
                approval_id.unwrap_or_default()
            ),
            "Sans confirmation, aucun checkout ne peut être créé.".to_string(),
        ],
        Decision::Refuse => vec![
            "Aucun checkout PayPal n'est créé.".to_string(),
            "L'agent reçoit l'explication ci-dessus et peut proposer une autre action.".to_string(),
        ],
    }
}

fn build_history(
    conn: &rusqlite::Connection,
    mandate: &Mandate,
    agent_id: Uuid,
    proposed: &ProposedAction,
    now: chrono::DateTime<Utc>,
) -> rusqlite::Result<HistoryWindow> {
    let committed_cents = store::mandates::committed_cents(conn, mandate.id)?;
    let recent_actions_15min =
        store::actions::recent_count(conn, agent_id, now - Duration::minutes(15))?;
    let known_merchants = store::actions::seen_merchants(conn, agent_id)?;
    let duplicate_action = match proposed.idempotency_key.as_deref() {
        Some(key) if !key.trim().is_empty() => {
            store::actions::find_by_idempotency_key(conn, mandate.id, key.trim())?.is_some()
        }
        _ => false,
    };
    let open_incidents = store::recovery::count_open_incidents(conn, agent_id)?;

    Ok(HistoryWindow {
        committed_cents,
        recent_actions_15min,
        known_merchants,
        duplicate_action,
        open_incidents,
    })
}

// ---------------------------------------------------------------------------
// Approbations humaines
// ---------------------------------------------------------------------------

pub fn decide_approval(
    db: &Db,
    approval_id: Uuid,
    confirmed: bool,
    decided_by: Option<&str>,
    note: Option<&str>,
) -> ApiResult<(Approval, AuditProof)> {
    let now = Utc::now();
    let approval = db
        .with_conn(|c| store::approvals::get(c, approval_id))
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::NotFound(format!("approbation {approval_id}")))?;

    if approval.status != ApprovalStatus::Pending {
        return Err(ApiError::Conflict(format!(
            "Cette escalade est déjà « {} ».",
            approval.status.as_str()
        )));
    }
    if now > approval.expires_at {
        let _ = db.with_conn(|c| {
            store::approvals::decide(c, approval_id, ApprovalStatus::Expired, None, None, now)
        });
        return Err(ApiError::Conflict(
            "Cette escalade a expiré : elle ne peut plus être confirmée.".to_string(),
        ));
    }

    let status = if confirmed {
        ApprovalStatus::Confirmed
    } else {
        ApprovalStatus::Denied
    };

    let proof = db
        .with_tx(|tx| {
            store::approvals::decide(tx, approval_id, status, decided_by, note, now)?;
            let (seq, hash, _prev) = store::audit::append(
                tx,
                if confirmed {
                    "approval.confirmed"
                } else {
                    "approval.denied"
                },
                &approval.action_id.to_string(),
                &json!({
                    "approval_id": approval_id,
                    "decided_by": decided_by.unwrap_or("propriétaire"),
                    "note": note,
                }),
            )?;
            Ok(AuditProof {
                seq,
                hash,
                chain_valid: true,
            })
        })
        .map_err(ApiError::from)?;

    let mut updated = approval;
    updated.status = status;
    updated.decided_at = Some(now);
    updated.decided_by = decided_by.map(str::to_string);
    updated.note = note.map(str::to_string);

    Ok((updated, proof))
}

// ---------------------------------------------------------------------------
// PayPal Sandbox
// ---------------------------------------------------------------------------

/// Crée un checkout PayPal pour une action approuvée (ou une escalade confirmée).
pub async fn create_order_for_action(
    db: &Db,
    config: &Config,
    paypal: &dyn PayPalGateway,
    action_id: Uuid,
) -> ApiResult<(SandboxOrder, ActionRecord)> {
    let action = db
        .with_conn(|c| store::actions::get(c, action_id))
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::NotFound(format!("action {action_id}")))?;

    match action.decision {
        Decision::Refuse => {
            return Err(ApiError::Denied(
                "Refus : aucun checkout PayPal ne peut être créé pour une action refusée."
                    .to_string(),
            ))
        }
        Decision::Escalate => {
            let approval = db
                .with_conn(|c| store::approvals::get_by_action(c, action_id))
                .map_err(ApiError::from)?
                .ok_or_else(|| {
                    ApiError::Conflict("Escalade sans approbation enregistrée.".to_string())
                })?;
            if !approval.is_usable(Utc::now()) {
                return Err(ApiError::Denied(format!(
                    "Escalade non confirmée (état « {} ») : aucun checkout ne peut être créé.",
                    approval.status.as_str()
                )));
            }
        }
        Decision::Approve => {}
    }

    if action.settled {
        return Err(ApiError::Conflict(
            "Cette action a déjà été réglée.".to_string(),
        ));
    }

    if let Some(existing) = db
        .with_conn(|c| store::orders::get_by_action(c, action_id))
        .map_err(ApiError::from)?
    {
        if existing.status != OrderStatus::Failed {
            return Ok((existing, action));
        }
    }

    let created = paypal
        .create_order(CreateOrderRequest {
            amount_cents: action.amount_cents,
            currency: action.currency,
            description: format!(
                "AgentShield — {} chez {}",
                action.description, action.merchant
            ),
            reference_id: action.id.to_string(),
            return_url: format!("{}/?paypal=return", config.public_base_url),
            cancel_url: format!("{}/?paypal=cancel", config.public_base_url),
        })
        .await
        .map_err(|e| ApiError::PayPal(e.to_string()))?;

    let order = SandboxOrder {
        id: Uuid::new_v4(),
        action_id: Some(action.id),
        purpose: OrderPurpose::Purchase,
        paypal_order_id: created.id.clone(),
        status: OrderStatus::Created,
        amount_cents: action.amount_cents,
        currency: action.currency,
        description: format!("{} — {}", action.merchant, action.description),
        approve_url: created.approve_url.clone(),
        capture_id: None,
        mode: paypal.mode().to_string(),
        failure_reason: None,
        created_at: Utc::now(),
        captured_at: None,
    };

    let order_for_tx = order.clone();
    db.with_tx(|tx| {
        store::orders::insert(tx, &order_for_tx)?;
        store::actions::attach_order(tx, action_id, order_for_tx.id)?;
        store::audit::append(
            tx,
            "paypal.order.created",
            &order_for_tx.paypal_order_id,
            &json!({
                "action_id": action_id,
                "amount_cents": order_for_tx.amount_cents,
                "currency": order_for_tx.currency.as_str(),
                "mode": order_for_tx.mode,
            }),
        )?;
        Ok(())
    })
    .map_err(ApiError::from)?;

    Ok((order, action))
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CaptureOutcome {
    pub order: SandboxOrder,
    pub wallet: Wallet,
    pub action: Option<ActionRecord>,
    pub audit: AuditProof,
}

/// Capture un paiement Sandbox et met à jour le solde simulé.
pub async fn capture_order(
    db: &Db,
    paypal: &dyn PayPalGateway,
    paypal_order_id: &str,
) -> ApiResult<CaptureOutcome> {
    let order = db
        .with_conn(|c| store::orders::get_by_paypal_id(c, paypal_order_id))
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::NotFound(format!("commande PayPal {paypal_order_id}")))?;

    if order.status == OrderStatus::Captured {
        let wallet = db
            .with_conn(|c| match order.action_id {
                Some(action_id) => store::actions::get(c, action_id).map(|a| a.map(|a| a.agent_id)),
                None => Ok(None),
            })
            .map_err(ApiError::from)?
            .and_then(|agent_id| {
                db.with_conn(|c| store::wallet::get(c, agent_id))
                    .ok()
                    .flatten()
            })
            .unwrap_or_else(|| empty_wallet(order.action_id.unwrap_or_default()));

        return Ok(CaptureOutcome {
            order,
            wallet,
            action: None,
            audit: AuditProof {
                seq: 0,
                hash: String::new(),
                chain_valid: true,
            },
        });
    }

    let captured = paypal
        .capture_order(&order.paypal_order_id)
        .await
        .map_err(|e| ApiError::PayPal(e.to_string()))?;

    let success = matches!(captured.status.as_str(), "COMPLETED" | "APPROVED");

    if !success {
        let _ = db.with_tx(|tx| {
            store::orders::mark_failed(tx, order.id, &captured.status)?;
            store::audit::append(
                tx,
                "paypal.order.failed",
                &order.paypal_order_id,
                &json!({ "status": captured.status }),
            )?;
            Ok(())
        });

        return Err(ApiError::PayPal(format!(
            "Capture non aboutie : statut {}.",
            captured.status
        )));
    }

    let action = match order.action_id {
        Some(id) => db
            .with_conn(|c| store::actions::get(c, id))
            .map_err(ApiError::from)?,
        None => None,
    };

    let agent_id = action
        .as_ref()
        .map(|a| a.agent_id)
        .ok_or_else(|| ApiError::Conflict("Commande sans agent associé.".to_string()))?;

    let (wallet, proof) = db
        .with_tx(|tx| {
            store::orders::mark_captured(tx, order.id, captured.capture_id.as_deref(), Utc::now())?;

            let (delta, kind, label) = match order.purpose {
                OrderPurpose::Purchase => (
                    -order.amount_cents,
                    "debit",
                    format!(
                        "Achat {} chez {}",
                        format_cents(order.amount_cents, order.currency),
                        order.description
                    ),
                ),
                OrderPurpose::Topup => (
                    order.amount_cents,
                    "credit",
                    format!(
                        "Rechargement wallet {}",
                        format_cents(order.amount_cents, order.currency)
                    ),
                ),
            };

            let wallet = store::wallet::move_balance(
                tx,
                agent_id,
                delta,
                kind,
                &label,
                Some(&order.paypal_order_id),
            )?;

            if order.purpose == OrderPurpose::Purchase {
                if let Some(a) = &action {
                    store::actions::mark_settled(tx, a.id)?;
                }
            }

            let (seq, hash, _prev) = store::audit::append(
                tx,
                "paypal.order.captured",
                &order.paypal_order_id,
                &json!({
                    "amount_cents": order.amount_cents,
                    "purpose": order.purpose.as_str(),
                    "capture_id": captured.capture_id,
                    "wallet_balance_cents": wallet.balance_cents,
                }),
            )?;

            Ok((
                wallet,
                AuditProof {
                    seq,
                    hash,
                    chain_valid: true,
                },
            ))
        })
        .map_err(ApiError::from)?;

    let mut updated_order = order;
    updated_order.status = OrderStatus::Captured;
    updated_order.capture_id = captured.capture_id;
    updated_order.captured_at = Some(Utc::now());

    Ok(CaptureOutcome {
        order: updated_order,
        wallet,
        action,
        audit: proof,
    })
}

fn empty_wallet(agent_id: Uuid) -> Wallet {
    Wallet {
        agent_id,
        balance_cents: 0,
        premium_cents: 0,
        bonus_malus_bps: 0,
        reserve_contribution_cents: 0,
        spent_this_period_cents: 0,
        updated_at: Utc::now(),
    }
}

/// Rechargement du wallet simulé via un checkout PayPal Sandbox.
pub async fn create_topup_order(
    db: &Db,
    config: &Config,
    paypal: &dyn PayPalGateway,
    agent_id: Uuid,
    amount_cents: i64,
) -> ApiResult<SandboxOrder> {
    if amount_cents <= 0 {
        return Err(ApiError::bad_request(
            "Le montant du rechargement doit être positif.",
        ));
    }
    if amount_cents > 100_000 {
        return Err(ApiError::bad_request(
            "Rechargement limité à 1 000,00 EUR dans le prototype.",
        ));
    }

    let wallet = db
        .with_conn(|c| store::wallet::get(c, agent_id))
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::NotFound(format!("wallet {agent_id}")))?;

    let created = paypal
        .create_order(CreateOrderRequest {
            amount_cents,
            currency: Currency::Eur,
            description: "AgentShield — rechargement du wallet simulé".to_string(),
            reference_id: format!("topup-{agent_id}"),
            return_url: format!("{}/?paypal=return", config.public_base_url),
            cancel_url: format!("{}/?paypal=cancel", config.public_base_url),
        })
        .await
        .map_err(|e| ApiError::PayPal(e.to_string()))?;

    let order = SandboxOrder {
        id: Uuid::new_v4(),
        action_id: None,
        purpose: OrderPurpose::Topup,
        paypal_order_id: created.id,
        status: OrderStatus::Created,
        amount_cents,
        currency: Currency::Eur,
        description: format!(
            "Rechargement wallet — solde actuel {}",
            format_cents(wallet.balance_cents, Currency::Eur)
        ),
        approve_url: created.approve_url,
        capture_id: None,
        mode: paypal.mode().to_string(),
        failure_reason: None,
        created_at: Utc::now(),
        captured_at: None,
    };

    let to_store = order.clone();
    db.with_tx(|tx| {
        store::orders::insert(tx, &to_store)?;
        store::audit::append(
            tx,
            "paypal.topup.created",
            &to_store.paypal_order_id,
            &json!({ "agent_id": agent_id, "amount_cents": to_store.amount_cents }),
        )?;
        Ok(())
    })
    .map_err(ApiError::from)?;

    Ok(order)
}

// ---------------------------------------------------------------------------
// Wallet et prime
// ---------------------------------------------------------------------------

pub fn wallet_quote(db: &Db, agent_id: Uuid) -> ApiResult<(Wallet, PriceQuote)> {
    let wallet = db
        .with_conn(|c| store::wallet::get(c, agent_id))
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::NotFound(format!("wallet de l'agent {agent_id}")))?;

    let (avg_risk, approvals, incidents, refusals) = db
        .with_conn(|c| {
            let approvals = c.query_row(
                "SELECT COUNT(*) FROM actions WHERE agent_id = ?1 AND decision = 'APPROVE'",
                rusqlite::params![agent_id.to_string()],
                |r| r.get::<_, i64>(0),
            )?;
            let refusals = c.query_row(
                "SELECT COUNT(*) FROM actions WHERE agent_id = ?1 AND decision = 'REFUSE'",
                rusqlite::params![agent_id.to_string()],
                |r| r.get::<_, i64>(0),
            )?;
            let avg_risk = c.query_row(
                "SELECT COALESCE(ROUND(AVG(risk_score)), 0) FROM actions WHERE agent_id = ?1",
                rusqlite::params![agent_id.to_string()],
                |r| r.get::<_, f64>(0),
            )?;
            let incidents = store::recovery::count_incidents(c, agent_id)?;
            Ok((
                avg_risk.clamp(0.0, 100.0) as u8,
                approvals,
                incidents,
                refusals,
            ))
        })
        .map_err(ApiError::from)?;

    let _ = refusals;
    let quote = pricing::quote(&pricing::PricingInput {
        wallet: &wallet,
        avg_risk_score: avg_risk,
        incidents,
        approvals,
    });

    Ok((wallet, quote))
}

// ---------------------------------------------------------------------------
// Récupération (scénario « action rogue »)
// ---------------------------------------------------------------------------

pub async fn run_recovery(
    db: &Db,
    config: &Config,
    expert: &Arc<ExpertService>,
    paypal: &dyn PayPalGateway,
    incident_id: Option<Uuid>,
) -> ApiResult<RecoveryOutcome> {
    let incident = match incident_id {
        Some(id) => db
            .with_conn(|c| store::recovery::get_incident(c, id))
            .map_err(ApiError::from)?
            .ok_or_else(|| ApiError::NotFound(format!("sinistre {id}")))?,
        None => db
            .with_conn(|c| store::recovery::latest_open_incident(c, None))
            .map_err(ApiError::from)?
            .ok_or_else(|| {
                ApiError::NotFound(
                    "Aucun sinistre ouvert : lancez d'abord le scénario de démonstration."
                        .to_string(),
                )
            })?,
    };

    let mandate = db
        .with_conn(|c| store::mandates::get(c, incident.mandate_id))
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::NotFound(format!("mandat {}", incident.mandate_id)))?;

    let action = match incident.action_id {
        Some(id) => db
            .with_conn(|c| store::actions::get(c, id))
            .map_err(ApiError::from)?,
        None => None,
    };

    let action_log = action
        .as_ref()
        .map(|a| a.raw_log.clone())
        .unwrap_or_else(|| {
            json!({
                "mandate_id": incident.mandate_id,
                "merchant": "RogueShop",
                "amount_cents": incident.amount_cents,
                "currency": incident.currency.as_str(),
                "description": incident.description,
                "note": "log reconstitué depuis le sinistre (aucune action enregistrée)",
            })
        });

    let proposed = ProposedAction {
        mandate_id: incident.mandate_id,
        agent_id: Some(incident.agent_id),
        kind: ActionKind::Purchase,
        merchant: action
            .as_ref()
            .map(|a| a.merchant.clone())
            .unwrap_or_else(|| "RogueShop".to_string()),
        merchant_country: action
            .as_ref()
            .and_then(|a| a.merchant_country.clone())
            .or_else(|| Some("FR".to_string())),
        amount_cents: incident.amount_cents,
        currency: Some(incident.currency),
        category: action
            .as_ref()
            .map(|a| a.category.clone())
            .unwrap_or_else(|| mandate.category.clone()),
        delivery_days: action.as_ref().and_then(|a| a.delivery_days),
        description: action
            .as_ref()
            .map(|a| a.description.clone())
            .unwrap_or_else(|| incident.description.clone()),
        idempotency_key: None,
        occurred_at: Some(incident.created_at),
        metadata: None,
    };

    // On rejoue le garde-fou pour obtenir la vérité déterministe sur ce log.
    let history = HistoryWindow::default();
    let enforced: GuardrailVerdict = guardrail::evaluate(&proposed, &mandate, &history, Utc::now());

    let input = ExpertInput {
        mandate: mandate.clone(),
        action_log: action_log.clone(),
        guardrail: enforced.clone(),
        amount_cents: incident.amount_cents,
    };
    let verdict = expert.judge(&input).await;

    let mut steps: Vec<String> = vec![
        format!(
            "1. Sinistre « {} » repris : {} chez {}.",
            incident.kind.as_str(),
            format_cents(incident.amount_cents, incident.currency),
            proposed.merchant
        ),
        format!(
            "2. Garde-fou rejoué sur le log : {} (score {}/100, règles violées : {}).",
            enforced.decision.as_str(),
            enforced.risk.score,
            if enforced.violated_rules().is_empty() {
                "aucune".to_string()
            } else {
                enforced.violated_rules().join(", ")
            }
        ),
        format!(
            "3. Expert « {} » : {} (confiance {:.0} %).",
            verdict.source,
            verdict.verdict,
            verdict.confidence * 100.0
        ),
    ];

    let overage = (incident.amount_cents - mandate.max_total_cents).max(0);
    let mut payout: Option<Payout> = None;
    let new_status: IncidentStatus;

    match verdict.recommended_resolution.as_str() {
        RESOLUTION_PAYOUT if overage > 0 => {
            let reserve_balance = db
                .with_conn(store::wallet::reserve_balance)
                .map_err(ApiError::from)?;

            let payable = overage.min(reserve_balance);
            if payable <= 0 {
                steps.push(
                    "4. Réserve commune insuffisante : aucun payout émis, revue humaine demandée."
                        .to_string(),
                );
                new_status = IncidentStatus::UnderReview;
            } else {
                let created = paypal
                    .create_payout(PayoutRequest {
                        amount_cents: payable,
                        currency: incident.currency,
                        recipient_email: config.owner_payout_email.clone(),
                        note: format!(
                            "AgentShield — indemnisation simulée du dépassement de mandat (sinistre {})",
                            incident.id
                        ),
                        sender_batch_id: format!("AS-{}", incident.id.simple()),
                    })
                    .await
                    .map_err(|e| ApiError::PayPal(e.to_string()))?;

                let record = Payout {
                    id: Uuid::new_v4(),
                    incident_id: Some(incident.id),
                    paypal_batch_id: created.batch_id,
                    amount_cents: payable,
                    currency: incident.currency,
                    status: PayoutStatus::parse(&created.status),
                    mode: paypal.mode().to_string(),
                    recipient_email: config.owner_payout_email.clone(),
                    note: format!(
                        "Écart de {} au delà du plafond du mandat.",
                        format_cents(payable, incident.currency)
                    ),
                    created_at: Utc::now(),
                    completed_at: Some(Utc::now()),
                };

                let record_for_tx = record.clone();
                let incident_for_tx = incident.id;
                let verdict_json = serde_json::to_value(&verdict).unwrap_or(Value::Null);
                db.with_tx(|tx| {
                    store::recovery::insert_payout(tx, &record_for_tx)?;
                    store::wallet::reserve_move(
                        tx,
                        -payable,
                        "payout",
                        &format!("Indemnisation sinistre {}", incident_for_tx),
                        Some(&record_for_tx.paypal_batch_id),
                    )?;
                    store::recovery::update_incident(
                        tx,
                        incident_for_tx,
                        IncidentStatus::Resolved,
                        Some(&verdict_json),
                        Some(record_for_tx.id),
                        Some(Utc::now()),
                    )?;
                    store::audit::append(
                        tx,
                        "recovery.payout",
                        &record_for_tx.paypal_batch_id,
                        &json!({
                            "incident_id": incident_for_tx,
                            "amount_cents": record_for_tx.amount_cents,
                            "mode": record_for_tx.mode,
                            "verdict": verdict_json,
                        }),
                    )?;
                    Ok(())
                })
                .map_err(ApiError::from)?;

                steps.push(format!(
                    "4. Payout Sandbox ({}) émis : {} vers {}.",
                    record.mode,
                    format_cents(payable, incident.currency),
                    record.recipient_email
                ));
                payout = Some(record);
                new_status = IncidentStatus::Resolved;
            }
        }
        RESOLUTION_REJECT => {
            let verdict_json = serde_json::to_value(&verdict).unwrap_or(Value::Null);
            let id = incident.id;
            db.with_tx(|tx| {
                store::recovery::update_incident(
                    tx,
                    id,
                    IncidentStatus::Rejected,
                    Some(&verdict_json),
                    None,
                    Some(Utc::now()),
                )?;
                store::audit::append(tx, "recovery.rejected", &id.to_string(), &verdict_json)?;
                Ok(())
            })
            .map_err(ApiError::from)?;
            steps.push("4. Demande rejetée : aucune indemnisation.".to_string());
            new_status = IncidentStatus::Rejected;
        }
        _ => {
            let verdict_json = serde_json::to_value(&verdict).unwrap_or(Value::Null);
            let id = incident.id;
            db.with_tx(|tx| {
                store::recovery::update_incident(
                    tx,
                    id,
                    IncidentStatus::UnderReview,
                    Some(&verdict_json),
                    None,
                    None,
                )?;
                store::audit::append(tx, "recovery.review", &id.to_string(), &verdict_json)?;
                Ok(())
            })
            .map_err(ApiError::from)?;
            steps.push("4. Revue humaine demandée : aucune décision automatique.".to_string());
            new_status = IncidentStatus::UnderReview;
        }
    }

    let reserve_balance_cents = db
        .with_conn(store::wallet::reserve_balance)
        .map_err(ApiError::from)?;

    let mut final_incident = incident;
    final_incident.status = new_status;
    final_incident.verdict = Some(serde_json::to_value(&verdict).unwrap_or(Value::Null));
    final_incident.payout_id = payout.as_ref().map(|p| p.id);
    if new_status != IncidentStatus::UnderReview {
        final_incident.resolved_at = Some(Utc::now());
    }

    Ok(RecoveryOutcome {
        incident: final_incident,
        verdict,
        payout,
        reserve_balance_cents,
        steps,
    })
}

/// Crée un sinistre « action rogue » de démonstration pour un agent.
pub fn spawn_rogue_scenario(db: &Db, config: &Config, agent_id: Uuid) -> ApiResult<Incident> {
    let agent = db
        .with_conn(|c| store::agents::get(c, agent_id))
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::NotFound(format!("agent {agent_id}")))?;

    let mandate = db
        .with_conn(|c| store::mandates::list(c, Some(agent.id), 1))
        .map_err(ApiError::from)?
        .into_iter()
        .next()
        .ok_or_else(|| ApiError::NotFound(format!("mandat actif pour l'agent {agent_id}")))?;

    let amount = mandate.max_total_cents + (mandate.max_total_cents / 2).max(1_000);

    let proposed = ProposedAction {
        mandate_id: mandate.id,
        agent_id: Some(agent.id),
        kind: ActionKind::Purchase,
        merchant: "RogueShop".to_string(),
        merchant_country: Some("FR".to_string()),
        amount_cents: amount,
        currency: Some(mandate.currency),
        category: mandate.category.clone(),
        delivery_days: Some(2),
        description: "Achat non autorisé simulé (contournement du garde-fou)".to_string(),
        idempotency_key: None,
        occurred_at: Some(Utc::now()),
        metadata: Some(json!({ "simulation": "rogue_action" })),
    };

    // Le log est enregistré avec un refus : c'est la preuve que l'action
    // n'aurait jamais dû passer. Le sinistre décrit le dommage, pas la décision.
    let outcome = propose(db, config, proposed)?;

    let incident = Incident {
        id: Uuid::new_v4(),
        agent_id: agent.id,
        mandate_id: mandate.id,
        action_id: Some(outcome.action.id),
        kind: IncidentKind::RogueAction,
        status: IncidentStatus::Open,
        description: format!(
            "Dépense de {} chez RogueShop enregistrée hors du garde-fou ; le mandat plafonne à {}.",
            format_cents(amount, mandate.currency),
            format_cents(mandate.max_total_cents, mandate.currency)
        ),
        amount_cents: amount,
        currency: mandate.currency,
        overseen_by: "system".to_string(),
        verdict: None,
        payout_id: None,
        created_at: Utc::now(),
        resolved_at: None,
    };

    let to_store = incident.clone();
    db.with_tx(|tx| {
        store::recovery::insert_incident(tx, &to_store)?;
        store::audit::append(
            tx,
            "incident.opened",
            &to_store.id.to_string(),
            &json!({
                "agent_id": to_store.agent_id,
                "mandate_id": to_store.mandate_id,
                "amount_cents": to_store.amount_cents,
                "kind": to_store.kind.as_str(),
            }),
        )?;
        Ok(())
    })
    .map_err(ApiError::from)?;

    Ok(incident)
}

// ---------------------------------------------------------------------------
// Tableau de bord et audit
// ---------------------------------------------------------------------------

pub fn chain_status(db: &Db) -> ApiResult<ChainStatus> {
    db.with_conn(store::audit::verify).map_err(ApiError::from)
}

pub fn dashboard(db: &Db, config: &Config) -> ApiResult<Value> {
    let (approved, escalated, refused) = db
        .with_conn(store::actions::counts_by_decision)
        .map_err(ApiError::from)?;
    let settled_cents = db
        .with_conn(store::actions::settled_total_cents)
        .map_err(ApiError::from)?;
    let reserve = db
        .with_conn(store::wallet::reserve_balance)
        .map_err(ApiError::from)?;
    let pending = db
        .with_conn(store::approvals::pending_count)
        .map_err(ApiError::from)?;
    let agents = db.with_conn(store::agents::list).map_err(ApiError::from)?;
    let chain = chain_status(db)?;

    let recent: Vec<Value> = db
        .with_conn(|c| {
            store::actions::list(
                c,
                &store::actions::ActionFilter {
                    limit: 8,
                    ..Default::default()
                },
            )
        })
        .map_err(ApiError::from)?
        .into_iter()
        .map(|a| {
            json!({
                "id": a.id,
                "agent_id": a.agent_id,
                "merchant": a.merchant,
                "amount_cents": a.amount_cents,
                "currency": a.currency.as_str(),
                "decision": a.decision.as_str(),
                "risk_score": a.risk_score,
                "created_at": a.created_at,
                "explanation": a.explanation,
            })
        })
        .collect();

    let incidents: Vec<Value> = db
        .with_conn(|c| store::recovery::list_incidents(c, None, 5))
        .map_err(ApiError::from)?
        .into_iter()
        .map(|i| {
            json!({
                "id": i.id,
                "agent_id": i.agent_id,
                "status": i.status.as_str(),
                "amount_cents": i.amount_cents,
                "currency": i.currency.as_str(),
                "description": i.description,
                "created_at": i.created_at,
            })
        })
        .collect();

    Ok(json!({
        "totals": {
            "approved": approved,
            "escalated": escalated,
            "refused": refused,
            "settled_cents": settled_cents,
            "reserve_cents": reserve,
            "pending_approvals": pending,
            "agents": agents.len(),
        },
        "agents": agents,
        "recent_actions": recent,
        "incidents": incidents,
        "audit": {
            "valid": chain.valid,
            "entries": chain.entries,
            "head_hash": chain.head_hash,
            "message": chain.message,
        },
        "modes": {
            "paypal": config.paypal_mode(),
            "expert": config.expert_mode(),
        },
        "sandbox_notice": "Prototype Sandbox : agents, soldes et remboursements sont simulés. Aucun argent réel.",
    }))
}
