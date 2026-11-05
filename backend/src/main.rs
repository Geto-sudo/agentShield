use std::net::SocketAddr;

use agentshield::{api, build_state, config::Config};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = dotenvy::dotenv();

    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,agentshield=debug")),
        )
        .with(tracing_subscriber::fmt::layer().with_target(false))
        .init();

    let config = Config::from_env();
    let addr: SocketAddr = config
        .addr
        .parse()
        .map_err(|e| format!("AGENTSHIELD_ADDR invalide : {e}"))?;
    let mode_paypal = config.paypal_mode();
    let mode_expert = config.expert_mode();
    let db_path = config.db_path.clone();

    let state = build_state(config)?;
    let audit_key = agentshield::store::audit::signing_info();
    let app = api::router(state);

    tracing::info!("AgentShield — http://{addr}");
    tracing::info!("SQLite      : {db_path}");
    tracing::info!("PayPal      : {mode_paypal}");
    tracing::info!("Expert      : {mode_expert}");
    tracing::info!("Audit       : {} ({})", audit_key.algorithm, audit_key.key_source);
    tracing::info!("Prototype Sandbox : données simulées, aucun argent réel.");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{signal, SignalKind};
        if let Ok(mut sig) = signal(SignalKind::terminate()) {
            sig.recv().await;
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("Arrêt demandé, fermeture propre.");
}
