//! Agent + workflow routes.

use actix_web::{web, HttpResponse};
use chrono::Datelike;
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;

use crate::audit::{self, AppendEventParams};
use crate::config;
use crate::conversations;
use crate::ctx::{require_csrf, AuthUser, RequestId};
use crate::errors::{self, ApiError};
use crate::llm;
use crate::llm_audit::{self, AuditContext};
use crate::memory;
use crate::models::{Account, Card, Payee, PendingAction, Reminder, SavingsGoal, Subscription, Transaction, User, WealthHolding, WealthProduct, Workflow, WorkflowNode};
use crate::planner;
use crate::policy_explainer;
use crate::retriever;
use crate::safety;
use crate::tools::{self, ToolError};
use serde_json::Value;

use crate::state::AppState;
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/agent/messages", web::post().to(agent_message))
        .route("/agent/conversations", web::get().to(list_conversations))
        .route("/agent/conversations/{id}", web::get().to(get_conversation))
        .route("/agent/memory/recall", web::get().to(memory_recall_preview))
        .route("/agent/advisor", web::post().to(advisor_query))
        .route("/workflows/{id}", web::get().to(get_workflow))
        .route("/workflows/{id}/pause", web::post().to(pause_workflow))
        .route("/workflows/{id}/resume", web::post().to(resume_workflow))
        .route("/workflows/{id}/cancel", web::post().to(cancel_workflow));
}

#[derive(Deserialize)]
pub struct AgentMessageIn {
    pub text: String,
    pub language: Option<String>,
    /// Optional conversation id. When omitted the server picks the most
    /// recent conversation for the user (creating one if none exists).
    pub conversation_id: Option<String>,
    /// When true, the route retrieves relevant policy docs and prepends
    /// them to the LLM context (advisor mode).
    #[serde(default)]
    pub advisor_mode: bool,
}

async fn user_state_summary(pool: &PgPool, user: &User) -> Result<serde_json::Value, sqlx::Error> {
    // Read every domain table for the user once. The LLM gets a complete
    // "wallet view" so it can answer read-only questions and propose
    // accurate actions without needing a read tool call.
    let accounts: Vec<Account> = sqlx::query_as(
        "SELECT * FROM accounts WHERE user_id = $1 ORDER BY created_at ASC",
    )
    .bind(&user.id)
    .fetch_all(pool)
    .await?;
    let payees: Vec<Payee> = sqlx::query_as("SELECT * FROM payees WHERE owner_user_id = $1")
        .bind(&user.id)
        .fetch_all(pool)
        .await?;
    let subscriptions: Vec<Subscription> = sqlx::query_as(
        "SELECT id, user_id, merchant, estimated_amount_fen, interval_days, next_expected_at, last_charge_at, evidence::text AS evidence, cancellation_status, cancelled_at, created_at FROM subscriptions WHERE user_id = $1 ORDER BY merchant ASC",
    )
    .bind(&user.id)
    .fetch_all(pool)
    .await?;
    let cards: Vec<Card> = sqlx::query_as("SELECT * FROM cards WHERE user_id = $1")
        .bind(&user.id)
        .fetch_all(pool)
        .await?;
    let wealth_products: Vec<WealthProduct> = sqlx::query_as("SELECT * FROM wealth_products")
        .fetch_all(pool)
        .await?;
    let wealth_holdings: Vec<WealthHolding> =
        sqlx::query_as("SELECT * FROM wealth_holdings WHERE user_id = $1")
            .bind(&user.id)
            .fetch_all(pool)
            .await?;
    let goals: Vec<SavingsGoal> =
        sqlx::query_as("SELECT * FROM savings_goals WHERE user_id = $1")
            .bind(&user.id)
            .fetch_all(pool)
            .await?;
    let reminders: Vec<Reminder> = sqlx::query_as(
        "SELECT * FROM reminders WHERE user_id = $1 AND status = 'scheduled' ORDER BY due_at ASC LIMIT 20",
    )
    .bind(&user.id)
    .fetch_all(pool)
    .await?;
    // Recent activity: last 30 days, capped at 100 rows.
    let recent_tx: Vec<Transaction> = sqlx::query_as(
        "SELECT t.* FROM transactions t JOIN accounts a ON a.id = t.account_id \
         WHERE a.user_id = $1 AND t.occurred_at >= NOW() - INTERVAL '30 days' \
         ORDER BY t.occurred_at DESC LIMIT 100",
    )
    .bind(&user.id)
    .fetch_all(pool)
    .await?;
    // Monthly aggregates: 30d income / spend / net.
    let mut income_30d: i64 = 0;
    let mut spend_30d: i64 = 0;
    let mut by_category: std::collections::BTreeMap<String, i64> = std::collections::BTreeMap::new();
    for t in &recent_tx {
        if t.amount_fen >= 0 {
            income_30d += t.amount_fen;
        } else {
            let amt = -t.amount_fen;
            spend_30d += amt;
            *by_category.entry(t.category.clone()).or_insert(0) += amt;
        }
    }
    let mut categories: Vec<(String, i64)> = by_category.into_iter().collect();
    categories.sort_by(|a, b| b.1.cmp(&a.1));
    let categories: Vec<_> = categories
        .into_iter()
        .take(8)
        .map(|(cat, amt)| json!({ "category": cat, "amount_fen": amt }))
        .collect();

    let primary_account = accounts.first();
    let primary_account_id = primary_account.map(|a| a.id.clone());
    let primary_account_balance_fen = primary_account.map(|a| a.available_balance_fen).unwrap_or(0);

    Ok(json!({
        "user": {
            "id": user.id,
            "display_name": user.display_name,
            "preferred_language": user.preferred_language,
            "role": user.role,
            "risk_tolerance": user.risk_tolerance,
            "horizon": user.horizon,
            "safety_lock_state": user.safety_lock_state,
        },
        "accounts": accounts.iter().map(|a| json!({
            "id": a.id,
            "display_name": a.display_name,
            "masked_number": a.masked_number,
            "currency": a.currency,
            "available_balance_fen": a.available_balance_fen,
            "reserved_balance_fen": a.reserved_balance_fen,
        })).collect::<Vec<_>>(),
        "primary_account_id": primary_account_id,
        "primary_account_balance_fen": primary_account_balance_fen,
        "payees": payees.iter().map(|p| json!({
            "id": p.id,
            "display_name": p.display_name,
            "phone_tail": p.phone_tail,
            "verified": p.verified,
        })).collect::<Vec<_>>(),
        "subscriptions": subscriptions.iter().map(|s| json!({
            "id": s.id,
            "merchant": s.merchant,
            "estimated_amount_fen": s.estimated_amount_fen,
            "interval_days": s.interval_days,
            "next_expected_at": s.next_expected_at,
            "last_charge_at": s.last_charge_at,
            "cancellation_status": s.cancellation_status,
        })).collect::<Vec<_>>(),
        "subscription_summary": {
            "active_count": subscriptions.iter().filter(|s| s.cancellation_status == "active").count(),
            "monthly_total_fen": subscriptions.iter()
                .filter(|s| s.cancellation_status == "active")
                .map(|s| (s.estimated_amount_fen as f64 * 30.0 / s.interval_days.max(1) as f64) as i64)
                .sum::<i64>(),
        },
        "cards": cards.iter().map(|c| json!({
            "id": c.id,
            "display_name": c.display_name,
            "masked_number": c.masked_number,
            "status": c.status,
            "single_tx_limit_fen": c.single_tx_limit_fen,
            "monthly_limit_fen": c.monthly_limit_fen,
        })).collect::<Vec<_>>(),
        "wealth": {
            "products": wealth_products.iter().map(|p| json!({
                "id": p.id,
                "name": p.name,
                "category": p.category,
                "risk_level": p.risk_level,
                "expected_annual_return_pct": p.expected_annual_return_pct,
                "min_subscription_fen": p.min_subscription_fen,
            })).collect::<Vec<_>>(),
            "holdings": wealth_holdings.iter().map(|h| json!({
                "id": h.id,
                "product_id": h.product_id,
                "amount_fen": h.amount_fen,
                "status": h.status,
            })).collect::<Vec<_>>(),
        },
        "savings_goals": goals.iter().map(|g| json!({
            "id": g.id,
            "name": g.name,
            "target_fen": g.target_fen,
        })).collect::<Vec<_>>(),
        "reminders": reminders.iter().map(|r| json!({
            "id": r.id,
            "kind": r.kind,
            "subject": r.subject,
            "due_at": r.due_at,
        })).collect::<Vec<_>>(),
        "activity_30d": {
            "income_fen": income_30d,
            "spend_fen": spend_30d,
            "net_fen": income_30d - spend_30d,
            "top_categories": categories,
        },
        "recent_transactions": recent_tx.iter().take(20).map(|t| json!({
            "id": t.id,
            "amount_fen": t.amount_fen,
            "direction": t.direction,
            "category": t.category,
            "merchant": t.merchant,
            "memo": t.memo,
            "occurred_at": t.occurred_at,
        })).collect::<Vec<_>>(),
    }))
}

async fn agent_message(
    state: web::Data<AppState>,
    auth: AuthUser,
    rid: RequestId,
    req: actix_web::HttpRequest,
    body: web::Json<AgentMessageIn>,
) -> Result<HttpResponse, ApiError> {
    require_csrf(&req, &auth).await?;
    let user = auth.0;
    safety::assert_unlocked(&user)?;
    let body = body.into_inner();
    let summary = user_state_summary(&state.pool, &user)
        .await
        .map_err(|_| errors::internal_error())?;

    // Open (or pick) a conversation and persist the user turn. This is
    // done up front so a later failure still leaves a row in
    // conversation_messages for the admin console to inspect.
    let conv = conversations::get_or_create(
        &state.pool,
        &user,
        body.conversation_id.as_deref(),
    )
    .await
    .map_err(|_| errors::internal_error())?;
    conversations::append_message(
        &state.pool,
        &conv,
        &user,
        "user",
        &body.text,
        None,
        None,
        None,
    )
    .await
    .ok();

    // Long-term memory: also store the user turn in `chat_memory`
    // with an intent-family one-hot. We don't know the LLM intent yet
    // (we haven't called it), so the intent is None — pure lexical
    // recall on this row. The assistant turn records the resolved
    // intent after the LLM responds.
    memory::record_turn(
        &state.pool,
        memory::RecordParams {
            user_id: &user.id,
            conversation_id: &conv.id,
            message_id: None,
            role: "user",
            text: &body.text,
            summary: None,
            intent: None,
        },
    )
    .await
    .ok();

    // Optionally pre-pend advisor-mode retrieval to the user state.
    let mut effective_summary = summary.clone();
    if body.advisor_mode {
        if let Ok(cites) = retriever::citations(&state.pool, &body.text, 4).await {
            let pack: Vec<Value> = cites
                .into_iter()
                .map(|c| {
                    json!({
                        "source": c.source,
                        "section": c.section,
                        "score": c.score,
                        "snippet": c.snippet,
                    })
                })
                .collect();
            effective_summary = json!({
                "wallet": summary,
                "retrieved": pack,
            });
        }
    }

    // Long-term memory: recall the user's prior chats that look similar
    // to the current message, so the LLM can resolve references like
    // "what did I cancel last time?". We always include the recall
    // block; if there's nothing to recall we pass an empty array and
    // the LLM just ignores it.
    let memory_hits = memory::recall_similar(
        &state.pool,
        &user.id,
        &body.text,
        Some(&conv.id),
        5,
    )
    .await
    .unwrap_or_default();
    log::info!(
        "[memory] recall query={:?} → {} hits (top score={:.3})",
        body.text,
        memory_hits.len(),
        memory_hits.first().map(|h| h.score).unwrap_or(0.0)
    );
    let memory_pack: Vec<Value> = memory_hits
        .iter()
        .map(|h| {
            json!({
                "score": h.score,
                "role": h.row.role,
                "intent": h.row.intent,
                "summary": h.row.summary.clone().unwrap_or_else(|| h.row.text.chars().take(120).collect::<String>()),
                "when": h.row.created_at.to_rfc3339(),
            })
        })
        .collect();
    if let Value::Object(ref mut m) = effective_summary {
        m.insert("memory_recall".into(), Value::Array(memory_pack));
    } else {
        effective_summary = json!({ "memory_recall": memory_pack });
    }

    let settings = config::get_settings();
    // Admin-configured rules. We fetch them once per turn and use them
    // both to:
    //   1. Enrich the LLM system prompt (admins get a soft steering lever).
    //   2. Filter `forbid_tool` rules out of the LLM's `proposed_actions`
    //      before they reach the planner — so the model never even gets
    //      to "propose" a forbidden action.
    // The planner additionally re-evaluates transfer-scoped rules
    // (restrict_payee / restrict_memo / restrict_amount_fen /
    // restrict_daily_fen) inside `planner::build_plan`.
    let admin_prefix = crate::agent_rules::system_prompt_prefix(&state.pool, &user.id).await;
    let forbidden = crate::agent_rules::forbidden_tools(&state.pool, &user.id).await;
    let forbidden_set: std::collections::HashSet<String> = forbidden
        .iter()
        .map(|r| r.pattern.clone())
        .collect();
    let mut proposal = llm_audit::audited_propose(
        &state.pool,
        &settings,
        AuditContext {
            user_id: Some(&user.id),
            request_id: Some(&rid.0),
            conversation_id: Some(&conv.id),
        },
        &body.text,
        &effective_summary,
        // Prefer the user's saved language preference (set by the
        // LanguageButton in the app shell) over the per-request hint so the
        // assistant always answers in the same language the user is reading
        // the UI in. The body hint is still useful as a per-turn override
        // when the user explicitly switches mid-conversation.
        Some(&user.preferred_language),
        &admin_prefix,
    )
    .await;

    // Auto-translate fallback. When the LLM was the source
    // (`settings.unsloth_enabled()`) AND the proposal's `language` does
    // not already match the user's preferred language, ask the same
    // Unsloth endpoint to re-translate the user-visible `questions[]`
    // into the user's chosen language. This is best-effort: the call
    // is wrapped in a 4s timeout, and any failure leaves the proposal
    // untouched. We deliberately do NOT mutate `proposal.language`
    // itself — the detection tag is honest about what the model saw.
    //
    // We only re-translate when unsloth is enabled because the regex
    // stub already produces questions in the detected language, so a
    // stub proposal matching a different preference is intentional, not
    // a drift bug.
    if settings.unsloth_enabled() && !proposal.questions.is_empty() {
        let user_lang = normalise_user_lang(&user.preferred_language);
        let prop_lang = normalise_proposal_lang(&proposal.language);
        if !user_lang.is_empty() && user_lang != prop_lang {
            let questions = proposal.questions.clone();
            let res = tokio::time::timeout(
                std::time::Duration::from_secs(4),
                crate::translate::translate_strings(&settings, &questions, &user_lang),
            )
            .await;
            match res {
                Ok(translated) if translated.len() == questions.len() && translated != questions => {
                    log::info!(
                        "agent: auto-translated {} question(s) from proposal.language={} → user.preferred_language={}",
                        translated.len(),
                        proposal.language,
                        user.preferred_language
                    );
                    proposal.questions = translated;
                    // Single-line audit note that the LLM tag is being
                    // deliberately preserved (it reflects what the
                    // model saw in the user's input).
                    audit::append_event(
                        &state.pool,
                        AppendEventParams {
                            action: "agent.translate_fallback",
                            actor_user_id: Some(&user.id),
                            session: None,
                            request_id: Some(&rid.0),
                            workflow_id: None,
                            node_id: None,
                            tool: None,
                            parameters_masked: Some(json!({
                                "from": proposal.language,
                                "to": user.preferred_language,
                                "count": proposal.questions.len(),
                                "conversation_id": conv.id,
                            })),
                            risk_tier: None,
                            policy_reason: Some("language_preference"),
                            confirmation_status: None,
                            mfa_status: None,
                            result: "ok",
                            error_code: None,
                        },
                    )
                    .await
                    .ok();
                }
                _ => {
                    // Timeout, error, or the model returned the
                    // originals. Either way: no-op. The chat continues
                    // with the original questions.
                }
            }
        }
    }

    // Slot binding: for each proposed action, lift the present-but-incomplete
    // arguments into the conversation's slot_bindings table. Subsequent
    // messages in the same conversation (e.g. "to Li Wei") can backfill the
    // missing slots.
    for action in &proposal.proposed_actions {
        let action_obj = match action.as_object() {
            Some(o) => o,
            None => continue,
        };
        let tool_name = match action_obj.get("tool").and_then(|v| v.as_str()) {
            Some(t) => t,
            None => continue,
        };
        let canonical = crate::tools::canonicalize_tool_name(tool_name);
        let Some(spec) = crate::tool_spec::spec_for(canonical) else { continue };
        let args = action_obj.get("arguments").cloned().unwrap_or(serde_json::Value::Object(Default::default()));
        if let Some(args_obj) = args.as_object() {
            for (k, v) in args_obj {
                if v.is_null() {
                    continue;
                }
                let sval = match v {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                conversations::bind_slot(
                    &state.pool,
                    &conv,
                    canonical,
                    k,
                    &sval,
                    "inferred",
                    0.9,
                )
                .await
                .ok();
            }
            if let Err(missing) = crate::tool_spec::validate_arguments(spec, &args) {
                for m in missing {
                    conversations::record_question(
                        &state.pool,
                        &conv,
                        canonical,
                        m.slot.name,
                        m.slot.question,
                    )
                    .await
                    .ok();
                }
            }
        }
    }

    // Persist the assistant turn so the multi-turn history is complete.
    let proposal_json = serde_json::to_value(&proposal).ok();
    let assistant_msg = conversations::append_message(
        &state.pool,
        &conv,
        &user,
        "assistant",
        "",
        proposal_json.as_ref(),
        None,
        None,
    )
    .await
    .ok();

    // Mirror the assistant turn into `chat_memory` with the resolved
    // intent so future recalls can use it as a strong signal.
    if let Some(msg) = assistant_msg.as_ref() {
        memory::record_turn(
            &state.pool,
            memory::RecordParams {
                user_id: &user.id,
                conversation_id: &conv.id,
                message_id: Some(&msg.id),
                role: "assistant",
                text: &proposal.questions.join(" / "),
                summary: Some(&proposal.intent),
                intent: Some(&proposal.intent),
            },
        )
        .await
        .ok();
    }

    // If the LLM only asked for read-only data, run those tools inline
    // and return the result. No workflow, no pending action — the LLM
    // is just answering a question.
    let reg = tools::registry();
    let read_only_tools: std::collections::HashSet<&str> = [
        "accounts.list",
        "payees.list",
        "subscriptions.list",
        "cards.list",
        "accounts.balance_summary",
        "spending.report",
        "transactions.recent",
        "wealth.summary",
        "user.profile",
    ]
    .into_iter()
    .collect();
    let canonical_actions: Vec<(String, Value)> = proposal
        .proposed_actions
        .iter()
        .filter_map(|raw| {
            let tool = raw.get("tool").and_then(|v| v.as_str())?;
            let args = raw.get("arguments").cloned().unwrap_or(Value::Object(Default::default()));
            let canon = tools::canonicalize_tool_name(tool);
            if read_only_tools.contains(canon) {
                Some((canon.to_string(), args))
            } else {
                None
            }
        })
        .collect();
    let only_reads = !proposal.proposed_actions.is_empty()
        && canonical_actions.len() == proposal.proposed_actions.len();
    if only_reads {
        let ctx = tools::ToolContext {
            pool: state.pool.clone(),
            user: user.clone(),
            request_id: rid.0.clone(),
            session_id: auth.1.as_ref().map(|s| s.id.clone()),
        };
        let mut data: Vec<Value> = Vec::new();
        for (tool, args) in canonical_actions {
            if let Some(f) = reg.get(tool.as_str()) {
                match f(&ctx, &args).await {
                    Ok(v) => data.push(json!({ "tool": tool, "result": v })),
                    Err(e) => data.push(json!({ "tool": tool, "error": e.to_string() })),
                }
            }
        }
        audit::append_event(
            &state.pool,
            AppendEventParams {
                action: "agent.read",
                actor_user_id: Some(&user.id),
                session: None,
                request_id: Some(&rid.0),
                workflow_id: None,
                node_id: None,
                tool: None,
                parameters_masked: Some(json!({
                    "text": body.text.chars().take(64).collect::<String>(),
                    "intent": proposal.intent,
                    "language": proposal.language,
                    "tools": data.iter().map(|d| d.get("tool").and_then(|v| v.as_str()).unwrap_or("")).collect::<Vec<_>>(),
                })),
                risk_tier: None,
                policy_reason: None,
                confirmation_status: None,
                mfa_status: None,
                result: "ok",
                error_code: None,
            },
        )
        .await
        .ok();
        return Ok(HttpResponse::Ok().json(json!({
            "intent": proposal.intent,
            "language": proposal.language,
            "entities": proposal.entities,
            "clarification_required": false,
            "questions": [],
            "plan": { "workflow_id": null, "nodes": [] },
            "pending_action": null,
            "read_results": data,
            "proposal_text": body.text,
            "trace_id": rid.0,
            "conversation_id": conv.id,
            "statement_url": report_spending_statement_url(&body.text),
            "statement_filename": report_spending_filename(&body.text),
        })));
    }

    if proposal.clarification_required || proposal.proposed_actions.is_empty() {
        audit::append_event(
            &state.pool,
            AppendEventParams {
                action: "agent.message",
                actor_user_id: Some(&user.id),
                session: None,
                request_id: Some(&rid.0),
                workflow_id: None,
                node_id: None,
                tool: None,
                parameters_masked: Some(json!({
                    "text": body.text.chars().take(64).collect::<String>(),
                    "intent": proposal.intent,
                    "language": proposal.language,
                })),
                risk_tier: None,
                policy_reason: Some("clarification_required"),
                confirmation_status: None,
                mfa_status: None,
                result: "ok",
                error_code: None,
            },
        )
        .await
        .ok();
        return Ok(HttpResponse::Ok().json(json!({
            "intent": proposal.intent,
            "language": proposal.language,
            "entities": proposal.entities,
            "clarification_required": true,
            "questions": proposal.questions,
            "plan": { "workflow_id": null, "nodes": [] },
            "pending_action": null,
            "proposal_text": body.text,
            "trace_id": rid.0,
            "conversation_id": conv.id,
            "statement_url": report_spending_statement_url(&body.text),
            "statement_filename": report_spending_filename(&body.text),
        })));
    }

    // Strip any `forbid_tool` rules out of the proposal before the planner
    // sees them. Each stripped action is recorded as an audit event so the
    // admin console can show "rule X blocked the LLM from proposing Y".
    let mut filtered_actions: Vec<Value> = Vec::with_capacity(proposal.proposed_actions.len());
    let mut rule_blocked_count: usize = 0;
    for raw in &proposal.proposed_actions {
        let tool_name = raw.get("tool").and_then(|v| v.as_str()).unwrap_or("");
        let canon = tools::canonicalize_tool_name(tool_name);
        if forbidden_set.contains(canon) {
            rule_blocked_count += 1;
            let rule = forbidden.iter().find(|r| r.pattern == canon);
            if let Some(rule) = rule {
                audit::append_event(
                    &state.pool,
                    AppendEventParams {
                        action: "agent_rule.match",
                        actor_user_id: Some(&user.id),
                        session: None,
                        request_id: Some(&rid.0),
                        workflow_id: None,
                        node_id: None,
                        tool: Some(canon),
                        parameters_masked: Some(json!({
                            "rule_id": rule.id,
                            "rule_name": rule.name,
                            "kind": "forbid_tool",
                            "severity": rule.severity,
                            "source": "planner_filter",
                        })),
                        risk_tier: Some("Red"),
                        policy_reason: Some(&rule.name),
                        confirmation_status: None,
                        mfa_status: None,
                        result: "blocked",
                        error_code: None,
                    },
                )
                .await
                .ok();
            }
            continue;
        }
        filtered_actions.push(raw.clone());
    }
    if rule_blocked_count > 0 {
        log::warn!(
            "agent: dropped {} proposed actions due to admin forbid_tool rules",
            rule_blocked_count
        );
    }

    let workflow = match planner::build_plan(
        &state.pool,
        &user,
        &filtered_actions,
        &rid.0,
    )
    .await
    {
        Ok(w) => w,
        Err(ToolError::Clarification(m)) => {
            return Ok(HttpResponse::Ok().json(json!({
                "intent": proposal.intent,
                "language": proposal.language,
                "entities": proposal.entities,
                "clarification_required": true,
                "questions": [m],
                "plan": { "workflow_id": null, "nodes": [] },
                "pending_action": null,
                "proposal_text": body.text,
                "trace_id": rid.0,
            })));
        }
        // Planner couldn't uniquely resolve a target (subscription / payee /
        // account / goal). Surface it as an in-chat clarification card rather
        // than a 4xx — the LLM sometimes passes a near-match merchant name
        // ("NetStream 订阅") that we want to ask the user to disambiguate.
        Err(ToolError::NotFound(m)) => {
            return Ok(HttpResponse::Ok().json(json!({
                "intent": proposal.intent,
                "language": proposal.language,
                "entities": proposal.entities,
                "clarification_required": true,
                "questions": [format!("I couldn't find that. {m}. Could you rephrase?")],
                "plan": { "workflow_id": null, "nodes": [] },
                "pending_action": null,
                "proposal_text": body.text,
                "trace_id": rid.0,
                "conversation_id": conv.id,
                "statement_url": report_spending_statement_url(&body.text),
                "statement_filename": report_spending_filename(&body.text),
            })));
        }
        Err(ToolError::InsufficientFunds(m)) => {
            return Err(errors::ApiError::new("insufficient_funds", m, 422));
        }
        // Admin rule refusal — surface as a 403 with a clear code so the
        // chat UI can show "this was blocked by an admin policy" instead
        // of a generic schema error.
        Err(ToolError::Forbidden(m)) => {
            return Err(errors::ApiError::new("forbidden", &m, 403));
        }
        Err(e) => return Err(errors::schema_invalid(&e.to_string())),
    };
    let nodes: Vec<WorkflowNode> = sqlx::query_as(
        "SELECT * FROM workflow_nodes WHERE workflow_id = $1 ORDER BY node_index",
    )
    .bind(&workflow.id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| errors::internal_error())?;
    let mut pending_action: Option<PendingAction> = None;
    for n in &nodes {
        if matches!(n.risk_tier.as_str(), "Yellow" | "Red") {
            if let Some(pid) = &n.pending_action_id {
                let pa: Option<PendingAction> =
                    sqlx::query_as("SELECT * FROM pending_actions WHERE id = $1")
                        .bind(pid)
                        .fetch_optional(&state.pool)
                        .await
                        .map_err(|_| errors::internal_error())?;
                pending_action = pa;
                break;
            }
        }
    }
    audit::append_event(
        &state.pool,
        AppendEventParams {
            action: "agent.message",
            actor_user_id: Some(&user.id),
            session: None,
            request_id: Some(&rid.0),
            workflow_id: Some(&workflow.id),
            node_id: None,
            tool: None,
            parameters_masked: Some(json!({
                "text": body.text.chars().take(64).collect::<String>(),
                "intent": proposal.intent,
                "language": proposal.language,
            })),
            risk_tier: None,
            policy_reason: None,
            confirmation_status: None,
            mfa_status: None,
            result: "ok",
            error_code: None,
        },
    )
    .await
    .ok();
    let nodes_json: Vec<_> = nodes
        .iter()
        .map(|n| {
            let params: serde_json::Value =
                serde_json::from_str::<serde_json::Value>(&n.parameters).unwrap_or(serde_json::json!({}));
            // Pull the fraud_assessment out of the resolved parameters
            // so the chat UI can render the warning before the user
            // confirms the transfer. The backend never trusts this
            // field for policy decisions — that's still done by
            // `policy::decide_transfer` above.
            let fraud_assessment = params.get("fraud_assessment").cloned();
            json!({
                "id": n.id,
                "index": n.node_index,
                "tool": n.tool,
                "parameters": params,
                "risk_tier": n.risk_tier,
                "approval_required": n.approval_required,
                "status": n.status,
                "fraud_assessment": fraud_assessment,
                "pending_action_id": n.pending_action_id,
            })
        })
        .collect();
    // Build a human-readable policy explanation per node so the chat UI
    // can render *why* this plan is Red/Yellow instead of just the tier.
    let explanations: Vec<Value> = nodes_json
        .iter()
        .filter_map(|n| {
            let tier = n.get("risk_tier").and_then(|v| v.as_str())?;
            if tier == "Green" {
                return None;
            }
            let reason = match n.get("tool").and_then(|v| v.as_str())? {
                "transfer.create_pending" => {
                    let amt = n.get("parameters").and_then(|p| p.get("amount_fen")).and_then(|v| v.as_i64()).unwrap_or(0);
                    Some(policy_explainer::explain_transfer(
                        &crate::policy::Decision {
                            tier: tier.into(),
                            reason: "policy".into(),
                            escalate_from: None,
                            flags: vec![],
                        },
                        amt,
                        None,
                    ))
                }
                "cards.action" => {
                    let kind = n.get("parameters").and_then(|p| p.get("kind")).and_then(|v| v.as_str()).unwrap_or("freeze");
                    Some(policy_explainer::explain_card(kind, &crate::policy::Decision {
                        tier: tier.into(),
                        reason: "card_action".into(),
                        escalate_from: None,
                        flags: vec![],
                    }))
                }
                _ => None,
            };
            reason.map(|e| json!({
                "node_id": n.get("id"),
                "headline": e.headline,
                "body": e.body,
                "tier": e.tier,
            }))
        })
        .collect();
    Ok(HttpResponse::Ok().json(json!({
        "intent": proposal.intent,
        "language": proposal.language,
        "entities": proposal.entities,
        "clarification_required": false,
        "questions": [],
        "plan": {
            "workflow_id": workflow.id,
            "status": workflow.status,
            "nodes": nodes_json,
        },
        "pending_action": pending_action.as_ref().map(|p| json!({
            "id": p.id,
            "tool": p.tool,
            "parameter_hash": p.parameter_hash,
            "risk_tier": p.risk_tier,
            "expires_at": p.expires_at,
        })),
        "proposal_text": body.text,
        "trace_id": rid.0,
        "conversation_id": conv.id,
        "policy_explanations": explanations,
        "statement_url": report_spending_statement_url(&body.text),
        "statement_filename": report_spending_filename(&body.text),
    })))
}

async fn list_conversations(
    state: web::Data<AppState>,
    auth: AuthUser,
) -> Result<HttpResponse, ApiError> {
    let user = auth.0;
    let rows: Vec<conversations::Conversation> = sqlx::query_as(
        "SELECT * FROM conversations WHERE user_id = $1 ORDER BY last_message_at DESC LIMIT 50",
    )
    .bind(&user.id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| errors::internal_error())?;
    Ok(HttpResponse::Ok().json(json!({ "conversations": rows })))
}

async fn get_conversation(
    state: web::Data<AppState>,
    auth: AuthUser,
    path: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let user = auth.0;
    let id = path.into_inner();
    let conv: Option<conversations::Conversation> = sqlx::query_as(
        "SELECT * FROM conversations WHERE id = $1 AND user_id = $2",
    )
    .bind(&id)
    .bind(&user.id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| errors::internal_error())?;
    let conv = conv.ok_or_else(|| errors::not_found("Conversation not found."))?;
    let messages = conversations::recent_messages(&state.pool, &conv, 100)
        .await
        .map_err(|_| errors::internal_error())?;
    let bindings = conversations::load_bindings(&state.pool, &conv)
        .await
        .map_err(|_| errors::internal_error())?;
    Ok(HttpResponse::Ok().json(json!({
        "conversation": conv,
        "messages": messages,
        "bindings": bindings,
    })))
}

#[derive(Deserialize)]
pub struct AdvisorQuery {
    pub text: String,
    pub limit: Option<i64>,
}

async fn advisor_query(
    state: web::Data<AppState>,
    auth: AuthUser,
    rid: RequestId,
    req: actix_web::HttpRequest,
    body: web::Json<AdvisorQuery>,
) -> Result<HttpResponse, ApiError> {
    require_csrf(&req, &auth).await?;
    let user = auth.0;
    let body = body.into_inner();
    let limit = body.limit.unwrap_or(4).clamp(1, 10);
    let cites = retriever::citations(&state.pool, &body.text, limit)
        .await
        .map_err(|_| errors::internal_error())?;
    let pack: Vec<Value> = cites
        .into_iter()
        .map(|c| {
            json!({
                "source": c.source,
                "section": c.section,
                "score": c.score,
                "snippet": c.snippet,
            })
        })
        .collect();
    audit::append_event(
        &state.pool,
        AppendEventParams {
            action: "agent.advisor_query",
            actor_user_id: Some(&user.id),
            session: None,
            request_id: Some(&rid.0),
            workflow_id: None,
            node_id: None,
            tool: None,
            parameters_masked: Some(json!({
                "text": body.text.chars().take(64).collect::<String>(),
                "limit": limit,
            })),
            risk_tier: None,
            policy_reason: None,
            confirmation_status: None,
            mfa_status: None,
            result: "ok",
            error_code: None,
        },
    )
    .await
    .ok();
    Ok(HttpResponse::Ok().json(json!({
        "citations": pack,
        "trace_id": rid.0,
    })))
}

async fn get_workflow(
    state: web::Data<AppState>,
    auth: AuthUser,
    path: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let user = auth.0;
    let id = path.into_inner();
    let wf: Option<Workflow> = sqlx::query_as("SELECT * FROM workflows WHERE id = $1")
        .bind(&id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| errors::internal_error())?;
    let wf = wf.ok_or_else(|| errors::not_found("Workflow not found."))?;
    if wf.user_id != user.id {
        return Err(errors::not_found("Workflow not found."));
    }
    let out = planner::workflow_status(&state.pool, &wf.id)
        .await
        .map_err(|_| errors::internal_error())?
        .ok_or_else(|| errors::not_found("Workflow not found."))?;
    Ok(HttpResponse::Ok().json(out))
}

async fn pause_workflow(
    state: web::Data<AppState>,
    auth: AuthUser,
    rid: RequestId,
    req: actix_web::HttpRequest,
    path: web::Path<String>,
    q: web::Query<std::collections::HashMap<String, String>>,
) -> Result<HttpResponse, ApiError> {
    require_csrf(&req, &auth).await?;
    let user = auth.0;
    let id = path.into_inner();
    let reason = q
        .get("reason")
        .cloned()
        .unwrap_or_else(|| "user_request".to_string());
    let wf: Option<Workflow> = sqlx::query_as("SELECT * FROM workflows WHERE id = $1")
        .bind(&id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| errors::internal_error())?;
    let wf = wf.ok_or_else(|| errors::not_found("Workflow not found."))?;
    if wf.user_id != user.id {
        return Err(errors::not_found("Workflow not found."));
    }
    if matches!(wf.status.as_str(), "completed" | "failed" | "cancelled") {
        let out = planner::workflow_status(&state.pool, &wf.id)
            .await
            .map_err(|_| errors::internal_error())?
            .unwrap_or(serde_json::json!({}));
        return Ok(HttpResponse::Ok().json(out));
    }
    sqlx::query("UPDATE workflows SET status = 'paused', interrupt_reason = $1 WHERE id = $2")
        .bind(&reason)
        .bind(&wf.id)
        .execute(&state.pool)
        .await
        .map_err(|_| errors::internal_error())?;
    let nodes: Vec<WorkflowNode> = sqlx::query_as(
        "SELECT * FROM workflow_nodes WHERE workflow_id = $1",
    )
    .bind(&wf.id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| errors::internal_error())?;
    for n in &nodes {
        if !matches!(
            n.status.as_str(),
            "completed" | "failed" | "compensated" | "skipped"
        ) {
            sqlx::query("UPDATE workflow_nodes SET status = 'paused' WHERE id = $1")
                .bind(&n.id)
                .execute(&state.pool)
                .await
                .map_err(|_| errors::internal_error())?;
        }
    }
    audit::append_event(
        &state.pool,
        AppendEventParams {
            action: "workflow.pause",
            actor_user_id: Some(&user.id),
            session: None,
            request_id: Some(&rid.0),
            workflow_id: Some(&wf.id),
            node_id: None,
            tool: None,
            parameters_masked: Some(json!({ "reason": reason })),
            risk_tier: None,
            policy_reason: None,
            confirmation_status: None,
            mfa_status: None,
            result: "ok",
            error_code: None,
        },
    )
    .await
    .ok();
    let out = planner::workflow_status(&state.pool, &wf.id)
        .await
        .map_err(|_| errors::internal_error())?
        .unwrap_or(serde_json::json!({}));
    Ok(HttpResponse::Ok().json(out))
}

// --- Language normalisation helpers (translate fallback) -------------------
//
// These are tiny and only used by the auto-translate step above, but
// pulling them out keeps the call site readable and gives us a single
// place to evolve the heuristics if we add more languages later.

/// Normalise a user's stored `preferred_language` to one of `"en"` / `"zh"`.
/// Anything we can't recognise is treated as English and surfaces as
/// `""` to the caller so the auto-translate step is skipped.
fn normalise_user_lang(s: &str) -> String {
    let t = s.trim().to_lowercase();
    match t.as_str() {
        "zh" | "zh-cn" | "zh-hans" | "chinese" | "中文" => "zh".to_string(),
        "en" | "english" | "" => "en".to_string(),
        _ => String::new(),
    }
}

/// Normalise the LLM-tagged `proposal.language` to the same canonical
/// two-letter form. Per the system prompt the LLM is asked to emit
/// `"en"`, `"zh"`, or `"mixed"`; we treat `"mixed"` and `""` as
/// English because the questions themselves will mostly be English in
/// that case, and a `mixed` proposal that's only partially translated
/// is worse than a consistent English one.
fn normalise_proposal_lang(s: &str) -> String {
    let t = s.trim().to_lowercase();
    match t.as_str() {
        "zh" | "zh-cn" | "zh-hans" => "zh".to_string(),
        _ => "en".to_string(),
    }
}

/// Build the `statement_url` field for a read-only response. Returns
/// `None` unless the user clearly asked for a monthly statement
/// ("show my spending this month", "download my statement", …).
/// When present the frontend renders a "📄 Download statement" link
/// pointing at `/api/v1/reports/statement?month=YYYY-MM&format=csv`.
///
/// The intent is "this is a monthly export" when the message contains
/// any of: "this month", "last month", "monthly", "statement", "invoice",
/// or a `YYYY-MM` token. Anything else gets `None` so the chat doesn't
/// spam download links.
fn report_spending_statement_url(text: &str) -> Option<String> {
    let target_month = statement_target_month(text)?;
    Some(format!("/api/v1/reports/statement?month={}&format=csv", target_month))
}

fn report_spending_filename(text: &str) -> Option<String> {
    let target_month = statement_target_month(text)?;
    Some(format!("bankpilot-statement-{}.csv", target_month))
}

fn statement_target_month(text: &str) -> Option<String> {
    let lower = text.to_lowercase();
    let trigger = ["this month", "last month", "monthly", "statement", "invoice", "账单", "对账单", "本月"];
    if !trigger.iter().any(|t| lower.contains(t)) {
        return None;
    }
    // Honour an explicit YYYY-MM in the message if present.
    for token in text.split(|c: char| !c.is_ascii_digit() && c != '-') {
        if token.len() == 7 && token.as_bytes()[4] == b'-' {
            if let (Ok(_y), Ok(m)) = (token[0..4].parse::<i32>(), token[5..7].parse::<u32>()) {
                if (1..=12).contains(&m) {
                    return Some(token.to_string());
                }
            }
        }
    }
    let now = chrono::Utc::now();
    if lower.contains("last month") {
        let (y, m) = if now.month() == 1 { (now.year() - 1, 12) } else { (now.year(), now.month() - 1) };
        return Some(format!("{:04}-{:02}", y, m));
    }
    Some(format!("{:04}-{:02}", now.year(), now.month()))
}

/// `GET /api/v1/agent/memory/recall?text=…` — same recall the agent
/// would do for a real chat turn, but exposed for the admin console /
/// debug. Useful to confirm the embedding + cosine path is ranking the
/// right prior chats.
async fn memory_recall_preview(
    state: web::Data<AppState>,
    auth: AuthUser,
    q: web::Query<std::collections::HashMap<String, String>>,
) -> Result<HttpResponse, ApiError> {
    let user = auth.0;
    let text = q.get("text").cloned().unwrap_or_default();
    let limit = q
        .get("limit")
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(5);
    let hits = memory::recall_similar(&state.pool, &user.id, &text, None, limit)
        .await
        .map_err(|_| errors::internal_error())?;
    let pack: Vec<Value> = hits
        .into_iter()
        .map(|h| {
            json!({
                "id": h.row.id,
                "score": h.score,
                "role": h.row.role,
                "intent": h.row.intent,
                "conversation_id": h.row.conversation_id,
                "text": h.row.text,
                "summary": h.row.summary,
                "created_at": h.row.created_at,
            })
        })
        .collect();
    Ok(HttpResponse::Ok().json(json!({
        "query": text,
        "limit": limit,
        "hits": pack,
    })))
}

async fn resume_workflow(
    state: web::Data<AppState>,
    auth: AuthUser,
    rid: RequestId,
    req: actix_web::HttpRequest,
    path: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    require_csrf(&req, &auth).await?;
    let user = auth.0;
    let id = path.into_inner();
    let wf: Option<Workflow> = sqlx::query_as("SELECT * FROM workflows WHERE id = $1")
        .bind(&id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| errors::internal_error())?;
    let wf = wf.ok_or_else(|| errors::not_found("Workflow not found."))?;
    if wf.user_id != user.id {
        return Err(errors::not_found("Workflow not found."));
    }
    if wf.status != "paused" {
        return Err(errors::ApiError::new(
            "schema_invalid",
            "Only paused workflows can be resumed.",
            409,
        ));
    }
    let nodes: Vec<WorkflowNode> = sqlx::query_as(
        "SELECT * FROM workflow_nodes WHERE workflow_id = $1",
    )
    .bind(&wf.id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| errors::internal_error())?;
    for n in &nodes {
        if n.status == "paused" {
            let new_status = if n.approval_required {
                "awaiting_confirmation"
            } else {
                "pending"
            };
            sqlx::query("UPDATE workflow_nodes SET status = $1 WHERE id = $2")
                .bind(new_status)
                .bind(&n.id)
                .execute(&state.pool)
                .await
                .map_err(|_| errors::internal_error())?;
        }
    }
    sqlx::query("UPDATE workflows SET status = 'awaiting_confirmation', interrupt_reason = NULL WHERE id = $1")
        .bind(&wf.id)
        .execute(&state.pool)
        .await
        .map_err(|_| errors::internal_error())?;
    audit::append_event(
        &state.pool,
        AppendEventParams {
            action: "workflow.resume",
            actor_user_id: Some(&user.id),
            session: None,
            request_id: Some(&rid.0),
            workflow_id: Some(&wf.id),
            node_id: None,
            tool: None,
            parameters_masked: None,
            risk_tier: None,
            policy_reason: None,
            confirmation_status: None,
            mfa_status: None,
            result: "ok",
            error_code: None,
        },
    )
    .await
    .ok();
    let out = planner::workflow_status(&state.pool, &wf.id)
        .await
        .map_err(|_| errors::internal_error())?
        .unwrap_or(serde_json::json!({}));
    Ok(HttpResponse::Ok().json(out))
}

async fn cancel_workflow(
    state: web::Data<AppState>,
    auth: AuthUser,
    rid: RequestId,
    req: actix_web::HttpRequest,
    path: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    require_csrf(&req, &auth).await?;
    let user = auth.0;
    let id = path.into_inner();
    let wf: Option<Workflow> = sqlx::query_as("SELECT * FROM workflows WHERE id = $1")
        .bind(&id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| errors::internal_error())?;
    let wf = wf.ok_or_else(|| errors::not_found("Workflow not found."))?;
    if wf.user_id != user.id {
        return Err(errors::not_found("Workflow not found."));
    }
    let nodes: Vec<WorkflowNode> = sqlx::query_as(
        "SELECT * FROM workflow_nodes WHERE workflow_id = $1 ORDER BY node_index DESC",
    )
    .bind(&wf.id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| errors::internal_error())?;
    for n in nodes {
        if !matches!(
            n.status.as_str(),
            "pending" | "awaiting_confirmation" | "awaiting_mfa" | "paused"
        ) {
            continue;
        }
        let params: serde_json::Value =
            serde_json::from_str(&n.parameters).unwrap_or(serde_json::json!({}));
        if n.tool == "goals.create_reservation" {
            if let Some(goal_id) = params.get("goal_id").and_then(|v| v.as_str()) {
                // Find any reservation for this goal.
                let res: Option<(String, i64, String)> = sqlx::query_as(
                    "SELECT id, amount_fen, source_account_id FROM reservations WHERE goal_id = $1 ORDER BY created_at DESC LIMIT 1",
                )
                .bind(goal_id)
                .fetch_optional(&state.pool)
                .await
                .map_err(|_| errors::internal_error())?;
                if let Some((rid_str, amount, src)) = res {
                    sqlx::query("UPDATE reservations SET state = 'released' WHERE id = $1")
                        .bind(&rid_str)
                        .execute(&state.pool)
                        .await
                        .ok();
                    let acc: Option<(i64, i64)> = sqlx::query_as(
                        "SELECT available_balance_fen, reserved_balance_fen FROM accounts WHERE id = $1",
                    )
                    .bind(&src)
                    .fetch_optional(&state.pool)
                    .await
                    .map_err(|_| errors::internal_error())?;
                    if let Some((avail, reserved)) = acc {
                        let new_avail = avail + amount;
                        let new_reserved = std::cmp::max(0, reserved - amount);
                        sqlx::query("UPDATE accounts SET available_balance_fen = $1, reserved_balance_fen = $2 WHERE id = $3")
                            .bind(new_avail)
                            .bind(new_reserved)
                            .bind(&src)
                            .execute(&state.pool)
                            .await
                            .ok();
                    }
                    audit::append_event(
                        &state.pool,
                        AppendEventParams {
                            action: "workflow.compensate",
                            actor_user_id: Some(&user.id),
                            session: None,
                            request_id: Some(&rid.0),
                            workflow_id: Some(&wf.id),
                            node_id: Some(&n.id),
                            tool: Some("goals.release_reservation"),
                            parameters_masked: Some(json!({ "reservation_id": rid_str })),
                            risk_tier: None,
                            policy_reason: None,
                            confirmation_status: None,
                            mfa_status: None,
                            result: "ok",
                            error_code: None,
                        },
                    )
                    .await
                    .ok();
                }
            }
            sqlx::query("UPDATE workflow_nodes SET status = 'compensated' WHERE id = $1")
                .bind(&n.id)
                .execute(&state.pool)
                .await
                .ok();
        } else {
            sqlx::query("UPDATE workflow_nodes SET status = 'skipped' WHERE id = $1")
                .bind(&n.id)
                .execute(&state.pool)
                .await
                .ok();
        }
    }
    sqlx::query("UPDATE workflows SET status = 'cancelled' WHERE id = $1")
        .bind(&wf.id)
        .execute(&state.pool)
        .await
        .map_err(|_| errors::internal_error())?;
    audit::append_event(
        &state.pool,
        AppendEventParams {
            action: "workflow.cancel",
            actor_user_id: Some(&user.id),
            session: None,
            request_id: Some(&rid.0),
            workflow_id: Some(&wf.id),
            node_id: None,
            tool: None,
            parameters_masked: None,
            risk_tier: None,
            policy_reason: None,
            confirmation_status: None,
            mfa_status: None,
            result: "ok",
            error_code: None,
        },
    )
    .await
    .ok();
    let out = planner::workflow_status(&state.pool, &wf.id)
        .await
        .map_err(|_| errors::internal_error())?
        .unwrap_or(serde_json::json!({}));
    Ok(HttpResponse::Ok().json(out))
}
