//! Planner + workflow DAG.
//!
//! Re-resolves every argument server-side, asks the policy engine for the
//! tier per node, creates pending actions for Yellow/Red nodes, and
//! executes Green nodes inline.

use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use serde_json::{json, Value};
use sqlx::PgPool;

use crate::audit::{self, AppendEventParams};
use crate::canonical::parameter_hash;
use crate::config;
use crate::ids;
use crate::models::{
    Account, Payee, PendingAction, SavingsGoal, Subscription, User, WealthProduct, Workflow,
    WorkflowNode,
};
use crate::money::coerce_amount_fen;
use crate::policy::{self, Decision, DecideResult};
use crate::safety;
use crate::tools::{self, ToolContext, ToolError};

#[derive(Debug, Clone, Serialize)]
pub struct PlannedNode {
    pub tool: String,
    pub arguments: Value,
    pub risk_tier: String,
    pub decision_reason: String,
    pub pending_action_id: Option<String>,
    pub idempotency_key: Option<String>,
    pub parameter_hash: Option<String>,
    pub compensation: Option<String>,
}

// --- Resolvers ------------------------------------------------------------

pub async fn resolve_account(
    pool: &PgPool,
    user: &User,
    account_id: Option<&str>,
) -> Result<Account, ToolError> {
    if let Some(id) = account_id {
        let acc: Option<Account> = sqlx::query_as("SELECT * FROM accounts WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
            .map_err(|e| ToolError::Other(e.to_string()))?;
        let acc = acc.ok_or(ToolError::NotFound("Account not found"))?;
        if acc.user_id != user.id {
            return Err(ToolError::NotFound("Account not found"));
        }
        return Ok(acc);
    }
    let acc: Option<Account> = sqlx::query_as(
        "SELECT * FROM accounts WHERE user_id = $1 ORDER BY created_at ASC LIMIT 1",
    )
    .bind(&user.id)
    .fetch_optional(pool)
    .await
    .map_err(|e| ToolError::Other(e.to_string()))?;
    acc.ok_or(ToolError::NotFound("No account for user"))
}

pub async fn resolve_payee_by_query(
    pool: &PgPool,
    user: &User,
    query: &str,
) -> Result<Payee, ToolError> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return Err(ToolError::Schema("recipient_query is required"));
    }
    let rows: Vec<Payee> = sqlx::query_as("SELECT * FROM payees WHERE owner_user_id = $1")
        .bind(&user.id)
        .fetch_all(pool)
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    let mut matches: Vec<Payee> = vec![];
    for p in rows {
        let d = p.display_name.to_lowercase();
        if d == q || p.phone_tail.as_deref() == Some(&q) {
            matches.push(p.clone());
            continue;
        }
        if d.starts_with(&q) || d.contains(&q) {
            matches.push(p.clone());
            continue;
        }
    }
    if matches.is_empty() {
        return Err(ToolError::NotFound("Payee not found"));
    }
    if matches.len() > 1 {
        let names: Vec<String> = matches.iter().map(|p| p.display_name.clone()).collect();
        return Err(ToolError::Clarification(format!(
            "Multiple payees match: {}",
            names.join(", ")
        )));
    }
    Ok(matches.remove(0))
}

pub async fn resolve_payee_by_phone_tail(
    pool: &PgPool,
    user: &User,
    tail: &str,
) -> Result<Payee, ToolError> {
    let rows: Vec<Payee> = sqlx::query_as(
        "SELECT * FROM payees WHERE owner_user_id = $1 AND phone_tail = $2",
    )
    .bind(&user.id)
    .bind(tail)
    .fetch_all(pool)
    .await
    .map_err(|e| ToolError::Other(e.to_string()))?;
    if rows.is_empty() {
        return Err(ToolError::NotFound("Payee not found"));
    }
    if rows.len() > 1 {
        let names: Vec<String> = rows.iter().map(|p| p.display_name.clone()).collect();
        return Err(ToolError::Clarification(format!(
            "Multiple payees with tail: {}",
            names.join(", ")
        )));
    }
    Ok(rows.into_iter().next().unwrap())
}

pub async fn resolve_subscription(
    pool: &PgPool,
    user: &User,
    merchant_query: &str,
) -> Result<Subscription, ToolError> {
    let q = merchant_query.trim().to_lowercase();
    let rows: Vec<Subscription> = sqlx::query_as("SELECT id, user_id, merchant, estimated_amount_fen, interval_days, next_expected_at, last_charge_at, evidence::text AS evidence, cancellation_status, cancelled_at, created_at FROM subscriptions WHERE user_id = $1")
        .bind(&user.id)
        .fetch_all(pool)
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    let matches: Vec<Subscription> = rows
        .into_iter()
        .filter(|s| {
            let sm = s.merchant.to_lowercase();
            sm.contains(&q) || q.contains(&sm)
        })
        .collect();
    if matches.is_empty() {
        return Err(ToolError::NotFound("Subscription not found"));
    }
    if matches.len() > 1 {
        let names: Vec<String> = matches.iter().map(|s| s.merchant.clone()).collect();
        return Err(ToolError::Clarification(format!(
            "Multiple subscriptions match: {}",
            names.join(", ")
        )));
    }
    Ok(matches.into_iter().next().unwrap())
}

// --- Node planning -------------------------------------------------------

async fn decide_node(
    pool: &PgPool,
    user: &User,
    tool: &str,
    args: &Value,
    request_id: &str,
) -> Result<PlannedNode, ToolError> {
    match tool {
        "transfer.create_pending" => {
            let account = resolve_account(pool, user, args.get("account_id").and_then(|v| v.as_str())).await?;
            let amount_fen = if let Some(v) = args.get("amount_fen") {
                coerce_amount_fen(v).map_err(|e| ToolError::Other(e))?
            } else if let Some(v) = args.get("amount_cny") {
                coerce_amount_fen(v).map_err(|e| ToolError::Other(e))?
            } else {
                return Err(ToolError::Schema("amount_fen or amount_cny required"));
            };
            let memo = args
                .get("memo")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let payee = if let Some(t) = args.get("phone_tail").and_then(|v| v.as_str()) {
                if !t.is_empty() {
                    resolve_payee_by_phone_tail(pool, user, t).await?
                } else {
                    resolve_payee_by_query(
                        pool,
                        user,
                        args.get("recipient_query")
                            .or(args.get("payee_id"))
                            .and_then(|v| v.as_str())
                            .unwrap_or(""),
                    )
                    .await?
                }
            } else {
                resolve_payee_by_query(
                    pool,
                    user,
                    args.get("recipient_query")
                        .or(args.get("payee_id"))
                        .and_then(|v| v.as_str())
                        .unwrap_or(""),
                )
                .await?
            };
            let decision = policy::decide_transfer(pool, user, &account, &payee, amount_fen, memo)
                .await
                .map_err(|e| ToolError::Other(e.to_string()))?;
            // Scam/fraud guard assessment. We run it here (in addition
            // to the policy engine) so the chat UI can render the
            // narrative alongside the tier. The policy engine already
            // called it and may have escalated the tier on its own.
            let mut fraud_assessment =
                crate::fraud::score_transfer(pool, user, &payee, amount_fen, memo)
                    .await
                    .map_err(|e| ToolError::Other(e.to_string()))?;
            if !fraud_assessment.flags.is_empty() {
                if let Some(narrative) = crate::fraud::narrate_scam_flags(
                    crate::config::get_settings(),
                    &fraud_assessment,
                    &payee.display_name,
                    amount_fen,
                )
                .await
                {
                    fraud_assessment.narrative = Some(narrative);
                }
            }
            // Admin-configured transfer rules. These run *after* the policy
            // engine and the fraud scorer — so they are an additional
            // defence-in-depth layer, not a replacement. If any rule
            // matches with severity=block, the transfer is denied.
            let rule_eval = crate::agent_rules::evaluate_transfer(
                pool,
                &user.id,
                &crate::agent_rules::TransferContext {
                    payee_name: &payee.display_name,
                    payee_phone_tail: payee.phone_tail.as_deref(),
                    memo,
                    amount_fen,
                    source_account_id: &account.id,
                },
            )
            .await
            .map_err(|e| ToolError::Other(e.to_string()))?;
            crate::agent_rules::record_hits(
                pool,
                &user.id,
                Some(request_id),
                &rule_eval,
            )
            .await;
            if rule_eval.blocked() {
                if let Some(reason) = rule_eval.block_reason() {
                    return Err(ToolError::Forbidden(reason));
                }
            }
            let mut resolved = json!({
                "account_id": account.id,
                "payee_id": payee.id,
                "amount_fen": amount_fen,
                "memo": memo,
                "fraud_assessment": serde_json::to_value(&fraud_assessment).unwrap_or(serde_json::json!({})),
            });
            if let Some(s) = args.get("schedule").and_then(|v| v.as_str()) {
                resolved["schedule"] = Value::String(s.to_string());
            }
            Ok(PlannedNode {
                tool: tool.to_string(),
                arguments: resolved,
                risk_tier: decision.tier,
                decision_reason: decision.reason,
                pending_action_id: None,
                idempotency_key: None,
                parameter_hash: None,
                compensation: Some("goals.release_reservation".to_string()),
            })
        }
        "subscriptions.cancel" => {
            let sub = resolve_subscription(
                pool,
                user,
                args.get("merchant_query")
                    .or(args.get("subscription_id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or(""),
            )
            .await?;
            let decision = policy::decide_subscription_cancel();
            Ok(PlannedNode {
                tool: tool.to_string(),
                arguments: json!({ "subscription_id": sub.id }),
                risk_tier: decision.tier,
                decision_reason: "subscription_cancel".to_string(),
                pending_action_id: None,
                idempotency_key: None,
                parameter_hash: None,
                compensation: None,
            })
        }
        "subscriptions.remind" => {
            let sub = resolve_subscription(
                pool,
                user,
                args.get("merchant_query")
                    .or(args.get("subscription_id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or(""),
            )
            .await?;
            Ok(PlannedNode {
                tool: tool.to_string(),
                arguments: json!({
                    "subscription_id": sub.id,
                    "days_before": args.get("days_before").and_then(|v| v.as_i64()).unwrap_or(3),
                }),
                risk_tier: "Green".to_string(),
                decision_reason: "subscription_reminder".to_string(),
                pending_action_id: None,
                idempotency_key: None,
                parameter_hash: None,
                compensation: None,
            })
        }
        "goals.create_reservation" => {
            let account = resolve_account(pool, user, args.get("account_id").and_then(|v| v.as_str())).await?;
            let amount_fen = if let Some(v) = args.get("amount_fen") {
                coerce_amount_fen(v).map_err(|e| ToolError::Other(e))?
            } else if let Some(v) = args.get("amount_cny") {
                coerce_amount_fen(v).map_err(|e| ToolError::Other(e))?
            } else {
                return Err(ToolError::Schema("amount_fen or amount_cny required"));
            };
            let goal_id = if let Some(g) = args.get("goal_id").and_then(|v| v.as_str()) {
                g.to_string()
            } else {
                let goal = SavingsGoal {
                    id: ids::new_id("goal"),
                    user_id: user.id.clone(),
                    name: args
                        .get("goal_name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Birthday")
                        .to_string(),
                    target_fen: amount_fen,
                    created_at: Utc::now(),
                };
                sqlx::query(
                    "INSERT INTO savings_goals (id, user_id, name, target_fen, created_at) VALUES ($1, $2, $3, $4, $5)",
                )
                .bind(&goal.id)
                .bind(&goal.user_id)
                .bind(&goal.name)
                .bind(goal.target_fen)
                .bind(goal.created_at)
                .execute(pool)
                .await
                .map_err(|e| ToolError::Other(e.to_string()))?;
                goal.id
            };
            let decision = policy::decide_goal_reservation();
            Ok(PlannedNode {
                tool: tool.to_string(),
                arguments: json!({
                    "goal_id": goal_id,
                    "amount_fen": amount_fen,
                    "account_id": account.id,
                }),
                risk_tier: decision.tier,
                decision_reason: "goal_reservation".to_string(),
                pending_action_id: None,
                idempotency_key: None,
                parameter_hash: None,
                compensation: Some("goals.release_reservation".to_string()),
            })
        }
        "goals.consume_reservation" => {
            let decision = policy::decide_goal_reservation();
            Ok(PlannedNode {
                tool: tool.to_string(),
                arguments: json!({
                    "reservation_id": args.get("reservation_id").and_then(|v| v.as_str()).unwrap_or(""),
                }),
                risk_tier: decision.tier,
                decision_reason: "goal_consume".to_string(),
                pending_action_id: None,
                idempotency_key: None,
                parameter_hash: None,
                compensation: None,
            })
        }
        "goals.release_reservation" => {
            let decision = policy::decide_goal_reservation();
            Ok(PlannedNode {
                tool: tool.to_string(),
                arguments: json!({
                    "reservation_id": args.get("reservation_id").and_then(|v| v.as_str()).unwrap_or(""),
                }),
                risk_tier: decision.tier,
                decision_reason: "goal_release".to_string(),
                pending_action_id: None,
                idempotency_key: None,
                parameter_hash: None,
                compensation: None,
            })
        }
        // -----------------------------------------------------------
        // Budgets — caps + cross-scenario reservations.
        // -----------------------------------------------------------
        "budgets.set" => {
            let category = args
                .get("category")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::Schema("category required"))?
                .to_string();
            let limit_fen = if let Some(v) = args.get("limit_fen") {
                coerce_amount_fen(v).map_err(ToolError::Other)?
            } else if let Some(v) = args.get("amount_cny") {
                coerce_amount_fen(v).map_err(ToolError::Other)?
            } else {
                return Err(ToolError::Schema("limit_fen required"));
            };
            let account_id = args
                .get("account_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            // If the user named a specific account, verify it exists + belongs
            // to them; otherwise leave it None and the tool applies it
            // globally.
            let resolved_account = if let Some(a) = &account_id {
                Some(resolve_account(pool, user, Some(a.as_str())).await?.id)
            } else {
                None
            };
            let decision = policy::decide_budget_set();
            Ok(PlannedNode {
                tool: tool.to_string(),
                arguments: json!({
                    "category": category,
                    "limit_fen": limit_fen,
                    "account_id": resolved_account,
                    "note": args.get("note").and_then(|v| v.as_str()),
                }),
                risk_tier: decision.tier,
                decision_reason: "budget_set".to_string(),
                pending_action_id: None,
                idempotency_key: None,
                parameter_hash: None,
                compensation: None,
            })
        }
        "budgets.reserve" => {
            let label = args
                .get("label")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::Schema("label required"))?
                .to_string();
            let amount_fen = if let Some(v) = args.get("amount_fen") {
                coerce_amount_fen(v).map_err(ToolError::Other)?
            } else if let Some(v) = args.get("amount_cny") {
                coerce_amount_fen(v).map_err(ToolError::Other)?
            } else {
                return Err(ToolError::Schema("amount_fen required"));
            };
            let account = resolve_account(pool, user, args.get("account_id").and_then(|v| v.as_str())).await?;
            let due_at = args.get("due_at").and_then(|v| v.as_str()).map(|s| s.to_string());
            let decision = policy::decide_budget_reserve();
            Ok(PlannedNode {
                tool: tool.to_string(),
                arguments: json!({
                    "label": label,
                    "amount_fen": amount_fen,
                    "account_id": account.id,
                    "due_at": due_at,
                    "budget_id": args.get("budget_id").and_then(|v| v.as_str()),
                }),
                risk_tier: decision.tier,
                decision_reason: "budget_reserve".to_string(),
                pending_action_id: None,
                idempotency_key: None,
                parameter_hash: None,
                compensation: Some("budgets.release_reservation".to_string()),
            })
        }
        "reminders.create" => {
            let subject = args
                .get("subject")
                .and_then(|v| v.as_str())
                .unwrap_or("Reminder");
            Ok(PlannedNode {
                tool: tool.to_string(),
                arguments: json!({
                    "subject": subject,
                    "due_at": args.get("due_at").and_then(|v| v.as_str()).unwrap_or(""),
                }),
                risk_tier: "Green".to_string(),
                decision_reason: "reminder_create".to_string(),
                pending_action_id: None,
                idempotency_key: None,
                parameter_hash: None,
                compensation: None,
            })
        }
        "profile.resolve_date" => {
            Ok(PlannedNode {
                tool: tool.to_string(),
                arguments: json!({
                    "date": args.get("date").and_then(|v| v.as_str()).unwrap_or(""),
                }),
                risk_tier: "Green".to_string(),
                decision_reason: "date_resolve".to_string(),
                pending_action_id: None,
                idempotency_key: None,
                parameter_hash: None,
                compensation: None,
            })
        }
        "accounts.balance_check" => {
            let account = resolve_account(pool, user, args.get("account_id").and_then(|v| v.as_str())).await?;
            Ok(PlannedNode {
                tool: tool.to_string(),
                arguments: json!({ "account_id": account.id }),
                risk_tier: "Green".to_string(),
                decision_reason: "balance_check".to_string(),
                pending_action_id: None,
                idempotency_key: None,
                parameter_hash: None,
                compensation: None,
            })
        }
        "connectors.merchant.order" => {
            let amount_fen = if let Some(v) = args.get("amount_fen") {
                coerce_amount_fen(v).unwrap_or(0)
            } else {
                coerce_amount_fen(args.get("amount_cny").unwrap_or(&Value::Null)).unwrap_or(0)
            };
            Ok(PlannedNode {
                tool: tool.to_string(),
                arguments: json!({
                    "merchant": args.get("merchant").and_then(|v| v.as_str()).unwrap_or("mock-merchant"),
                    "amount_fen": amount_fen,
                    "deliver_at": args.get("deliver_at").and_then(|v| v.as_str()),
                }),
                risk_tier: "Yellow".to_string(),
                decision_reason: "merchant_order".to_string(),
                pending_action_id: None,
                idempotency_key: None,
                parameter_hash: None,
                compensation: None,
            })
        }
        "cards.issue" => {
            let decision = policy::decide_card_action("issue");
            Ok(PlannedNode {
                tool: tool.to_string(),
                arguments: json!({
                    "display_name": args.get("display_name").and_then(|v| v.as_str()).unwrap_or("Virtual card"),
                }),
                risk_tier: decision.tier,
                decision_reason: "card_issue".to_string(),
                pending_action_id: None,
                idempotency_key: None,
                parameter_hash: None,
                compensation: None,
            })
        }
        "cards.action" => {
            let kind = args
                .get("kind")
                .and_then(|v| v.as_str())
                .ok_or(ToolError::Schema("kind required"))?;
            let decision = policy::decide_card_action(kind);
            let mut resolved = json!({
                "card_id": args.get("card_id").and_then(|v| v.as_str()).unwrap_or(""),
                "kind": kind,
            });
            if let Some(v) = args.get("single_tx_limit_fen") {
                resolved["single_tx_limit_fen"] = v.clone();
            }
            if let Some(v) = args.get("monthly_limit_fen") {
                resolved["monthly_limit_fen"] = v.clone();
            }
            Ok(PlannedNode {
                tool: tool.to_string(),
                arguments: resolved,
                risk_tier: decision.tier,
                decision_reason: format!("card_{kind}"),
                pending_action_id: None,
                idempotency_key: None,
                parameter_hash: None,
                compensation: None,
            })
        }
        "wealth.subscribe" => {
            let product_id = args
                .get("product_id")
                .and_then(|v| v.as_str())
                .ok_or(ToolError::Schema("product_id required"))?;
            let product: Option<WealthProduct> = sqlx::query_as(
                "SELECT * FROM wealth_products WHERE id = $1",
            )
            .bind(product_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| ToolError::Other(e.to_string()))?;
            let product = product.ok_or(ToolError::NotFound("Product not found"))?;
            let amount_fen = if let Some(v) = args.get("amount_fen") {
                coerce_amount_fen(v).map_err(|e| ToolError::Other(e))?
            } else if let Some(v) = args.get("amount_cny") {
                coerce_amount_fen(v).map_err(|e| ToolError::Other(e))?
            } else {
                return Err(ToolError::Schema("amount_fen or amount_cny required"));
            };
            let decision = match policy::decide_wealth_subscribe(&product, user) {
                DecideResult::Decision(d) => d,
                DecideResult::SuitabilityDenied(r) => {
                    return Err(ToolError::Other(format!("suitability_failed: {r}")));
                }
            };
            let account = resolve_account(pool, user, args.get("account_id").and_then(|v| v.as_str())).await?;
            Ok(PlannedNode {
                tool: tool.to_string(),
                arguments: json!({
                    "product_id": product.id,
                    "amount_fen": amount_fen,
                    "account_id": account.id,
                }),
                risk_tier: decision.tier,
                decision_reason: decision.reason,
                pending_action_id: None,
                idempotency_key: None,
                parameter_hash: None,
                compensation: None,
            })
        }
        "wealth.redeem" => {
            let account = resolve_account(pool, user, args.get("account_id").and_then(|v| v.as_str())).await?;
            let decision = policy::decide_wealth_redeem();
            Ok(PlannedNode {
                tool: tool.to_string(),
                arguments: json!({
                    "holding_id": args.get("holding_id").and_then(|v| v.as_str()).unwrap_or(""),
                    "account_id": account.id,
                }),
                risk_tier: decision.tier,
                decision_reason: "wealth_redeem".to_string(),
                pending_action_id: None,
                idempotency_key: None,
                parameter_hash: None,
                compensation: None,
            })
        }
        _ => Err(ToolError::Schema("Unknown tool")),
    }
}

// --- Plan creation -------------------------------------------------------

pub async fn build_plan(
    pool: &PgPool,
    user: &User,
    proposed_actions: &[Value],
    request_id: &str,
) -> Result<Workflow, ToolError> {
    safety::assert_unlocked(user).map_err(|e| ToolError::Other(e.message))?;
    let settings = config::get_settings();
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;

    let mut workflow = Workflow {
        id: ids::new_id("wf"),
        user_id: user.id.clone(),
        request_id: request_id.to_string(),
        plan_version: 1,
        status: "awaiting_confirmation".to_string(),
        interrupt_reason: None,
        created_at: Utc::now(),
    };
    sqlx::query(
        "INSERT INTO workflows (id, user_id, request_id, plan_version, status, created_at) VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(&workflow.id)
    .bind(&workflow.user_id)
    .bind(&workflow.request_id)
    .bind(workflow.plan_version)
    .bind(&workflow.status)
    .bind(workflow.created_at)
    .execute(&mut *tx)
    .await
    .map_err(|e| ToolError::Other(e.to_string()))?;

    let reg = tools::registry();
    let mut planned: Vec<PlannedNode> = vec![];
    for raw in proposed_actions {
        let tool_raw = raw.get("tool").and_then(|v| v.as_str()).unwrap_or("");
        let tool = tools::canonicalize_tool_name(tool_raw);
        let args = raw.get("arguments").cloned().unwrap_or(Value::Object(Default::default()));
        if !reg.contains_key(tool) {
            log::warn!("planner: tool not in allowlist: {tool_raw}");
            return Err(ToolError::Schema("Tool not in allowlist"));
        }
        planned.push(decide_node(pool, user, tool, &args, request_id).await?);
    }

    let mut any_failed = false;
    for (idx, pnode) in planned.iter().enumerate() {
        let mut full_params = pnode.arguments.clone();
        if let Value::Object(ref mut m) = full_params {
            m.insert("tier".into(), Value::String(pnode.risk_tier.clone()));
        }
        let idem = format!("{}:{}:{}", workflow.id, idx, parameter_hash(&pnode.arguments));
        let params_str = serde_json::to_string(&full_params).unwrap_or("{}".to_string());
        let deps_str = "[]";

        let status = if pnode.risk_tier == "Green" {
            "completed"
        } else if pnode.risk_tier == "Red" {
            "awaiting_mfa"
        } else {
            "awaiting_confirmation"
        };
        let approval_required = matches!(pnode.risk_tier.as_str(), "Yellow" | "Red");
        let node = WorkflowNode {
            id: ids::new_id("node"),
            workflow_id: workflow.id.clone(),
            node_index: idx as i32,
            tool: pnode.tool.clone(),
            parameters: params_str,
            dependencies: deps_str.to_string(),
            risk_tier: pnode.risk_tier.clone(),
            approval_required,
            idempotency_key: idem.clone(),
            status: status.to_string(),
            result: None,
            error: None,
            compensating_node_id: None,
            pending_action_id: None,
            depends_on: deps_str.to_string(),
            requires: "[]".to_string(),
            on_failure: "fail".to_string(),
            started_at: None,
            finished_at: None,
        };
        sqlx::query(
            "INSERT INTO workflow_nodes (id, workflow_id, node_index, tool, parameters, dependencies, risk_tier, approval_required, idempotency_key, status)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        )
        .bind(&node.id)
        .bind(&node.workflow_id)
        .bind(node.node_index)
        .bind(&node.tool)
        .bind(&node.parameters)
        .bind(&node.dependencies)
        .bind(&node.risk_tier)
        .bind(node.approval_required)
        .bind(&node.idempotency_key)
        .bind(&node.status)
        .execute(&mut *tx)
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
        drop(node);

        if pnode.risk_tier == "Green" {
            // Execute inline.
            let mut full = full_params.clone();
            if let Value::Object(ref mut m) = full {
                m.remove("tier");
            }
            let ctx = ToolContext {
                pool: pool.clone(),
                user: user.clone(),
                request_id: request_id.to_string(),
                session_id: None,
            };
            let tool_fn = reg.get(pnode.tool.as_str()).unwrap();
            let result = tool_fn(&ctx, &full).await;
            match result {
                Ok(v) => {
                    let result_str = serde_json::to_string(&v).unwrap_or("{}".to_string());
                    sqlx::query("UPDATE workflow_nodes SET result = $1 WHERE id = (SELECT id FROM workflow_nodes WHERE workflow_id = $2 AND node_index = $3)")
                        .bind(&result_str)
                        .bind(&workflow.id)
                        .bind(idx as i32)
                        .execute(&mut *tx)
                        .await
                        .map_err(|e| ToolError::Other(e.to_string()))?;
                }
                Err(e) => {
                    let err_str = serde_json::to_string(&json!({
                        "code": e.code(),
                        "message": e.to_string(),
                    }))
                    .unwrap_or("{}".to_string());
                    sqlx::query("UPDATE workflow_nodes SET status = 'failed', error = $1 WHERE id = (SELECT id FROM workflow_nodes WHERE workflow_id = $2 AND node_index = $3)")
                        .bind(&err_str)
                        .bind(&workflow.id)
                        .bind(idx as i32)
                        .execute(&mut *tx)
                        .await
                        .map_err(|e2| ToolError::Other(e2.to_string()))?;
                    sqlx::query("UPDATE workflows SET status = 'failed' WHERE id = $1")
                        .bind(&workflow.id)
                        .execute(&mut *tx)
                        .await
                        .map_err(|e2| ToolError::Other(e2.to_string()))?;
                    any_failed = true;
                    tx.commit()
                        .await
                        .map_err(|e2| ToolError::Other(e2.to_string()))?;
                    audit::append_event(
                        pool,
                        AppendEventParams {
                            action: "workflow.node.failed",
                            actor_user_id: Some(&user.id),
                            session: None,
                            request_id: Some(request_id),
                            workflow_id: Some(&workflow.id),
                            node_id: None,
                            tool: Some(&pnode.tool),
                            parameters_masked: Some(pnode.arguments.clone()),
                            risk_tier: Some(&pnode.risk_tier),
                            policy_reason: None,
                            confirmation_status: None,
                            mfa_status: None,
                            result: "error",
                            error_code: Some(e.code()),
                        },
                    )
                    .await
                    .ok();
                    return Err(e);
                }
            }
        } else {
            // Yellow/Red — create pending_action.
            let param_hash = parameter_hash(&pnode.arguments);
            let now = Utc::now();
            let expires = now + Duration::seconds(settings.pending_action_ttl_seconds);
            let pa = PendingAction {
                id: ids::new_id("act"),
                user_id: user.id.clone(),
                tool: pnode.tool.clone(),
                parameters: serde_json::to_string(&pnode.arguments).unwrap_or("{}".to_string()),
                parameter_hash: param_hash.clone(),
                risk_tier: pnode.risk_tier.clone(),
                expires_at: expires,
                confirmation_status: "none".to_string(),
                mfa_status: "none".to_string(),
                created_at: now,
                workflow_node_id: None, // filled below
                idempotency_key: Some(idem.clone()),
            };
            sqlx::query(
                "INSERT INTO pending_actions (id, user_id, tool, parameters, parameter_hash, risk_tier, expires_at, confirmation_status, mfa_status, created_at, idempotency_key)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
            )
            .bind(&pa.id)
            .bind(&pa.user_id)
            .bind(&pa.tool)
            .bind(&pa.parameters)
            .bind(&pa.parameter_hash)
            .bind(&pa.risk_tier)
            .bind(pa.expires_at)
            .bind(&pa.confirmation_status)
            .bind(&pa.mfa_status)
            .bind(pa.created_at)
            .bind(&pa.idempotency_key)
            .execute(&mut *tx)
            .await
            .map_err(|e| ToolError::Other(e.to_string()))?;
            // Patch the workflow_node to point at the pending_action.
            sqlx::query("UPDATE workflow_nodes SET pending_action_id = $1 WHERE workflow_id = $2 AND node_index = $3")
                .bind(&pa.id)
                .bind(&workflow.id)
                .bind(idx as i32)
                .execute(&mut *tx)
                .await
                .map_err(|e| ToolError::Other(e.to_string()))?;
            // And patch the pending_action back to point at the workflow_node
            // so /actions/{id}/execute can find its workflow. Without this
            // every Red-tier transfer fails with "not attached to a workflow
            // node" and the debit never lands.
            sqlx::query("UPDATE pending_actions SET workflow_node_id = (SELECT id FROM workflow_nodes WHERE workflow_id = $1 AND node_index = $2) WHERE id = $3")
                .bind(&workflow.id)
                .bind(idx as i32)
                .bind(&pa.id)
                .execute(&mut *tx)
                .await
                .map_err(|e| ToolError::Other(e.to_string()))?;
        }
    }

    if !any_failed {
        let has_pending: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM workflow_nodes WHERE workflow_id = $1 AND approval_required = TRUE",
        )
        .bind(&workflow.id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
        let final_status = if has_pending > 0 {
            "awaiting_confirmation"
        } else {
            "completed"
        };
        sqlx::query("UPDATE workflows SET status = $1 WHERE id = $2")
            .bind(final_status)
            .bind(&workflow.id)
            .execute(&mut *tx)
            .await
            .map_err(|e| ToolError::Other(e.to_string()))?;
        workflow.status = final_status.to_string();
    }

    tx.commit()
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    Ok(workflow)
}

pub async fn workflow_status(pool: &PgPool, workflow_id: &str) -> Result<Option<Value>, sqlx::Error> {
    let wf: Option<Workflow> = sqlx::query_as("SELECT * FROM workflows WHERE id = $1")
        .bind(workflow_id)
        .fetch_optional(pool)
        .await?;
    let Some(wf) = wf else { return Ok(None) };
    let nodes: Vec<WorkflowNode> = sqlx::query_as(
        "SELECT * FROM workflow_nodes WHERE workflow_id = $1 ORDER BY node_index",
    )
    .bind(workflow_id)
    .fetch_all(pool)
    .await?;
    let out_nodes: Vec<Value> = nodes
        .into_iter()
        .map(|n| {
            json!({
                "id": n.id,
                "index": n.node_index,
                "tool": n.tool,
                "parameters": serde_json::from_str::<Value>(&n.parameters).unwrap_or(Value::Null),
                "risk_tier": n.risk_tier,
                "approval_required": n.approval_required,
                "status": n.status,
                "result": serde_json::from_str::<Value>(&n.result.unwrap_or_else(|| "null".to_string())).ok(),
                "error": serde_json::from_str::<Value>(&n.error.unwrap_or_else(|| "null".to_string())).ok(),
                "pending_action_id": n.pending_action_id,
            })
        })
        .collect();
    Ok(Some(json!({
        "id": wf.id,
        "user_id": wf.user_id,
        "status": wf.status,
        "interrupt_reason": wf.interrupt_reason,
        "created_at": wf.created_at.to_rfc3339(),
        "nodes": out_nodes,
    })))
}
