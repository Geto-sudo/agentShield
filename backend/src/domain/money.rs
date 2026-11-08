use serde::{Deserialize, Serialize};

/// Currency used by a transaction. The prototype supports only two currencies,
/// which is sufficient to demonstrate the single-currency-per-mandate restriction.
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

    /// Corresponding PayPal currency code (the same ISO code, explicitly named
    /// to avoid magic strings in the PayPal client).
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

/// Formats an amount in cents using the prototype's French-style display format:
/// `6200` -> `"62,00 EUR"`.
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

/// Converts cents into the decimal string expected by the PayPal API (`"62.00"`).
pub fn cents_to_paypal_amount(cents: i64) -> String {
    format!("{}.{:02}", cents / 100, cents % 100)
}

/// Parses a PayPal decimal amount (`"62.00"`) into cents without using `f64`.
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
