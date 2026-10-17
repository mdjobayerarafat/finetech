"use client";

import { useEffect, useMemo, useState } from "react";
import { motion } from "framer-motion";
import {
  AlertTriangle,
  ArrowRight,
  BarChart3,
  Calendar,
  Lightbulb,
  PiggyBank,
  Plus,
  Search,
  ShieldAlert,
  Sparkles,
  Trash2,
  TrendingDown,
  TrendingUp,
  UserPlus,
  Users,
  Wallet,
} from "lucide-react";
import Link from "next/link";
import { AppShell } from "@/components/layout/app-shell";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { useToast } from "@/components/ui/toast";
import {
  accounts,
  budgets,
  reports,
  smart,
  transactions,
  type Account,
  type Anomaly,
  type Budget as BudgetRow,
  type BudgetSummary,
  type MonthlyReport,
  type OptimizationReport,
  type OptimizationSuggestion,
  type SpendingReport,
  type Txn,
} from "@/lib/api";
import { FadeIn, Stagger, StaggerItem } from "@/components/motion/aurora";
import { CurrencyCountUp } from "@/components/motion/count-up";
import { fenToCny, formatDate, formatDateTime } from "@/lib/utils";
import { cn } from "@/lib/utils";
import { useAccountsBusTick } from "@/lib/store";
import { useT } from "@/i18n/LanguageProvider";
import {
  Cell,
  Pie,
  PieChart,
  ResponsiveContainer,
  Tooltip,
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
} from "recharts";

const CATEGORIES = [
  "all",
  "food",
  "transport",
  "shopping",
  "subscription",
  "transfer",
  "income",
  "wealth",
] as const;

const CATEGORY_COLORS: Record<string, string> = {
  food: "#f97316",
  transport: "#22d3ee",
  shopping: "#a78bfa",
  subscription: "#34d399",
  transfer: "#94a3b8",
  income: "#22c55e",
  wealth: "#f59e0b",
  other: "#64748b",
};

export default function AccountsPage() {
  return (
    <AppShell>
      <Body />
    </AppShell>
  );
}

function Body() {
  const [list, setList] = useState<Account[]>([]);
  const [selected, setSelected] = useState<string | null>(null);
  const [txns, setTxns] = useState<Txn[]>([]);
  const [category, setCategory] = useState<(typeof CATEGORIES)[number]>("all");

  // Refresh whenever the cross-page bus ticks (after a transfer / wealth /
  // subscription / savings action). Also runs once on mount.
  const accountsTick = useAccountsBusTick();
  useEffect(() => {
    accounts.list().then((a) => {
      setList(a);
      if (a.length) setSelected(a[0].id);
    });
  }, [accountsTick]);

  useEffect(() => {
    if (!selected) return;
    transactions
      .list({ account_id: selected, limit: 200 })
      .then((t) => setTxns(t))
      .catch(() => setTxns([]));
  }, [selected]);

  const filtered = useMemo(() => {
    if (category === "all") return txns;
    return txns.filter((t) => t.category === category);
  }, [txns, category]);

  return (
    <div className="space-y-10">
      <FadeIn>
        <div>
          <p className="text-[11px] font-medium uppercase tracking-widest text-muted-foreground">
            Ledger
          </p>
          <h1 className="mt-1 font-display text-3xl font-semibold tracking-tight">Accounts</h1>
          <p className="mt-1 max-w-2xl text-sm text-muted-foreground">
            Browse your transactions, see how your money flows, and get tailored suggestions to
            spend smarter and save more.
          </p>
        </div>
      </FadeIn>

      {/* --- Ledger ------------------------------------------------------ */}
      <section className="space-y-4">
        <SectionTitle icon={<BarChart3 className="size-4" />} title="Transactions" subtitle="Pick an account to drill in" />
        <div className="grid gap-4 lg:grid-cols-[280px_1fr]">
          <Stagger>
            <div className="space-y-3">
              {list.map((a) => (
                <StaggerItem key={a.id}>
                  <motion.button
                    whileHover={{ y: -2 }}
                    transition={{ type: "spring", damping: 18, stiffness: 200 }}
                    onClick={() => setSelected(a.id)}
                    className={cn(
                      "block w-full rounded-xl border bg-card/60 p-4 text-left transition-colors",
                      selected === a.id && "ring-1 ring-primary"
                    )}
                  >
                    <div className="text-[10px] uppercase tracking-widest text-muted-foreground">
                      {a.masked_number}
                    </div>
                    <div className="mt-1 font-display text-lg font-semibold tracking-tight">
                      {a.display_name}
                    </div>
                    <div className="mt-2 text-2xl font-semibold">
                      <CurrencyCountUp fen={a.available_balance_fen} />
                    </div>
                    {a.reserved_balance_fen > 0 && (
                      <div className="mt-1 text-[10px] uppercase tracking-wider text-muted-foreground">
                        {fenToCny(a.reserved_balance_fen)} reserved
                      </div>
                    )}
                  </motion.button>
                </StaggerItem>
              ))}
            </div>
          </Stagger>

          <FadeIn delay={0.1}>
            <Card>
              <CardHeader>
                <div className="flex flex-wrap items-end justify-between gap-3">
                  <div>
                    <CardTitle>Transactions</CardTitle>
                    <CardDescription>
                      Newest first · {filtered.length} of {txns.length}
                    </CardDescription>
                  </div>
                  <div className="flex flex-wrap gap-1">
                    {CATEGORIES.map((c) => (
                      <button
                        key={c}
                        onClick={() => setCategory(c)}
                        className={cn(
                          "rounded-full border px-3 py-1 text-xs transition-colors",
                          category === c
                            ? "border-foreground/40 bg-foreground/5 text-foreground"
                            : "border-input bg-background/40 text-muted-foreground hover:bg-accent hover:text-foreground"
                        )}
                      >
                        {c}
                      </button>
                    ))}
                  </div>
                </div>
              </CardHeader>
              <CardContent>
                {filtered.length === 0 ? (
                  <div className="grid h-40 place-items-center text-sm text-muted-foreground">
                    No transactions.
                  </div>
                ) : (
                  <ul className="divide-y">
                    {filtered.map((t) => (
                      <motion.li
                        key={t.id}
                        layout
                        initial={{ opacity: 0, y: 4 }}
                        animate={{ opacity: 1, y: 0 }}
                        className="flex items-center justify-between gap-4 py-3"
                      >
                        <div className="flex min-w-0 items-center gap-3">
                          <span
                            className={cn(
                              "grid size-9 place-items-center rounded-lg text-xs font-semibold uppercase",
                              t.direction === "credit"
                                ? "bg-tier-green/15 text-tier-green"
                                : "bg-muted text-foreground/80"
                            )}
                          >
                            {t.category.slice(0, 2)}
                          </span>
                          <div className="min-w-0">
                            <div className="truncate text-sm font-medium">{t.merchant}</div>
                            <div className="truncate text-[11px] text-muted-foreground">
                              {formatDateTime(t.occurred_at)} · {t.category}
                              {t.memo && ` · "${t.memo}"`}
                            </div>
                          </div>
                        </div>
                        <div className="text-right">
                          <div
                            className={cn(
                              "text-sm font-semibold",
                              t.direction === "credit" ? "text-tier-green" : "text-foreground"
                            )}
                          >
                            {t.direction === "credit" ? "+" : "−"}
                            {fenToCny(Math.abs(t.amount_fen))}
                          </div>
                          <Badge variant={t.status === "posted" ? "muted" : "yellow"}>{t.status}</Badge>
                        </div>
                      </motion.li>
                    ))}
                  </ul>
                )}
              </CardContent>
            </Card>
          </FadeIn>
        </div>
      </section>

      {/* --- Bill analysis ----------------------------------------------- */}
      <BillAnalysis />

      {/* --- Beneficial accounts directory ------------------------------ */}
      <BeneficialAccountsSection />

      {/* --- Cost management: budgets --------------------------------- */}
      <BudgetsSection />

      {/* --- Reduce cost & optimize ------------------------------------- */}
      <Optimize />
    </div>
  );
}

// --- Sub-components ----------------------------------------------------

function SectionTitle({
  icon,
  title,
  subtitle,
}: {
  icon: React.ReactNode;
  title: string;
  subtitle?: string;
}) {
  return (
    <FadeIn>
      <div className="flex items-center gap-2">
        <span className="grid size-7 place-items-center rounded-lg bg-primary/10 text-primary ring-1 ring-primary/20">
          {icon}
        </span>
        <div>
          <h2 className="font-display text-lg font-semibold tracking-tight">{title}</h2>
          {subtitle && <p className="text-xs text-muted-foreground">{subtitle}</p>}
        </div>
      </div>
    </FadeIn>
  );
}

function BillAnalysis() {
  const [spending, setSpending] = useState<SpendingReport | null>(null);
  const [anomalies, setAnomalies] = useState<Anomaly[] | null>(null);
  const [monthly, setMonthly] = useState<MonthlyReport | null>(null);
  const [accts, setAccts] = useState<Account[]>([]);
  const [recentTxns, setRecentTxns] = useState<Txn[]>([]);
  const [loading, setLoading] = useState(true);

  // Re-fetch when any money-movement action happens elsewhere.
  const accountsTick = useAccountsBusTick();
  useEffect(() => {
    const now = new Date();
    Promise.all([
      reports.spending().catch(() => null),
      reports.anomalies().catch(() => [] as Anomaly[]),
      reports.monthly(now.getUTCFullYear(), now.getUTCMonth() + 1).catch(() => null),
      accounts.list().catch(() => [] as Account[]),
      // 500 most-recent txns across all accounts — enough to colour the per-account bars.
      transactions.list({ limit: 500 }).catch(() => [] as Txn[]),
    ]).then(([s, a, m, ac, tx]) => {
      setSpending(s);
      setAnomalies(a);
      setMonthly(m);
      setAccts(ac);
      setRecentTxns(tx);
      setLoading(false);
    });
  }, [accountsTick]);

  const pieData = useMemo(() => {
    if (!spending) return [];
    return spending.items.map((i) => ({
      name: i.category,
      value: i.total_fen,
      share: i.share_pct,
    }));
  }, [spending]);

  const totalMonthly = monthly?.expense_fen ?? 0;
  const totalIncome = monthly?.income_fen ?? 0;
  const savingsRate =
    totalIncome > 0 ? Math.max(0, Math.min(1, (totalIncome - totalMonthly) / totalIncome)) : 0;
  const momDelta = monthly?.mom_delta_fen ?? 0;
  const topCategories = (monthly?.by_category ?? []).slice(0, 3);

  return (
    <section className="space-y-4">
      <SectionTitle
        icon={<BarChart3 className="size-4" />}
        title="Bill analysis"
        subtitle="Spending categories · anomaly detection · monthly report"
      />

      <div className="grid gap-4 lg:grid-cols-3">
        {/* Categories pie */}
        <FadeIn delay={0.05}>
          <Card className="h-full">
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <PiggyBank className="size-4 text-primary" />
                Spending by category
              </CardTitle>
              <CardDescription>Last 30 days · all accounts</CardDescription>
            </CardHeader>
            <CardContent>
              {loading ? (
                <div className="grid h-56 place-items-center text-xs text-muted-foreground">Loading…</div>
              ) : pieData.length === 0 ? (
                <div className="grid h-56 place-items-center text-xs text-muted-foreground">No data yet.</div>
              ) : (
                <div className="grid h-56 grid-cols-1 items-center gap-3 sm:grid-cols-[1fr_1.1fr]">
                  <div className="h-full w-full">
                    <ResponsiveContainer width="100%" height="100%">
                      <PieChart>
                        <Tooltip
                          contentStyle={{
                            background: "rgba(15,23,42,0.92)",
                            border: "1px solid rgba(255,255,255,0.1)",
                            borderRadius: 8,
                            fontSize: 12,
                          }}
                          formatter={(v) => fenToCny(Number(v ?? 0))}
                        />
                        <Pie
                          data={pieData}
                          dataKey="value"
                          nameKey="name"
                          innerRadius={48}
                          outerRadius={80}
                          paddingAngle={2}
                          strokeWidth={0}
                        >
                          {pieData.map((d) => (
                            <Cell
                              key={d.name}
                              fill={CATEGORY_COLORS[d.name] ?? CATEGORY_COLORS.other}
                            />
                          ))}
                        </Pie>
                      </PieChart>
                    </ResponsiveContainer>
                  </div>
                  <ul className="space-y-1.5 text-xs">
                    {pieData.map((d) => (
                      <li key={d.name} className="flex items-center justify-between gap-2">
                        <span className="flex items-center gap-2">
                          <span
                            className="inline-block size-2.5 rounded-sm"
                            style={{ background: CATEGORY_COLORS[d.name] ?? CATEGORY_COLORS.other }}
                          />
                          <span className="text-foreground/80">{d.name}</span>
                        </span>
                        <span className="font-mono text-foreground/70">
                          {fenToCny(d.value)} · {d.share.toFixed(1)}%
                        </span>
                      </li>
                    ))}
                  </ul>
                </div>
              )}
            </CardContent>
          </Card>
        </FadeIn>

        {/* Monthly report */}
        <FadeIn delay={0.1}>
          <Card className="h-full">
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Calendar className="size-4 text-primary" />
                This month
              </CardTitle>
              <CardDescription>
                {monthly
                  ? `${new Date(monthly.year, monthly.month - 1, 1).toLocaleDateString(undefined, {
                      month: "long",
                      year: "numeric",
                    })}`
                  : "—"}
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              {loading ? (
                <div className="grid h-40 place-items-center text-xs text-muted-foreground">Loading…</div>
              ) : !monthly ? (
                <div className="grid h-40 place-items-center text-xs text-muted-foreground">No data.</div>
              ) : (
                <>
                  <div className="grid grid-cols-2 gap-3 text-sm">
                    <KpiMini label="Income" tone="up" fen={totalIncome} />
                    <KpiMini label="Expense" tone="down" fen={totalMonthly} />
                    <KpiMini label="Net" tone={totalIncome - totalMonthly >= 0 ? "up" : "down"} fen={totalIncome - totalMonthly} />
                    <KpiMini
                      label="vs. last"
                      tone={momDelta <= 0 ? "up" : "down"}
                      fen={Math.abs(momDelta)}
                      prefix={momDelta <= 0 ? "−" : "+"}
                      suffix=" less"
                    />
                  </div>
                  <div>
                    <div className="flex items-center justify-between text-xs">
                      <span className="text-muted-foreground">Savings rate</span>
                      <span className="font-mono text-foreground/80">
                        {(savingsRate * 100).toFixed(1)}%
                      </span>
                    </div>
                    <div className="mt-1 h-1.5 w-full overflow-hidden rounded-full bg-muted">
                      <div
                        className="h-full rounded-full bg-tier-green transition-all"
                        style={{ width: `${(savingsRate * 100).toFixed(1)}%` }}
                      />
                    </div>
                  </div>
                  {topCategories.length > 0 && (
                    <div>
                      <p className="text-[10px] uppercase tracking-widest text-muted-foreground">
                        Top categories
                      </p>
                      <ul className="mt-2 space-y-1.5 text-xs">
                        {topCategories.map((c) => (
                          <li key={c.category} className="flex items-center justify-between">
                            <span className="text-foreground/80">{c.category}</span>
                            <span className="font-mono text-foreground/70">
                              {fenToCny(c.total_fen)}
                            </span>
                          </li>
                        ))}
                      </ul>
                    </div>
                  )}
                </>
              )}
            </CardContent>
          </Card>
        </FadeIn>

        {/* Anomalies */}
        <FadeIn delay={0.15}>
          <Card className="h-full">
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <ShieldAlert className="size-4 text-tier-yellow" />
                Unusual transactions
              </CardTitle>
              <CardDescription>Things that look different from your normal pattern</CardDescription>
            </CardHeader>
            <CardContent>
              {loading ? (
                <div className="grid h-40 place-items-center text-xs text-muted-foreground">Loading…</div>
              ) : !anomalies || anomalies.length === 0 ? (
                <div className="grid h-40 place-items-center text-sm text-muted-foreground">
                  <div className="text-center">
                    <Sparkles className="mx-auto mb-2 size-5 text-tier-green" />
                    Nothing unusual in your recent activity.
                  </div>
                </div>
              ) : (
                <ul className="space-y-2">
                  {anomalies.slice(0, 6).map((a) => (
                    <li
                      key={a.transaction_id}
                      className="flex items-start gap-3 rounded-lg border border-tier-yellow/20 bg-tier-yellow/5 p-2.5 text-xs"
                    >
                      <AlertTriangle className="mt-0.5 size-3.5 shrink-0 text-tier-yellow" />
                      <div className="min-w-0 flex-1">
                        <div className="flex items-center justify-between gap-2">
                          <span className="truncate font-medium">{a.merchant}</span>
                          <span className="shrink-0 font-mono text-foreground/80">
                            {fenToCny(a.amount_fen)}
                          </span>
                        </div>
                        <div className="mt-0.5 flex items-center justify-between gap-2 text-[10px] text-muted-foreground">
                          <span>{formatDate(a.occurred_at)}</span>
                          <span>baseline {fenToCny(a.baseline_median_fen)}</span>
                        </div>
                        {a.reasons && a.reasons.length > 0 && (
                          <p className="mt-1 text-[10px] text-muted-foreground">
                            {a.reasons.join(" · ")}
                          </p>
                        )}
                      </div>
                    </li>
                  ))}
                </ul>
              )}
            </CardContent>
          </Card>
        </FadeIn>
      </div>

      {/* Bar chart for the last 6 months — light read; uses monthly API
          if the backend exposes it, otherwise we fall back to a single bar. */}
      {monthly && (
        <FadeIn delay={0.2}>
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <TrendingUp className="size-4 text-tier-green" />
                Income vs. expense
              </CardTitle>
              <CardDescription>This month so far · live totals</CardDescription>
            </CardHeader>
            <CardContent>
              <div className="h-44 w-full">
                <ResponsiveContainer width="100%" height="100%">
                  <BarChart
                    data={[
                      { name: "Income", fen: totalIncome, fill: "#22c55e" },
                      { name: "Expense", fen: totalMonthly, fill: "#f97316" },
                      { name: "Net", fen: totalIncome - totalMonthly, fill: "#22d3ee" },
                    ]}
                    margin={{ top: 8, right: 16, left: 8, bottom: 0 }}
                  >
                    <CartesianGrid stroke="rgba(255,255,255,0.06)" strokeDasharray="3 3" />
                    <XAxis dataKey="name" stroke="rgba(255,255,255,0.4)" fontSize={11} />
                    <YAxis
                      stroke="rgba(255,255,255,0.4)"
                      fontSize={11}
                      tickFormatter={(v) => `¥${(Number(v ?? 0) / 1000).toFixed(1)}k`}
                    />
                    <Tooltip
                      contentStyle={{
                        background: "rgba(15,23,42,0.92)",
                        border: "1px solid rgba(255,255,255,0.1)",
                        borderRadius: 8,
                        fontSize: 12,
                      }}
                      formatter={(v) => fenToCny(Number(v ?? 0))}
                    />
                    <Bar dataKey="fen" radius={[6, 6, 0, 0]}>
                      {[
                        { name: "Income", fen: totalIncome, fill: "#22c55e" },
                        { name: "Expense", fen: totalMonthly, fill: "#f97316" },
                        { name: "Net", fen: totalIncome - totalMonthly, fill: "#22d3ee" },
                      ].map((d) => (
                        <Cell key={d.name} fill={d.fill} />
                      ))}
                    </Bar>
                  </BarChart>
                </ResponsiveContainer>
              </div>
            </CardContent>
          </Card>
        </FadeIn>
      )}

      {/* Real-money breakdown: which account eats the most, and how long the
          current balance lasts at the current burn rate. */}
      <ByAccountAndRunway
        accts={accts}
        txns={recentTxns}
        monthlyExpenseFen={totalMonthly}
        monthlyIncomeFen={totalIncome}
        loading={loading}
      />
    </section>
  );
}

/**
 * Derive per-account spend from the recent-transactions window the parent
 * fetched. Positive-amount credit transactions are ignored; debits are
 * summed. Reserves (savings-goal holds) are kept separate.
 */
function ByAccountAndRunway({
  accts,
  txns,
  monthlyExpenseFen,
  monthlyIncomeFen,
  loading,
}: {
  accts: Account[];
  txns: Txn[];
  monthlyExpenseFen: number;
  monthlyIncomeFen: number;
  loading: boolean;
}) {
  // Group debits by account_id over the last 30 days.
  const since = Date.now() - 30 * 24 * 60 * 60 * 1000;
  const perAccount = new Map<string, number>();
  for (const t of txns) {
    if (t.direction !== "debit") continue;
    if (new Date(t.occurred_at).getTime() < since) continue;
    perAccount.set(
      t.account_id,
      (perAccount.get(t.account_id) ?? 0) + Math.abs(t.amount_fen),
    );
  }
  const rows = accts
    .map((a) => {
      const spend = perAccount.get(a.id) ?? 0;
      const total = a.available_balance_fen + a.reserved_balance_fen;
      return {
        id: a.id,
        name: a.display_name,
        masked: a.masked_number,
        balance: a.available_balance_fen,
        reserved: a.reserved_balance_fen,
        total,
        spend,
        // Months of runway at this account's recent spend share.
        runwayMonths:
          spend > 0 ? a.available_balance_fen / (spend / 30) : Infinity,
      };
    })
    .sort((a, b) => b.spend - a.spend);

  const totalSpend = rows.reduce((s, r) => s + r.spend, 0);
  const totalBalance = accts.reduce(
    (s, a) => s + a.available_balance_fen,
    0,
  );
  const totalReserved = accts.reduce(
    (s, a) => s + a.reserved_balance_fen,
    0,
  );
  // Burn rate: prefer the live monthly figure; fall back to the last-30-days
  // computed spend if the monthly report isn't loaded yet.
  const burnFenPerDay =
    monthlyExpenseFen > 0
      ? monthlyExpenseFen / 30
      : totalSpend / 30;
  const runwayMonths =
    burnFenPerDay > 0 ? totalBalance / (burnFenPerDay * 30) : Infinity;
  const runwayLabel =
    !isFinite(runwayMonths) || burnFenPerDay === 0
      ? "—"
      : runwayMonths >= 120
      ? "120+ mo"
      : `${runwayMonths.toFixed(1)} mo`;
  const net = monthlyIncomeFen - monthlyExpenseFen;
  const netPositive = net >= 0;

  return (
    <FadeIn delay={0.25}>
      <div className="grid gap-4 lg:grid-cols-3">
        {/* Spending by account */}
        <Card className="lg:col-span-2">
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <BarChart3 className="size-4" />
              Spending by account
            </CardTitle>
            <CardDescription>
              Last 30 days · debits only · share of your outflows
            </CardDescription>
          </CardHeader>
          <CardContent>
            {loading ? (
              <div className="grid h-40 place-items-center text-xs text-muted-foreground">
                Loading…
              </div>
            ) : rows.length === 0 ? (
              <div className="grid h-40 place-items-center text-xs text-muted-foreground">
                No accounts yet.
              </div>
            ) : (
              <ul className="space-y-2.5">
                {rows.map((r) => {
                  const share =
                    totalSpend > 0 ? (r.spend / totalSpend) * 100 : 0;
                  return (
                    <li key={r.id} className="space-y-1.5">
                      <div className="flex items-center justify-between gap-3 text-xs">
                        <div className="min-w-0 flex-1">
                          <span className="truncate font-medium text-foreground/90">
                            {r.name}
                          </span>
                          <span className="ml-2 text-[10px] text-muted-foreground">
                            {r.masked}
                          </span>
                        </div>
                        <div className="shrink-0 text-right">
                          <span className="font-mono text-foreground/80">
                            {fenToCny(r.spend)}
                          </span>
                          <span className="ml-2 text-[10px] text-muted-foreground">
                            {share.toFixed(1)}%
                          </span>
                        </div>
                      </div>
                      <div className="h-1.5 w-full overflow-hidden rounded-full bg-white/5">
                        <div
                          className="h-full rounded-full bg-gradient-to-r from-primary/70 to-cyan-400/70"
                          style={{ width: `${Math.min(100, share).toFixed(2)}%` }}
                        />
                      </div>
                    </li>
                  );
                })}
              </ul>
            )}
          </CardContent>
        </Card>

        {/* Balance runway + total balance */}
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <PiggyBank className="size-4 text-tier-green" />
              Balance & runway
            </CardTitle>
            <CardDescription>
              {burnFenPerDay > 0
                ? `At ${fenToCny(burnFenPerDay)}/day burn rate`
                : "No spend yet"}
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-3">
            <div>
              <p className="text-[10px] uppercase tracking-widest text-muted-foreground">
                Total available
              </p>
              <p className="font-display text-2xl font-semibold tracking-tight">
                {fenToCny(totalBalance)}
              </p>
              {totalReserved > 0 && (
                <p className="text-[10px] text-muted-foreground">
                  + {fenToCny(totalReserved)} reserved in goals
                </p>
              )}
            </div>
            <div className="grid grid-cols-2 gap-3">
              <div>
                <p className="text-[10px] uppercase tracking-widest text-muted-foreground">
                  Runway
                </p>
                <p
                  className={cn(
                    "font-display text-xl font-semibold tracking-tight",
                    !isFinite(runwayMonths) || runwayMonths >= 6
                      ? "text-tier-green"
                      : runwayMonths >= 2
                      ? "text-tier-yellow"
                      : "text-tier-red",
                  )}
                >
                  {runwayLabel}
                </p>
              </div>
              <div>
                <p className="text-[10px] uppercase tracking-widest text-muted-foreground">
                  Net this month
                </p>
                <p
                  className={cn(
                    "font-display text-xl font-semibold tracking-tight",
                    netPositive ? "text-tier-green" : "text-tier-red",
                  )}
                >
                  {netPositive ? "+" : ""}
                  {fenToCny(net)}
                </p>
              </div>
            </div>
            <p className="text-[10px] leading-relaxed text-muted-foreground">
              Real-time · transfers cut the available balance as soon as they
              post, so this number drops the moment you confirm a payment.
            </p>
          </CardContent>
        </Card>
      </div>
    </FadeIn>
  );
}

function Optimize() {
  const [report, setReport] = useState<OptimizationReport | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    reports
      .optimization()
      .then((r) => {
        setReport(r);
        setLoading(false);
      })
      .catch(() => setLoading(false));
  }, []);

  return (
    <section className="space-y-4">
      <SectionTitle
        icon={<Lightbulb className="size-4" />}
        title="Reduce cost & optimize"
        subtitle="Personalised actions to keep more of what you earn"
      />

      {/* Headline stat strip */}
      <FadeIn delay={0.05}>
        <Card>
          <CardContent className="grid gap-4 py-5 sm:grid-cols-3">
            <HeadlineStat
              icon={<PiggyBank className="size-4 text-tier-green" />}
              label="Potential monthly savings"
              fen={report?.total_monthly_savings_fen ?? 0}
              tone="up"
            />
            <HeadlineStat
              icon={<TrendingUp className="size-4 text-primary" />}
              label="Potential annual savings"
              fen={report?.total_annual_savings_fen ?? 0}
              tone="up"
            />
            <HeadlineStat
              icon={<Sparkles className="size-4 text-tier-yellow" />}
              label="Active subscriptions"
              value={String(report?.subscription_count ?? 0)}
              tone="muted"
            />
          </CardContent>
        </Card>
      </FadeIn>

      {loading ? (
        <Card>
          <CardContent className="grid h-40 place-items-center text-sm text-muted-foreground">
            Crunching the numbers…
          </CardContent>
        </Card>
      ) : !report || report.suggestions.length === 0 ? (
        <Card>
          <CardContent className="grid h-40 place-items-center text-sm text-muted-foreground">
            <div className="text-center">
              <Sparkles className="mx-auto mb-2 size-5 text-tier-green" />
              No suggestions right now — your spending looks healthy.
            </div>
          </CardContent>
        </Card>
      ) : (
        <div className="grid gap-3 md:grid-cols-2">
          {report.suggestions.map((s, i) => (
            <StaggerItem key={`${s.code}-${i}`}>
              <SuggestionCard s={s} />
            </StaggerItem>
          ))}
        </div>
      )}
    </section>
  );
}

function SuggestionCard({ s }: { s: OptimizationSuggestion }) {
  const tone =
    s.severity === "critical"
      ? "red"
      : s.severity === "warning"
      ? "yellow"
      : "muted";
  const icon =
    s.code === "low_balance_risk" ? (
      <ShieldAlert className="size-4 text-tier-red" />
    ) : s.code === "unused_subscription" ? (
      <TrendingDown className="size-4 text-tier-yellow" />
    ) : s.code === "subscription_bloat" ? (
      <PiggyBank className="size-4 text-tier-yellow" />
    ) : s.code === "category_overspend" ? (
      <BarChart3 className="size-4 text-primary" />
    ) : s.code === "transfer_fragmentation" ? (
      <ArrowRight className="size-4 text-primary" />
    ) : s.code === "budget_breached" ? (
      <AlertTriangle className="size-4 text-tier-red" />
    ) : s.code === "budget_near_limit" ? (
      <TrendingUp className="size-4 text-tier-yellow" />
    ) : (
      <Lightbulb className="size-4 text-primary" />
    );
  return (
    <motion.div whileHover={{ y: -2 }} transition={{ type: "spring", damping: 18, stiffness: 200 }}>
      <Card className="h-full">
        <CardHeader>
          <div className="flex items-start justify-between gap-3">
            <div className="flex items-start gap-2">
              <span className="mt-0.5 grid size-7 place-items-center rounded-lg bg-primary/10 text-primary ring-1 ring-primary/20">
                {icon}
              </span>
              <div className="min-w-0">
                <CardTitle className="text-sm">{s.title}</CardTitle>
              </div>
            </div>
            <Badge variant={tone as "muted" | "yellow" | "red"}>
              {s.monthly_savings_fen > 0
                ? `Save ${fenToCny(s.monthly_savings_fen)}/mo`
                : s.severity}
            </Badge>
          </div>
        </CardHeader>
        <CardContent className="space-y-3">
          <p className="text-sm text-muted-foreground">{s.body}</p>
          {s.annual_savings_fen > 0 && (
            <p className="text-[11px] text-muted-foreground">
              Annualised: {fenToCny(s.annual_savings_fen)}
            </p>
          )}
          {s.action_endpoint ? (
            <Link href={s.action_endpoint}>
              <Button size="sm" variant="outline" className="gap-1">
                {s.action_label}
                <ArrowRight className="size-3.5" />
              </Button>
            </Link>
          ) : (
            <Button size="sm" variant="outline" disabled>
              {s.action_label}
            </Button>
          )}
        </CardContent>
      </Card>
    </motion.div>
  );
}

function KpiMini({
  label,
  fen,
  tone,
  prefix,
  suffix,
}: {
  label: string;
  fen: number;
  tone: "up" | "down" | "muted";
  prefix?: string;
  suffix?: string;
}) {
  const colour =
    tone === "up"
      ? "text-tier-green"
      : tone === "down"
      ? "text-tier-red"
      : "text-foreground";
  return (
    <div className="rounded-lg border border-white/5 bg-white/[0.02] p-2.5">
      <p className="text-[10px] uppercase tracking-widest text-muted-foreground">{label}</p>
      <p className={cn("mt-1 font-display text-lg font-semibold", colour)}>
        {prefix ?? ""}
        {fenToCny(fen)}
        {suffix ?? ""}
      </p>
    </div>
  );
}

function HeadlineStat({
  icon,
  label,
  fen,
  value,
  tone,
}: {
  icon: React.ReactNode;
  label: string;
  fen?: number;
  value?: string;
  tone: "up" | "down" | "muted";
}) {
  const colour =
    tone === "up"
      ? "text-tier-green"
      : tone === "down"
      ? "text-tier-red"
      : "text-foreground";
  return (
    <div className="flex items-center gap-3">
      <span className="grid size-9 place-items-center rounded-lg bg-primary/10 text-primary ring-1 ring-primary/20">
        {icon}
      </span>
      <div>
        <p className="text-[10px] uppercase tracking-widest text-muted-foreground">{label}</p>
        <p className={cn("font-display text-2xl font-semibold tracking-tight", colour)}>
          {value ?? (fen !== undefined ? fenToCny(fen) : "—")}
        </p>
      </div>
    </div>
  );
}


// --- Beneficial accounts directory -------------------------------------
//
// Renders the user's persistent list of who-they-pay and gives them a form
// to add a new one. The list is the same `payees` table the smart-transfer
// resolver reads, so adding here makes the new payee immediately available
// when the user says "Pay Li Wei" in chat or on /transfers.

type BeneficialRow = {
  id: string;
  display_name: string;
  phone_tail: string | null;
  bank_hint: string | null;
  destination: string;
  verified: boolean;
  first_used_at: string | null;
  created_at: string;
};

function BeneficialAccountsSection() {
  const t = useT();
  const accountsTick = useAccountsBusTick();
  const { toast } = useToast();
  const [rows, setRows] = useState<BeneficialRow[]>([]);
  const [search, setSearch] = useState("");
  const [addOpen, setAddOpen] = useState(false);
  const [name, setName] = useState("");
  const [phone, setPhone] = useState("");
  const [bank, setBank] = useState("");
  const [note, setNote] = useState("");
  const [busy, setBusy] = useState(false);

  const refresh = () => {
    smart.beneficialAccounts
      .list(search.trim() || undefined)
      .then((r) => setRows(r as BeneficialRow[]))
      .catch(() => setRows([]));
  };
  useEffect(() => {
    const id = setTimeout(refresh, 200);
    return () => clearTimeout(id);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [search, accountsTick]);

  const submit = async () => {
    if (!name.trim()) {
      toast({
        variant: "error",
        title: t("beneficial.form.title"),
        description: t("beneficial.form.name"),
      });
      return;
    }
    setBusy(true);
    try {
      await smart.beneficialAccounts.create({
        display_name: name.trim(),
        phone_tail: phone.trim() || undefined,
        bank_hint: bank.trim() || undefined,
        note: note.trim() || undefined,
      });
      toast({ variant: "success", title: t("beneficial.form.added") });
      setName("");
      setPhone("");
      setBank("");
      setNote("");
      setAddOpen(false);
      refresh();
    } catch (e) {
      toast({
        variant: "error",
        title: t("beneficial.form.title"),
        description: e instanceof Error ? e.message : "Try again.",
      });
    } finally {
      setBusy(false);
    }
  };

  return (
    <section className="space-y-4">
      <SectionTitle
        icon={<Users className="size-4" />}
        title={t("beneficial.title")}
        subtitle={t("beneficial.eyebrow")}
      />
      <FadeIn>
        <Card>
          <CardHeader>
            <div className="flex flex-wrap items-end justify-between gap-3">
              <div>
                <CardDescription>{t("beneficial.subtitle")}</CardDescription>
              </div>
              <div className="flex items-center gap-2">
                <div className="relative">
                  <Search className="pointer-events-none absolute left-2.5 top-2.5 size-3.5 text-muted-foreground" />
                  <Input
                    value={search}
                    onChange={(e) => setSearch(e.target.value)}
                    placeholder={t("beneficial.searchPh")}
                    className="w-56 pl-8"
                  />
                </div>
                <Button onClick={() => setAddOpen((o) => !o)} className="gap-1.5">
                  <UserPlus className="size-4" />
                  {t("beneficial.add")}
                </Button>
              </div>
            </div>
            {addOpen && (
              <div className="mt-4 grid gap-3 rounded-xl border bg-background/40 p-4 md:grid-cols-2">
                <div className="md:col-span-1">
                  <Label>{t("beneficial.form.name")}</Label>
                  <Input
                    value={name}
                    onChange={(e) => setName(e.target.value)}
                    placeholder={t("beneficial.form.namePh")}
                  />
                </div>
                <div className="md:col-span-1">
                  <Label>{t("beneficial.form.phone")}</Label>
                  <Input
                    value={phone}
                    onChange={(e) => setPhone(e.target.value)}
                    placeholder={t("beneficial.form.phonePh")}
                    inputMode="numeric"
                    maxLength={6}
                  />
                </div>
                <div className="md:col-span-1">
                  <Label>{t("beneficial.form.bank")}</Label>
                  <Input
                    value={bank}
                    onChange={(e) => setBank(e.target.value)}
                    placeholder={t("beneficial.form.bankPh")}
                  />
                </div>
                <div className="md:col-span-1">
                  <Label>{t("beneficial.form.note")}</Label>
                  <Input
                    value={note}
                    onChange={(e) => setNote(e.target.value)}
                    placeholder={t("beneficial.form.notePh")}
                  />
                </div>
                <div className="md:col-span-2 flex items-center justify-end gap-2">
                  <Button
                    variant="outline"
                    onClick={() => setAddOpen(false)}
                    disabled={busy}
                  >
                    Cancel
                  </Button>
                  <Button onClick={submit} disabled={busy || !name.trim()}>
                    {busy
                      ? t("beneficial.form.submitting")
                      : t("beneficial.form.submit")}
                  </Button>
                </div>
              </div>
            )}
          </CardHeader>
          <CardContent>
            {rows.length === 0 ? (
              <div className="grid h-32 place-items-center text-sm text-muted-foreground">
                {t("beneficial.empty")}
              </div>
            ) : (
              <ul className="divide-y">
                {rows.map((r) => (
                  <motion.li
                    key={r.id}
                    layout
                    initial={{ opacity: 0, y: 4 }}
                    animate={{ opacity: 1, y: 0 }}
                    className="flex items-center justify-between gap-4 py-3"
                  >
                    <div className="flex min-w-0 items-center gap-3">
                      <span className="grid size-9 place-items-center rounded-lg bg-muted text-xs font-semibold uppercase">
                        {r.display_name.slice(0, 2)}
                      </span>
                      <div className="min-w-0">
                        <div className="truncate text-sm font-medium">
                          {r.display_name}
                        </div>
                        <div className="truncate text-[11px] text-muted-foreground">
                          {r.bank_hint ? `${r.bank_hint} · ` : ""}
                          {r.phone_tail ? `···${r.phone_tail} · ` : ""}
                          {r.first_used_at
                            ? t("beneficial.firstUsed", {
                                when: formatDateTime(r.first_used_at),
                              })
                            : t("beneficial.neverUsed")}
                        </div>
                      </div>
                    </div>
                    <Badge variant={r.verified ? "green" : "muted"}>
                      {r.verified
                        ? t("beneficial.verified")
                        : t("beneficial.unverified")}
                    </Badge>
                  </motion.li>
                ))}
              </ul>
            )}
          </CardContent>
        </Card>
      </FadeIn>
    </section>
  );
}

// --- Cost management: budgets -----------------------------------------
//
// This is the "workable like cost management" surface. It hits the
// /budgets list + summary endpoints, lets the user set / delete a per-
// category monthly cap, and re-fetches whenever the cross-page bus
// ticks (a transfer landing will move the spent bars in real time).

function BudgetsSection() {
  const t = useT();
  const accountsTick = useAccountsBusTick();
  const { toast } = useToast();
  const [list, setList] = useState<BudgetRow[]>([]);
  const [summary, setSummary] = useState<BudgetSummary | null>(null);
  const [accts, setAccts] = useState<Account[]>([]);
  const [category, setCategory] = useState("");
  const [amount, setAmount] = useState("");
  const [accountId, setAccountId] = useState<string>("");
  const [note, setNote] = useState("");
  const [saving, setSaving] = useState(false);

  const refresh = () => {
    budgets
      .list()
      .then((r) => setList(r.budgets ?? []))
      .catch(() => setList([]));
    budgets.summary().then(setSummary).catch(() => setSummary(null));
  };
  useEffect(() => {
    refresh();
    accounts.list().then((a) => setAccts(a)).catch(() => setAccts([]));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [accountsTick]);

  const save = async () => {
    if (!category.trim() || !amount.trim()) {
      toast({
        variant: "error",
        title: t("budgets.form.title"),
        description: "Category and amount are both required.",
      });
      return;
    }
    setSaving(true);
    try {
      await budgets.set({
        category: category.trim(),
        amount: Number(amount) || amount,
        account_id: accountId || undefined,
        note: note || undefined,
      });
      toast({ variant: "success", title: t("budgets.form.saved") });
      setCategory("");
      setAmount("");
      setNote("");
      setAccountId("");
      refresh();
    } catch (e) {
      toast({
        variant: "error",
        title: t("budgets.form.title"),
        description: e instanceof Error ? e.message : "Try again.",
      });
    } finally {
      setSaving(false);
    }
  };

  const remove = async (id: string) => {
    try {
      await budgets.remove(id);
      toast({ variant: "success", title: t("budgets.deleted") });
      refresh();
    } catch (e) {
      toast({
        variant: "error",
        title: t("budgets.delete"),
        description: e instanceof Error ? e.message : "Try again.",
      });
    }
  };

  return (
    <section className="space-y-4">
      <SectionTitle
        icon={<PiggyBank className="size-4" />}
        title={t("budgets.title")}
        subtitle={t("budgets.eyebrow")}
      />

      <FadeIn>
        <Card>
          <CardHeader>
            <div className="flex flex-wrap items-end justify-between gap-3">
              <div>
                <CardTitle>{t("budgets.summary.title")}</CardTitle>
                <CardDescription>{t("budgets.subtitle")}</CardDescription>
              </div>
              {summary ? (
                <div className="text-right">
                  <div className="text-2xl font-display font-semibold tracking-tight">
                    {summary.display.remaining}
                  </div>
                  <div className="text-[11px] text-muted-foreground">
                    {t("budgets.summary.remaining", {
                      amount: summary.display.remaining,
                      total: summary.display.limit,
                    })}
                  </div>
                </div>
              ) : null}
            </div>
          </CardHeader>
          <CardContent>
            <div className="grid gap-3 md:grid-cols-4">
              <div className="md:col-span-1">
                <Label>{t("budgets.form.category")}</Label>
                <Input
                  value={category}
                  onChange={(e) => setCategory(e.target.value)}
                  placeholder={t("budgets.form.categoryPh")}
                />
              </div>
              <div className="md:col-span-1">
                <Label>{t("budgets.form.amount")}</Label>
                <Input
                  value={amount}
                  onChange={(e) => setAmount(e.target.value)}
                  placeholder="3000"
                  inputMode="decimal"
                />
              </div>
              <div className="md:col-span-1">
                <Label>{t("budgets.form.account")}</Label>
                <select
                  className="h-9 w-full rounded-md border border-input bg-background px-2 text-sm"
                  value={accountId}
                  onChange={(e) => setAccountId(e.target.value)}
                >
                  <option value="">{t("budgets.form.account.all")}</option>
                  {accts.map((a) => (
                    <option key={a.id} value={a.id}>
                      {a.display_name} ···{a.masked_number}
                    </option>
                  ))}
                </select>
              </div>
              <div className="flex items-end">
                <Button
                  onClick={save}
                  disabled={saving || !category.trim() || !amount.trim()}
                  className="w-full"
                >
                  {saving ? t("budgets.form.submitting") : t("budgets.form.submit")}
                </Button>
              </div>
            </div>
          </CardContent>
        </Card>
      </FadeIn>

      <FadeIn delay={0.05}>
        <Card>
          <CardHeader>
            <CardTitle className="text-base">{t("budgets.eyebrow")}</CardTitle>
          </CardHeader>
          <CardContent>
            {list.length === 0 ? (
              <div className="grid h-32 place-items-center text-sm text-muted-foreground">
                {t("budgets.empty")}
              </div>
            ) : (
              <ul className="grid gap-3 md:grid-cols-2">
                {list.map((b) => {
                  const pct = Math.min(100, Math.round(b.utilization_pct));
                  const over = b.breached;
                  const barColor = over
                    ? "bg-rose-500"
                    : pct >= 80
                      ? "bg-amber-500"
                      : "bg-emerald-500";
                  return (
                    <motion.li
                      key={b.id}
                      layout
                      initial={{ opacity: 0, y: 4 }}
                      animate={{ opacity: 1, y: 0 }}
                      className={cn(
                        "rounded-xl border bg-card/60 p-4",
                        over && "border-rose-500/40",
                      )}
                    >
                      <div className="flex items-center justify-between gap-2">
                        <div className="min-w-0">
                          <div className="truncate text-sm font-semibold">
                            {b.category}
                          </div>
                          <div className="text-[11px] text-muted-foreground">
                            {t("budgets.spent", {
                              spent: fenToCny(b.spent_fen),
                              limit: fenToCny(b.limit_fen),
                            })}
                          </div>
                        </div>
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() => remove(b.id)}
                          className="text-muted-foreground hover:text-rose-500"
                        >
                          <Trash2 className="size-3.5" />
                        </Button>
                      </div>
                      <div className="mt-3 h-2 w-full overflow-hidden rounded-full bg-muted">
                        <div
                          className={cn("h-full rounded-full", barColor)}
                          style={{ width: `${pct}%` }}
                        />
                      </div>
                      <div className="mt-1.5 flex items-center justify-between text-[11px]">
                        <span className="text-muted-foreground">
                          {t("budgets.utilization", { pct })}
                        </span>
                        <span
                          className={cn(
                            over ? "text-rose-500" : "text-muted-foreground",
                          )}
                        >
                          {over
                            ? t("budgets.breached", {
                                amount: fenToCny(b.spent_fen - b.limit_fen),
                              })
                            : t("budgets.remaining", {
                                amount: fenToCny(b.remaining_fen),
                              })}
                        </span>
                      </div>
                    </motion.li>
                  );
                })}
              </ul>
            )}
          </CardContent>
        </Card>
      </FadeIn>
    </section>
  );
}
