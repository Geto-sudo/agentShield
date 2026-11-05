use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::money::Currency;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MandateStatus {
    #[default]
    Active,
    Revoked,
    Expired,
}

impl MandateStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            MandateStatus::Active => "active",
            MandateStatus::Revoked => "revoked",
            MandateStatus::Expired => "expired",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "revoked" => MandateStatus::Revoked,
            "expired" => MandateStatus::Expired,
            _ => MandateStatus::Active,
        }
    }
}

/// Le mandat est la seule source d'autorité. Il est fixé par l'humain *avant*
/// que l'agent propose quoi que ce soit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mandate {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub owner_name: String,
    /// Formulation libre du propriétaire, affichée dans l'audit.
    pub intent: String,
    /// Catégorie d'achat autorisée (comparaison insensible à la casse).
    pub category: String,
    pub currency: Currency,
    /// Plafond cumulé sur la durée du mandat.
    pub max_total_cents: i64,
    /// Plafond par transaction.
    pub per_txn_max_cents: i64,
    /// En dessous (ou égal), la règle déterministe approuve.
    pub auto_approve_max_cents: i64,
    /// Au dessus de `auto_approve_max_cents` et jusqu'à cette borne : escalade.
    /// Au delà : refus.
    pub escalate_until_cents: i64,
    /// Liste blanche de marchands. Vide = tout marchand non bloqué.
    pub allowed_merchants: Vec<String>,
    pub blocked_merchants: Vec<String>,
    /// Liste blanche de pays (`FR`, `EU`...). Vide = pas de contrôle.
    pub allowed_countries: Vec<String>,
    /// Délai de livraison maximal accepté, en jours. `0` = pas de contrôle.
    pub max_delivery_days: u32,
    pub valid_from: DateTime<Utc>,
    pub valid_until: DateTime<Utc>,
    pub status: MandateStatus,
    pub created_at: DateTime<Utc>,
}

impl Mandate {
    pub fn is_active_at(&self, now: DateTime<Utc>) -> bool {
        self.status == MandateStatus::Active && now >= self.valid_from && now <= self.valid_until
    }

    /// Les seuils réellement appliqués, bornés par le plafond du mandat.
    pub fn thresholds(&self) -> super::decision::Thresholds {
        super::decision::Thresholds {
            auto_approve_max_cents: self
                .auto_approve_max_cents
                .min(self.per_txn_max_cents)
                .min(self.escalate_until_cents),
            escalate_until_cents: self.escalate_until_cents.min(self.per_txn_max_cents),
        }
    }
}

/// Corps de `POST /api/mandates`. Tous les champs de politique sont optionnels
/// et retombent sur des valeurs dérivées du plafond total.
#[derive(Debug, Clone, Deserialize)]
pub struct CreateMandateRequest {
    pub agent_id: Uuid,
    pub owner_name: String,
    pub intent: String,
    pub category: String,
    pub max_total_cents: i64,
    #[serde(default)]
    pub currency: Option<Currency>,
    #[serde(default)]
    pub per_txn_max_cents: Option<i64>,
    #[serde(default)]
    pub auto_approve_max_cents: Option<i64>,
    #[serde(default)]
    pub escalate_until_cents: Option<i64>,
    #[serde(default)]
    pub allowed_merchants: Option<Vec<String>>,
    #[serde(default)]
    pub blocked_merchants: Option<Vec<String>>,
    #[serde(default)]
    pub allowed_countries: Option<Vec<String>>,
    #[serde(default)]
    pub max_delivery_days: Option<u32>,
    #[serde(default)]
    pub valid_days: Option<i64>,
}
