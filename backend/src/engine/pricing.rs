use crate::domain::money::{format_cents, Currency};
use crate::domain::wallet::{PriceLine, PriceQuote, Wallet};

/// Entrées du calcul de prime. Tout est entier : aucun arrondi flottant.
pub struct PricingInput<'a> {
    pub wallet: &'a Wallet,
    pub avg_risk_score: u8,
    pub incidents: i64,
    pub approvals: i64,
}

/// Bonus-malus en points de base (10 000 bps = 100 %).
///
/// * chaque sinistre constaté renchérit la prime ;
/// * un historique d'achats approuvés sans incident la réduit.
pub fn bonus_malus_bps(incidents: i64, approvals: i64) -> i32 {
    let malus = (incidents.clamp(0, 4) * 600) as i32; // 0 .. 2400 bps
    let bonus = -((approvals.clamp(0, 10) * 80) as i32); // 0 .. -800 bps
    (malus + bonus).clamp(-2_000, 3_000)
}

pub fn quote(input: &PricingInput<'_>) -> PriceQuote {
    let currency = Currency::Eur;
    let base = input.wallet.premium_cents.max(0);

    let bps = bonus_malus_bps(input.incidents, input.approvals);
    let risk_loading = base * input.avg_risk_score as i64 / 200; // jusqu'à +50 %
    let bonus_malus = base * bps as i64 / 10_000;
    let reserve = base / 10;

    let total = (base + risk_loading + bonus_malus + reserve).max(0);

    let lines = vec![
        PriceLine {
            code: "BASE".into(),
            label: "Prime de base".into(),
            amount_cents: base,
            explanation: "Tarif mensuel de référence du prototype, hors ajustements.".into(),
        },
        PriceLine {
            code: "RISK_LOADING".into(),
            label: "Chargement risque".into(),
            amount_cents: risk_loading,
            explanation: format!(
                "Score de risque moyen observé : {}/100. Le chargement vaut au plus 50 % de la prime de base.",
                input.avg_risk_score
            ),
        },
        PriceLine {
            code: "BONUS_MALUS".into(),
            label: if bonus_malus <= 0 { "Bonus historique".into() } else { "Malus sinistres".into() },
            amount_cents: bonus_malus,
            explanation: format!(
                "{} sinistre(s) constaté(s), {} achat(s) approuvé(s) : ajustement de {:+.2} %.",
                input.incidents,
                input.approvals,
                bps as f64 / 100.0
            ),
        },
        PriceLine {
            code: "RESERVE".into(),
            label: "Contribution réserve commune".into(),
            amount_cents: reserve,
            explanation: "10 % de la prime alimentent la réserve locale qui finance les remboursements simulés."
                .into(),
        },
    ];

    let headline = format!(
        "Prime simulée : {} (base {}, ajustements {}, réserve {}).",
        format_cents(total, currency),
        format_cents(base, currency),
        format_cents(risk_loading + bonus_malus, currency),
        format_cents(reserve, currency)
    );

    PriceQuote {
        currency,
        base_premium_cents: base,
        risk_loading_cents: risk_loading,
        bonus_malus_cents: bonus_malus,
        reserve_contribution_cents: reserve,
        total_cents: total,
        lines,
        headline,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    fn wallet() -> Wallet {
        Wallet {
            agent_id: Uuid::nil(),
            balance_cents: 12_000,
            premium_cents: 1_200,
            bonus_malus_bps: 0,
            reserve_contribution_cents: 120,
            spent_this_period_cents: 0,
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn quote_lines_add_up_to_total() {
        let w = wallet();
        let q = quote(&PricingInput {
            wallet: &w,
            avg_risk_score: 40,
            incidents: 1,
            approvals: 3,
        });
        assert_eq!(
            q.base_premium_cents
                + q.risk_loading_cents
                + q.bonus_malus_cents
                + q.reserve_contribution_cents,
            q.total_cents
        );
    }

    #[test]
    fn incidents_increase_premium() {
        let w = wallet();
        let clean = quote(&PricingInput {
            wallet: &w,
            avg_risk_score: 20,
            incidents: 0,
            approvals: 5,
        });
        let dirty = quote(&PricingInput {
            wallet: &w,
            avg_risk_score: 20,
            incidents: 3,
            approvals: 0,
        });
        assert!(dirty.total_cents > clean.total_cents);
    }
}
