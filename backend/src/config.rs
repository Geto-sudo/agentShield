use std::env;

/// Configuration lue depuis l'environnement (`.env` supporté).
#[derive(Debug, Clone)]
pub struct Config {
    pub addr: String,
    pub db_path: String,
    pub seed_demo: bool,
    /// Seuil d'approbation automatique par défaut, en centimes.
    pub default_auto_approve_max_cents: i64,
    pub approval_ttl_minutes: i64,
    /// URL publique utilisée pour les retours de checkout PayPal.
    pub public_base_url: String,
    /// Adresse (fictive) qui reçoit les payouts Sandbox dans la démo.
    pub owner_payout_email: String,
    pub openai: OpenAiConfig,
    pub paypal: PayPalConfig,
}

#[derive(Debug, Clone)]
pub struct OpenAiConfig {
    pub api_key: Option<String>,
    pub model: String,
    pub base_url: String,
    pub timeout_secs: u64,
}

#[derive(Debug, Clone)]
pub struct PayPalConfig {
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub base_url: String,
    /// Force le mode simulé même si des identifiants sont présents.
    pub force_mock: bool,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            addr: env_or("AGENTSHIELD_ADDR", "127.0.0.1:8080"),
            db_path: env_or("AGENTSHIELD_DB", "data/agentshield.db"),
            seed_demo: env_flag("AGENTSHIELD_SEED", true),
            default_auto_approve_max_cents: env_i64("AGENTSHIELD_AUTO_APPROVE_MAX_CENTS", 4_000),
            approval_ttl_minutes: env_i64("AGENTSHIELD_APPROVAL_TTL_MINUTES", 30),
            public_base_url: env_or("AGENTSHIELD_PUBLIC_URL", "http://127.0.0.1:8080"),
            owner_payout_email: env_or("AGENTSHIELD_PAYOUT_EMAIL", "sandbox-owner@example.com"),
            openai: OpenAiConfig {
                api_key: env_opt("OPENAI_API_KEY"),
                model: env_or("OPENAI_MODEL", "gpt-4o-mini"),
                base_url: env_or("OPENAI_BASE_URL", "https://api.openai.com/v1"),
                timeout_secs: env_i64("OPENAI_TIMEOUT_SECS", 20).max(1) as u64,
            },
            paypal: PayPalConfig {
                client_id: env_opt("PAYPAL_CLIENT_ID"),
                client_secret: env_opt("PAYPAL_CLIENT_SECRET"),
                base_url: env_or("PAYPAL_BASE_URL", "https://api-m.sandbox.paypal.com"),
                force_mock: env_flag("PAYPAL_FORCE_MOCK", false),
            },
        }
    }

    pub fn openai_enabled(&self) -> bool {
        self.openai
            .api_key
            .as_deref()
            .map(|k| !k.trim().is_empty())
            .unwrap_or(false)
    }

    pub fn paypal_live_enabled(&self) -> bool {
        !self.paypal.force_mock
            && self
                .paypal
                .client_id
                .as_deref()
                .map(|v| !v.trim().is_empty())
                .unwrap_or(false)
            && self
                .paypal
                .client_secret
                .as_deref()
                .map(|v| !v.trim().is_empty())
                .unwrap_or(false)
    }

    /// Mode PayPal exposé dans les réponses API et l'interface.
    pub fn paypal_mode(&self) -> &'static str {
        if self.paypal_live_enabled() {
            "live-sandbox"
        } else {
            "mock"
        }
    }

    pub fn expert_mode(&self) -> &'static str {
        if self.openai_enabled() {
            "openai"
        } else {
            "deterministic-fallback"
        }
    }
}

fn env_opt(key: &str) -> Option<String> {
    match env::var(key) {
        Ok(v) if !v.trim().is_empty() => Some(v.trim().to_string()),
        _ => None,
    }
}

fn env_or(key: &str, default: &str) -> String {
    env_opt(key).unwrap_or_else(|| default.to_string())
}

fn env_i64(key: &str, default: i64) -> i64 {
    env_opt(key)
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(default)
}

fn env_flag(key: &str, default: bool) -> bool {
    match env_opt(key) {
        Some(v) => matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"),
        None => default,
    }
}
