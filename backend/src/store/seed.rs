//! Demonstration dataset.
//!
//! The seeder uses the `service` layer, so the demo cannot create a state
//! that the API itself would reject.

use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

use crate::config::Config;
use crate::domain::action::{ActionKind, ProposedAction};
use crate::domain::mandate::CreateMandateRequest;
use crate::domain::money::Currency;
use crate::error::{ApiError, ApiResult};
use crate::service;
use crate::store::{self, Db};

/// Does nothing if agents already exist.
pub fn seed_demo(db: &Db, config: &Config) -> ApiResult<()> {
    let agents = db.with_conn(store::agents::count).map_err(ApiError::from)?;

    if agents > 0 {
        return Ok(());
    }

    tracing::info!("empty database: creating the AgentShield demonstration dataset");

    // --- Agent 1: Atlas Shopper, used in the interactive demo ---------------

    let (atlas, _) = service::create_agent(
        db,
        "Atlas Shopper",
        Some("Camille Renard"),
        Some("Searches for and purchases audio equipment for its owner."),
    )?;

    let (atlas_mandate, _) = service::create_mandate(
        db,
        config,
        CreateMandateRequest {
            agent_id: atlas.id,
            owner_name: "Camille Renard".to_string(),
            intent: "New headphones, maximum 50 EUR, delivery within 5 days.".to_string(),
            category: "electronics".to_string(),
            max_total_cents: 20_000,
            currency: Some(Currency::Eur),
            per_txn_max_cents: Some(5_000),
            auto_approve_max_cents: Some(4_000),
            escalate_until_cents: Some(5_000),
            allowed_merchants: Some(vec!["TechStore SAS".to_string(), "AudioLab".to_string()]),
            blocked_merchants: Some(vec!["RogueShop".to_string()]),
            allowed_countries: Some(vec!["FR".to_string()]),
            max_delivery_days: Some(5),
            valid_days: Some(30),
        },
    )?;

    db.with_tx(|tx| {
        store::wallet::ensure(tx, atlas.id, 1_200, 12_000)?;
        Ok(())
    })
    .map_err(ApiError::from)?;

    // --- Agent 2: Nova Ops, with preloaded history ---------------------------

    let (nova, _) = service::create_agent(
        db,
        "Nova Ops",
        Some("Camille Renard"),
        Some("Manages the team's recurring software subscriptions."),
    )?;

    let (nova_mandate, _) = service::create_mandate(
        db,
        config,
        CreateMandateRequest {
            agent_id: nova.id,
            owner_name: "Camille Renard".to_string(),
            intent: "SaaS subscriptions only, 90 EUR per month in total, 30 EUR per subscription."
                .to_string(),
            category: "subscription".to_string(),
            max_total_cents: 9_000,
            currency: Some(Currency::Eur),
            per_txn_max_cents: Some(3_000),
            auto_approve_max_cents: Some(2_000),
            escalate_until_cents: Some(3_000),
            allowed_merchants: None,
            blocked_merchants: None,
            allowed_countries: Some(vec!["FR".to_string(), "IE".to_string()]),
            max_delivery_days: Some(0),
            valid_days: Some(30),
        },
    )?;

    db.with_tx(|tx| {
        store::wallet::ensure(tx, nova.id, 900, 8_000)?;
        Ok(())
    })
    .map_err(ApiError::from)?;

    // --- Agent 3: Rogue Agent, compromised -----------------------------------

    let (rogue, _) = service::create_agent(
        db,
        "Rogue Agent",
        Some("Camille Renard"),
        Some("Legacy test agent retained for incident scenarios."),
    )?;

    let (rogue_mandate, _) = service::create_mandate(
        db,
        config,
        CreateMandateRequest {
            agent_id: rogue.id,
            owner_name: "Camille Renard".to_string(),
            intent: "Office supplies, maximum 50 EUR.".to_string(),
            category: "office".to_string(),
            max_total_cents: 5_000,
            currency: Some(Currency::Eur),
            per_txn_max_cents: Some(5_000),
            auto_approve_max_cents: Some(4_000),
            escalate_until_cents: Some(5_000),
            allowed_merchants: None,
            blocked_merchants: None,
            allowed_countries: Some(vec!["FR".to_string()]),
            max_delivery_days: Some(7),
            valid_days: Some(30),
        },
    )?;

    db.with_tx(|tx| {
        store::wallet::ensure(tx, rogue.id, 1_500, 3_000)?;
        Ok(())
    })
    .map_err(ApiError::from)?;

    // --- Nova Ops history ----------------------------------------------------

    propose_seed(
        db,
        config,
        nova_mandate.id,
        nova.id,
        "SaaSLicence.io",
        Some("FR"),
        1_990,
        "subscription",
        None,
        "Annual license — team plan (demonstration)",
    )?;

    propose_seed(
        db,
        config,
        nova_mandate.id,
        nova.id,
        "CloudInvoice SAS",
        Some("FR"),
        2_500,
        "subscription",
        None,
        "Electronic invoicing subscription (demonstration)",
    )?;

    // --- Rogue Agent scenario -----------------------------------------------

    service::spawn_rogue_scenario(db, config, rogue.id)?;

    tracing::info!(
        atlas_mandate = %atlas_mandate.id,
        nova_mandate = %nova_mandate.id,
        rogue_mandate = %rogue_mandate.id,
        "demonstration dataset ready"
    );

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn propose_seed(
    db: &Db,
    config: &Config,
    mandate_id: Uuid,
    agent_id: Uuid,
    merchant: &str,
    country: Option<&str>,
    amount_cents: i64,
    category: &str,
    delivery_days: Option<u32>,
    description: &str,
) -> ApiResult<()> {
    let proposed = ProposedAction {
        mandate_id,
        agent_id: Some(agent_id),
        kind: ActionKind::Purchase,
        merchant: merchant.to_string(),
        merchant_country: country.map(str::to_string),
        amount_cents,
        currency: Some(Currency::Eur),
        category: category.to_string(),
        delivery_days,
        description: description.to_string(),
        idempotency_key: Some(format!("seed-{}", Uuid::new_v4().simple())),
        occurred_at: Some(Utc::now()),
        metadata: Some(json!({ "seeded": true })),
    };

    service::propose(db, config, proposed)?;

    Ok(())
}
