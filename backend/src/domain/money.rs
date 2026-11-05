use serde::{Deserialize, Serialize};

/// Devise d'une opération. Le prototype ne gère que deux devises, ce qui suffit
/// à démontrer le contrôle « une seule devise par mandat ».
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "UPPERCASE")]
pub enum Currency {
    #[default]
    Eur,
    Usd,
}

impl Currency {
    pub const fn as_str(self) -> &'static str {
        match self {
            Currency::Eur => "EUR",
            Currency::Usd => "USD",
        }
    }

    pub const fn symbol(self) -> &'static str {
        match self {
            Currency::Eur => "EUR",
            Currency::Usd => "USD",
        }
    }

    /// Devise PayPal correspondante (même code ISO, mais nommé explicitement
    /// pour éviter les chaînes magiques dans le client PayPal).
    pub const fn paypal_code(self) -> &'static str {
        self.as_str()
    }
}

impl std::fmt::Display for Currency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for Currency {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_uppercase().as_str() {
            "EUR" | "€" => Ok(Currency::Eur),
            "USD" | "$" => Ok(Currency::Usd),
            _ => Err(()),
        }
    }
}

pub fn parse_currency(s: &str) -> Option<Currency> {
    s.parse().ok()
}

/// Rend un montant lisible en français : `6200` -> `"62,00 EUR"`.
pub fn format_cents(cents: i64, currency: Currency) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    let abs = cents.abs();
    format!(
        "{}{},{:02} {}",
        sign,
        abs / 100,
        abs % 100,
        currency.as_str()
    )
}

/// Conversion centimes -> chaîne décimale attendue par l'API PayPal (`"62.00"`).
pub fn cents_to_paypal_amount(cents: i64) -> String {
    format!("{}.{:02}", cents / 100, cents % 100)
}

/// Parse une chaîne décimale PayPal (`"62.00"`) en centimes, sans passer par `f64`.
pub fn paypal_amount_to_cents(s: &str) -> Option<i64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let negative = s.starts_with('-');
    let s = s.trim_start_matches(['-', '+']);
    let (int_part, dec_part) = match s.split_once('.') {
        Some((i, d)) => (i, d),
        None => (s, ""),
    };
    if int_part.is_empty() || !int_part.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let mut dec = dec_part.to_string();
    if dec.len() > 2 {
        dec.truncate(2);
    }
    while dec.len() < 2 {
        dec.push('0');
    }
    if !dec.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let units: i64 = int_part.parse().ok()?;
    let cents: i64 = dec.parse().ok()?;
    let total = units.checked_mul(100)?.checked_add(cents)?;
    Some(if negative { -total } else { total })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_eur_amounts() {
        assert_eq!(format_cents(6200, Currency::Eur), "62,00 EUR");
        assert_eq!(format_cents(5, Currency::Eur), "0,05 EUR");
        assert_eq!(format_cents(-1250, Currency::Eur), "-12,50 EUR");
    }

    #[test]
    fn converts_to_paypal_strings() {
        assert_eq!(cents_to_paypal_amount(6200), "62.00");
        assert_eq!(cents_to_paypal_amount(5), "0.05");
        assert_eq!(paypal_amount_to_cents("62.00"), Some(6200));
        assert_eq!(paypal_amount_to_cents("62.5"), Some(6250));
        assert_eq!(paypal_amount_to_cents("62"), Some(6200));
        assert_eq!(paypal_amount_to_cents("abc"), None);
    }
}
