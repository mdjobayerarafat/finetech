// API client. All calls go through the Next.js proxy (see next.config.mjs).
// Cookies are HttpOnly; the browser sends them automatically.

const BASE = "/api/v1";

export class ApiError extends Error {
  status: number;
  code: string;
  traceId: string;
  details: unknown;
  /** Raw JSON body of the error response. Useful when a non-2xx carries
   *  machine-readable fields (e.g. `{pending_action_id, tier}` on
   *  approval-required responses) that aren't surfaced through `code`. */
  body: unknown;
  constructor(opts: { status: number; code: string; message: string; traceId: string; details?: unknown; body?: unknown }) {
    super(opts.message);
    this.status = opts.status;
    this.code = opts.code;
    this.traceId = opts.traceId;
    this.details = opts.details;
    this.body = opts.body;
  }
}

function csrf(): string | undefined {
  if (typeof document === "undefined") return undefined;
  const m = document.cookie.match(/(?:^|;\s*)bp_csrf=([^;]+)/);
  return m ? decodeURIComponent(m[1]) : undefined;
}

async function request<T>(
  path: string,
  init: RequestInit = {}
): Promise<T> {
  const method = (init.method ?? "GET").toUpperCase();
  const headers = new Headers(init.headers ?? {});
  headers.set("Accept", "application/json");
  if (init.body && !headers.has("Content-Type")) {
    headers.set("Content-Type", "application/json");
  }
  if (method !== "GET" && method !== "HEAD" && method !== "OPTIONS") {
    const t = csrf();
    if (t) headers.set("X-CSRF-Token", t);
  }
  const res = await fetch(`${BASE}${path}`, {
    credentials: "include",
    ...init,
    headers,
  });
  if (res.status === 204) return undefined as T;
  const text = await res.text();
  let body: unknown = null;
  if (text) {
    try {
      body = JSON.parse(text);
    } catch {
      body = text;
    }
  }
  if (!res.ok) {
    const err = (body as { error?: { code: string; message: string; trace_id: string; details?: unknown } })?.error;
    throw new ApiError({
      status: res.status,
      code: err?.code ?? `http_${res.status}`,
      message: err?.message ?? res.statusText,
      traceId: err?.trace_id ?? "",
      details: err?.details,
      body,
    });
  }
  return body as T;
}

// --- Auth -----------------------------------------------------------------

export const auth = {
  login: (username: string, password: string) =>
    request<{ user_id: string; role: string; csrf_token: string }>(`/auth/login`, {
      method: "POST",
      body: JSON.stringify({ username, password }),
    }),
  logout: () => request<{ ok: boolean }>(`/auth/logout`, { method: "POST" }),
  me: () => request<MeOut>(`/me`),
  csrf: () => request<{ csrf_token: string }>(`/auth/csrf`),
};

export const users = {
  /** Persist the user's preferred interface language ("en" or "zh") on
   *  the server so the next chat / stub response can be rendered in the
   *  same language the UI is using. */
  setPreferredLanguage: (language: "en" | "zh") =>
    request<{ id: string; preferred_language: string }>(`/me/language`, {
      method: "POST",
      body: JSON.stringify({ language }),
    }),
};

export const util = {
  /** Best-effort translation of a list of short strings. The backend
   *  endpoint (POST /api/v1/util/translate) is intentionally infallible
   *  — on any failure it returns the originals. We mirror that here by
   *  also falling back to the inputs if the request itself fails. */
  translate: (texts: string[], target: "en" | "zh") =>
    request<{ translations: string[] }>(`/util/translate`, {
      method: "POST",
      body: JSON.stringify({ texts, target }),
    })
      .then((r) => r.translations)
      .catch(() => texts),
};

export type MeOut = {
  id: string;
  username: string;
  display_name: string;
  preferred_language: string;
  role: "customer" | "admin";
  safety_lock_state: string;
  safety_lock_reason: string | null;
  failed_mfa_count: number;
  risk_tolerance: string;
  horizon: string;
};

// --- Accounts / transactions --------------------------------------------

export type Account = {
  id: string;
  display_name: string;
  masked_number: string;
  currency: string;
  available_balance_fen: number;
  reserved_balance_fen: number;
};

export const accounts = {
  list: () => request<Account[]>(`/accounts`),
  balance: (id: string) => request<Account>(`/accounts/${id}/balance`),
};

export type Budget = {
  id: string;
  category: string;
  limit_fen: number;
  spent_fen: number;
  remaining_fen: number;
  utilization_pct: number;
  breached: boolean;
  account_id: string | null;
  note: string | null;
  created_at: string;
  updated_at: string;
};

export type BudgetSummary = {
  budget_count: number;
  total_limit_fen: number;
  total_spent_fen: number;
  remaining_fen: number;
  breached: BudgetBreachRow[];
  near_limit: BudgetBreachRow[];
  held_reservations_fen: number;
  display: {
    limit: string;
    spent: string;
    remaining: string;
  };
};

export type BudgetBreachRow = {
  category: string;
  limit_fen: number;
  spent_fen: number;
  utilization_pct: number;
};

export type BudgetReservation = {
  id: string;
  user_id: string;
  budget_id: string | null;
  goal_id: string | null;
  label: string;
  amount_fen: number;
  due_at: string | null;
  source_account_id: string | null;
  state: string;
  created_at: string;
};

export const budgets = {
  /** List all monthly category caps the user has set, with computed
   *  spent / remaining / utilization over the trailing 30 days. */
  list: () =>
    request<{ budgets: Budget[] }>(`/budgets`),
  /** Get the cross-budget rollup (count, totals, near-limit, breaches). */
  summary: () => request<BudgetSummary>(`/budgets/summary`),
  /** Set a per-category monthly cap. `amount` can be a CNY number or
   *  a string like "3000"; the backend normalises. */
  set: (body: {
    category: string;
    amount: number | string;
    account_id?: string;
    note?: string;
  }) =>
    request<Budget>(`/budgets`, {
      method: "POST",
      body: JSON.stringify(body),
    }),
  remove: (id: string) =>
    request<{ deleted: boolean; id: string }>(`/budgets/${id}`, {
      method: "DELETE",
    }),
  reservations: () =>
    request<{ reservations: BudgetReservation[]; total_held_fen: number }>(
      `/budgets/reservations`
    ),
};

export type Txn = {
  id: string;
  account_id: string;
  amount_fen: number;
  direction: "credit" | "debit";
  category: string;
  merchant: string;
  memo: string;
  occurred_at: string;
  status: string;
};

export const transactions = {
  list: (params?: { account_id?: string; from?: string; to?: string; category?: string; limit?: number }) => {
    const q = new URLSearchParams();
    if (params?.account_id) q.set("account_id", params.account_id);
    if (params?.from) q.set("from", params.from);
    if (params?.to) q.set("to", params.to);
    if (params?.category) q.set("category", params.category);
    if (params?.limit) q.set("limit", String(params.limit));
    return request<Txn[]>(`/transactions?${q.toString()}`);
  },
};

// --- Reports --------------------------------------------------------------

export type SpendingItem = {
  category: string;
  total_fen: number;
  share_pct: number;
  transaction_count: number;
};
export type SpendingReport = {
  from_date: string;
  to_date: string;
  group_by: "category" | "day";
  income_fen: number;
  expense_fen: number;
  items: SpendingItem[];
};
export type Anomaly = {
  transaction_id: string;
  merchant: string;
  amount_fen: number;
  occurred_at: string;
  reasons: string[];
  baseline_median_fen: number | null;
};
export type MonthlyReport = {
  year: number;
  month: number;
  income_fen: number;
  expense_fen: number;
  by_category: SpendingItem[];
  mom_delta_fen: number | null;
  has_baseline: boolean;
};
export type OptimizationSuggestion = {
  code:
    | "unused_subscription"
    | "subscription_bloat"
    | "category_overspend"
    | "low_balance_risk"
    | "transfer_fragmentation"
    | "micro_spend"
    | string;
  severity: "info" | "warning" | "critical" | string;
  title: string;
  body: string;
  monthly_savings_fen: number;
  annual_savings_fen: number;
  action_label: string;
  action_endpoint: string | null;
  evidence: Record<string, unknown>;
  rank: number;
};
export type OptimizationReport = {
  generated_at: string;
  subscription_count: number;
  total_monthly_savings_fen: number;
  total_annual_savings_fen: number;
  suggestions: OptimizationSuggestion[];
};

export const reports = {
  spending: (params?: { from?: string; to?: string; group_by?: "category" | "day" }) => {
    const q = new URLSearchParams();
    if (params?.from) q.set("from", params.from);
    if (params?.to) q.set("to", params.to);
    if (params?.group_by) q.set("group_by", params.group_by);
    return request<SpendingReport>(`/reports/spending?${q.toString()}`);
  },
  anomalies: (params?: { from?: string; to?: string }) => {
    const q = new URLSearchParams();
    if (params?.from) q.set("from", params.from);
    if (params?.to) q.set("to", params.to);
    return request<Anomaly[]>(`/reports/anomalies?${q.toString()}`);
  },
  monthly: (year: number, month: number) => request<MonthlyReport>(`/reports/monthly?year=${year}&month=${month}`),
  optimization: () => request<OptimizationReport>(`/reports/optimization`),
};

// --- Payees / transfers --------------------------------------------------

export const payees = {
  list: () => request<Payee[]>(`/payees`),
  create: (body: { display_name: string; phone_tail?: string }) =>
    request<Payee>(`/payees`, { method: "POST", body: JSON.stringify(body) }),
  // v3
  mergeDuplicates: (keep_id: string, merge_id: string) =>
    request<{ kept_payee_id: string; merged_payee_id: string; transfers_rewritten: number }>(
      `/payees/merge-duplicates`,
      { method: "POST", body: JSON.stringify({ keep_id, merge_id }) },
    ),
  startVerification: (id: string, challenge_fen: number) =>
    request<{ verification_id: string; payee_id: string; challenge_fen: number; expires_at: string; demo_code: string }>(
      `/payees/${id}/verify`,
      { method: "POST", body: JSON.stringify({ challenge_fen }) },
    ),
};

export type TransferPreview = {
  resolved_payee_id: string;
  resolved_payee_name: string;
  resolved_payee_destination: string;
  amount_fen: number;
  risk_tier: "Green" | "Yellow" | "Red";
  decision_reason: string;
  daily_remaining_fen: number;
};
export type TransferCreateOut = {
  transfer_id: string;
  pending_action_id: string | null;
  risk_tier: "Green" | "Yellow" | "Red";
};

export const transfers = {
  preview: (body: { recipient_query?: string; payee_id?: string; phone_tail?: string; amount: string | number; memo?: string; account_id?: string }) =>
    request<TransferPreview>(`/transfers/preview`, { method: "POST", body: JSON.stringify(body) }),
  create: (body: { recipient_query?: string; payee_id?: string; phone_tail?: string; amount: string | number; memo?: string; account_id?: string }) =>
    request<TransferCreateOut>(`/transfers`, { method: "POST", body: JSON.stringify(body) }),
  list: () => request<unknown[]>(`/transfers`),
  cancel: (id: string) =>
    request<{ transfer_id: string; status: string }>(`/transfers/${id}/cancel`, { method: "POST" }),
  // v3
  getStatus: (id: string) =>
    request<{
      id: string; status: string; amount_fen: number; memo: string;
      schedule: string | null; created_at: string; posted_at: string | null;
      reversible: boolean;
    }>(`/transfers/${id}/status`),
  reverse: (id: string, reason: string) =>
    request<{ pending_action_id: string; risk_tier: string; expires_at: string; next_step: string; mock_mfa_code: string }>(
      `/transfers/${id}/reverse`,
      { method: "POST", body: JSON.stringify({ reason }) },
    ),
};

// --- Smart transfer agent --------------------------------------------------
//
// The smart-transfer hub in the frontend talks to the `smart_transfers`
// module. The planner also calls into these same tools from chat, so a
// payee added in /transfers shows up in the next "Pay Li Wei" prompt.

export type Payee = {
  id: string;
  owner_user_id: string;
  display_name: string;
  phone_tail: string | null;
  mock_destination: string;
  verified: boolean;
  first_used_at: string | null;
  created_at: string;
};

export type PayeeSuggestion = {
  payee: Payee;
  match_kind: "exact_name" | "phone_tail" | "prefix_name" | "contains_name" | string;
  confidence: number;
  note: string;
};

export type SmartParseOut = {
  amount_fen: number;
  recipient_query: string;
  memo: string;
  schedule_iso: string | null;
  split_participants: string[] | null;
  needs_clarification: boolean;
  questions: string[];
  resolved_payee?: Payee | null;
  resolve_status?: "resolved" | "ambiguous" | "none" | string;
  candidates?: PayeeSuggestion[];
  resolve_note?: string;
};

export type SecurityReview = {
  recommended_tier: "Green" | "Yellow" | "Red" | string;
  fraud_score: number;
  flags: FraudFlag[];
  injection_phrase: string | null;
  urgency_detected: boolean;
  summary: string;
  schedule_note: string;
};

export type SmartPreviewOut = {
  payee: Payee;
  amount_fen: number;
  memo: string;
  schedule_iso: string | null;
  review: SecurityReview;
  balance_check: { account_id: string; available_balance_fen: number; after_transfer_fen: number } | null;
};

export type SplitBillPart = {
  id: string;
  split_bill_id: string;
  payee_id: string | null;
  participant_name: string;
  participant_phone_tail: string | null;
  share_fen: number;
  status: "unpaid" | "paid" | "declined" | string;
  paid_at: string | null;
  paid_transfer_id: string | null;
  created_at: string;
};

export type SplitBill = {
  id: string;
  owner_user_id: string;
  title: string;
  memo: string;
  total_fen: number;
  currency: string;
  source_account_id: string | null;
  status: "open" | "partial" | "settled" | "cancelled" | string;
  due_at: string | null;
  created_at: string;
  updated_at: string;
  parts?: SplitBillPart[];
  paid_fen?: number;
  remaining_fen?: number;
};

export const smart = {
  payees: {
    list: (q?: string) =>
      request<
        Payee[] | { query: string; suggestions: PayeeSuggestion[] }
      >(`/smart-transfers/payees${q ? `?q=${encodeURIComponent(q)}` : ""}`),
    create: (body: { display_name: string; phone_tail?: string; note?: string }) =>
      request<Payee>(`/smart-transfers/payees`, {
        method: "POST",
        body: JSON.stringify(body),
      }),
  },
  parse: (body: { text: string; language?: string }) =>
    request<SmartParseOut>(`/smart-transfers/parse`, {
      method: "POST",
      body: JSON.stringify(body),
    }),
  preview: (body: {
    payee_id?: string;
    payee_name?: string;
    amount_fen: number;
    memo?: string;
    schedule_iso?: string;
    account_id?: string;
  }) =>
    request<SmartPreviewOut>(`/smart-transfers/preview`, {
      method: "POST",
      body: JSON.stringify(body),
    }),
  schedule: (body: {
    payee_id: string;
    amount_fen: number;
    memo?: string;
    schedule_iso: string;
    account_id?: string;
  }) =>
    request<{
      transfer_id: string;
      payee_id: string;
      amount_fen: number;
      schedule_iso: string;
      status: string;
    }>(`/smart-transfers/schedule`, { method: "POST", body: JSON.stringify(body) }),
  /**
   * `execute` is the pay-now counterpart of `schedule`. On Green tier the
   * account is debited immediately and a `transfer_id` is returned. On
   * Yellow/Red the request returns 403 with a `pending_action_id` that
   * the frontend drives through the existing mock-MFA + confirm flow.
   */
  execute: (body: {
    payee_id: string;
    amount_fen: number;
    memo?: string;
    account_id?: string;
    idempotency_key?: string;
  }) =>
    request<{
      transfer_id: string | null;
      status: string;
      amount_fen: number;
      posted_at: string | null;
      pending_action_id: string | null;
      replay?: boolean;
    }>(`/smart-transfers/execute`, {
      method: "POST",
      body: JSON.stringify(body),
    }),
  scheduled: () =>
    request<
      Array<{
        id: string;
        payee_id: string;
        amount_fen: number;
        memo: string;
        schedule: string | null;
        status: string;
        created_at: string;
      }>
    >(`/smart-transfers/scheduled`),
  /**
   * "Beneficial accounts" = the payee's own account, in real banking
   * parlance. The directory is the same `payees` table the smart
   * resolver reads, but the form takes a richer shape (display name +
   * phone tail + bank hint + note) so the user can build a real,
   * persistent list of who they pay. The bank hint is stored alongside
   * the destination and surfaced back in the UI.
   */
  beneficialAccounts: {
    list: (q?: string) =>
      request<
        Array<{
          id: string;
          display_name: string;
          phone_tail: string | null;
          bank_hint: string | null;
          destination: string;
          verified: boolean;
          first_used_at: string | null;
          created_at: string;
        }>
      >(
        `/smart-transfers/beneficial-accounts${
          q ? `?q=${encodeURIComponent(q)}` : ""
        }`,
      ),
    create: (body: {
      display_name: string;
      phone_tail?: string;
      bank_hint?: string;
      note?: string;
    }) =>
      request<{
        id: string;
        display_name: string;
        phone_tail: string | null;
        bank_hint: string | null;
        destination: string;
        verified: boolean;
        first_used_at: string | null;
        created_at: string;
      }>(`/smart-transfers/beneficial-accounts`, {
        method: "POST",
        body: JSON.stringify(body),
      }),
  },
};

export const splitBills = {
  list: () => request<SplitBill[]>(`/split-bills`),
  create: (body: {
    title: string;
    memo?: string;
    total_fen: number;
    currency?: string;
    source_account_id?: string;
    parts: Array<{
      payee_id?: string;
      participant_name: string;
      participant_phone_tail?: string;
      share_fen: number;
    }>;
  }) =>
    request<SplitBill>(`/split-bills`, {
      method: "POST",
      body: JSON.stringify(body),
    }),
  pay: (billId: string, partId: string, transfer_id: string) =>
    request<SplitBillPart>(`/split-bills/${billId}/parts/${partId}/pay`, {
      method: "POST",
      body: JSON.stringify({ transfer_id }),
    }),
};

// --- Subscriptions / reminders -------------------------------------------

export type Subscription = {
  id: string;
  merchant: string;
  estimated_amount_fen: number;
  interval_days: number;
  next_expected_at: string;
  last_charge_at: string | null;
  cancellation_status: "active" | "cancellation_requested" | "cancelled" | "failed";
  evidence: Record<string, unknown>;
};
export type Reminder = {
  id: string;
  kind: string;
  subject: string;
  due_at: string;
  status: string;
  linked_subscription_id: string | null;
};

export const subscriptions = {
  list: () => request<Subscription[]>(`/subscriptions`),
  remind: (id: string, days_before = 3) =>
    request<Reminder>(`/subscriptions/${id}/remind?days_before=${days_before}`, { method: "POST" }),
  cancel: (id: string) =>
    request<{ subscription_id: string; status: string }>(`/subscriptions/${id}/cancel`, { method: "POST" }),
  // v3 — close-the-loop
  snooze: (id: string, days: number) =>
    request<{ id: string; merchant: string; next_expected_at: string; snooze_until: string }>(
      `/subscriptions/${id}/snooze`,
      { method: "POST", body: JSON.stringify({ days }) },
    ),
  defer: (id: string, reason = "") =>
    request<{ id: string; merchant: string; previous_expected_at: string; new_next_expected_at: string }>(
      `/subscriptions/${id}/defer`,
      { method: "POST", body: JSON.stringify({ reason }) },
    ),
  uncancel: (id: string) =>
    request<{ id: string; status: string }>(`/subscriptions/${id}/uncancel`, { method: "POST" }),
};
export const reminders = {
  list: () => request<Reminder[]>(`/reminders`),
  delete: (id: string) => request<{ reminder_id: string; status: string }>(`/reminders/${id}`, { method: "DELETE" }),
};

// --- Cards ---------------------------------------------------------------

export type Card = {
  id: string;
  display_name: string;
  masked_number: string;
  expiry_month: number;
  expiry_year: number;
  status: "issued" | "active" | "frozen" | "lost" | "cancelled" | "expired";
  single_tx_limit_fen: number | null;
  monthly_limit_fen: number | null;
};

export const cards = {
  list: () => request<Card[]>(`/cards`),
  issue: (body: { display_name: string }) =>
    request<Card>(`/cards`, { method: "POST", body: JSON.stringify(body) }),
  freeze: (id: string) => request<Card>(`/cards/${id}/freeze`, { method: "POST" }),
  unfreeze: (id: string) => request<Card>(`/cards/${id}/unfreeze`, { method: "POST" }),
  cancel: (id: string) => request<Card>(`/cards/${id}/cancel`, { method: "POST" }),
  reportLost: (id: string) => request<Card>(`/cards/${id}/report-lost`, { method: "POST" }),
};

// --- Wealth --------------------------------------------------------------

export type WealthProduct = {
  id: string;
  name: string;
  category: string;
  risk_level: "low" | "medium" | "high";
  expected_annual_return_pct: number;
  min_subscription_fen: number;
  suitability_required: string;
};
export type WealthHolding = {
  id: string;
  product_id: string;
  amount_fen: number;
  subscribed_at: string;
  status: "active" | "redeemed";
};

export const wealth = {
  products: () => request<WealthProduct[]>(`/wealth/products`),
  product: (id: string) => request<WealthProduct>(`/wealth/products/${id}`),
  holdings: () => request<WealthHolding[]>(`/wealth/holdings`),
  subscribe: (body: { product_id: string; amount: string | number; account_id?: string }) =>
    request<{ holding_id: string; product_id: string; amount_fen: number; risk_tier: string }>(
      `/wealth/subscribe`,
      { method: "POST", body: JSON.stringify(body) }
    ),
  redeem: (body: { holding_id: string; account_id?: string }) =>
    request<{ holding_id: string; status: string; amount_fen: number }>(`/wealth/redeem`, {
      method: "POST",
      body: JSON.stringify(body),
    }),
};

// --- Agent / workflows / approvals --------------------------------------

export type AgentNode = {
  id: string;
  index: number;
  tool: string;
  parameters: Record<string, unknown>;
  risk_tier: "Green" | "Yellow" | "Red";
  approval_required: boolean;
  status: string;
  pending_action_id: string | null;
  result?: Record<string, unknown> | null;
  fraud_assessment?: FraudAssessment | null;
};
export type AgentReadResult = {
  tool: string;
  result?: Record<string, unknown>;
  error?: string;
};

export type AgentMessageOut = {
  intent: string;
  language: string;
  entities: Record<string, unknown>;
  clarification_required: boolean;
  questions: string[];
  plan: { workflow_id: string | null; status?: string; nodes: AgentNode[] };
  pending_action: { id: string; tool: string; parameter_hash: string; risk_tier: string; expires_at: string } | null;
  proposal_text: string | null;
  trace_id: string;
  read_results?: AgentReadResult[];
  /** Relative URL to a downloadable statement file (CSV/TSV/JSON), set
   *  when the user asked for a monthly report. The browser triggers
   *  Save As because the backend sends Content-Disposition: attachment. */
  statement_url?: string | null;
  /** Suggested filename for the statement download. */
  statement_filename?: string | null;
};
export type PendingActionOut = {
  id: string;
  tool: string;
  parameters: Record<string, unknown>;
  parameter_hash: string;
  risk_tier: "Green" | "Yellow" | "Red";
  expires_at: string;
  confirmation_status: string;
  mfa_status: string;
};
export type WorkflowOut = {
  id: string;
  status: string;
  nodes: Array<{
    id: string;
    index: number;
    tool: string;
    parameters: Record<string, unknown>;
    risk_tier: "Green" | "Yellow" | "Red";
    status: string;
    pending_action_id: string | null;
    result: Record<string, unknown> | null;
  }>;
};
export type ConfirmOut = {
  pending_action_id: string;
  confirmation_status: string;
  mfa_status: string;
  workflow_id: string | null;
  workflow_node_id: string | null;
  executed: boolean;
  result: Record<string, unknown> | null;
};

export const agent = {
  message: (text: string, language?: string) =>
    request<AgentMessageOut>(`/agent/messages`, {
      method: "POST",
      body: JSON.stringify({ text, language }),
    }),
  workflow: (id: string) => request<WorkflowOut>(`/workflows/${id}`),
  pauseWorkflow: (id: string) => request<WorkflowOut>(`/workflows/${id}/pause`, { method: "POST" }),
  resumeWorkflow: (id: string) => request<WorkflowOut>(`/workflows/${id}/resume`, { method: "POST" }),
  cancelWorkflow: (id: string) => request<WorkflowOut>(`/workflows/${id}/cancel`, { method: "POST" }),
  getAction: (id: string) => request<PendingActionOut>(`/actions/${id}`),
  verify: (id: string, code: string) =>
    request<ConfirmOut>(`/actions/${id}/verify`, { method: "POST", body: JSON.stringify({ code }) }),
  confirm: (id: string) => request<ConfirmOut>(`/actions/${id}/confirm`, { method: "POST" }),
};

// --- Audit / admin -------------------------------------------------------

export type AuditEvent = {
  id: string;
  created_at: string;
  actor_user_id: string | null;
  action: string;
  tool: string | null;
  risk_tier: string | null;
  policy_reason: string | null;
  confirmation_status: string | null;
  mfa_status: string | null;
  parameters_masked: Record<string, unknown>;
  result: string;
  error_code: string | null;
};
export type SafetyEvent = {
  id: string;
  user_id: string;
  kind: string;
  created_at: string;
  metadata: Record<string, unknown>;
};
export type ChainVerify = {
  ok: boolean;
  events: number;
  break_at: string | null;
  reason: string | null;
};

export const auditApi = {
  list: (limit = 100) => request<AuditEvent[]>(`/audit?limit=${limit}`),
  verifyChain: () => request<ChainVerify>(`/audit/verify-chain`),
  safetyEvents: () => request<SafetyEvent[]>(`/admin/safety-events`),
  unlock: (user_id: string, reason: string) =>
    request<{ user_id: string; safety_lock_state: string }>(`/admin/safety-lock/${user_id}/unlock`, {
      method: "POST",
      body: JSON.stringify({ reason }),
    }),
};

export type AdminUser = {
  id: string;
  username: string;
  display_name: string;
  preferred_language: string;
  role: "customer" | "admin";
  safety_lock_state: string;
  safety_lock_reason: string | null;
  failed_mfa_count: number;
  risk_tolerance: string;
  horizon: string;
  created_at: string;
  account_count: number;
  transaction_count: number;
  locked_event_count: number;
};

export type AdminOverview = {
  counts: {
    users: number;
    customers: number;
    admins: number;
    locked: number;
    transactions: number;
    safety_events: number;
    audit_events: number;
  };
  chain: {
    ok: boolean;
    events: number;
    break_at: string | null;
    reason: string | null;
  };
  recent_safety: SafetyEvent[];
  locked_users: Array<{
    id: string;
    username: string;
    display_name: string;
    safety_lock_state: string;
    safety_lock_reason: string | null;
    failed_mfa_count: number;
    role: string;
  }>;
};

export const adminApi = {
  overview: () => request<AdminOverview>(`/admin/overview`),
  users: () => request<AdminUser[]>(`/admin/users`),
};

// --- Phase 5 demo features -----------------------------------------------

// 1. Scam & fraud guard
export type FraudFlag = {
  code: string;
  message: string;
  weight: number;
};
export type FraudAssessment = {
  flags: FraudFlag[];
  score: number;
  templated: string;
  narrative: string | null;
};
export type FraudScoreOut = {
  assessment: FraudAssessment;
  payee: {
    id: string;
    display_name: string;
    verified: boolean;
    first_used_at: string | null;
  };
  policy_injection_phrase: string | null;
};

export const fraud = {
  score: (body: {
    payee_id?: string;
    payee_name?: string;
    amount_fen: number;
    memo?: string;
    with_narrative?: boolean;
  }) => request<FraudScoreOut>(`/fraud/score`, { method: "POST", body: JSON.stringify(body) }),
  narrate: (body: {
    flags: FraudFlag[];
    score: number;
    payee_name: string;
    amount_fen: number;
  }) =>
    request<{ narrative: string }>(`/fraud/narrate`, {
      method: "POST",
      body: JSON.stringify(body),
    }),
};

// 2. Red-team panel
export type RedTeamSample = { where: string; text: string };
export type RedTeamHit = {
  where_: string;
  blocked: boolean;
  phrase: string | null;
  sanitized_length: number;
  sanitized: string;
  llm_view: string;
  safety_event_id: string | null;
};

export const redteam = {
  samples: () => request<{ samples: RedTeamSample[] }>(`/redteam/samples`),
  test: (body: { text: string; where_: string; persist?: boolean }) =>
    request<RedTeamHit>(`/redteam/test`, { method: "POST", body: JSON.stringify(body) }),
};

// 3. Forecast / "Can I afford it?"
export type ForecastPoint = {
  month_index: number;
  label: string;
  projected_balance_fen: number;
  baseline_fen: number;
};
export type UpcomingBill = {
  subject: string;
  due_at: string;
  amount_fen: number;
};
export type ForecastBudgetRow = {
  category: string;
  limit_fen: number;
  spent_fen: number;
  remaining_fen: number;
  breached: boolean;
};
export type ForecastOut = {
  months: number;
  starting_balance_fen: number;
  avg_income_fen: number;
  avg_expense_fen: number;
  monthly_subscriptions_fen: number;
  upcoming_bills: UpcomingBill[];
  series: ForecastPoint[];
  scenario: Record<string, unknown>;
  verdict: "yes" | "tight" | "no";
  narrative: string;
  held_reservations_fen: number;
  total_budget_limit_fen: number;
  budgets: ForecastBudgetRow[];
};

export const forecast = {
  simulate: (body: { amount_cny: number; spend_month_index?: number; months?: number }) =>
    request<ForecastOut>(`/forecast/simulate`, {
      method: "POST",
      body: JSON.stringify(body),
    }),
};

// 4. Receipt scan
export type ReceiptDraft = {
  payee_hint: string;
  amount_cny: number;
  amount_fen: number;
  due_at: string;
  currency: string;
  confidence: number;
  notes: string[];
  proposed_arguments: Record<string, unknown>;
  source: "stub" | "llm";
};

export const receipts = {
  scan: (body: {
    merchant_hint?: string;
    amount_cny?: number;
    filename?: string;
    image_b64?: string;
  }) =>
    request<ReceiptDraft>(`/receipts/scan`, {
      method: "POST",
      body: JSON.stringify(body),
    }),
};

// 5. Voice
export type SttOut = {
  text: string;
  language: string;
  auto_run: boolean;
  echo_text: string;
};
export type TtsOut = {
  audio_b64: string;
  mime: string;
  text: string;
  prefer_browser_tts: boolean;
};

export const voice = {
  stt: (audio_b64: string, mime = "audio/webm") =>
    request<SttOut>(`/voice/stt`, {
      method: "POST",
      body: JSON.stringify({ audio_b64, mime }),
    }),
  tts: (text: string, language?: string) =>
    request<TtsOut>(`/voice/tts`, {
      method: "POST",
      body: JSON.stringify({ text, language }),
    }),
};

// --- Admin: agent rules + monitoring -------------------------------------

export type AgentRule = {
  id: string;
  kind:
    | "forbid_tool"
    | "restrict_payee"
    | "restrict_memo"
    | "restrict_amount_fen"
    | "restrict_daily_fen"
    | "system_prompt_prefix";
  name: string;
  pattern: string;
  severity: "block" | "warn" | "log";
  applies_to: string; // 'all' or a user id
  enabled: boolean;
  notes: string | null;
  created_by: string | null;
  created_at: string;
  updated_at: string;
};

export type SystemPromptPreview = {
  user_id: string;
  prefix: string;
  snapshot: { rules: AgentRule[] };
  forbidden_tools: AgentRule[];
};

export type AdminUserDetail = {
  id: string;
  username: string;
  display_name: string;
  preferred_language: string;
  role: "customer" | "admin";
  safety_lock_state: string;
  safety_lock_reason: string | null;
  failed_mfa_count: number;
  risk_tolerance: string;
  horizon: string;
  created_at: string;
  accounts: Array<{
    id: string;
    display_name: string;
    masked_number: string;
    currency: string;
    available_balance_fen: number;
    reserved_balance_fen: number;
  }>;
  transaction_count: number;
  conversation_count: number;
  safety_event_count: number;
  balance_total_fen: number;
};

export type AdminTransaction = {
  id: string;
  account_id: string;
  amount_fen: number;
  direction: "debit" | "credit";
  category: string;
  merchant: string;
  memo: string;
  occurred_at: string;
  status: string;
};

export type AdminConversation = {
  id: string;
  user_id: string;
  title: string | null;
  last_message_at: string;
  slot_state: string;
  pending_workflow_id: string | null;
  advisor_mode: boolean;
  created_at: string;
};

export type AdminConversationDetail = {
  conversation: AdminConversation;
  messages: Array<{
    id: string;
    conversation_id: string;
    user_id: string;
    role: "user" | "assistant" | "system" | "tool";
    content: string;
    proposal_json: string | null;
    workflow_id: string | null;
    tool_calls_json: string | null;
    created_at: string;
  }>;
};

export const agentRulesApi = {
  list: () => request<{ rules: AgentRule[] }>(`/admin/agent-rules`),
  create: (body: {
    kind: AgentRule["kind"];
    name: string;
    pattern: string;
    severity?: "block" | "warn" | "log";
    applies_to?: string;
    enabled?: boolean;
    notes?: string;
  }) =>
    request<AgentRule>(`/admin/agent-rules`, {
      method: "POST",
      body: JSON.stringify(body),
    }),
  update: (
    id: string,
    patch: {
      name?: string;
      pattern?: string;
      severity?: "block" | "warn" | "log";
      applies_to?: string;
      enabled?: boolean;
      notes?: string;
    },
  ) =>
    request<AgentRule>(`/admin/agent-rules/${id}`, {
      method: "PATCH",
      body: JSON.stringify(patch),
    }),
  remove: (id: string) =>
    request<{ ok: boolean; id: string }>(`/admin/agent-rules/${id}`, {
      method: "DELETE",
    }),
  systemPromptPreview: (user_id?: string) =>
    request<SystemPromptPreview>(
      `/admin/system-prompt-preview${user_id ? `?user_id=${encodeURIComponent(user_id)}` : ""}`,
    ),
};

export const adminUserApi = {
  detail: (id: string) => request<AdminUserDetail>(`/admin/users/${id}`),
  transactions: (id: string, limit = 100) =>
    request<{ transactions: AdminTransaction[]; limit: number }>(
      `/admin/users/${id}/transactions?limit=${limit}`,
    ),
  conversations: (id: string) =>
    request<{ conversations: AdminConversation[] }>(
      `/admin/users/${id}/conversations`,
    ),
  safetyEvents: (id: string) =>
    request<{ events: SafetyEvent[] }>(`/admin/users/${id}/safety-events`),
};

export const adminConversationsApi = {
  list: () =>
    request<{ conversations: AdminConversation[] }>(`/admin/conversations`),
  detail: (id: string) =>
    request<AdminConversationDetail>(`/admin/conversations/${id}`),
};

// --- Agent v3 — close-the-loop, plan-persistence, proactive intelligence ----

export type DigestRenewal = {
  id: string;
  merchant: string;
  amount_fen: number;
  next_expected_at: string;
};
export type DigestGoal = {
  id: string;
  name: string;
  target_fen: number;
  this_week_fen: number;
  progress_pct: number;
};
export type DailyDigest = {
  generated_at: string;
  items: {
    renewals: DigestRenewal[];
    goals: DigestGoal[];
    anomaly_count_24h: number;
    spend_24h_fen: number;
    pending_workflows: number;
  };
  hash: string;
};

export const insights = {
  daily: () => request<DailyDigest>(`/insights/daily`),
  ack: () => request<DailyDigest>(`/insights/daily/ack`, { method: "POST" }),
};

export type PendingWorkflow = {
  workflow_id: string;
  title: string;
  created_at: string;
  status: string;
};

export const workflows = {
  pending: () =>
    request<{ items: PendingWorkflow[]; count: number }>(`/workflows-pending`),
  resume: (id: string) =>
    request<{
      workflow_id: string;
      status: string;
      next_pending_action: { id: string; tool: string; expires_at: string } | null;
    }>(`/workflows/${id}/resume`, { method: "POST" }),
};

export const actions = {
  defer: (id: string, minutes: number) =>
    request<{ pending_action_id: string; new_expires_at: string }>(
      `/actions/${id}/defer`,
      { method: "POST", body: JSON.stringify({ minutes }) },
    ),
};

export type TransactionRow = {
  id: string;
  account_id: string;
  amount_fen: number;
  category: string;
  occurred_at: string;
  memo: string;
  merchant?: string;
  direction: "credit" | "debit";
};

export const transactionsV3 = {
  search: (query: string, opts?: { from?: string; to?: string; min_amount_fen?: number; max_amount_fen?: number; category?: string; limit?: number }) => {
    const params = new URLSearchParams({ query });
    if (opts?.from) params.set("from", opts.from);
    if (opts?.to) params.set("to", opts.to);
    if (opts?.min_amount_fen != null) params.set("min_amount_fen", String(opts.min_amount_fen));
    if (opts?.max_amount_fen != null) params.set("max_amount_fen", String(opts.max_amount_fen));
    if (opts?.category) params.set("category", opts.category);
    if (opts?.limit != null) params.set("limit", String(opts.limit));
    return request<{ items: TransactionRow[]; count: number }>(`/transactions/search?${params.toString()}`);
  },
  merchantHistory: (merchant: string) =>
    request<{
      merchant: string; months: number; count: number; total_fen: number;
      average_fen: number; recent: TransactionRow[];
    }>(`/transactions/merchant/${encodeURIComponent(merchant)}/history`),
  disputeDraft: (id: string, reason: string) =>
    request<{
      transaction_id: string;
      dispute_status: string;
      draft: string;
      // Convenience: backend embeds the dispute reference id in the draft text
      // (e.g. "Reference: dispute-<id>"). Surfacing it lets the UI toast a
      // short, scannable id without re-parsing.
      reference_id?: string;
    }>(
      `/transactions/${id}/dispute-draft`,
      { method: "POST", body: JSON.stringify({ reason }) },
    ),
};

export type GoalProgress = {
  id: string;
  name: string;
  target_fen: number;
  saved_fen: number;
  this_month_fen: number;
  progress_pct: number;
  daily_pace_fen: number;
  days_to_goal: number | null;
};

export const goals = {
  list: () =>
    request<{
      items: GoalProgress[];
      count: number;
    }>(`/goals`),
  progress: (id: string) => request<GoalProgress>(`/goals/${id}/progress`),
  rebalance: (id: string, monthly_amount_fen: number, source_account_id?: string) =>
    request<{
      goal_id: string; name: string; saved_fen: number; target_fen: number;
      remaining_fen: number; monthly_amount_fen: number; months_to_goal: number | null;
      note: string;
    }>(`/goals/${id}/rebalance`, {
      method: "POST",
      body: JSON.stringify({ monthly_amount_fen, source_account_id }),
    }),
  contribute: (id: string, amount_fen: number, source_account_id: string, memo = "") =>
    request<{ id: string; goal_id: string; amount_fen: number; new_balance_fen: number }>(
      `/goals/${id}/contribute`,
      { method: "POST", body: JSON.stringify({ amount_fen, source_account_id, memo }) },
    ),
};

// --- Layer 5.5 — friendly error mapper ---------------------------------------
//
// Translate machine `code` values into the user-facing "what to do next" copy
// the chat surface renders in its error toast. Falls back to the raw message
// when the code is unknown.

export type FriendlyError = {
  title: string;
  body: string;
  action?: { label: string; href?: string };
};

export function friendlyError(err: ApiError): FriendlyError {
  const d = (err.details ?? {}) as Record<string, unknown>;
  const whatNext = typeof d.what_next === "string" ? (d.what_next as string) : null;
  const question = typeof d.question === "string" ? (d.question as string) : null;
  switch (err.code) {
    case "insufficient_funds":
      return {
        title: "Not enough in this account",
        body: whatNext ?? "Top up, pick a different source, or reduce the amount.",
        action: { label: "Open accounts", href: "/accounts" },
      };
    case "below_minimum":
      return {
        title: "Amount is below the minimum",
        body: whatNext ?? "Try a higher amount, or pick a different goal.",
      };
    case "clarification_required":
      return {
        title: "One more thing",
        body: question ?? "The agent needs more information to continue.",
      };
    case "forbidden":
      return {
        title: "Blocked by an admin rule",
        body: whatNext ?? err.message,
        action: { label: "Open chat", href: "/chat" },
      };
    case "schema_invalid":
      return { title: "Check the input", body: whatNext ?? err.message };
    case "not_found":
      return { title: "Couldn't find it", body: err.message };
    case "mfa_required":
    case "mfa_failed":
      return {
        title: err.code === "mfa_failed" ? "That code didn't match" : "MFA required",
        body: err.message,
        action: { label: "Open the action", href: "/chat" },
      };
    case "expired":
      return { title: "This expired", body: "Open the chat to start a new confirmation." };
    case "parameter_changed":
      return { title: "The parameters changed", body: "Please re-confirm with the new values." };
    default:
      return { title: "Something went wrong", body: err.message };
  }
}

