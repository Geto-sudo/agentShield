use chrono::{Timelike, Utc};

use crate::domain::action::ProposedAction;
use crate::domain::decision::{RiskAssessment, RiskBand, RiskFactor};
use crate::domain::mandate::Mandate;

use super::HistoryWindow;

/// Score de risque 0-100, entièrement déterministe.
///
/// Le score ne change **jamais** l'issue décidée par les règles strictes : il
/// sert à trier, à expliquer et à alimenter la prime simulée.
pub fn assess(
    action: &ProposedAction,
    mandate: &Mandate,
    history: &HistoryWindow,
    now_chrono: chrono::DateTime<Utc>,
) -> RiskAssessment {
    let mut factors: Vec<RiskFactor> = Vec::new();

    // 1. Poids du montant dans le plafond par transaction (0-25).
    if mandate.per_txn_max_cents > 0 {
        let ratio_bp = (action.amount_cents.max(0) * 10_000) / mandate.per_txn_max_cents;
        let points = (ratio_bp.clamp(0, 10_000) * 25 / 10_000) as i32;
        factors.push(RiskFactor {
            code: "AMOUNT_INTENSITY".into(),
            label: "Part du plafond par transaction consommée".into(),
            points,
        });
    }

    // 2. Franchissement du seuil d'approbation automatique.
    if action.amount_cents > mandate.thresholds().auto_approve_max_cents {
        factors.push(RiskFactor {
            code: "ABOVE_AUTO_APPROVE".into(),
            label: "Au dessus du seuil d'approbation automatique".into(),
            points: 15,
        });
    }

    // 3. Marchand inconnu pour cet agent.
    if !history.merchant_is_known(&action.merchant) {
        factors.push(RiskFactor {
            code: "NEW_MERCHANT".into(),
            label: "Marchand jamais vu pour cet agent".into(),
            points: 12,
        });
    }

    // 4. Vélocité : un agent qui propose beaucoup en peu de temps.
    if history.recent_actions_15min >= 5 {
        factors.push(RiskFactor {
            code: "VELOCITY_HIGH".into(),
            label: "Plus de 5 actions en 15 minutes".into(),
            points: 20,
        });
    } else if history.recent_actions_15min >= 3 {
        factors.push(RiskFactor {
            code: "VELOCITY_MEDIUM".into(),
            label: "Plus de 3 actions en 15 minutes".into(),
            points: 10,
        });
    }

    // 5. Consommation cumulée du budget du mandat (0-20).
    if mandate.max_total_cents > 0 {
        let projected = history.committed_cents.max(0) + action.amount_cents.max(0);
        let ratio_bp = (projected * 10_000) / mandate.max_total_cents;
        let points = (ratio_bp.clamp(0, 10_000) * 20 / 10_000) as i32;
        factors.push(RiskFactor {
            code: "BUDGET_CONSUMPTION".into(),
            label: "Consommation cumulée du budget".into(),
            points,
        });
    }

    // 6. Tentative de doublon.
    if history.duplicate_action {
        factors.push(RiskFactor {
            code: "DUPLICATE_ATTEMPT".into(),
            label: "Rejeu d'une action déjà soumise".into(),
            points: 30,
        });
    }

    // 7. Sinistres ouverts pour cet agent.
    if history.open_incidents > 0 {
        factors.push(RiskFactor {
            code: "OPEN_INCIDENTS".into(),
            label: "Sinistre(s) en cours pour cet agent".into(),
            points: (history.open_incidents.min(3) * 5) as i32,
        });
    }

    // 8. Heure atypique (entre 00:00 et 06:00 UTC).
    let hour = now_chrono.hour();
    if hour < 6 {
        factors.push(RiskFactor {
            code: "ODD_HOUR".into(),
            label: "Action proposée pendant la nuit (UTC)".into(),
            points: 5,
        });
    }

    let raw: i32 = factors.iter().map(|f| f.points).sum();
    let score = raw.clamp(0, 100) as u8;

    RiskAssessment {
        score,
        band: RiskBand::from_score(score),
        factors,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::action::ActionKind;
    use crate::domain::money::Currency;
    use chrono::TimeZone;

    fn mandate() -> Mandate {
        Mandate {
            id: uuid::Uuid::nil(),
            agent_id: uuid::Uuid::nil(),
            owner_name: "Test".into(),
            intent: "Casque".into(),
            category: "electronics".into(),
            currency: Currency::Eur,
            max_total_cents: 5_000,
            per_txn_max_cents: 5_000,
            auto_approve_max_cents: 4_000,
            escalate_until_cents: 5_000,
            allowed_merchants: vec![],
            blocked_merchants: vec![],
            allowed_countries: vec![],
            max_delivery_days: 5,
            valid_from: Utc::now(),
            valid_until: Utc::now(),
            status: crate::domain::mandate::MandateStatus::Active,
            created_at: Utc::now(),
        }
    }

    fn action(amount: i64) -> ProposedAction {
        ProposedAction {
            mandate_id: uuid::Uuid::nil(),
            agent_id: None,
            kind: ActionKind::Purchase,
            merchant: "TechStore SAS".into(),
            merchant_country: Some("FR".into()),
            amount_cents: amount,
            currency: Some(Currency::Eur),
            category: "electronics".into(),
            delivery_days: Some(4),
            description: "Casque".into(),
            idempotency_key: None,
            occurred_at: None,
            metadata: None,
        }
    }

    #[test]
    fn small_spend_on_known_merchant_is_low_risk() {
        let now = Utc.with_ymd_and_hms(2026, 3, 10, 14, 0, 0).unwrap();
        let history = HistoryWindow {
            known_merchants: vec!["techstore sas".into()],
            ..Default::default()
        };
        let risk = assess(&action(1_000), &mandate(), &history, now);
        assert!(risk.score < 40, "score {}", risk.score);
    }

    #[test]
    fn big_spend_on_unknown_merchant_is_riskier() {
        let now = Utc.with_ymd_and_hms(2026, 3, 10, 14, 0, 0).unwrap();
        let risk = assess(&action(4_900), &mandate(), &HistoryWindow::default(), now);
        assert!(risk.score > 50, "score {}", risk.score);
        assert_eq!(risk.band, RiskBand::High);
    }

    #[test]
    fn score_is_capped_at_100() {
        let now = Utc.with_ymd_and_hms(2026, 3, 10, 2, 0, 0).unwrap();
        let history = HistoryWindow {
            committed_cents: 5_000,
            recent_actions_15min: 9,
            known_merchants: vec![],
            duplicate_action: true,
            open_incidents: 3,
        };
        let risk = assess(&action(9_999), &mandate(), &history, now);
        assert!(risk.score <= 100);
    }
}
