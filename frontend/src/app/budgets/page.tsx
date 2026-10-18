"use client";

import { useEffect, useState, useCallback } from "react";
import { motion } from "framer-motion";
import {
  Plus,
  Trash2,
  TrendingUp,
  AlertTriangle,
  CheckCircle2,
  Wallet,
  RefreshCcw,
} from "lucide-react";
import { AppShell } from "@/components/layout/app-shell";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Badge } from "@/components/ui/badge";
import { budgets, ApiError, type Budget, type BudgetSummary } from "@/lib/api";
import { useToast } from "@/components/ui/toast";
import { FadeIn, Stagger, StaggerItem } from "@/components/motion/aurora";
import { cnyToFen, fenToCny, cn } from "@/lib/utils";
import {
  PageHeader,
  EmptyState,
  LoadingState,
  ErrorState,
  KpiGrid,
  StatCard,
  SectionCard,
  Toolbar,
} from "@/components/ui/feedback";
import { useT } from "@/i18n/LanguageProvider";
import { usePageTitle } from "@/lib/use-page-title";

export default function BudgetsPage() {
  return (
    <AppShell>
      <Body />
    </AppShell>
  );
}

function Body() {
  const t = useT();
  usePageTitle("meta.budgets.title", "Budgets");
  const [list, setList] = useState<Budget[] | null>(null);
  const [summary, setSummary] = useState<BudgetSummary | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [draft, setDraft] = useState({ category: "", amount: "", note: "" });
  const [saving, setSaving] = useState(false);
  const [removing, setRemoving] = useState<string | null>(null);
  const { toast } = useToast();

  const refresh = useCallback(() => {
    setError(null);
    Promise.all([budgets.list(), budgets.summary()])
      .then(([l, s]) => {
        setList(l.budgets);
        setSummary(s);
      })
      .catch((e) => {
        setError(e instanceof ApiError ? e.message : "Could not load budgets");
        setList([]);
      });
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  const onSave = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!draft.category.trim() || !draft.amount) return;
    setSaving(true);
    try {
      await budgets.set({
        category: draft.category.trim(),
        amount: draft.amount,
        note: draft.note.trim() || undefined,
      });
      toast({
        variant: "success",
        title: t("budgets.toast.saved"),
        description: draft.category,
      });
      setDraft({ category: "", amount: "", note: "" });
      refresh();
    } catch (err) {
      toast({
        variant: "error",
        title: t("budgets.toast.saveFail"),
        description: err instanceof ApiError ? err.message : undefined,
      });
    } finally {
      setSaving(false);
    }
  };

  const onRemove = async (b: Budget) => {
    setRemoving(b.id);
    try {
      await budgets.remove(b.id);
      toast({
        variant: "success",
        title: t("budgets.toast.deleted"),
        description: b.category,
      });
      refresh();
    } catch {
      toast({ variant: "error", title: t("budgets.toast.deleteFail") });
    } finally {
      setRemoving(null);
    }
  };

  const isLoading = list === null && error === null;

  return (
    <div className="space-y-6">
      <FadeIn>
        <PageHeader
          eyebrow={t("budgets.page.eyebrow")}
          title={t("budgets.page.title")}
          description={t("budgets.page.subtitle")}
          actions={
            <Button
              variant="outline"
              size="sm"
              onClick={refresh}
              disabled={isLoading}
            >
              <RefreshCcw className="size-3.5" />
              {t("common.retry")}
            </Button>
          }
        />
      </FadeIn>

      {isLoading ? (
        <LoadingState />
      ) : error ? (
        <ErrorState
          title={t("common.error.title")}
          body={error}
          onRetry={refresh}
          retryLabel={t("common.retry")}
        />
      ) : (
        <>
          {/* Summary strip */}
          {summary && (
            <KpiGrid>
              <StatCard
                label={t("budgets.summary.title")}
                value={fenToCny(summary.remaining_fen)}
                icon={Wallet}
                tone={summary.breached.length ? "warning" : "neutral"}
                hint={t("budgets.summary.remaining", {
                  amount: fenToCny(summary.remaining_fen),
                  total: fenToCny(summary.total_limit_fen),
                })}
              />
              <StatCard
                label={t("budgets.summary.breached")}
                value={String(summary.breached.length)}
                icon={AlertTriangle}
                tone={summary.breached.length ? "down" : "up"}
                hint={t("budgets.summary.breached", {
                  count: summary.breached.length,
                })}
              />
              <StatCard
                label={t("budgets.summary.near")}
                value={String(summary.near_limit.length)}
                icon={TrendingUp}
                tone={summary.near_limit.length ? "down" : "up"}
                hint={t("budgets.summary.near", {
                  count: summary.near_limit.length,
                })}
              />
              <StatCard
                label={t("common.simulationBadge")}
                value="—"
                hint="Sim only"
                icon={CheckCircle2}
                tone="up"
              />
            </KpiGrid>
          )}

          {/* Add form */}
          <FadeIn>
            <SectionCard
              title={t("budgets.fields.add")}
              description={t("budgets.subtitle", {
                defaultValue: t("budgets.page.subtitle"),
              })}
            >
              <form onSubmit={onSave} className="grid gap-3 sm:grid-cols-[1fr_1fr_2fr_auto]">
                <div className="space-y-1.5">
                  <Label htmlFor="bcat">{t("budgets.fields.name")}</Label>
                  <Input
                    id="bcat"
                    value={draft.category}
                    onChange={(e) =>
                      setDraft({ ...draft, category: e.target.value })
                    }
                    placeholder={t("budgets.fields.namePh")}
                    required
                  />
                </div>
                <div className="space-y-1.5">
                  <Label htmlFor="bamt">{t("budgets.fields.amount")}</Label>
                  <Input
                    id="bamt"
                    inputMode="decimal"
                    value={draft.amount}
                    onChange={(e) =>
                      setDraft({ ...draft, amount: e.target.value })
                    }
                    placeholder="300"
                    required
                  />
                </div>
                <div className="space-y-1.5">
                  <Label htmlFor="bnote">{t("budgets.fields.note")}</Label>
                  <Input
                    id="bnote"
                    value={draft.note}
                    onChange={(e) =>
                      setDraft({ ...draft, note: e.target.value })
                    }
                  />
                </div>
                <div className="flex items-end">
                  <Button
                    type="submit"
                    variant="gradient"
                    className="w-full sm:w-auto"
                    disabled={saving}
                  >
                    <Plus className="size-4" />
                    {saving
                      ? t("budgets.fields.saving")
                      : t("budgets.fields.save")}
                  </Button>
                </div>
              </form>
            </SectionCard>
          </FadeIn>

          {/* List */}
          {list && list.length === 0 ? (
            <EmptyState
              icon={Wallet}
              title={t("budgets.empty.title")}
              body={t("budgets.empty.body")}
            />
          ) : list ? (
            <Stagger>
              <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
                {list.map((b) => (
                  <StaggerItem key={b.id}>
                    <BudgetCard
                      budget={b}
                      onRemove={() => onRemove(b)}
                      busy={removing === b.id}
                    />
                  </StaggerItem>
                ))}
              </div>
            </Stagger>
          ) : null}
        </>
      )}
    </div>
  );
}

function BudgetCard({
  budget,
  onRemove,
  busy,
}: {
  budget: Budget;
  onRemove: () => void;
  busy: boolean;
}) {
  const t = useT();
  const pct = Math.min(100, Math.round(budget.utilization_pct));
  const over = budget.breached || pct >= 100;
  const near = !over && pct >= 80;
  return (
    <motion.div whileHover={{ y: -2 }} transition={{ type: "spring", damping: 18, stiffness: 200 }}>
      <Card>
        <CardContent className="p-5">
          <div className="flex items-start justify-between">
            <div>
              <h3 className="font-display text-base font-semibold tracking-tight">
                {budget.category}
              </h3>
              {budget.note && (
                <p className="mt-0.5 text-[11px] text-muted-foreground">
                  {budget.note}
                </p>
              )}
            </div>
            <Badge
              variant={over ? "red" : near ? "yellow" : "muted"}
            >
              {pct}%
            </Badge>
          </div>
          <div className="mt-4">
            <div className="h-2 overflow-hidden rounded-full bg-white/5">
              <div
                className={cn(
                  "h-full rounded-full transition-[width] duration-500",
                  over
                    ? "bg-gradient-to-r from-tier-red to-rose-400"
                    : near
                      ? "bg-gradient-to-r from-tier-yellow to-amber-300"
                      : "bg-gradient-to-r from-primary to-cyan-400",
                )}
                style={{ width: `${pct}%` }}
              />
            </div>
            <div className="mt-2 flex items-center justify-between text-xs text-muted-foreground">
              <span>
                {t("budgets.spent", {
                  spent: fenToCny(budget.spent_fen),
                  limit: fenToCny(budget.limit_fen),
                })}
              </span>
              <span className="font-mono">
                {t("budgets.util.label", { limit: fenToCny(budget.limit_fen) })}
              </span>
            </div>
            <p
              className={cn(
                "mt-1 text-xs",
                over ? "text-tier-red" : "text-muted-foreground",
              )}
            >
              {over
                ? t("budgets.util.breached", { amount: fenToCny(budget.spent_fen - budget.limit_fen) })
                : t("budgets.util.remaining", { amount: fenToCny(budget.remaining_fen) })}
            </p>
          </div>
          <Toolbar className="mt-4 p-1.5">
            <Button
              variant="ghost"
              size="sm"
              onClick={onRemove}
              disabled={busy}
              className="w-full"
            >
              <Trash2 className="size-3.5" />
              {busy ? t("budgets.deleting") : t("budgets.delete")}
            </Button>
          </Toolbar>
        </CardContent>
      </Card>
    </motion.div>
  );
}
