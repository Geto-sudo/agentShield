//! Moteur de décision d'AgentShield.
//!
//! Séparation stricte des responsabilités :
//! * [`guardrail`] et [`risk`] sont **déterministes** et font autorité ;
//! * [`pricing`] explique un prix à partir de règles fixes ;
//! * [`expert`] peut *expliquer* et *qualifier*, jamais fixer un montant.

pub mod expert;
pub mod guardrail;
pub mod pricing;
pub mod risk;

/// Fenêtre d'historique injectée dans le moteur pour rester une fonction pure.
#[derive(Debug, Clone, Default)]
pub struct HistoryWindow {
    /// Somme des actions déjà approuvées ou escaladées pour ce mandat.
    pub committed_cents: i64,
    /// Nombre d'actions proposées par cet agent dans les 15 dernières minutes.
    pub recent_actions_15min: i64,
    /// Marchands déjà vus pour cet agent (minuscules).
    pub known_merchants: Vec<String>,
    /// Une action portant la même clé d'idempotence existe déjà.
    pub duplicate_action: bool,
    /// Nombre de sinistres ouverts pour cet agent.
    pub open_incidents: i64,
}

impl HistoryWindow {
    pub fn merchant_is_known(&self, merchant: &str) -> bool {
        let needle = merchant.trim().to_ascii_lowercase();
        self.known_merchants.iter().any(|m| m == &needle)
    }
}
