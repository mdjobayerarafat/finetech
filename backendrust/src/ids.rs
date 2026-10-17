//! Opaque ID helpers.

use uuid::Uuid;

pub fn new_id(prefix: &str) -> String {
    format!("{}_{}", prefix, Uuid::new_v4().simple())
}

pub fn is_valid_id(value: &str, prefix: &str) -> bool {
    if !value.starts_with(prefix) {
        return false;
    }
    let rest = &value[prefix.len() + 1..];
    rest.len() == 32 && rest.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
}
