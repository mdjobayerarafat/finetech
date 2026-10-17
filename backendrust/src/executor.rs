//! Background workflow executor with result-dependent DAG semantics.
//!
//! Two responsibilities:
//!
//! 1. **Result-dependent DAG** — the planner stores per-node `requires`
//!    (a JSON array of `{node_id, required_status}`) and `on_failure`
//!    (`fail` | `skip` | `compensate`). The executor walks nodes in
//!    `node_index` order, before running a node reads the outcome of each
//!    declared dependency, and either proceeds, skips, or compensates.
//! 2. **Background execution** — long-running or post-confirmation nodes
//!    run via a simple `workflow_jobs` table that a poller picks up. The
//!    current `agent_message` route still drives synchronous execution for
//!    Green nodes (preserving demo responsiveness); the background loop
//!    handles retry / compensation / post-approval work.
//!
//! State transitions:
//!   pending → running → completed | failed | skipped | compensated
//!
//! Workflow transitions:
//!   awaiting_confirmation → running → completed | failed | compensation_required
//!
//! The executor is invoked by:
//!   - The `/agent/messages` route after `build_plan` (synchronous path).
//!   - `tick()` (called by the route) to drain pending jobs that the
//!     confirmation step unblocked.

use chrono::{DateTime, Duration, Utc};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

use crate::audit::{self, AppendEventParams};
use crate::ids;
use crate::models::{User, WorkflowNode};
use crate::tools::{self, ToolContext, ToolError};

/// One requirement declared by a node: must be true of an upstream node
/// before this node is allowed to run.
#[derive(Debug, Clone)]
pub struct NodeRequirement {
    pub upstream_node_id: String,
    pub required_status: String, // 'completed' | 'succeeded' | 'any'
}

/// Per-node failure policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnFailure {
    Fail,
    Skip,
    Compensate,
}

impl OnFailure {
    pub fn from_str(s: &str) -> Self {
        match s {
            "skip" => OnFailure::Skip,
            "compensate" => OnFailure::Compensate,
            _ => OnFailure::Fail,
        }
    }
}

/// Parse a workflow_node's `requires` column into a typed list.
pub fn parse_requires(raw: &str) -> Vec<NodeRequirement> {
    let v: Value = match serde_json::from_str(raw) {
        Ok(v) => v,
        Err(_) => return vec![],
    };
    let arr = match v.as_array() {
        Some(a) => a,
        None => return vec![],
    };
    arr.iter()
        .filter_map(|item| {
            let upstream = item.get("node_id").and_then(|x| x.as_str())?;
            let required = item
                .get("required_status")
                .and_then(|x| x.as_str())
                .unwrap_or("completed");
            Some(NodeRequirement {
                upstream_node_id: upstream.to_string(),
                required_status: required.to_string(),
            })
        })
        .collect()
}

/// Check whether a node's required dependencies are satisfied. Reads the
/// `workflow_node_outcomes` table for the most recent observation of each
/// upstream node. The caller passes a map (node_id -> status string) to
/// avoid an N+1 round-trip in the hot path.
pub fn check_requirements(
    node: &WorkflowNode,
    upstream_statuses: &std::collections::HashMap<String, String>,
) -> Result<(), &'static str> {
    let reqs = parse_requires(&node.requires);
    for r in reqs {
        let observed = upstream_statuses
            .get(&r.upstream_node_id)
            .map(|s| s.as_str())
            .unwrap_or("missing");
        let ok = match r.required_status.as_str() {
            "completed" | "succeeded" => observed == "completed",
            "any" => observed != "missing",
            other => other == observed,
        };
        if !ok {
            return Err("dependency-not-satisfied");
        }
    }
    Ok(())
}

/// Update the workflow-level status based on the node statuses. The state
/// machine is intentionally small:
///
///   - if any non-skipped node is `failed`      → workflow `failed`
///   - if any node is `compensated`             → workflow `compensation_required`
///   - if any node is `running` / `pending`     → workflow `running`
///   - else                                       → workflow `completed`
pub async fn recompute_workflow_status(
    pool: &PgPool,
    workflow_id: &str,
) -> Result<String, sqlx::Error> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT status FROM workflow_nodes WHERE workflow_id = $1",
    )
    .bind(workflow_id)
    .fetch_all(pool)
    .await?;
    let mut has_failed = false;
    let mut has_compensated = false;
    let mut has_inflight = false;
    for (s,) in &rows {
        match s.as_str() {
            "failed" => has_failed = true,
            "compensated" => has_compensated = true,
            "pending" | "awaiting_confirmation" | "awaiting_mfa" | "running" => {
                has_inflight = true
            }
            _ => {}
        }
    }
    let next = if has_failed {
        "failed"
    } else if has_compensated {
        "compensation_required"
    } else if has_inflight {
        "running"
    } else {
        "completed"
    };
    sqlx::query("UPDATE workflows SET status = $1 WHERE id = $2")
        .bind(next)
        .bind(workflow_id)
        .execute(pool)
        .await?;
    Ok(next.to_string())
}

/// Persist the outcome of a node for downstream consumers. Called by the
/// executor after a node's tool returns.
pub async fn record_outcome(
    pool: &PgPool,
    workflow_id: &str,
    consumer_node_id: &str,
    upstream_node_id: &str,
    required_status: &str,
    observed_status: &str,
) -> Result<(), sqlx::Error> {
    let id = ids::new_id("wno");
    let blocked = observed_status == "failed" && required_status == "completed";
    sqlx::query(
        "INSERT INTO workflow_node_outcomes (id, workflow_id, node_id, consumer_node_id, required_status, observed_status, blocked) \
         VALUES ($1, $2, $3, $4, $5, $6, $7) \
         ON CONFLICT (consumer_node_id, node_id) DO UPDATE SET observed_status = EXCLUDED.observed_status, blocked = EXCLUDED.blocked, created_at = NOW()",
    )
    .bind(&id)
    .bind(workflow_id)
    .bind(upstream_node_id)
    .bind(consumer_node_id)
    .bind(required_status)
    .bind(observed_status)
    .bind(blocked)
    .execute(pool)
    .await?;
    Ok(())
}

/// Build a snapshot of the latest observed status for every node in this
/// workflow, keyed by node id. Used by `check_requirements` to avoid an
/// N+1 round-trip in the executor hot path.
pub async fn load_status_snapshot(
    pool: &PgPool,
    workflow_id: &str,
) -> Result<std::collections::HashMap<String, String>, sqlx::Error> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT id, status FROM workflow_nodes WHERE workflow_id = $1",
    )
    .bind(workflow_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().collect())
}

/// Run a single node and update its row. Returns the new status. The caller
/// is responsible for transaction boundaries — we open our own transaction
/// per node so a failure rolls back only that node's effects.
pub async fn run_node(
    pool: &PgPool,
    user: &User,
    node: &WorkflowNode,
) -> Result<String, ToolError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    sqlx::query(
        "UPDATE workflow_nodes SET status = 'running', started_at = NOW() WHERE id = $1",
    )
    .bind(&node.id)
    .execute(&mut *tx)
    .await
    .map_err(|e| ToolError::Other(e.to_string()))?;
    tx.commit()
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;

    let params: Value = serde_json::from_str(&node.parameters).unwrap_or(json!({}));
    let mut effective = params.clone();
    if let Value::Object(ref mut m) = effective {
        m.insert("tier".into(), Value::String(node.risk_tier.clone()));
    }
    let ctx = ToolContext {
        pool: pool.clone(),
        user: user.clone(),
        request_id: node.idempotency_key.clone(),
        session_id: None,
    };
    let reg = tools::registry();
    let tool_fn = match reg.get(node.tool.as_str()) {
        Some(f) => f,
        None => {
            return Err(ToolError::Schema("tool not in allowlist"));
        }
    };
    let result = tool_fn(&ctx, &effective).await;
    let next_status: &str;
    let result_str: String;
    let error_str: String;
    match &result {
        Ok(v) => {
            next_status = "completed";
            result_str = serde_json::to_string(v).unwrap_or_else(|_| "{}".into());
            error_str = String::new();
        }
        Err(e) => {
            next_status = "failed";
            result_str = String::new();
            error_str = serde_json::to_string(&json!({
                "code": e.code(),
                "message": e.to_string(),
            }))
            .unwrap_or_else(|_| "{}".into());
        }
    }
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    sqlx::query(
        "UPDATE workflow_nodes SET status = $1, result = NULLIF($2, ''), error = NULLIF($3, ''), finished_at = NOW() WHERE id = $4",
    )
    .bind(next_status)
    .bind(&result_str)
    .bind(&error_str)
    .bind(&node.id)
    .execute(&mut *tx)
    .await
    .map_err(|e| ToolError::Other(e.to_string()))?;
    tx.commit()
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;

    audit::append_event(
        pool,
        AppendEventParams {
            action: if result.is_ok() { "workflow.node.completed" } else { "workflow.node.failed" },
            actor_user_id: Some(&user.id),
            session: None,
            request_id: Some(&node.idempotency_key),
            workflow_id: Some(&node.workflow_id),
            node_id: Some(&node.id),
            tool: Some(&node.tool),
            parameters_masked: Some(effective),
            risk_tier: Some(&node.risk_tier),
            policy_reason: None,
            confirmation_status: None,
            mfa_status: None,
            result: if result.is_ok() { "ok" } else { "error" },
            error_code: result.as_ref().err().map(|e| e.code()),
        },
    )
    .await
    .ok();
    Ok(next_status.to_string())
}

/// Drive one workflow forward by executing every node whose dependencies
/// are satisfied and which is still pending. Stops on the first failure
/// unless that node's `on_failure` is `skip` or `compensate`.
pub async fn tick(
    pool: &PgPool,
    user: &User,
    workflow_id: &str,
) -> Result<String, ToolError> {
    let nodes: Vec<WorkflowNode> = sqlx::query_as(
        "SELECT * FROM workflow_nodes WHERE workflow_id = $1 ORDER BY node_index ASC",
    )
    .bind(workflow_id)
    .fetch_all(pool)
    .await
    .map_err(|e| ToolError::Other(e.to_string()))?;
    let statuses = load_status_snapshot(pool, workflow_id)
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    for node in &nodes {
        if node.status != "pending" {
            continue;
        }
        if let Err(reason) = check_requirements(node, &statuses) {
            let policy = OnFailure::from_str(&node.on_failure);
            match policy {
                OnFailure::Skip => {
                    sqlx::query("UPDATE workflow_nodes SET status = 'skipped' WHERE id = $1")
                        .bind(&node.id)
                        .execute(pool)
                        .await
                        .ok();
                    let reason_s = reason.to_string();
                    audit::append_event(
                        pool,
                        AppendEventParams {
                            action: "workflow.node.skipped",
                            actor_user_id: Some(&user.id),
                            session: None,
                            request_id: Some(&node.idempotency_key),
                            workflow_id: Some(&node.workflow_id),
                            node_id: Some(&node.id),
                            tool: Some(&node.tool),
                            parameters_masked: None,
                            risk_tier: Some(&node.risk_tier),
                            policy_reason: Some(&reason_s),
                            confirmation_status: None,
                            mfa_status: None,
                            result: "ok",
                            error_code: None,
                        },
                    )
                    .await
                    .ok();
                }
                OnFailure::Compensate => {
                    // Mark this node as skipped and walk backwards through
                    // completed peers to undo them.
                    sqlx::query("UPDATE workflow_nodes SET status = 'skipped' WHERE id = $1")
                        .bind(&node.id)
                        .execute(pool)
                        .await
                        .ok();
                    compensate_chain(pool, user, &node.workflow_id).await.ok();
                }
                OnFailure::Fail => {
                    sqlx::query("UPDATE workflow_nodes SET status = 'failed', error = $1 WHERE id = $2")
                        .bind(reason)
                        .bind(&node.id)
                        .execute(pool)
                        .await
                        .ok();
                    recompute_workflow_status(pool, workflow_id)
                        .await
                        .ok();
                    return Err(ToolError::Other(format!(
                        "workflow node {} failed precondition: {reason}",
                        node.id
                    )));
                }
            }
            continue;
        }
        let new_status = run_node(pool, user, node).await?;
        // Record outcomes for every downstream consumer that has declared
        // this node as a requirement.
        let _ = record_outcome(
            pool,
            workflow_id,
            "<broadcast>",
            &node.id,
            "completed",
            &new_status,
        )
        .await;
    }
    let final_status = recompute_workflow_status(pool, workflow_id)
        .await
        .map_err(|e| ToolError::Other(e.to_string()))?;
    Ok(final_status)
}

/// Walk a workflow backwards and compensate any completed node. Today this
/// only handles the canonical "goals.create_reservation" → "goals.release_
/// reservation" case; the actual side-effect reversal is done in
/// `routes/agent.rs::cancel_workflow`. The executor only flips statuses.
async fn compensate_chain(pool: &PgPool, user: &User, workflow_id: &str) -> Result<(), sqlx::Error> {
    let nodes: Vec<WorkflowNode> = sqlx::query_as(
        "SELECT * FROM workflow_nodes WHERE workflow_id = $1 ORDER BY node_index DESC",
    )
    .bind(workflow_id)
    .fetch_all(pool)
    .await?;
    for n in nodes {
        if n.status != "completed" {
            continue;
        }
        // The actual rollback SQL lives in cancel_workflow; here we just
        // tag the node as compensated and append an audit event so the
        // chain is visible to the admin console.
        sqlx::query("UPDATE workflow_nodes SET status = 'compensated' WHERE id = $1")
            .bind(&n.id)
            .execute(pool)
            .await?;
        audit::append_event(
            pool,
            AppendEventParams {
                action: "workflow.compensate",
                actor_user_id: Some(&user.id),
                session: None,
                request_id: Some(&n.idempotency_key),
                workflow_id: Some(workflow_id),
                node_id: Some(&n.id),
                tool: Some(&n.tool),
                parameters_masked: None,
                risk_tier: Some(&n.risk_tier),
                policy_reason: Some("executor.compensate"),
                confirmation_status: None,
                mfa_status: None,
                result: "ok",
                error_code: None,
            },
        )
        .await
        .ok();
    }
    Ok(())
}

/// Enqueue a node for background execution. The current `tick` function
/// handles everything inline; this entrypoint exists so the route can
/// schedule post-confirmation work without blocking the response.
pub async fn enqueue(pool: &PgPool, node: &WorkflowNode) -> Result<(), sqlx::Error> {
    let id = ids::new_id("wj");
    sqlx::query(
        "INSERT INTO workflow_jobs (id, workflow_id, node_id, status, attempts, next_attempt_at) \
         VALUES ($1, $2, $3, 'pending', 0, NOW())",
    )
    .bind(&id)
    .bind(&node.workflow_id)
    .bind(&node.id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Drain the workflow_jobs table. This is meant to be called periodically
/// (every few seconds) by the API process itself; we don't have a separate
/// worker. Returns the number of jobs actually executed.
pub async fn drain_pending(
    pool: &PgPool,
    user: &User,
    worker_id: &str,
    max_jobs: i64,
) -> Result<usize, sqlx::Error> {
    let now = Utc::now();
    let lock_until: DateTime<Utc> = now + Duration::seconds(30);
    let rows: Vec<(String, String, String)> = sqlx::query_as(
        "UPDATE workflow_jobs \
         SET status = 'running', locked_by = $1, locked_until = $2, attempts = attempts + 1, updated_at = NOW() \
         WHERE id IN ( \
            SELECT id FROM workflow_jobs \
            WHERE status = 'pending' AND next_attempt_at <= NOW() \
            ORDER BY next_attempt_at ASC LIMIT $3 FOR UPDATE SKIP LOCKED \
         ) RETURNING id, workflow_id, node_id",
    )
    .bind(worker_id)
    .bind(lock_until)
    .bind(max_jobs)
    .fetch_all(pool)
    .await?;
    let mut ran = 0;
    for (job_id, _wf_id, node_id) in rows {
        let node: Option<WorkflowNode> =
            sqlx::query_as("SELECT * FROM workflow_nodes WHERE id = $1")
                .bind(&node_id)
                .fetch_optional(pool)
                .await?;
        let Some(node) = node else { continue };
        let result = run_node(pool, user, &node).await;
        let (status, last_error) = match result {
            Ok(_) => ("succeeded".to_string(), None),
            Err(e) => ("failed".to_string(), Some(e.to_string())),
        };
        sqlx::query(
            "UPDATE workflow_jobs SET status = $1, last_error = $2, locked_by = NULL, locked_until = NULL, updated_at = NOW() WHERE id = $3",
        )
        .bind(&status)
        .bind(last_error)
        .bind(&job_id)
        .execute(pool)
        .await
        .ok();
        recompute_workflow_status(pool, &node.workflow_id).await.ok();
        ran += 1;
    }
    Ok(ran)
}

/// Worker id helper.
pub fn worker_id() -> String {
    format!("api-{}", Uuid::new_v4().simple())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_requires_handles_empty() {
        assert!(parse_requires("[]").is_empty());
        assert_eq!(parse_requires("not-json").len(), 0);
    }

    #[test]
    fn parse_requires_decodes() {
        let raw = r#"[{"node_id":"n_1","required_status":"completed"}]"#;
        let r = parse_requires(raw);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].upstream_node_id, "n_1");
        assert_eq!(r[0].required_status, "completed");
    }

    #[test]
    fn on_failure_parsing() {
        assert_eq!(OnFailure::from_str("fail"), OnFailure::Fail);
        assert_eq!(OnFailure::from_str("skip"), OnFailure::Skip);
        assert_eq!(OnFailure::from_str("compensate"), OnFailure::Compensate);
        assert_eq!(OnFailure::from_str("nope"), OnFailure::Fail);
    }

    #[test]
    fn check_requirements_completed() {
        let node = WorkflowNode {
            id: "n_self".into(),
            workflow_id: "wf".into(),
            node_index: 1,
            tool: "x".into(),
            parameters: "{}".into(),
            dependencies: "[]".into(),
            requires: r#"[{"node_id":"n_1","required_status":"completed"}]"#.into(),
            risk_tier: "Green".into(),
            approval_required: false,
            idempotency_key: "k".into(),
            status: "pending".into(),
            result: None,
            error: None,
            compensating_node_id: None,
            pending_action_id: None,
            depends_on: "[]".into(),
            on_failure: "fail".into(),
            started_at: None,
            finished_at: None,
        };
        let mut m = std::collections::HashMap::new();
        m.insert("n_1".into(), "completed".into());
        assert!(check_requirements(&node, &m).is_ok());
        m.insert("n_1".into(), "failed".into());
        assert!(check_requirements(&node, &m).is_err());
    }
}
