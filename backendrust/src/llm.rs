//! LLM adapter — Unsloth (OpenAI-compatible) by default, with a local
//! regex-based stub as a deterministic fallback. Any HTTP failure, JSON
//! parse error, or schema violation falls through to the stub so the
//! planner always has a usable proposal.
//!
//! The planner calls :func:`propose` with the user message + a structured
//! user state summary, and gets back a JSON-serialisable plan proposal.

use log::info;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::Serialize;
use serde_json::{json, Value};

use crate::config;
use crate::llm_http;

#[derive(Debug, Clone, Serialize)]
pub struct Proposal {
    pub intent: String,
    pub language: String,
    pub entities: Value,
    pub clarification_required: bool,
    pub questions: Vec<String>,
    pub proposed_actions: Vec<Value>,
}

pub async fn propose_async(
    user_message: &str,
    state_summary: &Value,
    language_hint: Option<&str>,
    admin_prefix: &str,
) -> Proposal {
    let settings = config::get_settings();
    if settings.unsloth_enabled() {
        match llm_http::propose_unsloth(settings, user_message, state_summary, language_hint, admin_prefix).await {
            Some(p) => {
                info!(
                    "llm: using Unsloth proposal (intent={}, actions={})",
                    p.intent,
                    p.proposed_actions.len()
                );
                return p;
            }
            None => {
                info!("llm: Unsloth unavailable, falling back to stub");
            }
        }
    }
    stub_propose(user_message, state_summary, language_hint)
}

/// Async entry point — call this from any async handler. Falls through to
/// the regex stub if the configured LLM is unavailable.
pub async fn propose(
    user_message: &str,
    state_summary: &Value,
    language_hint: Option<&str>,
    admin_prefix: &str,
) -> Proposal {
    propose_async(user_message, state_summary, language_hint, admin_prefix).await
}

static CNY_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| {
    vec![
        Regex::new(r"(\d+(?:\.\d+)?)\s*(?:元|块|块钱)?").unwrap(),
        Regex::new(r"[¥￥]\s*(\d+(?:\.\d+)?)").unwrap(),
        Regex::new(r"(\d+(?:\.\d+)?)\s*元").unwrap(),
        Regex::new(r"(?i)(\d+(?:\.\d+)?)\s*rmb").unwrap(),
    ]
});

fn extract_amount_cny(text: &str) -> Option<String> {
    for pat in CNY_PATTERNS.iter() {
        if let Some(m) = pat.captures(text) {
            return Some(m.get(1).unwrap().as_str().to_string());
        }
    }
    let re = Regex::new(r"(?i)(?:pay|send|转|给)\s+(\d+(?:\.\d+)?)").unwrap();
    if let Some(m) = re.captures(text) {
        return Some(m.get(1).unwrap().as_str().to_string());
    }
    None
}

/// Pull a recipient name out of a birthday message. The cross-scenario
/// test in the spec uses "spouse's birthday" / "spouse birthday" / "妻子
/// 生日" / "老公生日". We return the un-inflected noun as the label so
/// "spouse's birthday" → "spouse's birthday".
fn extract_birthday_recipient(text: &str) -> Option<String> {
    let re = Regex::new(r"(?i)([A-Za-z]+(?:'s|s)?)\s+birthday").unwrap();
    if let Some(m) = re.captures(text) {
        return Some(m.get(1).unwrap().as_str().to_string());
    }
    if text.contains("生日") {
        // For Chinese: pick the chars immediately preceding "生日" as the
        // recipient. e.g. "妻子生日" → "妻子".
        let idx = text.find("生日")?;
        let start = idx.saturating_sub(2);
        let slice = &text[start..idx];
        if !slice.trim().is_empty() {
            return Some(slice.to_string());
        }
    }
    None
}

/// Lift a "due date" out of a chat message. The spec asks for cases like
/// "in October" (next-occurrence) and absolute dates. We don't try to be
/// clever here — anything we can't parse returns None and the stub falls
/// back to a clarifying question.
fn extract_due_at(text: &str) -> Option<String> {
    // ISO date (YYYY-MM-DD)
    let iso = Regex::new(r"(\d{4}-\d{2}-\d{2})").unwrap();
    if let Some(m) = iso.captures(text) {
        return Some(m.get(1).unwrap().as_str().to_string());
    }
    // Month name: e.g. "in October" / "next October"
    let months = [
        ("january", 1u32),
        ("february", 2),
        ("march", 3),
        ("april", 4),
        ("may", 5),
        ("june", 6),
        ("july", 7),
        ("august", 8),
        ("september", 9),
        ("october", 10),
        ("november", 11),
        ("december", 12),
    ];
    let lower = text.to_lowercase();
    for (name, num) in months {
        if lower.contains(name) {
            let now = chrono::Utc::now();
            let mut y = now.format("%Y").to_string().parse::<i32>().unwrap_or(2025);
            // If the named month has already passed this year, push to next
            // year so the reservation isn't in the past.
            if (num as i32) < now.format("%m").to_string().parse::<i32>().unwrap_or(1) {
                y += 1;
            }
            return Some(format!("{:04}-{:02}-15T00:00:00Z", y, num));
        }
    }
    if text.contains("下个月") || lower.contains("next month") {
        let now = chrono::Utc::now();
        let next = if now.format("%m").to_string().parse::<i32>().unwrap_or(1) == 12 {
            format!(
                "{:04}-01-15T00:00:00Z",
                now.format("%Y").to_string().parse::<i32>().unwrap_or(2025) + 1
            )
        } else {
            format!(
                "{:04}-{:02}-15T00:00:00Z",
                now.format("%Y").to_string().parse::<i32>().unwrap_or(2025),
                now.format("%m").to_string().parse::<i32>().unwrap_or(1) + 1
            )
        };
        return Some(next);
    }
    None
}

/// Map a chat string to a `transactions.category` enum. Mirrors the
/// `category` list in `tool_spec.rs` for `budgets.set` so the planner
/// doesn't have to second-guess the LLM.
fn extract_category(text: &str) -> Option<String> {
    let lower = text.to_lowercase();
    let pairs: &[(&[&str], &str)] = &[
        (&["food", "dining", "groceries", "eat", "restaurants", "吃饭", "餐饮", "吃"], "food"),
        (&["transport", "rides", "taxi", "subway", "metro", "bus", "交通", "打车", "地铁"], "transport"),
        (&["shop", "shopping", "clothes", "amazon", "taobao", "购物", "衣服"], "shopping"),
        (&["subscription", "subscriptions", "netflix", "spotify", "订阅"], "subscription"),
        (&["entertainment", "movies", "games", "music", "娱乐", "游戏"], "entertainment"),
        (&["travel", "flight", "hotel", "airbnb", "旅游", "旅行"], "travel"),
        (&["utilities", "bills", "electricity", "water", "utility", "水电", "电费"], "utilities"),
        (&["health", "medical", "doctor", "pharmacy", "医保", "医院"], "health"),
        (&["education", "tuition", "books", "course", "教育", "学习"], "education"),
        (&["transfer", "transfers"], "transfer"),
    ];
    for (kws, cat) in pairs {
        for kw in *kws {
            if lower.contains(kw) {
                return Some((*cat).to_string());
            }
        }
    }
    None
}

fn detect_language(text: &str) -> String {
    let has_cn = text.chars().any(|c| matches!(c, '\u{4e00}'..='\u{9fff}'));
    let has_en = text.chars().any(|c| c.is_ascii_alphabetic());
    match (has_cn, has_en) {
        (true, true) => "mixed".to_string(),
        (true, false) => "zh".to_string(),
        _ => "en".to_string(),
    }
}

pub fn stub_propose(text: &str, state_summary: &Value, language_hint: Option<&str>) -> Proposal {
    let lower = text.to_lowercase();
    let language = language_hint
        .map(|s| s.to_string())
        .unwrap_or_else(|| detect_language(text));

    let primary_account_id = state_summary
        .get("primary_account_id")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if ["report", "spending", "bills", "analyze", "analysis"]
        .iter()
        .any(|kw| lower.contains(kw))
    {
        return Proposal {
            intent: "report_spending".to_string(),
            language: language.clone(),
            entities: json!({ "raw": text }),
            clarification_required: false,
            questions: vec![],
            proposed_actions: vec![json!({
                "tool": "accounts.balance_check",
                "arguments": { "account_id": primary_account_id },
            })],
        };
    }

    if ["subscriptions", "recurring", "direct debit", "auto pay"]
        .iter()
        .any(|kw| lower.contains(kw))
    {
        return Proposal {
            intent: "list_subscriptions".to_string(),
            language: language.clone(),
            entities: json!({}),
            clarification_required: false,
            questions: vec![],
            proposed_actions: vec![json!({
                "tool": "subscriptions.list",
                "arguments": { "status": "all" },
            })],
        };
    }

    if lower.contains("balance") || lower.contains("how much do i have") || lower.contains("余额") {
        return Proposal {
            intent: "balance".to_string(),
            language: language.clone(),
            entities: json!({}),
            clarification_required: false,
            questions: vec![],
            proposed_actions: vec![json!({
                "tool": "accounts.balance_summary",
                "arguments": {},
            })],
        };
    }

    if lower.contains("my cards") || lower.contains("my card") || lower.contains("list cards")
        || (lower.contains("cards") && (lower.contains("what") || lower.contains("show") || lower.contains("list")))
    {
        return Proposal {
            intent: "list_cards".to_string(),
            language: language.clone(),
            entities: json!({}),
            clarification_required: false,
            questions: vec![],
            proposed_actions: vec![json!({
                "tool": "cards.list",
                "arguments": {},
            })],
        };
    }

    if lower.contains("my accounts") || lower.contains("my account") || lower.contains("list accounts")
        || (lower.contains("accounts") && (lower.contains("what") || lower.contains("show") || lower.contains("list")))
    {
        return Proposal {
            intent: "list_accounts".to_string(),
            language: language.clone(),
            entities: json!({}),
            clarification_required: false,
            questions: vec![],
            proposed_actions: vec![json!({
                "tool": "accounts.list",
                "arguments": {},
            })],
        };
    }

    if lower.contains("wealth") || lower.contains("fund") || lower.contains("invest") || lower.contains("portfolio") {
        return Proposal {
            intent: "wealth_summary".to_string(),
            language: language.clone(),
            entities: json!({}),
            clarification_required: false,
            questions: vec![],
            proposed_actions: vec![json!({
                "tool": "wealth.summary",
                "arguments": {},
            })],
        };
    }

    if lower.contains("recent") || lower.contains("transactions") || lower.contains("ledger") {
        return Proposal {
            intent: "recent_transactions".to_string(),
            language: language.clone(),
            entities: json!({}),
            clarification_required: false,
            questions: vec![],
            proposed_actions: vec![json!({
                "tool": "transactions.recent",
                "arguments": { "limit": 10 },
            })],
        };
    }

    let cancel_re = Regex::new(r"cancel\s+(?:the\s+)?([\w\u{4e00}-\u{9fff} ]+?)(?:\s+subscription|\s*\.|$)").unwrap();
    if let Some(cap) = cancel_re.captures(&lower) {
        let merchant = cap.get(1).unwrap().as_str().trim().to_string();
        return Proposal {
            intent: "cancel_subscription".to_string(),
            language: language.clone(),
            entities: json!({ "merchant_query": merchant }),
            clarification_required: false,
            questions: vec![],
            proposed_actions: vec![json!({
                "tool": "subscriptions.cancel",
                "arguments": {
                    "subscription_id": "__resolve_subscription__",
                    "merchant_query": merchant,
                },
            })],
        };
    }

    if lower.contains("birthday") || text.contains("生日") || text.contains("蛋糕") || lower.contains("flower") {
        // Cross-scenario orchestration:
        //   "spouse's birthday, reserve ¥1000 from my checking in October,
        //    order flowers and a cake two days before"
        // The stub tries to lift an amount from the text. If it finds one
        // + a date-ish phrase, it emits a `budgets.reserve` action (Red tier)
        // — the planner + risk layer handle the rest, including wiring a
        // reminder for the 2-days-before shopping run. Otherwise it asks
        // clarifying questions the way the original branch did.
        let amount = extract_amount_cny(text);
        let label = if let Some(who) = extract_birthday_recipient(text) {
            format!("{}'s birthday", who)
        } else {
            "Birthday".to_string()
        };
        let due_at = extract_due_at(text);
        if let (Some(amt), Some(when)) = (amount.clone(), due_at.clone()) {
            return Proposal {
                intent: "plan_birthday".to_string(),
                language: language.clone(),
                entities: json!({
                    "raw": text,
                    "amount_cny": amt,
                    "due_at": when,
                }),
                clarification_required: false,
                questions: vec![],
                proposed_actions: vec![json!({
                    "tool": "budgets.reserve",
                    "arguments": {
                        "label": label,
                        "amount_cny": amt,
                        "account_id": primary_account_id,
                        "due_at": when,
                    },
                })],
            };
        }
        return Proposal {
            intent: "plan_birthday".to_string(),
            language: language.clone(),
            entities: json!({ "raw": text }),
            clarification_required: true,
            questions: vec![
                "What is the recipient's name?".to_string(),
                "What is the birthday date? (YYYY-MM-DD or e.g. 'next month')".to_string(),
                "How much should we reserve? (in CNY)".to_string(),
            ],
            proposed_actions: vec![json!({
                "tool": "profile.resolve_date",
                "arguments": { "date": "__needs_clarification__" },
            })],
        };
    }

    // "Set a budget for food at ¥2000 a month" — emit a `budgets.set` action
    // when the user names a category and a number. Falls through to a
    // clarifying question otherwise so the LLM can take over.
    if lower.contains("budget") || text.contains("预算") || text.contains("限额") {
        let category = extract_category(text);
        let amount = extract_amount_cny(text);
        if let (Some(cat), Some(amt)) = (category.clone(), amount.clone()) {
            return Proposal {
                intent: "set_budget".to_string(),
                language: language.clone(),
                entities: json!({
                    "raw": text,
                    "category": cat,
                    "amount_cny": amt,
                }),
                clarification_required: false,
                questions: vec![],
                proposed_actions: vec![json!({
                    "tool": "budgets.set",
                    "arguments": {
                        "category": cat,
                        "limit_cny": amt,
                    },
                })],
            };
        }
        return Proposal {
            intent: "set_budget".to_string(),
            language: language.clone(),
            entities: json!({ "raw": text }),
            clarification_required: true,
            questions: vec![
                "Which category should the cap cover? (food, transport, shopping, …)".to_string(),
                "How much per month, in CNY?".to_string(),
            ],
            proposed_actions: vec![],
        };
    }

    let phone_re = Regex::new(r"(\d{4})(?:\s|$)").unwrap();
    if let Some(cap) = phone_re.captures(text) {
        if lower.contains("send") || text.contains("转") || lower.contains("pay") {
            let tail = cap.get(1).unwrap().as_str();
            if let Some(amount) = extract_amount_cny(text) {
                return Proposal {
                    intent: "transfer".to_string(),
                    language: language.clone(),
                    entities: json!({
                        "phone_tail": tail,
                        "amount_cny": amount,
                        "memo": "",
                    }),
                    clarification_required: false,
                    questions: vec![],
                    proposed_actions: vec![json!({
                        "tool": "transfer.create_pending",
                        "arguments": {
                            "account_id": primary_account_id,
                            "phone_tail": tail,
                            "amount_cny": amount,
                            "memo": "",
                        },
                    })],
                };
            }
        }
    }

    // Name-based transfer.
    let verb_re = Regex::new(r"(?i)^\s*(?:please\s+)?(?:pay|send|transfer|给|转给?|付给?)\s+").unwrap();
    let cleaned = verb_re.replace_all(text, "").to_string();
    let amount_re = Regex::new(r"(?i)^\s*\d+(?:\.\d+)?\s*(?:元|块|块钱|rmb)?\s*(?:给|付给?|转给?)?\s*").unwrap();
    let cleaned = amount_re.replace_all(&cleaned, "").to_string();
    let name_re = Regex::new(r"([A-Za-z\u{4e00}-\u{9fff}][\w\u{4e00}-\u{9fff} ]+?)(?:\s+[¥￥]?\s*\d|\s+for|\s*[,，]|$)").unwrap();
    if let Some(cap) = name_re.captures(&cleaned) {
        let mut name = cap.get(1).unwrap().as_str().trim().to_string();
        let trail = Regex::new(r"(?i)\s+(for|to|with)$").unwrap();
        name = trail.replace_all(&name, "").to_string();
        if let Some(amount) = extract_amount_cny(text) {
            let memo_re = Regex::new(r"[,，]\s*(.+?)$").unwrap();
            let memo = memo_re
                .captures(text)
                .and_then(|m| m.get(1).map(|x| x.as_str().trim().to_string()))
                .unwrap_or_default();
            return Proposal {
                intent: "transfer".to_string(),
                language: language.clone(),
                entities: json!({
                    "recipient_query": name,
                    "amount_cny": amount,
                    "memo": memo,
                }),
                clarification_required: false,
                questions: vec![],
                proposed_actions: vec![json!({
                    "tool": "transfer.create_pending",
                    "arguments": {
                        "account_id": primary_account_id,
                        "recipient_query": name,
                        "amount_cny": amount,
                        "memo": memo,
                    },
                })],
            };
        }
    }

    if ["schedule", "every month", "each month", "monthly", "每月", "下个月"]
        .iter()
        .any(|kw| lower.contains(kw))
    {
        if let Some(amount) = extract_amount_cny(text) {
            return Proposal {
                intent: "schedule_transfer".to_string(),
                language: language.clone(),
                entities: json!({ "amount_cny": amount, "memo": text }),
                clarification_required: true,
                questions: vec!["Who should we pay?".to_string(), "On which day of the month?".to_string()],
                proposed_actions: vec![],
            };
        }
    }

    Proposal {
        intent: "unknown".to_string(),
        language,
        entities: json!({ "raw": text }),
        clarification_required: true,
        questions: vec![
            "Could you rephrase that? For example: 'Pay Li Wei ¥200 for lunch'.".to_string(),
        ],
        proposed_actions: vec![],
    }
}
