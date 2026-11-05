use chrono::{DateTime, Utc};

use crate::domain::action::ProposedAction;
use crate::domain::decision::{Decision, GuardrailVerdict, RuleCheck, Thresholds};
use crate::domain::mandate::{Mandate, MandateStatus};
use crate::domain::money::{format_cents, Currency};

use super::risk;
use super::HistoryWindow;

/// Évalue une action proposée contre un mandat.
///
/// Fonction pure : aucun accès réseau, aucune horloge implicite, aucun aléa.
/// C'est ce qui rend la décision auditable et reproductible.
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

    // --- 1. Validité du mandat -------------------------------------------
    checks.push(RuleCheck::strict(
        "MANDATE_ACTIVE",
        "Mandat actif",
        mandate.status == MandateStatus::Active,
        match mandate.status {
            MandateStatus::Active => "Le mandat est actif.".to_string(),
            MandateStatus::Revoked => "Le mandat a été révoqué par le propriétaire.".to_string(),
            MandateStatus::Expired => "Le mandat est expiré.".to_string(),
        },
    ));

    let in_period = now >= mandate.valid_from && now <= mandate.valid_until;
    checks.push(RuleCheck::strict(
        "MANDATE_PERIOD",
        "Mandat dans sa période de validité",
        in_period,
        if in_period {
            format!(
                "Valide jusqu'au {}.",
                mandate.valid_until.format("%d/%m/%Y %H:%M UTC")
            )
        } else {
            format!(
                "Hors période : le mandat couvre du {} au {}.",
                mandate.valid_from.format("%d/%m/%Y %H:%M UTC"),
                mandate.valid_until.format("%d/%m/%Y %H:%M UTC")
            )
        },
    ));

    // --- 2. Nature de l'action -------------------------------------------
    checks.push(RuleCheck::strict(
        "ACTION_KIND",
        "Type d'action autorisé",
        action.kind.is_spend(),
        format!(
            "Type reçu : {}. Un mandat de dépense ne couvre que les achats et abonnements.",
            action.kind.as_str()
        ),
    ));

    // --- 3. Devise --------------------------------------------------------
    let currency_ok = currency == mandate.currency;
    checks.push(RuleCheck::strict(
        "CURRENCY_MATCH",
        "Devise identique au mandat",
        currency_ok,
        format!(
            "Action en {}, mandat en {}.",
            currency.as_str(),
            mandate.currency.as_str()
        ),
    ));

    // --- 4. Montants ------------------------------------------------------
    checks.push(RuleCheck::strict(
        "AMOUNT_POSITIVE",
        "Montant strictement positif",
        action.amount_cents > 0,
        format!("Montant reçu : {} centimes.", action.amount_cents),
    ));

    let per_txn_ok = action.amount_cents <= mandate.per_txn_max_cents;
    checks.push(RuleCheck::strict(
        "AMOUNT_PER_TXN",
        "Montant sous le plafond par transaction",
        per_txn_ok,
        format!(
            "{} demandés, plafond {}.",
            format_cents(action.amount_cents, currency),
            format_cents(mandate.per_txn_max_cents, currency)
        ),
    ));

    let projected = history.committed_cents + action.amount_cents;
    let total_ok = projected <= mandate.max_total_cents;
    checks.push(RuleCheck::strict(
        "AMOUNT_TOTAL_BUDGET",
        "Cumul sous le plafond total du mandat",
        total_ok,
        format!(
            "Déjà engagé {} + {} = {}, plafond {}.",
            format_cents(history.committed_cents, currency),
            format_cents(action.amount_cents, currency),
            format_cents(projected, currency),
            format_cents(mandate.max_total_cents, currency)
        ),
    ));

    // --- 5. Catégorie -----------------------------------------------------
    let category_ok = mandate.category.eq_ignore_ascii_case("any")
        || mandate
            .category
            .eq_ignore_ascii_case(action.category.trim());
    checks.push(RuleCheck::strict(
        "CATEGORY_ALLOWED",
        "Catégorie autorisée par le mandat",
        category_ok,
        format!(
            "Catégorie demandée « {} », mandat « {} ».",
            action.category, mandate.category
        ),
    ));

    // --- 6. Marchand ------------------------------------------------------
    let merchant_norm = action.merchant.trim().to_ascii_lowercase();
    let blocked = mandate
        .blocked_merchants
        .iter()
        .any(|m| m.trim().eq_ignore_ascii_case(&merchant_norm));
    checks.push(RuleCheck::strict(
        "MERCHANT_NOT_BLOCKED",
        "Marchand non bloqué",
        !blocked,
        if blocked {
            format!(
                "« {} » figure sur la liste des marchands bloqués.",
                action.merchant
            )
        } else {
            "Aucun blocage pour ce marchand.".to_string()
        },
    ));

    let allowlist_ok = mandate.allowed_merchants.is_empty()
        || mandate
            .allowed_merchants
            .iter()
            .any(|m| m.trim().eq_ignore_ascii_case(&merchant_norm));
    checks.push(RuleCheck::strict(
        "MERCHANT_ALLOWED",
        "Marchand autorisé par le mandat",
        allowlist_ok,
        if mandate.allowed_merchants.is_empty() {
            "Le mandat n'impose pas de liste blanche.".to_string()
        } else {
            format!(
                "Liste blanche : {}. Marchand reçu : « {} ».",
                mandate.allowed_merchants.join(", "),
                action.merchant
            )
        },
    ));

    // --- 7. Pays ----------------------------------------------------------
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
        "Pays du marchand autorisé",
        country_ok,
        if mandate.allowed_countries.is_empty() {
            "Le mandat n'impose pas de restriction de pays.".to_string()
        } else {
            format!(
                "Pays autorisés : {}, pays reçu : {}.",
                mandate.allowed_countries.join(", "),
                action
                    .merchant_country
                    .clone()
                    .unwrap_or_else(|| "non fourni".to_string())
            )
        },
    ));

    // --- 8. Délai de livraison -------------------------------------------
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
        "Livraison dans le délai exigé",
        delivery_ok,
        if mandate.max_delivery_days == 0 {
            "Le mandat n'impose pas de délai.".to_string()
        } else {
            format!(
                "Délai maximal {} jours, délai annoncé {}.",
                mandate.max_delivery_days,
                action
                    .delivery_days
                    .map(|d| format!("{d} jours"))
                    .unwrap_or_else(|| "inconnu".to_string())
            )
        },
    ));

    // --- 9. Idempotence ---------------------------------------------------
    checks.push(RuleCheck::strict(
        "IDEMPOTENCY",
        "Action non dupliquée",
        !history.duplicate_action,
        if history.duplicate_action {
            "Une action portant la même clé d'idempotence a déjà été enregistrée.".to_string()
        } else {
            "Aucune clé d'idempotence en doublon.".to_string()
        },
    ));

    // --- 10. Seuils (informatifs, ils fixent l'issue si aucune violation) --
    let auto_ok = action.amount_cents <= thresholds.auto_approve_max_cents;
    checks.push(RuleCheck::soft(
        "THRESHOLD_AUTO_APPROVE",
        "Sous le seuil d'approbation automatique",
        auto_ok,
        format!(
            "Seuil automatique : {}.",
            format_cents(thresholds.auto_approve_max_cents, currency)
        ),
    ));

    let escalate_ok = action.amount_cents <= thresholds.escalate_until_cents;
    checks.push(RuleCheck::soft(
        "THRESHOLD_ESCALATION",
        "Sous la borne d'escalade",
        escalate_ok,
        format!(
            "Borne d'escalade : {}.",
            format_cents(thresholds.escalate_until_cents, currency)
        ),
    ));

    // --- 11. Issue --------------------------------------------------------
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
        // Filet de sécurité : un montant au delà de toutes les bornes est refusé.
        Decision::Refuse
    };

    for v in &strict_violations {
        reasons.push(if v.code == "AMOUNT_PER_TXN" {
            format!(
                "Budget dépassé : {} demandés pour un plafond de {} sur ce mandat.",
                format_cents(action.amount_cents, currency),
                format_cents(mandate.per_txn_max_cents, currency)
            )
        } else if v.code == "AMOUNT_TOTAL_BUDGET" {
            format!(
                "Plafond cumulé dépassé : {} engagés sur {} autorisés.",
                format_cents(projected, currency),
                format_cents(mandate.max_total_cents, currency)
            )
        } else {
            format!("{} : {}", v.label, v.detail)
        });
    }

    if decision == Decision::Escalate {
        reasons.push(format!(
            "Montant entre {} et {} : l'action nécessite une confirmation explicite du propriétaire.",
            format_cents(thresholds.auto_approve_max_cents, currency),
            format_cents(thresholds.escalate_until_cents, currency)
        ));
    }
    if decision == Decision::Approve {
        reasons.push(format!(
            "Toutes les règles strictes du mandat sont respectées et {} est sous le seuil automatique de {}.",
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
        "{} de {} chez « {} » proposé par l'agent, mandat « {} » (plafond {}).",
        if action.kind.is_spend() {
            "Achat"
        } else {
            "Action"
        },
        format_cents(action.amount_cents, currency),
        action.merchant,
        mandate.intent,
        format_cents(mandate.max_total_cents, currency)
    );

    let verdict = match decision {
        Decision::Approve => "Décision : APPROUVER.",
        Decision::Escalate => "Décision : ESCALADER.",
        Decision::Refuse => "Décision : REFUSER.",
    };

    let detail = if reasons.is_empty() {
        String::new()
    } else {
        format!(" {}", reasons.join(" "))
    };

    format!("{head}{detail} {verdict} Score de risque : {risk_score}/100.")
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
            intent: "Casque neuf, maximum 50 EUR, livraison sous 5 jours.".into(),
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
            description: "Casque bluetooth".into(),
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
        assert!(v.reasons.iter().any(|r| r.contains("Budget dépassé")));
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
