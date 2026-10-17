//! ORM-style row models. Maps 1:1 to the schema in `migrations/`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: String,
    pub username: String,
    pub password_hash: String,
    pub display_name: String,
    pub preferred_language: String,
    pub role: String,
    pub safety_lock_state: String,
    pub safety_lock_reason: Option<String>,
    pub failed_mfa_count: i32,
    pub risk_tolerance: String,
    pub horizon: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SessionRow {
    pub id: String,
    pub user_id: String,
    pub csrf_token: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Account {
    pub id: String,
    pub user_id: String,
    pub display_name: String,
    pub masked_number: String,
    pub currency: String,
    pub available_balance_fen: i64,
    pub reserved_balance_fen: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Transaction {
    pub id: String,
    pub account_id: String,
    pub amount_fen: i64,
    pub direction: String,
    pub category: String,
    pub merchant: String,
    pub memo: String,
    pub occurred_at: DateTime<Utc>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Payee {
    pub id: String,
    pub owner_user_id: String,
    pub display_name: String,
    pub phone_tail: Option<String>,
    pub mock_destination: String,
    pub verified: bool,
    pub first_used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Transfer {
    pub id: String,
    pub source_account_id: String,
    pub payee_id: String,
    pub amount_fen: i64,
    pub memo: String,
    pub schedule: Option<DateTime<Utc>>,
    pub idempotency_key: String,
    pub status: String,
    pub posted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Subscription {
    pub id: String,
    pub user_id: String,
    pub merchant: String,
    pub estimated_amount_fen: i64,
    pub interval_days: i32,
    pub next_expected_at: DateTime<Utc>,
    pub last_charge_at: Option<DateTime<Utc>>,
    pub evidence: String,
    pub cancellation_status: String,
    pub cancelled_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Reminder {
    pub id: String,
    pub user_id: String,
    pub kind: String,
    pub subject: String,
    pub due_at: DateTime<Utc>,
    pub status: String,
    pub linked_subscription_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct SavingsGoal {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub target_fen: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Reservation {
    pub id: String,
    pub goal_id: String,
    pub amount_fen: i64,
    pub state: String,
    pub source_account_id: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Card {
    pub id: String,
    pub user_id: String,
    pub display_name: String,
    pub masked_number: String,
    pub expiry_month: i32,
    pub expiry_year: i32,
    pub status: String,
    pub single_tx_limit_fen: Option<i64>,
    pub monthly_limit_fen: Option<i64>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct WealthProduct {
    pub id: String,
    pub name: String,
    pub category: String,
    pub risk_level: String,
    pub expected_annual_return_pct: f64,
    pub min_subscription_fen: i64,
    pub suitability_required: String,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct WealthHolding {
    pub id: String,
    pub user_id: String,
    pub product_id: String,
    pub amount_fen: i64,
    pub subscribed_at: DateTime<Utc>,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Workflow {
    pub id: String,
    pub user_id: String,
    pub request_id: String,
    pub plan_version: i32,
    pub status: String,
    pub interrupt_reason: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct WorkflowNode {
    pub id: String,
    pub workflow_id: String,
    pub node_index: i32,
    pub tool: String,
    pub parameters: String,
    pub dependencies: String,
    pub risk_tier: String,
    pub approval_required: bool,
    pub idempotency_key: String,
    pub status: String,
    pub result: Option<String>,
    pub error: Option<String>,
    pub compensating_node_id: Option<String>,
    pub pending_action_id: Option<String>,
    // v2 fields — added by migration 20251115000000_agent_v2.sql.
    pub depends_on: String,
    pub requires: String,
    pub on_failure: String,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PendingAction {
    pub id: String,
    pub user_id: String,
    pub tool: String,
    pub parameters: String,
    pub parameter_hash: String,
    pub risk_tier: String,
    pub expires_at: DateTime<Utc>,
    pub confirmation_status: String,
    pub mfa_status: String,
    pub created_at: DateTime<Utc>,
    pub workflow_node_id: Option<String>,
    pub idempotency_key: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AuditEvent {
    pub id: String,
    pub created_at: DateTime<Utc>,
    pub actor_user_id: Option<String>,
    pub session_id: Option<String>,
    pub request_id: Option<String>,
    pub workflow_id: Option<String>,
    pub node_id: Option<String>,
    pub action: String,
    pub tool: Option<String>,
    pub parameters_masked: String,
    pub risk_tier: Option<String>,
    pub policy_reason: Option<String>,
    pub confirmation_status: Option<String>,
    pub mfa_status: Option<String>,
    pub result: String,
    pub error_code: Option<String>,
    pub prev_hash: String,
    pub hash: String,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct SafetyEvent {
    pub id: String,
    pub user_id: String,
    pub kind: String,
    pub created_at: DateTime<Utc>,
    pub metadata: String,
}

// --- Phase 6: Smart transfers ---------------------------------------------

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct SplitBill {
    pub id: String,
    pub owner_user_id: String,
    pub title: String,
    pub memo: String,
    pub total_fen: i64,
    pub currency: String,
    pub source_account_id: Option<String>,
    pub status: String,
    pub due_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct SplitBillPart {
    pub id: String,
    pub split_bill_id: String,
    pub payee_id: Option<String>,
    pub participant_name: String,
    pub participant_phone_tail: Option<String>,
    pub share_fen: i64,
    pub status: String,
    pub paid_at: Option<DateTime<Utc>>,
    pub paid_transfer_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Per-category monthly spending cap. The agent and optimization report use
/// `category` to match against `transactions.category`; `account_id` is
/// optional and narrows the scope to a single account. `period_start` /
/// `period_end` are optional half-open windows — a NULL pair means the
/// default "rolling 30 days from NOW()" the API exposes.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Budget {
    pub id: String,
    pub user_id: String,
    pub category: String,
    pub limit_fen: i64,
    pub period_start: Option<DateTime<Utc>>,
    pub period_end: Option<DateTime<Utc>>,
    pub account_id: Option<String>,
    pub note: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A pre-allocated hold tied to a budget or a future event (e.g. spouse's
/// birthday reservation). Backed by a savings goal so the existing
/// reservation/release machinery can settle it when the date arrives.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct BudgetReservation {
    pub id: String,
    pub user_id: String,
    pub budget_id: Option<String>,
    pub goal_id: Option<String>,
    pub label: String,
    pub amount_fen: i64,
    pub due_at: Option<DateTime<Utc>>,
    pub source_account_id: Option<String>,
    pub state: String,
    pub created_at: DateTime<Utc>,
}
