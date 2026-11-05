//! AgentShield — garde-fou de dépenses PayPal pour agents IA.
//!
//! Prototype Sandbox du PayPal AI Hackathon 2026. Toutes les données sont
//! simulées et locales : aucun argent réel n'est mobilisé.
//!
//! Organisation :
//! * [`domain`] — types métier, argent en centimes entiers ;
//! * [`engine`] — décision déterministe, risque, prime, expert IA ;
//! * [`store`] — persistance SQLite et chaîne d'audit ;
//! * [`paypal`] — passerelle Sandbox (réelle ou simulée) ;
//! * [`service`] — logique métier réutilisée par l'API et le seeder ;
//! * [`api`] — routes HTTP axum.

pub mod api;
pub mod config;
pub mod domain;
pub mod engine;
pub mod error;
pub mod paypal;
pub mod service;
pub mod state;
pub mod store;

pub use config::Config;
pub use state::AppState;

/// Assemble l'état applicatif complet (base, config, expert, PayPal).
pub fn build_state(config: Config) -> Result<AppState, String> {
    use std::sync::Arc;

    let config = Arc::new(config);

    // La clé d'audit doit être installée avant toute écriture : le seed de
    // démonstration produit lui aussi des entrées signées. `init_signing_key`
    // journalise la clé publique (jamais la graine privée).
    store::audit::init_signing_key(config.audit_signing_key.as_deref())
        .map_err(|e| format!("clé de signature d'audit : {e}"))?;

    let db = store::Db::open(&config.db_path).map_err(|e| format!("ouverture SQLite : {e}"))?;

    if config.seed_demo {
        store::seed::seed_demo(&db, &config).map_err(|e| format!("seed de démonstration : {e}"))?;
    }

    let expert = Arc::new(engine::expert::ExpertService::new(&config));

    let paypal: Arc<dyn paypal::PayPalGateway> = if config.paypal_live_enabled() {
        Arc::new(
            paypal::live::SandboxGateway::new(
                config.paypal.base_url.clone(),
                config.paypal.client_id.clone().unwrap_or_default(),
                config.paypal.client_secret.clone().unwrap_or_default(),
            )
            .map_err(|e| format!("configuration PayPal : {e}"))?,
        )
    } else {
        Arc::new(paypal::mock::MockGateway::new())
    };

    Ok(AppState::new(db, config, expert, paypal))
}
