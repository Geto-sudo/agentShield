use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::config::Config;
use crate::domain::decision::GuardrailVerdict;
use crate::domain::mandate::Mandate;
use crate::domain::money::format_cents;
use crate::domain::recovery::ExpertVerdict;

/// Tout ce que l'expert reçoit. Aucun secret, aucune donnée bancaire.
pub struct ExpertInput {
    pub mandate: Mandate,
    pub action_log: serde_json::Value,
    pub guardrail: GuardrailVerdict,
    pub amount_cents: i64,
}

pub const RESOLUTION_PAYOUT: &str = "PAYOUT_OVERAGE";
pub const RESOLUTION_REJECT: &str = "REJECT_CLAIM";
pub const RESOLUTION_REVIEW: &str = "REQUEST_HUMAN_REVIEW";

const SYSTEM_PROMPT: &str = "Tu es l'expert sinistres d'AgentShield, un prototype Sandbox \
utilisé dans un hackathon. Tu compares un mandat de dépense fixé par un humain et le log JSON \
d'une action proposée par un agent IA. Tu ne décides jamais d'un montant à payer : tu qualifies \
l'écart entre le mandat et le log, et tu recommandes une résolution. Réponds UNIQUEMENT en JSON \
avec les clés : verdict (\"UPHOLD\", \"FLAG\" ou \"INCONCLUSIVE\"), confidence (nombre 0-1), \
summary_fr (2 phrases maximum, en français), cited_rules (codes de règles violés), \
recommended_resolution (\"PAYOUT_OVERAGE\", \"REJECT_CLAIM\" ou \"REQUEST_HUMAN_REVIEW\"). \
\"UPHOLD\" signifie que la dépense respectait le mandat. \"FLAG\" signifie qu'une règle du mandat \
a été violée. Tu n'inventes jamais de règle : tu te fondes sur les codes fournis.";

#[derive(Debug, Default, Serialize, Deserialize)]
struct LlmVerdict {
    #[serde(default)]
    verdict: String,
    #[serde(default)]
    confidence: f64,
    #[serde(default)]
    summary_fr: String,
    #[serde(default)]
    cited_rules: Vec<String>,
    #[serde(default)]
    recommended_resolution: String,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    #[serde(default)]
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

#[derive(Debug, Deserialize)]
struct ChatMessage {
    #[serde(default)]
    content: Option<String>,
}

/// Service expert : OpenAI si une clé est configurée, sinon repli déterministe.
/// `judge` ne renvoie jamais d'erreur : une panne du modèle ne casse pas la décision.
pub struct ExpertService {
    client: Option<reqwest::Client>,
    api_key: Option<String>,
    model: String,
    base_url: String,
    pub mode: &'static str,
    pub enabled: bool,
}

impl ExpertService {
    pub fn new(config: &Config) -> Self {
        let enabled = config.openai_enabled();
        let client = if enabled {
            reqwest::Client::builder()
                .timeout(Duration::from_secs(config.openai.timeout_secs))
                .build()
                .ok()
        } else {
            None
        };
        let usable = enabled && client.is_some();

        Self {
            client,
            api_key: config.openai.api_key.clone(),
            model: config.openai.model.clone(),
            base_url: config.openai.base_url.trim_end_matches('/').to_string(),
            mode: config.expert_mode(),
            enabled: usable,
        }
    }

    pub async fn judge(&self, input: &ExpertInput) -> ExpertVerdict {
        let enforced = deterministic_verdict(input);

        if self.enabled {
            if let (Some(client), Some(key)) = (self.client.as_ref(), self.api_key.as_ref()) {
                match self.call_openai(client, key, input).await {
                    Ok(llm) => return reconcile(llm, enforced, &self.model),
                    Err(err) => {
                        tracing::warn!(error = %err, "expert OpenAI indisponible, repli déterministe");
                    }
                }
            }
        }

        enforced
    }

    async fn call_openai(
        &self,
        client: &reqwest::Client,
        key: &str,
        input: &ExpertInput,
    ) -> Result<ExpertVerdict, String> {
        let started = Instant::now();
        let url = format!("{}/chat/completions", self.base_url);

        let body = json!({
            "model": self.model,
            "temperature": 0.0,
            "response_format": { "type": "json_object" },
            "messages": [
                { "role": "system", "content": SYSTEM_PROMPT },
                { "role": "user", "content": user_prompt(input) }
            ]
        });

        let response = client
            .post(&url)
            .bearer_auth(key)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("requête OpenAI: {e}"))?;

        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(format!("OpenAI {status}: {}", truncate(&text, 300)));
        }

        let parsed: ChatResponse = response
            .json()
            .await
            .map_err(|e| format!("réponse OpenAI illisible: {e}"))?;

        let content = parsed
            .choices
            .first()
            .and_then(|c| c.message.content.clone())
            .ok_or_else(|| "réponse OpenAI vide".to_string())?;

        let llm: LlmVerdict =
            serde_json::from_str(&content).map_err(|e| format!("JSON expert invalide: {e}"))?;

        Ok(ExpertVerdict {
            verdict: normalize_verdict(&llm.verdict),
            confidence: llm.confidence.clamp(0.0, 1.0),
            summary_fr: if llm.summary_fr.trim().is_empty() {
                "Verdict rendu sans commentaire.".to_string()
            } else {
                llm.summary_fr.trim().to_string()
            },
            cited_rules: llm.cited_rules,
            recommended_resolution: normalize_resolution(&llm.recommended_resolution),
            source: format!("openai:{}", self.model),
            latency_ms: started.elapsed().as_millis() as u64,
        })
    }
}

/// Le verdict déterministe de repli — aussi la **borne supérieure** de ce que
/// l'expert peut recommander.
pub fn deterministic_verdict(input: &ExpertInput) -> ExpertVerdict {
    let violations = input.guardrail.violated_rules();
    let overage = (input.amount_cents - input.mandate.max_total_cents).max(0);

    let (verdict, resolution, confidence, summary) = if !violations.is_empty() && overage > 0 {
        (
            "FLAG",
            RESOLUTION_PAYOUT,
            0.88,
            format!(
                "La dépense de {} dépasse le plafond de {} fixé par le mandat. L'écart de {} n'a jamais été autorisé.",
                format_cents(input.amount_cents, input.mandate.currency),
                format_cents(input.mandate.max_total_cents, input.mandate.currency),
                format_cents(overage, input.mandate.currency)
            ),
        )
    } else if !violations.is_empty() {
        (
            "FLAG",
            RESOLUTION_REVIEW,
            0.74,
            format!(
                "Une règle stricte du mandat a été violée ({}). Le montant reste dans le plafond, une revue humaine est nécessaire.",
                violations.join(", ")
            ),
        )
    } else {
        (
            "UPHOLD",
            RESOLUTION_REJECT,
            0.91,
            "Le log est cohérent avec le mandat : aucune règle stricte n'a été violée. La demande d'indemnisation est rejetée."
                .to_string(),
        )
    };

    ExpertVerdict {
        verdict: verdict.to_string(),
        confidence,
        summary_fr: summary,
        cited_rules: violations,
        recommended_resolution: resolution.to_string(),
        source: "deterministic-fallback".to_string(),
        latency_ms: 0,
    }
}

/// Garde-fou sur le garde-fou : le LLM peut durcir une décision, jamais élargir
/// la couverture décidée par les règles fixes.
pub fn reconcile(llm: ExpertVerdict, enforced: ExpertVerdict, model: &str) -> ExpertVerdict {
    let llm_widens = llm.recommended_resolution == RESOLUTION_PAYOUT
        && enforced.recommended_resolution != RESOLUTION_PAYOUT;

    if llm_widens {
        return ExpertVerdict {
            verdict: enforced.verdict.clone(),
            confidence: enforced.confidence,
            summary_fr: format!(
                "{} (Verdict IA ramené au verdict déterministe : le modèle ne peut pas ouvrir un remboursement que les règles du mandat refusent.)",
                enforced.summary_fr
            ),
            cited_rules: enforced.cited_rules.clone(),
            recommended_resolution: enforced.recommended_resolution.clone(),
            source: format!("openai:{model}+guardrail-clamp"),
            latency_ms: llm.latency_ms,
        };
    }

    ExpertVerdict {
        cited_rules: if llm.cited_rules.is_empty() {
            enforced.cited_rules
        } else {
            llm.cited_rules
        },
        ..llm
    }
}

fn normalize_verdict(raw: &str) -> String {
    match raw.trim().to_ascii_uppercase().as_str() {
        "UPHOLD" => "UPHOLD",
        "FLAG" => "FLAG",
        _ => "INCONCLUSIVE",
    }
    .to_string()
}

fn normalize_resolution(raw: &str) -> String {
    match raw.trim().to_ascii_uppercase().as_str() {
        "PAYOUT_OVERAGE" => RESOLUTION_PAYOUT,
        "REQUEST_HUMAN_REVIEW" => RESOLUTION_REVIEW,
        _ => RESOLUTION_REJECT,
    }
    .to_string()
}

fn user_prompt(input: &ExpertInput) -> String {
    let m = &input.mandate;
    let violated = input.guardrail.violated_rules();
    let checks = input
        .guardrail
        .checks
        .iter()
        .map(|c| {
            format!(
                "- {} [{}] {} : {}",
                c.code,
                if c.passed { "OK" } else { "VIOLÉ" },
                c.label,
                c.detail
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "MANDAT (fixé par l'humain)\n\
         - intention : {intent}\n\
         - catégorie : {category}\n\
         - plafond total : {max_total} {currency}\n\
         - plafond par transaction : {per_txn} {currency}\n\
         - approbation automatique jusqu'à : {auto} {currency}\n\
         - escalade jusqu'à : {escalate} {currency}\n\
         - marchands autorisés : {allowed}\n\
         - marchands bloqués : {blocked}\n\
         - pays autorisés : {countries}\n\
         - délai de livraison maximal : {delivery} jours\n\n\
         LOG JSON DE L'AGENT\n{log}\n\n\
         DÉCISION DÉTERMINISTE (qui fait autorité)\n\
         - décision : {decision}\n\
         - score de risque : {risk}/100\n\
         - règles violées : {violated}\n\
         - détail des contrôles :\n{checks}\n",
        intent = m.intent,
        category = m.category,
        max_total = format_cents(m.max_total_cents, m.currency),
        currency = m.currency.as_str(),
        per_txn = format_cents(m.per_txn_max_cents, m.currency),
        auto = format_cents(m.auto_approve_max_cents, m.currency),
        escalate = format_cents(m.escalate_until_cents, m.currency),
        allowed = if m.allowed_merchants.is_empty() {
            "aucune restriction".to_string()
        } else {
            m.allowed_merchants.join(", ")
        },
        blocked = if m.blocked_merchants.is_empty() {
            "aucun".to_string()
        } else {
            m.blocked_merchants.join(", ")
        },
        countries = if m.allowed_countries.is_empty() {
            "aucune restriction".to_string()
        } else {
            m.allowed_countries.join(", ")
        },
        delivery = m.max_delivery_days,
        log = serde_json::to_string_pretty(&input.action_log).unwrap_or_default(),
        decision = input.guardrail.decision.as_str(),
        risk = input.guardrail.risk.score,
        violated = if violated.is_empty() {
            "aucune".to_string()
        } else {
            violated.join(", ")
        },
        checks = checks,
    )
}

fn truncate(s: &str, max: usize) -> String {
    let s = s.replace('\n', " ");
    if s.chars().count() <= max {
        s
    } else {
        s.chars().take(max).collect::<String>() + "…"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::action::ProposedAction;
    use crate::domain::mandate::MandateStatus;
    use crate::domain::money::Currency;
    use crate::engine::{guardrail, HistoryWindow};
    use chrono::Utc;
    use uuid::Uuid;

    fn mandate() -> Mandate {
        Mandate {
            id: Uuid::nil(),
            agent_id: Uuid::nil(),
            owner_name: "Camille".into(),
            intent: "Casque maximum 50 EUR".into(),
            category: "electronics".into(),
            currency: Currency::Eur,
            max_total_cents: 5_000,
            per_txn_max_cents: 5_000,
            auto_approve_max_cents: 4_000,
            escalate_until_cents: 5_000,
            allowed_merchants: vec![],
            blocked_merchants: vec![],
            allowed_countries: vec!["FR".into()],
            max_delivery_days: 5,
            valid_from: Utc::now() - chrono::Duration::days(1),
            valid_until: Utc::now() + chrono::Duration::days(5),
            status: MandateStatus::Active,
            created_at: Utc::now(),
        }
    }

    fn input(amount: i64) -> ExpertInput {
        let m = mandate();
        let action = ProposedAction {
            mandate_id: m.id,
            agent_id: None,
            kind: crate::domain::action::ActionKind::Purchase,
            merchant: "RogueShop".into(),
            merchant_country: Some("FR".into()),
            amount_cents: amount,
            currency: Some(Currency::Eur),
            category: "electronics".into(),
            delivery_days: Some(4),
            description: "Casque".into(),
            idempotency_key: None,
            occurred_at: None,
            metadata: None,
        };
        let verdict = guardrail::evaluate(&action, &m, &HistoryWindow::default(), Utc::now());
        let log = serde_json::to_value(&action).unwrap_or_default();
        ExpertInput {
            mandate: m,
            action_log: log,
            guardrail: verdict,
            amount_cents: amount,
        }
    }

    #[test]
    fn deterministic_flags_overage_and_recommends_payout() {
        let v = deterministic_verdict(&input(9_000));
        assert_eq!(v.verdict, "FLAG");
        assert_eq!(v.recommended_resolution, RESOLUTION_PAYOUT);
        assert!(v.cited_rules.contains(&"AMOUNT_PER_TXN".to_string()));
    }

    #[test]
    fn deterministic_upholds_clean_action() {
        let v = deterministic_verdict(&input(2_000));
        assert_eq!(v.verdict, "UPHOLD");
        assert_eq!(v.recommended_resolution, RESOLUTION_REJECT);
    }

    #[test]
    fn llm_cannot_open_a_payout_the_rules_refuse() {
        let enforced = deterministic_verdict(&input(2_000));
        let greedy = ExpertVerdict {
            verdict: "FLAG".into(),
            confidence: 0.99,
            summary_fr: "Le modèle veut rembourser.".into(),
            cited_rules: vec![],
            recommended_resolution: RESOLUTION_PAYOUT.into(),
            source: "openai:test".into(),
            latency_ms: 12,
        };
        let reconciled = reconcile(greedy, enforced, "test");
        assert_eq!(reconciled.recommended_resolution, RESOLUTION_REJECT);
        assert!(reconciled.source.contains("guardrail-clamp"));
    }

    #[test]
    fn llm_may_keep_a_stricter_verdict() {
        let enforced = deterministic_verdict(&input(9_000));
        let stricter = ExpertVerdict {
            verdict: "FLAG".into(),
            confidence: 0.9,
            summary_fr: "Violation claire.".into(),
            cited_rules: vec!["AMOUNT_PER_TXN".into()],
            recommended_resolution: RESOLUTION_PAYOUT.into(),
            source: "openai:test".into(),
            latency_ms: 30,
        };
        let reconciled = reconcile(stricter, enforced, "test");
        assert_eq!(reconciled.recommended_resolution, RESOLUTION_PAYOUT);
    }
}
