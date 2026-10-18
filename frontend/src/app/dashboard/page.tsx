"use client";

import { useEffect, useState } from "react";
import { motion } from "framer-motion";
import {
  TrendingUp,
  TrendingDown,
  ArrowUpRight,
  PiggyBank,
  CreditCard,
  AlertTriangle,
  Sparkles,
  ShieldCheck,
  Play,
  BarChart3,
  ShieldHalf,
  Compass,
} from "lucide-react";
import { AppShell } from "@/components/layout/app-shell";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { accounts, reports, subscriptions, insights, type Account, type SpendingReport, type Anomaly, type Subscription, type DailyDigest } from "@/lib/api";
import { CurrencyCountUp } from "@/components/motion/count-up";
import { FadeIn, Stagger, StaggerItem, AuroraBackdrop, MountainHero, PortfolioLine } from "@/components/motion/aurora";
import { Link005 } from "@/components/ui/skiper-ui/skiper40";
import { useLanguage, useT } from "@/i18n/LanguageProvider";
import { usePageTitle } from "@/lib/use-page-title";
import { QuickstartBanner } from "@/components/feedback/quickstart-banner";
import { cn, fenToCny, formatDate, relativeTime } from "@/lib/utils";
import { useAccountsBusTick } from "@/lib/store";
import {
  PieChart,
  Pie,
  Cell,
  ResponsiveContainer,
  Tooltip,
} from "recharts";

const TIER_COLORS = ["#7c3aed", "#22d3ee", "#34d399", "#f59e0b", "#f87171", "#94a3b8"];

export default function DashboardPage() {
  return (
    <AppShell>
      <DashboardBody />
    </AppShell>
  );
}

function DashboardBody() {
  const t = useT();
  const { lang } = useLanguage();
  usePageTitle("meta.dashboard.title", "Dashboard");
  // 12-month axis labels — English short names by default, Chinese 1-12月
  // when the user has switched the UI to 中文.
  const chartMonths = lang === "zh"
    ? ["1月", "2月", "3月", "4月", "5月", "6月", "7月", "8月", "9月", "10月", "11月", "12月"]
    : ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
  const [accts, setAccts] = useState<Account[]>([]);
  const [report, setReport] = useState<SpendingReport | null>(null);
  const [anomalies, setAnomalies] = useState<Anomaly[]>([]);
  const [subs, setSubs] = useState<Subscription[]>([]);
  const [digest, setDigest] = useState<DailyDigest | null>(null);
  const [digestAcknowledged, setDigestAcknowledged] = useState(false);

  // Refresh whenever the cross-page bus ticks (after a transfer / wealth /
  // subscription / savings action). Also runs once on mount.
  const accountsTick = useAccountsBusTick();
  useEffect(() => {
    Promise.all([
      accounts.list(),
      reports.spending(),
      reports.anomalies(),
      subscriptions.list(),
      insights.daily().catch(() => null),
    ])
      .then(([a, r, an, s, d]) => {
        setAccts(a);
        setReport(r);
        setAnomalies(an);
        setSubs(s);
        if (d) setDigest(d);
      })
      .catch(() => {});
  }, [accountsTick]);

  const ackDigest = () => {
    if (!digest) return;
    insights
      .ack()
      .then(() => setDigestAcknowledged(true))
      .catch(() => setDigestAcknowledged(true));
  };

  const totalAvailable = accts.reduce((s, a) => s + a.available_balance_fen, 0);
  const totalReserved = accts.reduce((s, a) => s + a.reserved_balance_fen, 0);
  const activeSubs = subs.filter((s) => s.cancellation_status === "active");

  return (
    <div className="space-y-8">
      <AuroraBackdrop />
      <QuickstartBanner />
      {digest && (
        <FadeIn>
          <Card className="border border-white/10 bg-white/5 backdrop-blur">
            <CardHeader className="flex flex-row items-center justify-between">
              <div>
                <CardTitle className="flex items-center gap-2 text-base">
                  <Sparkles className="h-4 w-4 text-brand" />
                  {t("agent.daily_digest.title")}
                </CardTitle>
                <CardDescription>
                  {t("agent.daily_digest.renewals")} · {t("agent.daily_digest.goals")} · {t("agent.daily_digest.anomalies")}
                </CardDescription>
              </div>
              <Button
                variant="ghost"
                size="sm"
                onClick={ackDigest}
                disabled={digestAcknowledged}
              >
                {digestAcknowledged ? "✓" : t("agent.daily_digest.cta")}
              </Button>
            </CardHeader>
            <CardContent>
              <div className="grid gap-3 sm:grid-cols-3">
                <div className="rounded-lg border border-white/5 bg-white/5 p-3">
                  <div className="text-xs uppercase tracking-wide text-muted-foreground">
                    {t("agent.daily_digest.renewals")}
                  </div>
                  <div className="mt-1 text-2xl font-semibold">
                    {digest.items.renewals.length}
                  </div>
                  <div className="mt-1 text-xs text-muted-foreground">
                    {digest.items.renewals.slice(0, 2).map((r) => r.merchant).join(" · ") || "—"}
                  </div>
                </div>
                <div className="rounded-lg border border-white/5 bg-white/5 p-3">
                  <div className="text-xs uppercase tracking-wide text-muted-foreground">
                    {t("agent.daily_digest.spend")}
                  </div>
                  <div className="mt-1 text-2xl font-semibold">
                    {fenToCny(digest.items.spend_24h_fen)}
                  </div>
                  <div className="mt-1 text-xs text-muted-foreground">
                    {t("agent.daily_digest.anomalies")}: {digest.items.anomaly_count_24h}
                  </div>
                </div>
                <div className="rounded-lg border border-white/5 bg-white/5 p-3">
                  <div className="text-xs uppercase tracking-wide text-muted-foreground">
                    {t("agent.daily_digest.pending_plans")}
                  </div>
                  <div className="mt-1 text-2xl font-semibold">
                    {digest.items.pending_workflows}
                  </div>
                  <div className="mt-1 text-xs text-muted-foreground">
                    {t("agent.daily_digest.goals")}: {digest.items.goals.length}
                  </div>
                </div>
              </div>
            </CardContent>
          </Card>
        </FadeIn>
      )}

      {/* HERO — Novara style */}
      <FadeIn>
        <section className="relative overflow-hidden rounded-3xl border border-white/5 bg-hero-sky px-6 shadow-card sm:px-10">
          <MountainHero />
          <div className="relative z-10 grid gap-8 py-12 lg:grid-cols-[1.05fr_0.95fr] lg:gap-10 lg:py-16">
            {/* Left: headline */}
            <div>
              <motion.p
                initial={{ opacity: 0, y: 6 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ delay: 0.05 }}
                className="text-[11px] font-medium uppercase tracking-[0.22em] text-primary"
              >
                {t("dash.hero.eyebrow")}
              </motion.p>
              <motion.h1
                initial={{ opacity: 0, y: 10 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ duration: 0.7, delay: 0.1, ease: [0.16, 1, 0.3, 1] }}
                className="mt-4 font-display text-4xl font-semibold leading-[1.04] tracking-tight text-foreground sm:text-5xl lg:text-6xl"
              >
                {t("dash.hero.titleLead")}
                <br />
                <span className="text-brand">{t("dash.hero.titleBrand")}</span>
              </motion.h1>
              <motion.p
                initial={{ opacity: 0, y: 10 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ delay: 0.2 }}
                className="mt-5 max-w-md text-base text-muted-foreground"
              >
                {t("dash.hero.body")}
              </motion.p>

              <motion.div
                initial={{ opacity: 0, y: 10 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ delay: 0.3 }}
                className="mt-7 flex flex-wrap items-center gap-3"
              >
                <Button variant="gradient" size="lg">
                  {t("dash.hero.cta.consultation")}
                  <ArrowUpRight className="size-4" />
                </Button>
                <Button variant="outline" size="lg">
                  <Play className="size-4" />
                  {t("dash.hero.cta.demo")}
                </Button>
              </motion.div>

              <motion.ul
                initial="hidden"
                animate="show"
                variants={{
                  hidden: { opacity: 1 },
                  show: { opacity: 1, transition: { delayChildren: 0.4, staggerChildren: 0.08 } },
                }}
                className="mt-8 flex flex-wrap gap-x-6 gap-y-3 text-sm"
              >
                {[
                  { icon: BarChart3, label: t("dash.hero.trust.expertise") },
                  { icon: ShieldHalf, label: t("dash.hero.trust.clientFirst") },
                  { icon: Compass, label: t("dash.hero.trust.partnership") },
                ].map((it) => (
                  <motion.li
                    key={it.label}
                    variants={{
                      hidden: { opacity: 0, y: 8 },
                      show: { opacity: 1, y: 0, transition: { duration: 0.4 } },
                    }}
                    className="flex items-center gap-2 text-muted-foreground"
                  >
                    <span className="grid size-7 place-items-center rounded-md bg-white/5 text-primary ring-1 ring-white/10">
                      <it.icon className="size-3.5" />
                    </span>
                    {it.label}
                  </motion.li>
                ))}
              </motion.ul>
            </div>

            {/* Right: portfolio card */}
            <motion.div
              initial={{ opacity: 0, y: 16 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ duration: 0.7, delay: 0.2, ease: [0.16, 1, 0.3, 1] }}
              className="glass-strong relative rounded-2xl p-6 ring-glow"
            >
              <div className="flex items-start justify-between">
                <div>
                  <p className="text-[11px] font-medium uppercase tracking-[0.18em] text-muted-foreground">
                    {t("dash.hero.portfolio.title")}
                  </p>
                  <div className="mt-2 font-display text-4xl font-semibold tracking-tight">
                    <CurrencyCountUp fen={totalAvailable} />
                  </div>
                  <div className="mt-2 flex items-center gap-1.5 text-sm">
                    <TrendingUp className="size-3.5 text-primary" />
                    <span className="font-medium text-primary">+12.4%</span>
                    <span className="text-muted-foreground">{t("dash.hero.portfolio.delta")}</span>
                  </div>
                </div>
                <button className="flex items-center gap-1.5 rounded-md border border-white/10 bg-white/5 px-2.5 py-1 text-[11px] text-muted-foreground transition-colors hover:bg-white/10">
                  {t("dash.hero.portfolio.last12")}
                </button>
              </div>

              <div className="mt-4">
                <PortfolioLine months={chartMonths} />
              </div>

              <div className="mt-5 grid grid-cols-3 gap-3">
                {[
                  { icon: TrendingUp, label: t("dash.hero.portfolio.wealthGrowth"), value: "+12.4%" },
                  { icon: ShieldHalf, label: t("dash.hero.portfolio.diversification"), value: t("dash.hero.portfolio.diversificationValue") },
                  { icon: ShieldCheck, label: t("dash.hero.portfolio.riskLevel"), value: t("dash.hero.portfolio.riskLevelValue") },
                ].map((s, i) => (
                  <motion.div
                    key={s.label}
                    initial={{ opacity: 0, y: 8 }}
                    animate={{ opacity: 1, y: 0 }}
                    transition={{ delay: 0.6 + i * 0.08 }}
                    className="rounded-xl border border-white/5 bg-white/[0.03] p-3"
                  >
                    <s.icon className="size-4 text-primary" />
                    <p className="mt-2 text-[10px] uppercase tracking-[0.18em] text-muted-foreground">
                      {s.label}
                    </p>
                    <p className="mt-1 text-sm font-semibold">{s.value}</p>
                  </motion.div>
                ))}
              </div>
            </motion.div>
          </div>
        </section>
      </FadeIn>

      {/* KPI strip */}
      <Stagger>
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
          <StaggerItem>
            <KpiCard
              label="Available"
              valueFen={totalAvailable}
              tone="up"
              icon={<TrendingUp className="size-4" />}
            />
          </StaggerItem>
          <StaggerItem>
            <KpiCard
              label="Reserved"
              valueFen={totalReserved}
              tone="muted"
              icon={<PiggyBank className="size-4" />}
            />
          </StaggerItem>
          <StaggerItem>
            <KpiCard
              label="30d expenses"
              valueFen={report?.expense_fen ?? 0}
              tone="down"
              icon={<TrendingDown className="size-4" />}
            />
          </StaggerItem>
          <StaggerItem>
            <KpiCard
              label="30d income"
              valueFen={report?.income_fen ?? 0}
              tone="up"
              icon={<ShieldCheck className="size-4" />}
            />
          </StaggerItem>
        </div>
      </Stagger>

      {/* Quick actions row */}
      <FadeIn delay={0.1}>
        <div className="flex flex-wrap items-center justify-between gap-3 rounded-2xl border border-white/5 bg-card/40 p-4 backdrop-blur">
          <div className="flex items-center gap-3">
            <span className="grid size-9 place-items-center rounded-lg bg-gradient-to-br from-primary/30 to-indigo-500/30 text-primary ring-1 ring-primary/30">
              <Sparkles className="size-4" />
            </span>
            <div>
              <p className="text-sm font-medium">{t("dash.simBanner.title")}</p>
              <p className="text-xs text-muted-foreground">
                {t("dash.simBanner.body")}
              </p>
            </div>
          </div>
          <div className="flex flex-wrap gap-2">
            <Link005 href="/chat">
              <Button variant="gradient" size="sm">
                <Sparkles className="size-3.5" />
                {t("dash.cta.assistant")}
              </Button>
            </Link005>
            <Link005 href="/accounts">
              <Button variant="outline" size="sm">
                {t("dash.cta.ledger")}
                <ArrowUpRight className="size-3.5" />
              </Button>
            </Link005>
            <Link005 href="/budgets">
              <Button variant="outline" size="sm">
                {t("dash.cta.budgets")}
                <ArrowUpRight className="size-3.5" />
              </Button>
            </Link005>
          </div>
        </div>
      </FadeIn>

      <div className="grid gap-4 lg:grid-cols-3">
        <FadeIn delay={0.1} className="lg:col-span-2">
          <Card>
            <CardHeader>
              <div className="flex items-start justify-between">
                <div>
                  <CardTitle>{t("dash.spendByCategory")}</CardTitle>
                  <CardDescription>
                    Last 30 days · {report ? fenToCny(report.expense_fen) : "—"} total
                  </CardDescription>
                </div>
                <Badge variant="muted">auto</Badge>
              </div>
            </CardHeader>
            <CardContent>
              {!report ? (
                <div className="grid h-64 place-items-center text-sm text-muted-foreground">
                  Loading…
                </div>
              ) : report.items.length === 0 ? (
                <div className="grid h-64 place-items-center text-sm text-muted-foreground">
                  No expenses in this window.
                </div>
              ) : (
                <div className="grid grid-cols-1 gap-6 md:grid-cols-2">
                  <div className="h-64">
                    <ResponsiveContainer width="100%" height="100%">
                      <PieChart>
                        <Pie
                          data={report.items}
                          dataKey="total_fen"
                          nameKey="category"
                          innerRadius={56}
                          outerRadius={92}
                          paddingAngle={2}
                          stroke="none"
                        >
                          {report.items.map((_, i) => (
                            <Cell key={i} fill={TIER_COLORS[i % TIER_COLORS.length]} />
                          ))}
                        </Pie>
                        <Tooltip
                          contentStyle={{
                            background: "hsl(var(--card))",
                            border: "1px solid hsl(var(--border))",
                            borderRadius: 10,
                            fontSize: 12,
                          }}
                          formatter={(value) => fenToCny(Number(value))}
                        />
                      </PieChart>
                    </ResponsiveContainer>
                  </div>
                  <ul className="space-y-2 self-center">
                    {report.items.map((it, i) => (
                      <li
                        key={it.category}
                        className="flex items-center justify-between gap-3 rounded-md border border-white/5 bg-white/[0.03] px-3 py-2 transition-colors hover:bg-white/[0.06]"
                      >
                        <div className="flex items-center gap-2">
                          <span
                            className="size-2.5 rounded-full"
                            style={{ background: TIER_COLORS[i % TIER_COLORS.length] }}
                          />
                          <span className="text-sm">{it.category}</span>
                        </div>
                        <div className="text-right">
                          <div className="text-sm font-medium">{fenToCny(it.total_fen)}</div>
                          <div className="text-[10px] uppercase tracking-wider text-muted-foreground">
                            {it.share_pct.toFixed(1)}% · {it.transaction_count} txns
                          </div>
                        </div>
                      </li>
                    ))}
                  </ul>
                </div>
              )}
            </CardContent>
          </Card>
        </FadeIn>

        <FadeIn delay={0.2}>
          <Card className="h-full">
            <CardHeader>
              <CardTitle>Accounts</CardTitle>
              <CardDescription>Masked numbers only</CardDescription>
            </CardHeader>
            <CardContent className="space-y-3">
              {accts.length === 0 ? (
                <p className="text-sm text-muted-foreground">Loading…</p>
              ) : (
                accts.map((a, i) => (
                  <motion.div
                    key={a.id}
                    initial={{ opacity: 0, y: 6 }}
                    animate={{ opacity: 1, y: 0 }}
                    transition={{ delay: 0.2 + i * 0.05 }}
                    className="group/acct flex items-center justify-between rounded-lg border border-white/5 bg-white/[0.03] p-3 transition-colors hover:border-primary/30 hover:bg-white/[0.06]"
                  >
                    <div className="flex items-center gap-3">
                      <div className="grid size-9 place-items-center rounded-lg bg-gradient-to-br from-primary/30 to-indigo-500/20 text-primary ring-1 ring-primary/30">
                        <CreditCard className="size-4" />
                      </div>
                      <div>
                        <div className="text-sm font-medium">{a.display_name}</div>
                        <div className="text-[10px] font-mono uppercase tracking-wider text-muted-foreground">
                          {a.masked_number}
                        </div>
                      </div>
                    </div>
                    <div className="text-right">
                      <div className="text-sm font-semibold">
                        {fenToCny(a.available_balance_fen)}
                      </div>
                      {a.reserved_balance_fen > 0 && (
                        <div className="text-[10px] uppercase tracking-wider text-muted-foreground">
                          {fenToCny(a.reserved_balance_fen)} reserved
                        </div>
                      )}
                    </div>
                  </motion.div>
                ))
              )}
              <Link005 href="/accounts" className="block pt-1">
                <Button variant="ghost" size="sm" className="w-full">
                  Open ledger
                  <ArrowUpRight className="size-3.5" />
                </Button>
              </Link005>
            </CardContent>
          </Card>
        </FadeIn>
      </div>

      <div className="grid gap-4 lg:grid-cols-2">
        <FadeIn delay={0.3}>
          <Card>
            <CardHeader>
              <CardTitle>Detected subscriptions</CardTitle>
              <CardDescription>Recurring charges flagged by the detector</CardDescription>
            </CardHeader>
            <CardContent>
              {activeSubs.length === 0 ? (
                <p className="text-sm text-muted-foreground">No active subscriptions.</p>
              ) : (
                <ul className="space-y-2">
                  {activeSubs.slice(0, 5).map((s, i) => (
                    <motion.li
                      key={s.id}
                      initial={{ opacity: 0, y: 6 }}
                      animate={{ opacity: 1, y: 0 }}
                      transition={{ delay: 0.3 + i * 0.05 }}
                      className="flex items-center justify-between rounded-md border border-white/5 bg-white/[0.03] p-3 text-sm transition-colors hover:border-primary/30 hover:bg-white/[0.06]"
                    >
                      <div>
                        <div className="font-medium">{s.merchant}</div>
                        <div className="text-[10px] uppercase tracking-wider text-muted-foreground">
                          every {s.interval_days}d · next {formatDate(s.next_expected_at)}
                        </div>
                      </div>
                      <div className="text-right">
                        <div className="text-sm font-semibold">
                          {fenToCny(s.estimated_amount_fen)}
                        </div>
                        <Badge variant={s.cancellation_status === "active" ? "green" : "muted"}>
                          {s.cancellation_status}
                        </Badge>
                      </div>
                    </motion.li>
                  ))}
                </ul>
              )}
              <Link005 href="/subscriptions" className="block pt-1">
                <Button variant="ghost" size="sm" className="mt-3 w-full">
                  Manage subscriptions
                  <ArrowUpRight className="size-3.5" />
                </Button>
              </Link005>
            </CardContent>
          </Card>
        </FadeIn>

        <FadeIn delay={0.4}>
          <Card>
            <CardHeader>
              <CardTitle>Potentially unusual activity</CardTitle>
              <CardDescription>Rule-based flags · no fraud claim</CardDescription>
            </CardHeader>
            <CardContent>
              {anomalies.length === 0 ? (
                <p className="text-sm text-muted-foreground">No anomalies in the last 60 days.</p>
              ) : (
                <ul className="space-y-2">
                  {anomalies.slice(0, 5).map((a, i) => (
                    <motion.li
                      key={a.transaction_id}
                      initial={{ opacity: 0, y: 6 }}
                      animate={{ opacity: 1, y: 0 }}
                      transition={{ delay: 0.4 + i * 0.05 }}
                      className="rounded-md border border-white/5 bg-white/[0.03] p-3 text-sm transition-colors hover:border-tier-yellow/30 hover:bg-white/[0.06]"
                    >
                      <div className="flex items-center justify-between">
                        <div className="font-medium">{a.merchant}</div>
                        <div className="text-sm font-semibold">
                          {fenToCny(a.amount_fen)}
                        </div>
                      </div>
                      <div className="mt-0.5 text-[10px] uppercase tracking-wider text-muted-foreground">
                        {relativeTime(a.occurred_at)}
                      </div>
                      <ul className="mt-2 flex flex-wrap gap-1">
                        {a.reasons.map((r) => (
                          <Badge key={r} variant="yellow">
                            <AlertTriangle className="size-2.5" />
                            {r}
                          </Badge>
                        ))}
                      </ul>
                    </motion.li>
                  ))}
                </ul>
              )}
            </CardContent>
          </Card>
        </FadeIn>
      </div>
    </div>
  );
}

function KpiCard({
  label,
  valueFen,
  tone,
  icon,
}: {
  label: string;
  valueFen: number;
  tone: "up" | "down" | "muted";
  icon: React.ReactNode;
}) {
  const toneCls =
    tone === "up"
      ? "text-tier-green"
      : tone === "down"
      ? "text-tier-red"
      : "text-muted-foreground";
  return (
    <Card>
      <CardContent className="p-5">
        <div className="flex items-center justify-between">
          <p className="text-[10px] font-medium uppercase tracking-[0.18em] text-muted-foreground">
            {label}
          </p>
          <span className={cn("grid size-7 place-items-center rounded-md bg-white/5 ring-1 ring-white/10", toneCls)}>
            {icon}
          </span>
        </div>
        <div className="mt-2 font-display text-2xl font-semibold tracking-tight">
          <CurrencyCountUp fen={valueFen} />
        </div>
        <div className="mt-1 h-1 overflow-hidden rounded-full bg-white/5">
          <motion.div
            initial={{ width: 0 }}
            animate={{ width: tone === "up" ? "78%" : tone === "down" ? "52%" : "35%" }}
            transition={{ duration: 1.1, ease: [0.16, 1, 0.3, 1] }}
            className={cn(
              "h-full rounded-full",
              tone === "up" && "bg-gradient-to-r from-primary to-cyan-400",
              tone === "down" && "bg-gradient-to-r from-tier-red to-amber-400",
              tone === "muted" && "bg-gradient-to-r from-muted to-white/20"
            )}
          />
        </div>
      </CardContent>
    </Card>
  );
}