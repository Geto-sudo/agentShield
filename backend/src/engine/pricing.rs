use crate::domain::money::{format_cents, Currency};
use crate::domain::wallet::{PriceLine, PriceQuote, Wallet};

/// Inputs used to calculate the premium.
/// All values are integers: no floating-point rounding is involved.
pub struct PricingInput<'a> {
    pub wallet: &'a Wallet,
    pub avg_risk_score: u8,
    pub incidents: i64,
    pub approvals: i64,
}

/// Bonus-malus calculation in basis points (10,000 bps = 100%).
///
/// * Each recorded incident increases the premium.
/// * A history of approved purchases without incidents reduces it.
pub fn bonus_malus_bps(incidents: i64, approvals: i64) -> i32 {
    let malus = (incidents.clamp(0, 4) * 600) as i32; // 0 .. 2400 bps
    let bonus = -((approvals.clamp(0, 10) * 80) as i32); // 0 .. -800 bps
    (malus + bonus).clamp(-2_000, 3_000)
}

pub fn quote(input: &PricingInput<'_>) -> PriceQuote {
    let currency = Currency::Eur;
    let base = input.wallet.premium_cents.max(0);

    let bps = bonus_malus_bps(input.incidents, input.approvals);
    let risk_loading = base * input.avg_risk_score as i64 / 200; // Up to +50%
    let bonus_malus = base * bps as i64 / 10_000;
    let reserve = base / 10;

    let total = (base + risk_loading + bonus_malus + reserve).max(0);

    let lines = vec![
        PriceLine {
            code: "BASE".into(),
            label: "Base premium".into(),
            amount_cents: base,
            explanation: "Prototype reference monthly rate, excluding adjustments.".into(),
        },
        PriceLine {
            code: "RISK_LOADING".into(),
            label: "Risk loading".into(),
            amount_cents: risk_loading,
            explanation: format!(
                "Average observed risk score: {}/100. Risk loading is capped at 50% of the base premium.",
                input.avg_risk_score
            ),
        },
        PriceLine {
            code: "BONUS_MALUS".into(),
            label: if bonus_malus <= 0 {
                "Historical bonus".into()
            } else {
                "Incident penalty".into()
            },
            amount_cents: bonus_malus,
            explanation: format!(
                "{} incident(s) recorded, {} approved purchase(s): adjustment of {:+.2}%.",
                input.incidents,
                input.approvals,
                bps as f64 / 100.0
            ),
        },
        PriceLine {
            code: "RESERVE".into(),
            label: "Shared reserve contribution".into(),
            amount_cents: reserve,
            explanation: "10% of the premium contributes to the local reserve used to fund simulated refunds."
                .into(),
        },
    ];

    let headline = format!(
        "Simulated premium: {} (base {}, adjustments {}, reserve {}).",
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
