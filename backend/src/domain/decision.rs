use serde::{Deserialize, Serialize};

/// Les trois seules issues possibles. `Escalate` n'est jamais un accord implicite.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Decision {
    Approve,
    Escalate,
    Refuse,
}

impl Decision {
    pub const fn as_str(self) -> &'static str {
        match self {
            Decision::Approve => "APPROVE",
            Decision::Escalate => "ESCALATE",
            Decision::Refuse => "REFUSE",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s.to_ascii_uppercase().as_str() {
            "APPROVE" | "APPROUVER" => Decision::Approve,
            "ESCALATE" | "ESCALADER" => Decision::Escalate,
            _ => Decision::Refuse,
        }
    }

    pub fn parse_opt(s: &str) -> Option<Self> {
        match s.to_ascii_uppercase().as_str() {
            "APPROVE" => Some(Decision::Approve),
            "ESCALATE" => Some(Decision::Escalate),
            "REFUSE" => Some(Decision::Refuse),
            _ => None,
        }
    }
}

impl std::fmt::Display for Decision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckSeverity {
    /// Une violation stricte entraîne un refus, quels que soient les seuils.
    Strict,
    /// Contrôle informatif : n'empêche rien, alimente l'explication.
    Soft,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleCheck {
    /// Code stable, utilisé dans l'audit et par l'expert IA.
    pub code: String,
    pub label: String,
    pub severity: CheckSeverity,
    pub passed: bool,
    pub detail: String,
}

impl RuleCheck {
    pub fn strict(
        code: &str,
        label: impl Into<String>,
        passed: bool,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            code: code.to_string(),
            label: label.into(),
            severity: CheckSeverity::Strict,
            passed,
            detail: detail.into(),
        }
    }

    pub fn soft(
        code: &str,
        label: impl Into<String>,
        passed: bool,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            code: code.to_string(),
            label: label.into(),
            severity: CheckSeverity::Soft,
            passed,
            detail: detail.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RiskBand {
    Low,
    Moderate,
    High,
    Critical,
}

impl RiskBand {
    pub const fn as_str(self) -> &'static str {
        match self {
            RiskBand::Low => "low",
            RiskBand::Moderate => "moderate",
            RiskBand::High => "high",
            RiskBand::Critical => "critical",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "moderate" => RiskBand::Moderate,
            "high" => RiskBand::High,
            "critical" => RiskBand::Critical,
            _ => RiskBand::Low,
        }
    }

    pub const fn from_score(score: u8) -> Self {
        match score {
            0..=29 => RiskBand::Low,
            30..=59 => RiskBand::Moderate,
            60..=79 => RiskBand::High,
            _ => RiskBand::Critical,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskFactor {
    pub code: String,
    pub label: String,
    pub points: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    pub score: u8,
    pub band: RiskBand,
    pub factors: Vec<RiskFactor>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Thresholds {
    pub auto_approve_max_cents: i64,
    pub escalate_until_cents: i64,
}

/// Sortie complète et reproductible du moteur déterministe.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardrailVerdict {
    pub decision: Decision,
    pub risk: RiskAssessment,
    pub checks: Vec<RuleCheck>,
    pub reasons: Vec<String>,
    pub explanation: String,
    pub thresholds: Thresholds,
}

impl GuardrailVerdict {
    /// Codes de règles violés (uniquement les règles strictes).
    pub fn violated_rules(&self) -> Vec<String> {
        self.checks
            .iter()
            .filter(|c| !c.passed && c.severity == CheckSeverity::Strict)
            .map(|c| c.code.clone())
            .collect()
    }

    pub fn all_strict_passed(&self) -> bool {
        self.checks
            .iter()
            .filter(|c| c.severity == CheckSeverity::Strict)
            .all(|c| c.passed)
    }
}
