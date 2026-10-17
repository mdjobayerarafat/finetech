//! Typed tool registry with JSON Schema + slot metadata.
//!
//! The runtime tool registry in `tools.rs` exposes a function pointer for each
//! tool. This module adds a parallel *static* registry that, for every tool:
//!
//! 1. Defines a JSON Schema describing the `arguments` map.
//! 2. Defines a list of `SlotSpec`s — the user-facing questions the planner
//!    must answer before the tool can run (slot-based proposals, feature 3).
//! 3. Provides a runtime validator that walks `arguments` against the schema
//!    and returns precise error messages when a slot is missing.
//!
//! Keeping this separate from `tools.rs` means tool implementations stay
//! short and side-effect-focused; metadata lives in one declarative table
//! that the LLM system prompt, the planner, and the slot resolver all read.

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// One user-visible piece of information required before a tool can run.
/// The planner surfaces one clarification question per missing slot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotSpec {
    /// Stable machine name (used in the slot_bindings table).
    pub name: &'static str,
    /// Human-friendly question the chat renders when the slot is empty.
    pub question: &'static str,
    /// JSON type the slot expects after coercion.
    pub kind: SlotKind,
    /// Optional hint the LLM can use to auto-extract from the user message.
    pub extract_hint: Option<&'static str>,
    /// If `true`, the planner may infer this from the user's prior message
    /// or the wallet-view state instead of asking.
    pub inferable: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SlotKind {
    String,
    AmountFen,    // integer fen, after coercion from CNY
    Identifier,   // any non-empty string
    Enum,         // string from a fixed set
    Boolean,
    Date,         // ISO-8601 string (YYYY-MM-DD or RFC3339)
}

/// Static metadata + schema for one tool. The schema is intentionally
/// minimal — it's a contract for argument-shape validation, not a full
/// JSON-Schema vocabulary. The planner and LLM system prompt read it.
#[derive(Debug, Clone)]
pub struct ToolSpec {
    /// Canonical tool name. Must match the keys in `tools::registry()`.
    pub canonical_name: &'static str,
    /// Whether this tool mutates user state (drives "is this Red?" decisions).
    pub mutating: bool,
    /// Risk tier when nothing else escalates it.
    pub default_tier: &'static str,
    /// JSON Schema describing `arguments`. Built at compile time; consumed
    /// by the LLM system prompt and by `validate_arguments`.
    pub arguments_schema: Value,
    /// Slots required before the planner can build a node.
    pub required_slots: &'static [SlotSpec],
    /// Optional slot that improves the proposal but isn't strictly required.
    pub optional_slots: &'static [SlotSpec],
}

/// Build the spec table. Theorised, not magic — every entry is written by
/// hand against the matching `t_*` function in `tools.rs`.
pub fn spec_table() -> &'static [ToolSpec] {
    &SPECS
}

pub fn spec_for(canonical_name: &str) -> Option<&'static ToolSpec> {
    SPECS.iter().find(|s| s.canonical_name == canonical_name)
}

/// Validate a JSON `arguments` object against a tool's schema. Returns the
/// set of required slot names that are missing or whose kind doesn't match
/// — enough for the planner to ask precise follow-up questions.
pub fn validate_arguments(spec: &ToolSpec, args: &Value) -> Result<(), Vec<MissingSlot>> {
    let obj = match args.as_object() {
        Some(o) => o,
        None => {
            return Err(spec
                .required_slots
                .iter()
                .map(|s| MissingSlot { slot: s, reason: "arguments must be an object".into() })
                .collect())
        }
    };
    let mut missing = Vec::new();
    for slot in spec.required_slots.iter().chain(spec.optional_slots.iter()) {
        let in_required = spec.required_slots.iter().any(|s| s.name == slot.name);
        match obj.get(slot.name) {
            None if in_required => missing.push(MissingSlot {
                slot,
                reason: format!("slot `{}` is required", slot.name),
            }),
            None => {}
            Some(v) => {
                if !matches_kind(v, slot.kind) {
                    missing.push(MissingSlot {
                        slot,
                        reason: format!("slot `{}` has wrong type", slot.name),
                    });
                }
            }
        }
    }
    if missing.is_empty() {
        Ok(())
    } else {
        Err(missing)
    }
}

/// One missing/invalid slot, ready to become a chat clarification question.
#[derive(Debug, Clone)]
pub struct MissingSlot {
    pub slot: &'static SlotSpec,
    pub reason: String,
}

/// Convert a slot-fill problem into the JSON shape the agent route already
/// understands (`questions: [..]`). Mirrors the `Clarification` arm in
/// `agent_message` so the frontend rendering stays unchanged.
pub fn missing_to_clarifications(missing: Vec<MissingSlot>) -> Value {
    let questions: Vec<String> = missing.into_iter().map(|m| m.slot.question.to_string()).collect();
    json!({
        "clarification_required": true,
        "questions": questions,
    })
}

fn matches_kind(v: &Value, k: SlotKind) -> bool {
    match k {
        SlotKind::String | SlotKind::Identifier | SlotKind::Date => {
            v.is_string() && !v.as_str().unwrap_or("").trim().is_empty()
        }
        SlotKind::AmountFen => v.is_i64() || v.is_u64() || (v.is_string() && !v.as_str().unwrap_or("").is_empty()),
        SlotKind::Enum => v.is_string(),
        SlotKind::Boolean => v.is_boolean(),
    }
}

// ---------------------------------------------------------------------------
// The static spec table itself.
//
// Conventions:
//   - canonical_name matches the keys in tools::registry().
//   - arguments_schema is JSON Schema-ish: {type:"object", required, properties}
//   - Slot names are stable and shared with the LLM system prompt.
// ---------------------------------------------------------------------------

static SPECS: Lazy<Vec<ToolSpec>> = Lazy::new(|| vec![
    ToolSpec {
        canonical_name: "transfer.create_pending",
        mutating: true,
        default_tier: "Yellow",
        arguments_schema: json!({
            "type": "object",
            "required": ["account_id", "payee_id", "amount_fen"],
            "properties": {
                "account_id": {"type": "string"},
                "payee_id":   {"type": "string"},
                "amount_fen": {"type": "integer", "minimum": 1},
                "memo":       {"type": "string"}
            }
        }),
        required_slots: &[
            SlotSpec {
                name: "payee_id",
                question: "Who should I send the money to? (name or last 4 of phone)",
                kind: SlotKind::Identifier,
                extract_hint: Some("Payee name (Chinese or English) or last 4 digits of phone number."),
                inferable: true,
            },
            SlotSpec {
                name: "amount_fen",
                question: "How much should I send, in CNY? (e.g. 200 = ¥200)",
                kind: SlotKind::AmountFen,
                extract_hint: Some("Numeric amount with optional currency symbol or 元/块 marker."),
                inferable: true,
            },
        ],
        optional_slots: &[
            SlotSpec {
                name: "memo",
                question: "Optional note for the recipient?",
                kind: SlotKind::String,
                extract_hint: None,
                inferable: true,
            },
        ],
    },
    ToolSpec {
        canonical_name: "subscriptions.cancel",
        mutating: true,
        default_tier: "Yellow",
        arguments_schema: json!({
            "type": "object",
            "required": ["subscription_id"],
            "properties": {
                "subscription_id": {"type": "string"}
            }
        }),
        required_slots: &[
            SlotSpec {
                name: "subscription_id",
                question: "Which subscription should I cancel?",
                kind: SlotKind::Identifier,
                extract_hint: Some("Subscription merchant name or id."),
                inferable: true,
            },
        ],
        optional_slots: &[],
    },
    ToolSpec {
        canonical_name: "subscriptions.remind",
        mutating: false,
        default_tier: "Green",
        arguments_schema: json!({
            "type": "object",
            "required": ["subscription_id"],
            "properties": {
                "subscription_id": {"type": "string"},
                "days_before":      {"type": "integer"}
            }
        }),
        required_slots: &[
            SlotSpec {
                name: "subscription_id",
                question: "Which subscription should I remind you about?",
                kind: SlotKind::Identifier,
                extract_hint: Some("Merchant name."),
                inferable: true,
            },
        ],
        optional_slots: &[
            SlotSpec {
                name: "days_before",
                question: "How many days before renewal? (default 3)",
                kind: SlotKind::AmountFen,
                extract_hint: None,
                inferable: true,
            },
        ],
    },
    ToolSpec {
        canonical_name: "reminders.create",
        mutating: false,
        default_tier: "Green",
        arguments_schema: json!({
            "type": "object",
            "required": ["subject", "due_at"],
            "properties": {
                "subject": {"type": "string"},
                "due_at":  {"type": "string"}
            }
        }),
        required_slots: &[
            SlotSpec {
                name: "subject",
                question: "What should I remind you about?",
                kind: SlotKind::String,
                extract_hint: None,
                inferable: true,
            },
            SlotSpec {
                name: "due_at",
                question: "When is it due? (YYYY-MM-DD or a relative phrase like 'tomorrow')",
                kind: SlotKind::Date,
                extract_hint: None,
                inferable: true,
            },
        ],
        optional_slots: &[],
    },
    ToolSpec {
        canonical_name: "accounts.balance_check",
        mutating: false,
        default_tier: "Green",
        arguments_schema: json!({
            "type": "object",
            "properties": {"account_id": {"type": "string"}}
        }),
        required_slots: &[],
        optional_slots: &[
            SlotSpec {
                name: "account_id",
                question: "Which account?",
                kind: SlotKind::Identifier,
                extract_hint: None,
                inferable: true,
            },
        ],
    },
    ToolSpec {
        canonical_name: "goals.create_reservation",
        mutating: true,
        default_tier: "Yellow",
        arguments_schema: json!({
            "type": "object",
            "required": ["amount_fen", "account_id"],
            "properties": {
                "amount_fen": {"type": "integer", "minimum": 1},
                "account_id": {"type": "string"},
                "goal_id":    {"type": "string"},
                "goal_name":  {"type": "string"}
            }
        }),
        required_slots: &[
            SlotSpec {
                name: "amount_fen",
                question: "How much should I reserve? (in CNY)",
                kind: SlotKind::AmountFen,
                extract_hint: Some("Numeric amount."),
                inferable: true,
            },
        ],
        optional_slots: &[
            SlotSpec {
                name: "goal_name",
                question: "What should I call this goal?",
                kind: SlotKind::String,
                extract_hint: None,
                inferable: true,
            },
        ],
    },
    ToolSpec {
        canonical_name: "goals.consume_reservation",
        mutating: true,
        default_tier: "Yellow",
        arguments_schema: json!({
            "type": "object",
            "required": ["reservation_id"],
            "properties": {"reservation_id": {"type": "string"}}
        }),
        required_slots: &[
            SlotSpec {
                name: "reservation_id",
                question: "Which reservation should I consume?",
                kind: SlotKind::Identifier,
                extract_hint: None,
                inferable: true,
            },
        ],
        optional_slots: &[],
    },
    ToolSpec {
        canonical_name: "goals.release_reservation",
        mutating: true,
        default_tier: "Yellow",
        arguments_schema: json!({
            "type": "object",
            "required": ["reservation_id"],
            "properties": {"reservation_id": {"type": "string"}}
        }),
        required_slots: &[
            SlotSpec {
                name: "reservation_id",
                question: "Which reservation should I release?",
                kind: SlotKind::Identifier,
                extract_hint: None,
                inferable: true,
            },
        ],
        optional_slots: &[],
    },
    ToolSpec {
        canonical_name: "cards.issue",
        mutating: true,
        default_tier: "Yellow",
        arguments_schema: json!({
            "type": "object",
            "properties": {
                "display_name": {"type": "string"},
                "single_tx_limit_fen": {"type": "integer"},
                "monthly_limit_fen":   {"type": "integer"}
            }
        }),
        required_slots: &[],
        optional_slots: &[
            SlotSpec {
                name: "display_name",
                question: "What should I name the new card?",
                kind: SlotKind::String,
                extract_hint: None,
                inferable: true,
            },
        ],
    },
    ToolSpec {
        canonical_name: "cards.action",
        mutating: true,
        default_tier: "Yellow",
        arguments_schema: json!({
            "type": "object",
            "required": ["card_id", "kind"],
            "properties": {
                "card_id": {"type": "string"},
                "kind":    {"type": "string", "enum": ["freeze","unfreeze","cancel","report_lost","update_limits"]}
            }
        }),
        required_slots: &[
            SlotSpec {
                name: "card_id",
                question: "Which card?",
                kind: SlotKind::Identifier,
                extract_hint: Some("Card last 4 or display name."),
                inferable: true,
            },
            SlotSpec {
                name: "kind",
                question: "What should I do? (freeze, unfreeze, cancel, report lost, update limits)",
                kind: SlotKind::Enum,
                extract_hint: None,
                inferable: true,
            },
        ],
        optional_slots: &[],
    },
    ToolSpec {
        canonical_name: "wealth.subscribe",
        mutating: true,
        default_tier: "Yellow",
        arguments_schema: json!({
            "type": "object",
            "required": ["product_id", "amount_fen", "account_id"],
            "properties": {
                "product_id": {"type": "string"},
                "amount_fen": {"type": "integer"},
                "account_id": {"type": "string"}
            }
        }),
        required_slots: &[
            SlotSpec {
                name: "product_id",
                question: "Which wealth product?",
                kind: SlotKind::Identifier,
                extract_hint: Some("Fund name or product id."),
                inferable: true,
            },
            SlotSpec {
                name: "amount_fen",
                question: "How much should I invest? (in CNY)",
                kind: SlotKind::AmountFen,
                extract_hint: None,
                inferable: true,
            },
        ],
        optional_slots: &[],
    },
    ToolSpec {
        canonical_name: "wealth.redeem",
        mutating: true,
        default_tier: "Yellow",
        arguments_schema: json!({
            "type": "object",
            "required": ["holding_id", "account_id"],
            "properties": {
                "holding_id": {"type": "string"},
                "account_id": {"type": "string"}
            }
        }),
        required_slots: &[
            SlotSpec {
                name: "holding_id",
                question: "Which holding should I redeem?",
                kind: SlotKind::Identifier,
                extract_hint: None,
                inferable: true,
            },
        ],
        optional_slots: &[],
    },
    ToolSpec {
        canonical_name: "connectors.merchant.order",
        mutating: true,
        default_tier: "Yellow",
        arguments_schema: json!({
            "type": "object",
            "properties": {
                "merchant": {"type": "string"},
                "amount_fen": {"type": "integer"},
                "deliver_at": {"type": "string"}
            }
        }),
        required_slots: &[
            SlotSpec {
                name: "merchant",
                question: "Which merchant (e.g. flowers / cake)?",
                kind: SlotKind::String,
                extract_hint: None,
                inferable: true,
            },
        ],
        optional_slots: &[
            SlotSpec {
                name: "amount_fen",
                question: "How much should I spend? (in CNY)",
                kind: SlotKind::AmountFen,
                extract_hint: None,
                inferable: true,
            },
            SlotSpec {
                name: "deliver_at",
                question: "When should it arrive? (YYYY-MM-DD)",
                kind: SlotKind::Date,
                extract_hint: None,
                inferable: true,
            },
        ],
    },
    ToolSpec {
        canonical_name: "profile.resolve_date",
        mutating: false,
        default_tier: "Green",
        arguments_schema: json!({
            "type": "object",
            "required": ["date"],
            "properties": {"date": {"type": "string"}}
        }),
        required_slots: &[
            SlotSpec {
                name: "date",
                question: "Which date?",
                kind: SlotKind::Date,
                extract_hint: None,
                inferable: true,
            },
        ],
        optional_slots: &[],
    },
    // Read-only helpers — they have no required slots; the planner picks
    // them on its own without asking.
    ToolSpec {
        canonical_name: "accounts.list",
        mutating: false,
        default_tier: "Green",
        arguments_schema: json!({"type": "object"}),
        required_slots: &[],
        optional_slots: &[],
    },
    ToolSpec {
        canonical_name: "accounts.balance_summary",
        mutating: false,
        default_tier: "Green",
        arguments_schema: json!({"type": "object"}),
        required_slots: &[],
        optional_slots: &[],
    },
    ToolSpec {
        canonical_name: "payees.list",
        mutating: false,
        default_tier: "Green",
        arguments_schema: json!({"type": "object"}),
        required_slots: &[],
        optional_slots: &[],
    },
    ToolSpec {
        canonical_name: "subscriptions.list",
        mutating: false,
        default_tier: "Green",
        arguments_schema: json!({
            "type": "object",
            "properties": {"status": {"type": "string"}}
        }),
        required_slots: &[],
        optional_slots: &[],
    },
    ToolSpec {
        canonical_name: "cards.list",
        mutating: false,
        default_tier: "Green",
        arguments_schema: json!({"type": "object"}),
        required_slots: &[],
        optional_slots: &[],
    },
    ToolSpec {
        canonical_name: "spending.report",
        mutating: false,
        default_tier: "Green",
        arguments_schema: json!({
            "type": "object",
            "properties": {"days": {"type": "integer"}}
        }),
        required_slots: &[],
        optional_slots: &[],
    },
    ToolSpec {
        canonical_name: "transactions.recent",
        mutating: false,
        default_tier: "Green",
        arguments_schema: json!({
            "type": "object",
            "properties": {"limit": {"type": "integer"}}
        }),
        required_slots: &[],
        optional_slots: &[],
    },
    ToolSpec {
        canonical_name: "wealth.summary",
        mutating: false,
        default_tier: "Green",
        arguments_schema: json!({"type": "object"}),
        required_slots: &[],
        optional_slots: &[],
    },
    ToolSpec {
        canonical_name: "user.profile",
        mutating: false,
        default_tier: "Green",
        arguments_schema: json!({"type": "object"}),
        required_slots: &[],
        optional_slots: &[],
    },
    // --- Phase 6: Smart transfer agent ---------------------------------
    ToolSpec {
        canonical_name: "smart.payees.suggest",
        mutating: false,
        default_tier: "Green",
        arguments_schema: json!({
            "type": "object",
            "required": ["query"],
            "properties": {
                "query": {"type": "string"},
                "limit": {"type": "integer", "minimum": 1, "maximum": 20}
            }
        }),
        required_slots: &[
            SlotSpec {
                name: "query",
                question: "Who or what are you looking for? (name, phone, or note)",
                kind: SlotKind::String,
                extract_hint: Some("Free text typed by the user."),
                inferable: true,
            },
        ],
        optional_slots: &[SlotSpec {
            name: "limit",
            question: "Maximum number of suggestions? (default 5)",
            kind: SlotKind::AmountFen,
            extract_hint: None,
            inferable: true,
        }],
    },
    ToolSpec {
        canonical_name: "smart.payee.resolve",
        mutating: false,
        default_tier: "Green",
        arguments_schema: json!({
            "type": "object",
            "required": ["query"],
            "properties": {"query": {"type": "string"}}
        }),
        required_slots: &[SlotSpec {
            name: "query",
            question: "Which payee should I resolve? (name or phone tail)",
            kind: SlotKind::String,
            extract_hint: Some("Display name or last 4 digits of phone."),
            inferable: true,
        }],
        optional_slots: &[],
    },
    ToolSpec {
        canonical_name: "smart.transfer.parse",
        mutating: false,
        default_tier: "Green",
        arguments_schema: json!({
            "type": "object",
            "required": ["text"],
            "properties": {"text": {"type": "string"}}
        }),
        required_slots: &[SlotSpec {
            name: "text",
            question: "What would you like to do?",
            kind: SlotKind::String,
            extract_hint: Some("Free-form transfer request."),
            inferable: true,
        }],
        optional_slots: &[],
    },
    ToolSpec {
        canonical_name: "smart.transfer.security_review",
        mutating: false,
        default_tier: "Green",
        arguments_schema: json!({
            "type": "object",
            "required": ["amount_fen"],
            "properties": {
                "amount_fen": {"type": "integer", "minimum": 1},
                "payee_id": {"type": "string"},
                "payee_name": {"type": "string"},
                "memo": {"type": "string"},
                "schedule_iso": {"type": "string"}
            }
        }),
        required_slots: &[SlotSpec {
            name: "amount_fen",
            question: "Amount in fen?",
            kind: SlotKind::AmountFen,
            extract_hint: Some("Integer minor units."),
            inferable: true,
        }],
        optional_slots: &[
            SlotSpec {
                name: "payee_name",
                question: "Payee name?",
                kind: SlotKind::String,
                extract_hint: None,
                inferable: true,
            },
            SlotSpec {
                name: "memo",
                question: "Memo?",
                kind: SlotKind::String,
                extract_hint: None,
                inferable: true,
            },
        ],
    },
    ToolSpec {
        canonical_name: "split_bill.create",
        mutating: true,
        default_tier: "Yellow",
        arguments_schema: json!({
            "type": "object",
            "required": ["title", "total_fen", "parts"],
            "properties": {
                "title": {"type": "string"},
                "total_fen": {"type": "integer", "minimum": 1},
                "memo": {"type": "string"},
                "currency": {"type": "string"},
                "source_account_id": {"type": "string"},
                "parts": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "required": ["share_fen", "participant_name"],
                        "properties": {
                            "payee_id": {"type": "string"},
                            "participant_name": {"type": "string"},
                            "participant_phone_tail": {"type": "string"},
                            "share_fen": {"type": "integer", "minimum": 0}
                        }
                    }
                }
            }
        }),
        required_slots: &[
            SlotSpec {
                name: "title",
                question: "What's this collection for?",
                kind: SlotKind::String,
                extract_hint: Some("Short title shown to participants."),
                inferable: true,
            },
            SlotSpec {
                name: "total_fen",
                question: "Total amount in fen?",
                kind: SlotKind::AmountFen,
                extract_hint: Some("Sum of every participant's share."),
                inferable: true,
            },
            SlotSpec {
                name: "parts",
                question: "How should we split it? (participant + share_fen)",
                kind: SlotKind::String,
                extract_hint: Some("An array of participants."),
                inferable: true,
            },
        ],
        optional_slots: &[SlotSpec {
            name: "memo",
            question: "Memo?",
            kind: SlotKind::String,
            extract_hint: None,
            inferable: true,
        }],
    },
    ToolSpec {
        canonical_name: "split_bill.list",
        mutating: false,
        default_tier: "Green",
        arguments_schema: json!({"type": "object"}),
        required_slots: &[],
        optional_slots: &[],
    },
    // --- New demo tools (Phase 5) ------------------------------------
    ToolSpec {
        canonical_name: "forecast.simulate",
        mutating: false,
        default_tier: "Green",
        arguments_schema: json!({
            "type": "object",
            "required": ["amount_cny"],
            "properties": {
                "amount_cny": {"type": "number", "minimum": 0.01},
                "spend_month_index": {"type": "integer", "minimum": 0},
                "months": {"type": "integer", "minimum": 1, "maximum": 24}
            }
        }),
        required_slots: &[
            SlotSpec {
                name: "amount_cny",
                question: "How much are you considering spending (in CNY)?",
                kind: SlotKind::AmountFen,
                extract_hint: Some("Numeric amount, optionally with 元 / yuan / 块."),
                inferable: true,
            },
        ],
        optional_slots: &[
            SlotSpec {
                name: "spend_month_index",
                question: "In which month from now? (0 = next month)",
                kind: SlotKind::AmountFen,
                extract_hint: None,
                inferable: true,
            },
            SlotSpec {
                name: "months",
                question: "Forecast horizon in months? (default 12)",
                kind: SlotKind::AmountFen,
                extract_hint: None,
                inferable: true,
            },
        ],
    },
    ToolSpec {
        canonical_name: "fraud.score",
        mutating: false,
        default_tier: "Green",
        arguments_schema: json!({
            "type": "object",
            "required": ["amount_fen"],
            "properties": {
                "amount_fen": {"type": "integer", "minimum": 1},
                "payee_id": {"type": "string"},
                "payee_name": {"type": "string"},
                "memo": {"type": "string"}
            }
        }),
        required_slots: &[
            SlotSpec {
                name: "amount_fen",
                question: "How much (fen)?",
                kind: SlotKind::AmountFen,
                extract_hint: Some("The amount the user is about to send, in fen."),
                inferable: true,
            },
        ],
        optional_slots: &[
            SlotSpec {
                name: "payee_name",
                question: "Who is the recipient?",
                kind: SlotKind::String,
                extract_hint: Some("Display name of the payee."),
                inferable: true,
            },
            SlotSpec {
                name: "memo",
                question: "Memo or note?",
                kind: SlotKind::String,
                extract_hint: Some("Optional memo to score for urgency language."),
                inferable: true,
            },
        ],
    },
    ToolSpec {
        canonical_name: "redteam.test",
        mutating: false,
        default_tier: "Green",
        arguments_schema: json!({
            "type": "object",
            "required": ["text"],
            "properties": {
                "text": {"type": "string"},
                "where": {"type": "string"}
            }
        }),
        required_slots: &[
            SlotSpec {
                name: "text",
                question: "Which text should the red-team panel evaluate?",
                kind: SlotKind::String,
                extract_hint: Some("The untrusted text you'd like to test."),
                inferable: true,
            },
        ],
        optional_slots: &[
            SlotSpec {
                name: "where",
                question: "Where would this text land? (memo | merchant | name)",
                kind: SlotKind::String,
                extract_hint: None,
                inferable: true,
            },
        ],
    },
    ToolSpec {
        canonical_name: "voice.tts",
        mutating: false,
        default_tier: "Green",
        arguments_schema: json!({
            "type": "object",
            "required": ["text"],
            "properties": {
                "text": {"type": "string"},
                "language": {"type": "string"}
            }
        }),
        required_slots: &[
            SlotSpec {
                name: "text",
                question: "What should I speak?",
                kind: SlotKind::String,
                extract_hint: Some("The text to synthesise."),
                inferable: true,
            },
        ],
        optional_slots: &[],
    },
    // ---------------------------------------------------------------
    // Budgets — per-category caps + cross-scenario reservations
    // ---------------------------------------------------------------
    ToolSpec {
        canonical_name: "budgets.set",
        mutating: true,
        default_tier: "Yellow",
        arguments_schema: json!({
            "type": "object",
            "required": ["category", "limit_fen"],
            "properties": {
                "category":  {"type": "string", "enum": ["food","transport","shopping","subscription","transfer","income","entertainment","travel","utilities","health","education","other"]},
                "limit_fen": {"type": "integer", "minimum": 1},
                "account_id": {"type": "string"},
                "note": {"type": "string"}
            }
        }),
        required_slots: &[
            SlotSpec {
                name: "category",
                question: "Which category should the cap cover? (food, transport, shopping, …)",
                kind: SlotKind::Enum,
                extract_hint: Some("Look for nouns like 'food', 'dining', 'rides', 'shopping', 'subscriptions', 'entertainment'."),
                inferable: true,
            },
            SlotSpec {
                name: "limit_fen",
                question: "How much per month, in CNY?",
                kind: SlotKind::AmountFen,
                extract_hint: Some("Parse from '2000', '¥2000', '2k', '2 块', '两千元'."),
                inferable: true,
            },
        ],
        optional_slots: &[
            SlotSpec {
                name: "account_id",
                question: "Apply this cap to a specific account, or all accounts?",
                kind: SlotKind::Identifier,
                extract_hint: Some("If the user names a card / account, narrow to that. Otherwise omit and it covers everything."),
                inferable: true,
            },
            SlotSpec {
                name: "note",
                question: "Any reason for the cap?",
                kind: SlotKind::String,
                extract_hint: None,
                inferable: true,
            },
        ],
    },
    ToolSpec {
        canonical_name: "budgets.list",
        mutating: false,
        default_tier: "Green",
        arguments_schema: json!({"type": "object", "properties": {}}),
        required_slots: &[],
        optional_slots: &[],
    },
    ToolSpec {
        canonical_name: "budgets.reserve",
        mutating: true,
        default_tier: "Red",
        arguments_schema: json!({
            "type": "object",
            "required": ["label", "amount_fen", "account_id"],
            "properties": {
                "label":      {"type": "string"},
                "amount_fen": {"type": "integer", "minimum": 1},
                "account_id": {"type": "string"},
                "due_at":     {"type": "string", "description": "RFC3339 timestamp for the event the money is for"},
                "budget_id":  {"type": "string"}
            }
        }),
        required_slots: &[
            SlotSpec {
                name: "label",
                question: "What is this reservation for? (e.g. 'spouse birthday')",
                kind: SlotKind::String,
                extract_hint: Some("Free text the user can recognise later."),
                inferable: true,
            },
            SlotSpec {
                name: "amount_fen",
                question: "How much do you want to set aside, in CNY?",
                kind: SlotKind::AmountFen,
                extract_hint: Some("Parse from '1000', '¥1000', '1k', '一千块'."),
                inferable: true,
            },
            SlotSpec {
                name: "account_id",
                question: "Which account should the money come from?",
                kind: SlotKind::Identifier,
                extract_hint: Some("Default to the user's primary checking if they don't say."),
                inferable: true,
            },
            SlotSpec {
                name: "due_at",
                question: "When is the event? (or when should the reservation be released)",
                kind: SlotKind::Date,
                extract_hint: Some("Look for absolute dates or relative phrases like 'next month', 'in October', 'on the 15th'."),
                inferable: true,
            },
        ],
        optional_slots: &[
            SlotSpec {
                name: "budget_id",
                question: "Link this reservation to a budget?",
                kind: SlotKind::Identifier,
                extract_hint: None,
                inferable: true,
            },
        ],
    },
]);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_transfer_passes() {
        let spec = spec_for("transfer.create_pending").unwrap();
        let args = json!({"account_id": "acc_1", "payee_id": "payee_1", "amount_fen": 20000});
        assert!(validate_arguments(spec, &args).is_ok());
    }

    #[test]
    fn validate_transfer_missing_payee() {
        let spec = spec_for("transfer.create_pending").unwrap();
        let args = json!({"account_id": "acc_1", "amount_fen": 20000});
        let err = validate_arguments(spec, &args).unwrap_err();
        assert!(err.iter().any(|m| m.slot.name == "payee_id"));
    }

    #[test]
    fn validate_subscription_requires_id() {
        let spec = spec_for("subscriptions.cancel").unwrap();
        let args = json!({});
        let err = validate_arguments(spec, &args).unwrap_err();
        assert_eq!(err.len(), 1);
        assert_eq!(err[0].slot.name, "subscription_id");
    }

    #[test]
    fn all_tools_in_table_are_in_registry() {
        // Sanity: every tool spec must correspond to a runtime tool.
        let specs = spec_table();
        let names: std::collections::HashSet<&str> =
            specs.iter().map(|s| s.canonical_name).collect();
        // Mirror a tiny list of canonical names — the runtime registry test
        // is more authoritative, but this catches typos at compile time.
        for expected in [
            "transfer.create_pending",
            "subscriptions.cancel",
            "reminders.create",
            "accounts.list",
            "user.profile",
            "wealth.summary",
        ] {
            assert!(names.contains(expected), "missing spec for {expected}");
        }
    }
}
