use chrono::{DateTime, Utc};

use crate::domain::action::ProposedAction;
use crate::domain::decision::{Decision, GuardrailVerdict, RuleCheck, Thresholds};
use crate::domain::mandate::{Mandate, MandateStatus};
use crate::domain::money::{format_cents, Currency};

use super::risk;
use super::HistoryWindow;

/// Evaluates a proposed action against a spending mandate.
///
/// This is a pure function: it performs no network access, uses no implicit
/// clock, and introduces no randomness. This makes every decision auditable
/// and reproducible.
pub fn evaluate(
    action: &ProposedAction,
    mandate: &Mandate,
    history: &HistoryWindow,
    now: DateTime<Utc>,
) -> GuardrailVerdict {
    let currency = action.currency.unwrap_or(mandate.currency);
    let thresholds: Thresholds = mandate.thresholds();
    let mut checks: Vec<RuleCheck> = Vec::new();
    let mut reasons: Vec<String> = Vec::new();

    // --- 1. Mandate validity ----------------------------------------------
    checks.push(RuleCheck::strict(
        "MANDATE_ACTIVE",
        "Active mandate",
        mandate.status == MandateStatus::Active,
        match mandate.status {
            MandateStatus::Active => "The mandate is active.".to_string(),
            MandateStatus::Revoked => "The mandate has been revoked by its owner.".to_string(),
            MandateStatus::Expired => "The mandate has expired.".to_string(),
        },
    ));

    let in_period = now >= mandate.valid_from && now <= mandate.valid_until;
    checks.push(RuleCheck::strict(
        "MANDATE_PERIOD",
        "Mandate validity period",
        in_period,
        if in_period {
            format!(
                "Valid until {}.",
                mandate.valid_until.format("%d/%m/%Y %H:%M UTC")
            )
        } else {
            format!(
                "Outside the validity period: the mandate covers {} to {}.",
                mandate.valid_from.format("%d/%m/%Y %H:%M UTC"),
                mandate.valid_until.format("%d/%m/%Y %H:%M UTC")
            )
        },
    ));

    // --- 2. Action type ----------------------------------------------------
    checks.push(RuleCheck::strict(
        "ACTION_KIND",
        "Authorized action type",
        action.kind.is_spend(),
        format!(
            "Received action type: {}. A spending mandate only covers purchases and subscriptions.",
            action.kind.as_str()
        ),
    ));

    // --- 3. Currency ------------------------------------------------------
    let currency_ok = currency == mandate.currency;
    checks.push(RuleCheck::strict(
        "CURRENCY_MATCH",
        "Currency matches the mandate",
        currency_ok,
        format!(
            "Action currency: {}, mandate currency: {}.",
            currency.as_str(),
            mandate.currency.as_str()
        ),
    ));

    // --- 4. Amounts -------------------------------------------------------
    checks.push(RuleCheck::strict(
        "AMOUNT_POSITIVE",
        "Amount must be positive",
        action.amount_cents > 0,
        format!("Received amount: {} cents.", action.amount_cents),
    ));

    let per_txn_ok = action.amount_cents <= mandate.per_txn_max_cents;
    checks.push(RuleCheck::strict(
        "AMOUNT_PER_TXN",
        "Amount within the per-transaction limit",
        per_txn_ok,
        format!(
            "{} requested, transaction limit: {}.",
            format_cents(action.amount_cents, currency),
            format_cents(mandate.per_txn_max_cents, currency)
        ),
    ));

    let projected = history.committed_cents + action.amount_cents;
    let total_ok = projected <= mandate.max_total_cents;
    checks.push(RuleCheck::strict(
        "AMOUNT_TOTAL_BUDGET",
        "Total spending within the mandate budget",
        total_ok,
        format!(
            "Already committed: {} + requested: {} = projected total: {}. Mandate limit: {}.",
            format_cents(history.committed_cents, currency),
            format_cents(action.amount_cents, currency),
            format_cents(projected, currency),
            format_cents(mandate.max_total_cents, currency)
        ),
    ));

    // --- 5. Category ------------------------------------------------------
    let category_ok = mandate.category.eq_ignore_ascii_case("any")
        || mandate
            .category
            .eq_ignore_ascii_case(action.category.trim());

    checks.push(RuleCheck::strict(
        "CATEGORY_ALLOWED",
        "Category authorized by the mandate",
        category_ok,
        format!(
            "Requested category: \"{}\". Mandate category: \"{}\".",
            action.category, mandate.category
        ),
    ));

    // --- 6. Merchant ------------------------------------------------------
    let merchant_norm = action.merchant.trim().to_ascii_lowercase();

    let blocked = mandate
        .blocked_merchants
        .iter()
        .any(|m| m.trim().eq_ignore_ascii_case(&merchant_norm));

    checks.push(RuleCheck::strict(
        "MERCHANT_NOT_BLOCKED",
        "Merchant is not blocked",
        !blocked,
        if blocked {
            format!("\"{}\" is on the blocked merchants list.", action.merchant)
        } else {
            "No restriction applies to this merchant.".to_string()
        },
    ));

    let allowlist_ok = mandate.allowed_merchants.is_empty()
        || mandate
            .allowed_merchants
            .iter()
            .any(|m| m.trim().eq_ignore_ascii_case(&merchant_norm));

    checks.push(RuleCheck::strict(
        "MERCHANT_ALLOWED",
        "Merchant authorized by the mandate",
        allowlist_ok,
        if mandate.allowed_merchants.is_empty() {
            "The mandate does not require a merchant allowlist.".to_string()
        } else {
            format!(
                "Merchant allowlist: {}. Received merchant: \"{}\".",
                mandate.allowed_merchants.join(", "),
                action.merchant
            )
        },
    ));

    // --- 7. Country -------------------------------------------------------
    let country_ok = if mandate.allowed_countries.is_empty() {
        true
    } else {
        match action.merchant_country.as_deref() {
            Some(c) => mandate
                .allowed_countries
                .iter()
                .any(|allowed| allowed.eq_ignore_ascii_case(c.trim())),
            None => false,
        }
    };

    checks.push(RuleCheck::strict(
        "COUNTRY_ALLOWED",
        "Merchant country is authorized",
        country_ok,
        if mandate.allowed_countries.is_empty() {
            "The mandate does not impose any country restrictions.".to_string()
        } else {
            format!(
                "Allowed countries: {}. Received country: {}.",
                mandate.allowed_countries.join(", "),
                action
                    .merchant_country
                    .clone()
                    .unwrap_or_else(|| "not provided".to_string())
            )
        },
    ));

    // --- 8. Delivery time -------------------------------------------------
    let delivery_ok = if mandate.max_delivery_days == 0 {
        true
    } else {
        match action.delivery_days {
            Some(days) => days <= mandate.max_delivery_days,
            None => false,
        }
    };

    checks.push(RuleCheck::strict(
        "DELIVERY_WINDOW",
        "Delivery within the required timeframe",
        delivery_ok,
        if mandate.max_delivery_days == 0 {
            "The mandate does not impose a delivery deadline.".to_string()
        } else {
            format!(
                "Maximum delivery time: {} days. Reported delivery time: {}.",
                mandate.max_delivery_days,
                action
                    .delivery_days
                    .map(|d| format!("{d} days"))
                    .unwrap_or_else(|| "unknown".to_string())
            )
        },
    ));

    // --- 9. Idempotency ---------------------------------------------------
    checks.push(RuleCheck::strict(
        "IDEMPOTENCY",
        "Action has not been duplicated",
        !history.duplicate_action,
        if history.duplicate_action {
            "An action with the same idempotency key has already been recorded.".to_string()
        } else {
            "No duplicate idempotency key was detected.".to_string()
        },
    ));

    // --- 10. Thresholds (informational; determine the outcome if no rule fails) ---
    let auto_ok = action.amount_cents <= thresholds.auto_approve_max_cents;

    checks.push(RuleCheck::soft(
        "THRESHOLD_AUTO_APPROVE",
        "Within the automatic approval threshold",
        auto_ok,
        format!(
            "Automatic approval threshold: {}.",
            format_cents(thresholds.auto_approve_max_cents, currency)
        ),
    ));

    let escalate_ok = action.amount_cents <= thresholds.escalate_until_cents;

    checks.push(RuleCheck::soft(
        "THRESHOLD_ESCALATION",
        "Within the escalation threshold",
        escalate_ok,
        format!(
            "Escalation threshold: {}.",
            format_cents(thresholds.escalate_until_cents, currency)
        ),
    ));

    // --- 11. Final decision ------------------------------------------------
    let strict_violations: Vec<&RuleCheck> = checks
        .iter()
        .filter(|c| !c.passed && c.severity == crate::domain::decision::CheckSeverity::Strict)
        .collect();

    let decision = if !strict_violations.is_empty() {
        Decision::Refuse
    } else if auto_ok {
        Decision::Approve
    } else if escalate_ok {
        Decision::Escalate
    } else {
        // Safety fallback: amounts above all thresholds are refused.
        Decision::Refuse
    };

    for violation in &strict_violations {
        reasons.push(if violation.code == "AMOUNT_PER_TXN" {
            format!(
                "Budget exceeded: {} requested against a mandate limit of {}.",
                format_cents(action.amount_cents, currency),
                format_cents(mandate.per_txn_max_cents, currency)
            )
        } else if violation.code == "AMOUNT_TOTAL_BUDGET" {
            format!(
                "Cumulative spending limit exceeded: {} committed out of {} authorized.",
                format_cents(projected, currency),
                format_cents(mandate.max_total_cents, currency)
            )
        } else {
            format!("{}: {}", violation.label, violation.detail)
        });
    }

    if decision == Decision::Escalate {
        reasons.push(format!(
            "Amount between {} and {}: the action requires explicit owner confirmation.",
            format_cents(thresholds.auto_approve_max_cents, currency),
            format_cents(thresholds.escalate_until_cents, currency)
        ));
    }

    if decision == Decision::Approve {
        reasons.push(format!(
            "All strict mandate rules are satisfied, and {} is below the automatic approval threshold of {}.",
            format_cents(action.amount_cents, currency),
            format_cents(thresholds.auto_approve_max_cents, currency)
        ));
    }

    let risk_assessment = risk::assess(action, mandate, history, now);

    let explanation = build_explanation(
        action,
        mandate,
        currency,
        decision,
        &reasons,
        risk_assessment.score,
    );

    GuardrailVerdict {
        decision,
        risk: risk_assessment,
        checks,
        reasons,
        explanation,
        thresholds,
    }
}

fn build_explanation(
    action: &ProposedAction,
    mandate: &Mandate,
    currency: Currency,
    decision: Decision,
    reasons: &[String],
    risk_score: u8,
) -> String {
    let head = format!(
        "{} of {} at \"{}\" proposed by the agent, under mandate \"{}\" (limit {}).",
        if action.kind.is_spend() {
            "Purchase"
        } else {
            "Action"
        },
        format_cents(action.amount_cents, currency),
        action.merchant,
        mandate.intent,
        format_cents(mandate.max_total_cents, currency)
    );

    let verdict = match decision {
        Decision::Approve => "Decision: APPROVE.",
        Decision::Escalate => "Decision: ESCALATE.",
        Decision::Refuse => "Decision: REJECT.",
    };

    let detail = if reasons.is_empty() {
        String::new()
    } else {
        format!(" {}", reasons.join(" "))
    };

    format!("{head}{detail} {verdict} Risk score: {risk_score}/100.")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::action::ActionKind;

    fn mandate() -> Mandate {
        Mandate {
            id: uuid::Uuid::nil(),
            agent_id: uuid::Uuid::nil(),
            owner_name: "Camille".into(),
            intent: "New headphones, maximum 50 EUR, delivery within 5 days.".into(),
            category: "electronics".into(),
            currency: Currency::Eur,
            max_total_cents: 5_000,
            per_txn_max_cents: 5_000,
            auto_approve_max_cents: 4_000,
            escalate_until_cents: 5_000,
            allowed_merchants: vec![],
            blocked_merchants: vec!["RogueShop".into()],
            allowed_countries: vec!["FR".into()],
            max_delivery_days: 5,
            valid_from: Utc::now() - chrono::Duration::days(1),
            valid_until: Utc::now() + chrono::Duration::days(10),
            status: MandateStatus::Active,
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
            description: "Bluetooth headphones".into(),
            idempotency_key: None,
            occurred_at: None,
            metadata: None,
        }
    }

    fn run(a: &ProposedAction, h: &HistoryWindow) -> GuardrailVerdict {
        evaluate(a, &mandate(), h, Utc::now())
    }

    #[test]
    fn approves_below_auto_threshold() {
        let v = run(&action(3_800), &HistoryWindow::default());

        assert_eq!(v.decision, Decision::Approve);
        assert!(v.all_strict_passed());
    }

    #[test]
    fn escalates_between_thresholds() {
        let v = run(&action(4_500), &HistoryWindow::default());

        assert_eq!(v.decision, Decision::Escalate);
        assert!(v.all_strict_passed());
    }

    #[test]
    fn refuses_above_mandate_ceiling() {
        let v = run(&action(6_200), &HistoryWindow::default());

        assert_eq!(v.decision, Decision::Refuse);
        assert!(v.violated_rules().contains(&"AMOUNT_PER_TXN".to_string()));
        assert!(v.reasons.iter().any(|r| r.contains("Budget exceeded")));
    }

    #[test]
    fn refuses_blocked_merchant() {
        let mut a = action(1_000);
        a.merchant = "RogueShop".into();

        let v = run(&a, &HistoryWindow::default());

        assert_eq!(v.decision, Decision::Refuse);
        assert!(v
            .violated_rules()
            .contains(&"MERCHANT_NOT_BLOCKED".to_string()));
    }

    #[test]
    fn refuses_unknown_country() {
        let mut a = action(1_000);
        a.merchant_country = None;

        let v = run(&a, &HistoryWindow::default());

        assert_eq!(v.decision, Decision::Refuse);
        assert!(v.violated_rules().contains(&"COUNTRY_ALLOWED".to_string()));
    }

    #[test]
    fn refuses_late_delivery() {
        let mut a = action(1_000);
        a.delivery_days = Some(9);

        let v = run(&a, &HistoryWindow::default());

        assert_eq!(v.decision, Decision::Refuse);
        assert!(v.violated_rules().contains(&"DELIVERY_WINDOW".to_string()));
    }

    #[test]
    fn refuses_wrong_currency() {
        let mut a = action(1_000);
        a.currency = Some(Currency::Usd);

        let v = run(&a, &HistoryWindow::default());

        assert_eq!(v.decision, Decision::Refuse);
        assert!(v.violated_rules().contains(&"CURRENCY_MATCH".to_string()));
    }

    #[test]
    fn refuses_when_cumulative_budget_exhausted() {
        let history = HistoryWindow {
            committed_cents: 4_800,
            ..Default::default()
        };

        let v = run(&action(1_000), &history);

        assert_eq!(v.decision, Decision::Refuse);
        assert!(v
            .violated_rules()
            .contains(&"AMOUNT_TOTAL_BUDGET".to_string()));
    }

    #[test]
    fn refuses_duplicate() {
        let history = HistoryWindow {
            duplicate_action: true,
            ..Default::default()
        };

        let v = run(&action(1_000), &history);

        assert_eq!(v.decision, Decision::Refuse);
    }

    #[test]
    fn refuses_revoked_mandate() {
        let mut m = mandate();
        m.status = MandateStatus::Revoked;

        let v = evaluate(&action(1_000), &m, &HistoryWindow::default(), Utc::now());

        assert_eq!(v.decision, Decision::Refuse);
        assert!(v.violated_rules().contains(&"MANDATE_ACTIVE".to_string()));
    }

    #[test]
    fn decision_is_reproducible() {
        let a = action(4_500);
        let now = Utc::now();
        let m = mandate();

        let first = evaluate(&a, &m, &HistoryWindow::default(), now);
        let second = evaluate(&a, &m, &HistoryWindow::default(), now);

        assert_eq!(first.decision, second.decision);
        assert_eq!(first.risk.score, second.risk.score);
        assert_eq!(first.explanation, second.explanation);
    }
}
