//! AgentShield — PayPal spending guardrail for AI agents.
//!
//! Sandbox prototype for the PayPal AI Hackathon 2026. All data is
//! simulated and stored locally: no real money is involved.
//!
//! Architecture:
//! * [`domain`] — business types and monetary values stored as integer cents;
//! * [`engine`] — deterministic decisions, risk assessment, pricing, and AI expert;
//! * [`store`] — SQLite persistence and audit chain;
//! * [`paypal`] — PayPal Sandbox gateway (live or simulated);
//! * [`service`] — business logic shared by the API and the seeder;
//! * [`api`] — Axum HTTP routes.

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

/// Builds the complete application state (database, configuration, expert, PayPal).
pub fn build_state(config: Config) -> Result<AppState, String> {
    use std::sync::Arc;

    let config = Arc::new(config);
    let db = store::Db::open(&config.db_path).map_err(|e| format!("SQLite initialization: {e}"))?;

    if config.seed_demo {
        store::seed::seed_demo(&db, &config).map_err(|e| format!("Demo data seeding: {e}"))?;
    }

    let expert = Arc::new(engine::expert::ExpertService::new(&config));

    let paypal: Arc<dyn paypal::PayPalGateway> = if config.paypal_live_enabled() {
        Arc::new(
            paypal::live::SandboxGateway::new(
                config.paypal.base_url.clone(),
                config.paypal.client_id.clone().unwrap_or_default(),
                config.paypal.client_secret.clone().unwrap_or_default(),
            )
            .map_err(|e| format!("PayPal configuration: {e}"))?,
        )
    } else {
        Arc::new(paypal::mock::MockGateway::new())
    };

    Ok(AppState::new(db, config, expert, paypal))
}
