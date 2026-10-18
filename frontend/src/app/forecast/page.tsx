"use client";

import { useEffect, useMemo, useState } from "react";
import { motion } from "framer-motion";
import {
  Calculator,
  Loader2,
  Sparkles,
  TrendingDown,
  TrendingUp,
  Wallet,
} from "lucide-react";
import {
  CartesianGrid,
  Line,
  LineChart,
  ReferenceLine,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import { AppShell } from "@/components/layout/app-shell";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { useToast } from "@/components/ui/toast";
import { forecast, type ForecastOut, type ForecastPoint } from "@/lib/api";
import { FadeIn, Stagger, StaggerItem, AuroraBackdrop } from "@/components/motion/aurora";
import { cn, fenToCny } from "@/lib/utils";
import { useT, useLanguage } from "@/i18n/LanguageProvider";
import { usePageTitle } from "@/lib/use-page-title";
import { useTranslated } from "@/i18n/use-translate";

const PRESETS: { label: string; amount: number; months: number; spend_month: number }[] = [
  { label: "6,000 laptop in December", amount: 6000, months: 12, spend_month: 11 },
  { label: "2,000 phone upgrade", amount: 2000, months: 6, spend_month: 0 },
  { label: "Tuition next term", amount: 5400, months: 8, spend_month: 2 },
  { label: "10,000 trip in 4 months", amount: 10000, months: 8, spend_month: 3 },
];

export default function ForecastPage() {
  return (
    <AppShell>
      <Body />
    </AppShell>
  );
}

function Body() {
  const t = useT();
  usePageTitle("meta.forecast.title", "Forecast");
  const [amountCny, setAmountCny] = useState<string>("6000");
  const [months, setMonths] = useState<number>(12);
  const [spendMonth, setSpendMonth] = useState<number>(11);
  const [out, setOut] = useState<ForecastOut | null>(null);
  const [busy, setBusy] = useState(false);
  const { toast } = useToast();

  const run = async (override?: { amount: number; months: number; spend_month: number }) => {
    const a = override?.amount ?? Number(amountCny);
    const m = override?.months ?? months;
    const s = override?.spend_month ?? spendMonth;
    if (!a || a <= 0) {
      toast({ variant: "error", title: "Enter a positive amount in CNY" });
      return;
    }
    setBusy(true);
    try {
      const r = await forecast.simulate({ amount_cny: a, months: m, spend_month_index: s });
      setOut(r);
    } catch (e) {
      const msg = e instanceof Error ? e.message : "Forecast failed";
      toast({ variant: "error", title: "Forecast failed", description: msg });
    } finally {
      setBusy(false);
    }
  };

  // Auto-run on mount for a delightful first paint.
  useEffect(() => {
    run();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <div className="space-y-6">
      <AuroraBackdrop />
      <FadeIn>
        <div>
          <p className="text-[11px] font-medium uppercase tracking-widest text-muted-foreground">
            {t("forecast.eyebrow")} · what-if
          </p>
          <h1 className="mt-1 font-display text-3xl font-semibold tracking-tight">
            {t("forecast.title")}
          </h1>
          <p className="mt-1 max-w-2xl text-sm text-muted-foreground">
            Project your balance forward using the ledger, upcoming bills, and
            active subscriptions. All numbers come from the backend — the LLM
            only narrates the answer.
          </p>
        </div>
      </FadeIn>

      <div className="grid gap-4 lg:grid-cols-[320px_1fr]">
        <FadeIn delay={0.05}>
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Calculator className="size-4 text-primary" />
                Scenario
              </CardTitle>
              <CardDescription>One number the planner already knows.</CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div>
                <Label htmlFor="amount">Amount (CNY)</Label>
                <Input
                  id="amount"
                  type="number"
                  min={0.01}
                  step="0.01"
                  value={amountCny}
                  onChange={(e) => setAmountCny(e.target.value)}
                />
              </div>
              <div className="grid grid-cols-2 gap-3">
                <div>
                  <Label htmlFor="months">Horizon (months)</Label>
                  <Input
                    id="months"
                    type="number"
                    min={1}
                    max={24}
                    value={months}
                    onChange={(e) => setMonths(Math.max(1, Math.min(24, Number(e.target.value) || 1)))}
                  />
                </div>
                <div>
                  <Label htmlFor="spend">Spend in month</Label>
                  <Input
                    id="spend"
                    type="number"
                    min={0}
                    max={months - 1}
                    value={spendMonth}
                    onChange={(e) => setSpendMonth(Math.max(0, Number(e.target.value) || 0))}
                  />
                </div>
              </div>
              <Button onClick={() => run()} disabled={busy} className="w-full">
                {busy ? <Loader2 className="size-4 animate-spin" /> : <Sparkles className="size-4" />}
                Project
              </Button>
              <div>
                <p className="mb-1 text-[10px] font-medium uppercase tracking-wider text-muted-foreground">
                  Quick presets
                </p>
                <Stagger>
                  <ul className="space-y-1.5">
                    {PRESETS.map((p) => (
                      <StaggerItem key={p.label}>
                        <motion.li layout>
                          <button
                            onClick={() => {
                              setAmountCny(String(p.amount));
                              setMonths(p.months);
                              setSpendMonth(p.spend_month);
                              run({ amount: p.amount, months: p.months, spend_month: p.spend_month });
                            }}
                            className="w-full rounded-md border border-white/5 bg-white/[0.03] px-2.5 py-1.5 text-left text-xs transition-colors hover:bg-accent"
                          >
                            {p.label}
                          </button>
                        </motion.li>
                      </StaggerItem>
                    ))}
                  </ul>
                </Stagger>
              </div>
            </CardContent>
          </Card>
        </FadeIn>

        <div className="space-y-4">
          <FadeIn delay={0.1}>
            <ResultPanel out={out} busy={busy} />
          </FadeIn>
          <FadeIn delay={0.15}>
            <ChartPanel out={out} busy={busy} />
          </FadeIn>
        </div>
      </div>
    </div>
  );
}

function ResultPanel({ out, busy }: { out: ForecastOut | null; busy: boolean }) {
  const t = useT();
  const { translated: narrative, isTranslating } = useTranslated(out?.narrative ?? null);
  if (busy) {
    return (
      <Card>
        <CardContent className="grid h-32 place-items-center text-sm text-muted-foreground">
          <Loader2 className="size-4 animate-spin" />
        </CardContent>
      </Card>
    );
  }
  if (!out) {
    return (
      <Card>
        <CardContent className="grid h-32 place-items-center text-sm text-muted-foreground">
          Run a scenario to see the verdict.
        </CardContent>
      </Card>
    );
  }
  const end = out.series[out.series.length - 1];
  const baselineEnd = end?.baseline_fen ?? out.starting_balance_fen;
  const diff = (end?.projected_balance_fen ?? 0) - baselineEnd;
  const verdictTone =
    out.verdict === "yes" ? "green" : out.verdict === "tight" ? "yellow" : "red";
  const verdictLabel =
    out.verdict === "yes"
      ? t("forecast.result.can" as never)
      : out.verdict === "tight"
      ? t("forecast.result.tight" as never)
      : t("forecast.result.cannot" as never);
  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          {t("forecast.result.narrative")}
          <Badge variant={verdictTone as "green" | "yellow" | "red"}>{verdictLabel}</Badge>
        </CardTitle>
        <CardDescription>
          {out.months}-month projection from {fenToCny(out.starting_balance_fen)} starting balance.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-3">
        <div className="flex items-start gap-2">
          <p className="text-sm leading-relaxed">{narrative}</p>
          {isTranslating && (
            <span className="text-[10px] uppercase tracking-wider text-muted-foreground">
              {t("forecast.result.translating")}
            </span>
)}
        </div>
        <div className="grid grid-cols-2 gap-3 sm:grid-cols-4">
          <Stat
            label={diff < 0 ? "After scenario" : "After scenario"}
            value={fenToCny(end?.projected_balance_fen ?? 0)}
            tone={diff < 0 ? "down" : "up"}
          />
          <Stat label="Avg income / mo" value={fenToCny(out.avg_income_fen)} tone="up" />
          <Stat label="Avg spend / mo" value={fenToCny(out.avg_expense_fen)} tone="down" />
        </div>
        {out.upcoming_bills.length > 0 && (
          <div>
            <p className="mb-1 text-[10px] font-medium uppercase tracking-wider text-muted-foreground">
              Upcoming bills in window
            </p>
            <ul className="space-y-1 text-xs">
              {out.upcoming_bills.map((b) => (
                <li
                  key={b.subject + b.due_at}
                  className="flex items-center justify-between rounded-md border border-white/5 bg-white/[0.03] px-2 py-1.5"
                >
                  <span>{b.subject}</span>
                  <span className="font-mono">{fenToCny(b.amount_fen)}</span>
                </li>
              ))}
            </ul>
          </div>
        )}
      </CardContent>
    </Card>
  );
}

function Stat({
  label,
  value,
  tone,
}: {
  label: string;
  value: string;
  tone?: "up" | "down";
}) {
  return (
    <div className="rounded-md border border-white/5 bg-white/[0.03] p-2.5">
      <p className="text-[10px] font-medium uppercase tracking-wider text-muted-foreground">
        {label}
      </p>
      <p
        className={cn(
          "mt-1 font-mono text-sm font-semibold",
          tone === "up" && "text-tier-green",
          tone === "down" && "text-tier-red"
        )}
      >
        {value}
      </p>
    </div>
  );
}

function ChartPanel({ out, busy }: { out: ForecastOut | null; busy: boolean }) {
  const data = useMemo(() => {
    if (!out) return [];
    return out.series.map((p) => ({
      label: p.label,
      "After scenario": p.projected_balance_fen / 100,
      "Without scenario": p.baseline_fen / 100,
    }));
  }, [out]);

  if (busy) {
    return (
      <Card>
        <CardContent className="grid h-64 place-items-center text-sm text-muted-foreground">
          <Loader2 className="size-4 animate-spin" />
        </CardContent>
      </Card>
    );
  }
  if (!out) {
    return (
      <Card>
        <CardContent className="grid h-64 place-items-center text-sm text-muted-foreground">
          No data yet.
        </CardContent>
      </Card>
    );
  }
  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <TrendingUp className="size-4 text-tier-green" />
          Balance projection
        </CardTitle>
        <CardDescription>Solid line: after your scenario. Dashed: without it.</CardDescription>
      </CardHeader>
      <CardContent>
        <div className="h-72 w-full">
          <ResponsiveContainer width="100%" height="100%">
            <LineChart data={data} margin={{ top: 8, right: 16, left: 8, bottom: 0 }}>
              <CartesianGrid stroke="rgba(255,255,255,0.06)" strokeDasharray="3 3" />
              <XAxis dataKey="label" stroke="rgba(255,255,255,0.4)" fontSize={11} />
              <YAxis
                stroke="rgba(255,255,255,0.4)"
                fontSize={11}
                tickFormatter={(v: number) => `¥${(v / 1000).toFixed(1)}k`}
              />
              <Tooltip
                contentStyle={{
                  background: "rgba(0,0,0,0.8)",
                  border: "1px solid rgba(255,255,255,0.1)",
                  borderRadius: 8,
                  fontSize: 12,
                }}
                formatter={(v) => `¥${(Number(v ?? 0) * 100).toLocaleString()}`}
              />
              <ReferenceLine y={0} stroke="rgba(248,113,113,0.6)" strokeDasharray="2 4" />
              <Line
                type="monotone"
                dataKey="Without scenario"
                stroke="rgba(125,211,252,0.9)"
                strokeWidth={1.5}
                strokeDasharray="4 4"
                dot={false}
              />
              <Line
                type="monotone"
                dataKey="After scenario"
                stroke="rgba(167,139,250,1)"
                strokeWidth={2.5}
                dot={{ r: 3, stroke: "rgba(167,139,250,1)", fill: "rgba(167,139,250,1)" }}
                activeDot={{ r: 5 }}
              />
            </LineChart>
          </ResponsiveContainer>
        </div>
      </CardContent>
    </Card>
  );
}
