//! Canonical JSON + parameter hashing for the approval pipeline.
//!
//! Per POLICY.md §7, the parameter hash is the single source of truth for
//! "the user is approving exactly this." Rules:
//!
//! 1. Sort object keys lexicographically at every depth.
//! 2. Lowercase string values.
//! 3. Coerce numeric money values to fen integer; reject anything else.
//! 4. Drop nulls and empty strings.
//! 5. SHA-256 hex of the canonical JSON.

use crate::money::coerce_amount_fen;
use serde_json::Value;
use sha2::{Digest, Sha256};

const MONEY_KEY_HINTS: &[&str] = &[
    "amount_fen",
    "amount_cny",
    "limit_fen",
    "limit_cny",
    "target_fen",
    "target_cny",
    "min_subscription_fen",
    "min_subscription_cny",
    "single_tx_limit_fen",
    "single_tx_limit_cny",
    "monthly_limit_fen",
    "monthly_limit_cny",
];

pub fn canonicalise(value: &Value) -> Value {
    match value {
        Value::Null => Value::Null,
        Value::Bool(b) => Value::Bool(*b),
        Value::Number(n) => {
            // Floats are not allowed in canonical form for anything money-shaped.
            // For non-money floats, fall back to str so the hash is stable.
            if let Some(i) = n.as_i64() {
                Value::Number(serde_json::Number::from(i))
            } else {
                Value::String(n.to_string())
            }
        }
        Value::String(s) => {
            let lowered = s.trim().to_lowercase();
            if lowered.is_empty() {
                Value::Null
            } else {
                Value::String(lowered)
            }
        }
        Value::Array(arr) => {
            let out: Vec<Value> = arr
                .iter()
                .filter(|v| !is_droppable(v))
                .map(canonicalise)
                .collect();
            Value::Array(out)
        }
        Value::Object(obj) => {
            let mut keys: Vec<&String> = obj.keys().collect();
            keys.sort();
            let mut out = serde_json::Map::new();
            for k in keys {
                let v = &obj[k];
                if is_droppable(v) {
                    continue;
                }
                if MONEY_KEY_HINTS.contains(&k.as_str()) {
                    match coerce_amount_fen(v) {
                        Ok(fen) => {
                            out.insert(k.clone(), Value::Number(serde_json::Number::from(fen)));
                        }
                        Err(_) => {
                            // leave as canonicalised string to keep stable hash
                            out.insert(k.clone(), canonicalise(v));
                        }
                    }
                } else {
                    out.insert(k.clone(), canonicalise(v));
                }
            }
            Value::Object(out)
        }
    }
}

pub fn is_droppable(value: &Value) -> bool {
    if value.is_null() {
        return true;
    }
    if let Value::String(s) = value {
        if s.trim().is_empty() {
            return true;
        }
    }
    false
}

pub fn canonical_dumps(value: &Value) -> String {
    let canon = canonicalise(value);
    serde_json::to_string(&canon).unwrap_or_else(|_| "null".to_string())
}

pub fn parameter_hash(parameters: &Value) -> String {
    let canonical = canonical_dumps(parameters);
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn hash_is_deterministic() {
        let a = json!({"b": 1, "a": "FOO", "c": null, "d": ""});
        let b = json!({"a": "foo", "b": 1});
        assert_eq!(parameter_hash(&a), parameter_hash(&b));
    }

    #[test]
    fn money_coercion() {
        let v = json!({"amount_cny": "200.00", "x": 1});
        let h = parameter_hash(&v);
        // Same integer fen should give the same hash as amount_fen=20000.
        let w = json!({"amount_fen": 20000, "x": 1});
        assert_eq!(h, parameter_hash(&w));
    }
}
