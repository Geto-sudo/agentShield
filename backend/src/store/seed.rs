//! Jeu de démonstration.
//!
//! Le seeder utilise la couche `service` : la démo ne peut donc pas créer un
//! état que l'API elle-même rendrait impossible.

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

/// Ne fait rien si des agents existent déjà.
pub fn seed_demo(db: &Db, config: &Config) -> ApiResult<()> {
    let agents = db.with_conn(store::agents::count).map_err(ApiError::from)?;
    if agents > 0 {
        return Ok(());
    }

    tracing::info!("base vide : création du jeu de démonstration AgentShield");

    // --- Agent 1 : Atlas Shopper, celui de la démo interactive -----------
    let (atlas, _) = service::create_agent(
        db,
        "Atlas Shopper",
        Some("Camille Renard"),
        Some("Recherche et achète du matériel audio pour son propriétaire."),
    )?;

    let (atlas_mandate, _) = service::create_mandate(
        db,
        config,
        CreateMandateRequest {
            agent_id: atlas.id,
            owner_name: "Camille Renard".to_string(),
            intent: "Casque neuf, maximum 50 EUR, livraison sous 5 jours.".to_string(),
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

    // --- Agent 2 : Nova Ops, historique déjà chargé ----------------------
    let (nova, _) = service::create_agent(
        db,
        "Nova Ops",
        Some("Camille Renard"),
        Some("Gère les abonnements logiciels récurrents de l'équipe."),
    )?;

    let (nova_mandate, _) = service::create_mandate(
        db,
        config,
        CreateMandateRequest {
            agent_id: nova.id,
            owner_name: "Camille Renard".to_string(),
            intent: "Abonnements SaaS uniquement, 90 EUR par mois au total, 30 EUR par abonnement."
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

    // --- Agent 3 : Rogue Agent, compromis ---------------------------------
    let (rogue, _) = service::create_agent(
        db,
        "Rogue Agent",
        Some("Camille Renard"),
        Some("Ancien agent de test conservé pour les scénarios de sinistre."),
    )?;

    let (rogue_mandate, _) = service::create_mandate(
        db,
        config,
        CreateMandateRequest {
            agent_id: rogue.id,
            owner_name: "Camille Renard".to_string(),
            intent: "Accessoires de bureau, 50 EUR maximum.".to_string(),
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

    // --- Historique de Nova Ops ------------------------------------------
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
        "Licence annuelle — plan équipe (démonstration)",
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
        "Abonnement facturation électronique (démonstration)",
    )?;

    // --- Scénario « action rogue » pour Rogue Agent ----------------------
    service::spawn_rogue_scenario(db, config, rogue.id)?;

    tracing::info!(
        atlas_mandate = %atlas_mandate.id,
        nova_mandate = %nova_mandate.id,
        rogue_mandate = %rogue_mandate.id,
        "jeu de démonstration prêt"
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
