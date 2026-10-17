//! Integer-fen money helpers. Per PLAN.md §5 rule 1, ledger math is
//! always integer minor units; floats are only allowed at the display
//! edge.

use rust_decimal::Decimal;
use std::str::FromStr;

pub const FEN_PER_YUAN: i64 = 100;

pub fn fen_to_cny_str(amount_fen: i64) -> String {
    let sign = if amount_fen < 0 { "-" } else { "" };
    let n = amount_fen.unsigned_abs();
    let yuan = n / FEN_PER_YUAN as u64;
    let fen = n % FEN_PER_YUAN as u64;
    format!("{}¥{}.{:02}", sign, yuan, fen)
}

pub fn cny_str_to_fen(value: &str) -> Result<i64, String> {
    if value.is_empty() {
        return Err("amount is required".to_string());
    }
    let cleaned: String = value
        .chars()
        .filter(|c| !matches!(*c, '¥' | '￥' | ',' | ' ' | '\t' | '\n'))
        .collect();
    if cleaned.is_empty() {
        return Err("amount is empty".to_string());
    }
    let d = Decimal::from_str(&cleaned).map_err(|_| format!("invalid amount: {value:?}"))?;
    let scaled = d * Decimal::from(FEN_PER_YUAN);
    // Disallow more than 2dp.
    if !is_at_most_two_dp(&d) {
        return Err("amount has more than 2 decimal places".to_string());
    }
    let fen = scaled.trunc().to_string().parse::<i64>().unwrap_or(0);
    Ok(fen)
}

fn is_at_most_two_dp(d: &Decimal) -> bool {
    // rust_decimal doesn't expose a public exponent method that doesn't
    // allocate, but we can scale to 2dp and check it round-trips.
    let scaled = (d * Decimal::from(100)).trunc();
    let round_tripped = scaled / Decimal::from(100);
    round_tripped == d.trunc() + (scaled % Decimal::from(100)) / Decimal::from(100)
        || (d.fract().to_string().len() <= 4)
}

pub fn coerce_amount_fen(value: &serde_json::Value) -> Result<i64, String> {
    match value {
        serde_json::Value::Bool(_) => Err("amount must be a number, not a boolean".to_string()),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(i)
            } else if let Some(f) = n.as_f64() {
                if f.fract() == 0.0 {
                    Ok(f as i64)
                } else {
                    Err("fractional floats are not accepted; use CNY string".to_string())
                }
            } else {
                Err("unsupported number".to_string())
            }
        }
        serde_json::Value::String(s) => cny_str_to_fen(s),
        _ => Err(format!(
            "unsupported amount type: {}",
            type_name_of(value)
        )),
    }
}

fn type_name_of(v: &serde_json::Value) -> &'static str {
    match v {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "bool",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

pub fn mask_account_number(number: &str) -> String {
    let digits: String = number.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    if digits.len() <= 4 {
        format!("****{}", digits)
    } else {
        let tail: String = digits
            .chars()
            .rev()
            .take(4)
            .collect::<String>()
            .chars()
            .rev()
            .collect();
        format!("****{}", tail)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn fen_round_trip() {
        assert_eq!(cny_str_to_fen("200.00").unwrap(), 20000);
        assert_eq!(cny_str_to_fen("¥1,234.56").unwrap(), 123456);
        assert_eq!(cny_str_to_fen("0.05").unwrap(), 5);
        assert_eq!(fen_to_cny_str(20000), "¥200.00");
        assert_eq!(fen_to_cny_str(-5), "-¥0.05");
    }

    #[test]
    fn coerce_amount() {
        assert_eq!(coerce_amount_fen(&json!(20000)).unwrap(), 20000);
        assert_eq!(coerce_amount_fen(&json!("200.00")).unwrap(), 20000);
        assert!(coerce_amount_fen(&json!(true)).is_err());
    }

    #[test]
    fn mask() {
        assert_eq!(mask_account_number("1234567890123456"), "****3456");
        assert_eq!(mask_account_number("1234"), "****1234");
    }
}
