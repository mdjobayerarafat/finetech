"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import { motion } from "framer-motion";
import {
  CheckCircle2,
  Goal as GoalIcon,
  Loader2,
  PiggyBank,
  Sliders,
  Wallet,
} from "lucide-react";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  accounts,
  ApiError,
  friendlyError,
  goals,
  type Account,
  type GoalProgress,
} from "@/lib/api";
import { useToast } from "@/components/ui/toast";
import { useT } from "@/i18n/LanguageProvider";
import { cn, cnyToFen, fenToCny } from "@/lib/utils";

type Props = {
  /** Bump to force a re-fetch (e.g. after a successful contribution). */
  refreshKey?: number;
};

/**
 * Layer 3.4 — Savings goals panel for the wealth page.
 *
 * Lists every goal with a progress bar, daily-pace ETA, on-track/behind
 * badge, and two CTAs:
 *   * Contribute — open a dialog to pick an amount + source account, then
 *     POST /goals/:id/contribute.
 *   * Rebalance — open a dialog to set a monthly contribution amount, then
 *     POST /goals/:id/rebalance. Returns a months-to-goal forecast.
 *
 * Uses `friendlyError()` for all error toasts (Layer 5.5).
 */
export function GoalsPanel({ refreshKey = 0 }: Props) {
  const t = useT();
  const { toast } = useToast();
  const [items, setItems] = useState<GoalProgress[]>([]);
  const [accts, setAccts] = useState<Account[]>([]);
  const [loading, setLoading] = useState(false);
  const [contrib, setContrib] = useState<GoalProgress | null>(null);
  const [rebal, setRebal] = useState<GoalProgress | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const [g, a] = await Promise.all([goals.list(), accounts.list()]);
      setItems(g.items);
      setAccts(a);
    } catch (err) {
      const f = err instanceof ApiError ? friendlyError(err) : null;
      toast({
        variant: "error",
        title: f?.title ?? "Load failed",
        description: f?.body,
      });
    } finally {
      setLoading(false);
    }
  }, [toast]);

  useEffect(() => {
    refresh();
  }, [refresh, refreshKey]);

  const hasGoals = items.length > 0;

  return (
    <Card>
      <CardContent className="p-5">
        <div className="mb-4 flex items-center justify-between">
          <div className="flex items-center gap-2">
            <span className="grid size-7 place-items-center rounded-md bg-tier-yellow/10 text-tier-yellow">
              <GoalIcon className="size-3.5" />
            </span>
            <div>
              <div className="text-sm font-semibold leading-tight">
                {t("wealth.goals.title")}
              </div>
              <div className="text-[11px] text-muted-foreground">
                {t("wealth.goals.subtitle")}
              </div>
            </div>
          </div>
          {loading && <Loader2 className="size-3.5 animate-spin text-muted-foreground" />}
        </div>

        {!hasGoals ? (
          <p className="text-[12px] text-muted-foreground">
            {t("wealth.goals.empty")}
          </p>
        ) : (
          <ul className="space-y-3">
            {items.map((g) => (
              <GoalRow
                key={g.id}
                goal={g}
                onContribute={() => setContrib(g)}
                onRebalance={() => setRebal(g)}
              />
            ))}
          </ul>
        )}

        {contrib && (
          <ContributeDialog
            goal={contrib}
            accounts={accts}
            onClose={() => setContrib(null)}
            onDone={() => {
              setContrib(null);
              refresh();
            }}
          />
        )}
        {rebal && (
          <RebalanceDialog
            goal={rebal}
            accounts={accts}
            onClose={() => setRebal(null)}
            onDone={() => {
              setRebal(null);
              refresh();
            }}
          />
        )}
      </CardContent>
    </Card>
  );
}

function GoalRow({
  goal,
  onContribute,
  onRebalance,
}: {
  goal: GoalProgress;
  onContribute: () => void;
  onRebalance: () => void;
}) {
  const t = useT();
  const pct = Math.max(0, Math.min(100, goal.progress_pct));
  const onTrack = goal.days_to_goal != null;
  const daily = fenToCny(goal.daily_pace_fen || 0);
  return (
    <li className="rounded-lg border bg-background/30 p-3">
      <div className="flex flex-wrap items-start justify-between gap-2">
        <div className="min-w-0">
          <div className="flex flex-wrap items-center gap-2">
            <span className="font-medium">{goal.name}</span>
            <Badge variant={onTrack ? "green" : "yellow"}>
              {onTrack ? t("wealth.goals.onTrack") : t("wealth.goals.behind")}
            </Badge>
          </div>
          <div className="mt-1 text-[11px] text-muted-foreground">
            {t("wealth.goals.progress", {
              pct: pct.toString(),
              saved: fenToCny(goal.saved_fen),
              target: fenToCny(goal.target_fen),
            })}
          </div>
          <div className="mt-0.5 text-[11px] text-muted-foreground">
            {goal.days_to_goal != null
              ? t("wealth.goals.pace", {
                  daily,
                  days: goal.days_to_goal.toString(),
                })
              : t("wealth.goals.paceUnknown", { daily })}
          </div>
        </div>
        <div className="flex items-center gap-1.5">
          <Button size="sm" variant="outline" onClick={onContribute}>
            <PiggyBank className="size-3.5" />
            {t("wealth.goals.contribute")}
          </Button>
          <Button size="sm" variant="ghost" onClick={onRebalance}>
            <Sliders className="size-3.5" />
            {t("wealth.goals.rebalance")}
          </Button>
        </div>
      </div>
      <div className="mt-3 h-2 w-full overflow-hidden rounded-full bg-muted">
        <motion.div
          className="h-full rounded-full bg-gradient-to-r from-primary to-tier-green"
          initial={{ width: 0 }}
          animate={{ width: `${pct}%` }}
          transition={{ duration: 0.6, ease: "easeOut" }}
        />
      </div>
    </li>
  );
}

function ContributeDialog({
  goal,
  accounts: accts,
  onClose,
  onDone,
}: {
  goal: GoalProgress;
  accounts: Account[];
  onClose: () => void;
  onDone: () => void;
}) {
  const t = useT();
  const { toast } = useToast();
  const [amount, setAmount] = useState("100");
  const [acctId, setAcctId] = useState<string | undefined>(accts[0]?.id);
  const [busy, setBusy] = useState(false);

  const submit = useCallback(async () => {
    if (!acctId) return;
    const fen = cnyToFen(amount);
    if (fen <= 0) {
      toast({ variant: "warning", title: t("wealth.goals.amount") + " > 0" });
      return;
    }
    setBusy(true);
    try {
      const res = await goals.contribute(goal.id, fen, acctId, "");
      toast({
        variant: "success",
        title: t("wealth.goals.contributed"),
        description: t("wealth.goals.contributedBody", {
          amount: fenToCny(res.amount_fen),
          saved: fenToCny(res.new_balance_fen),
        }),
      });
      onDone();
    } catch (err) {
      const f = err instanceof ApiError ? friendlyError(err) : null;
      toast({
        variant: "error",
        title: f?.title ?? t("wealth.goals.contribute") + " failed",
        description: f?.body,
      });
    } finally {
      setBusy(false);
    }
  }, [acctId, amount, goal.id, onDone, toast, t]);

  return (
    <Dialog open onOpenChange={(o) => !o && onClose()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <PiggyBank className="size-4 text-primary" />
            {t("wealth.goals.contribute")} · {goal.name}
          </DialogTitle>
          <DialogDescription>
            {t("wealth.goals.progress", {
              pct: goal.progress_pct.toString(),
              saved: fenToCny(goal.saved_fen),
              target: fenToCny(goal.target_fen),
            })}
          </DialogDescription>
        </DialogHeader>
        <div className="space-y-3">
          <div className="space-y-1.5">
            <Label htmlFor="goal-amount">{t("wealth.goals.amount")}</Label>
            <Input
              id="goal-amount"
              inputMode="decimal"
              value={amount}
              onChange={(e) => setAmount(e.target.value)}
            />
          </div>
          <div className="space-y-1.5">
            <Label>{t("wealth.goals.from")}</Label>
            <div className="grid gap-1.5">
              {accts.map((a) => (
                <label
                  key={a.id}
                  className={cn(
                    "flex cursor-pointer items-center justify-between rounded-md border bg-background/40 p-2 text-sm transition-colors",
                    acctId === a.id ? "ring-1 ring-primary" : "",
                  )}
                >
                  <span>
                    {a.display_name}{" "}
                    <span className="font-mono text-xs text-muted-foreground">
                      {a.masked_number}
                    </span>
                  </span>
                  <span className="flex items-center gap-2">
                    <span className="font-medium">
                      {fenToCny(a.available_balance_fen)}
                    </span>
                    <input
                      type="radio"
                      className="ml-2"
                      checked={acctId === a.id}
                      onChange={() => setAcctId(a.id)}
                    />
                  </span>
                </label>
              ))}
            </div>
          </div>
        </div>
        <DialogFooter>
          <Button variant="ghost" onClick={onClose} disabled={busy}>
            {t("common.cancel")}
          </Button>
          <Button
            variant="gradient"
            onClick={submit}
            disabled={busy || !acctId || cnyToFen(amount) <= 0}
          >
            {busy ? <Loader2 className="size-4 animate-spin" /> : <CheckCircle2 className="size-4" />}
            {t("wealth.goals.contribute")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function RebalanceDialog({
  goal,
  accounts: accts,
  onClose,
  onDone,
}: {
  goal: GoalProgress;
  accounts: Account[];
  onClose: () => void;
  onDone: () => void;
}) {
  const t = useT();
  const { toast } = useToast();
  const [monthly, setMonthly] = useState("200");
  const [acctId, setAcctId] = useState<string | undefined>(accts[0]?.id);
  const [busy, setBusy] = useState(false);

  const submit = useCallback(async () => {
    if (!acctId) return;
    const fen = cnyToFen(monthly);
    if (fen <= 0) {
      toast({ variant: "warning", title: t("wealth.goals.monthly") + " > 0" });
      return;
    }
    setBusy(true);
    try {
      const res = await goals.rebalance(goal.id, fen, acctId);
      toast({
        variant: "success",
        title: t("wealth.goals.rebalanced"),
        description: t("wealth.goals.rebalancedBody", {
          months: res.months_to_goal != null ? res.months_to_goal.toString() : "—",
          monthly: fenToCny(res.monthly_amount_fen),
        }),
      });
      onDone();
    } catch (err) {
      const f = err instanceof ApiError ? friendlyError(err) : null;
      toast({
        variant: "error",
        title: f?.title ?? t("wealth.goals.rebalance") + " failed",
        description: f?.body,
      });
    } finally {
      setBusy(false);
    }
  }, [acctId, goal.id, monthly, onDone, toast, t]);

  return (
    <Dialog open onOpenChange={(o) => !o && onClose()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Sliders className="size-4 text-primary" />
            {t("wealth.goals.rebalance")} · {goal.name}
          </DialogTitle>
          <DialogDescription>{t("wealth.goals.monthlyHelp")}</DialogDescription>
        </DialogHeader>
        <div className="space-y-3">
          <div className="space-y-1.5">
            <Label htmlFor="goal-monthly">{t("wealth.goals.monthly")}</Label>
            <Input
              id="goal-monthly"
              inputMode="decimal"
              value={monthly}
              onChange={(e) => setMonthly(e.target.value)}
            />
            <p className="text-[11px] text-muted-foreground">
              {t("wealth.goals.pace", {
                daily: fenToCny(Math.floor(cnyToFen(monthly) / 30)),
                days: goal.days_to_goal != null ? goal.days_to_goal.toString() : "—",
              })}
            </p>
          </div>
          <div className="space-y-1.5">
            <Label>{t("wealth.goals.from")}</Label>
            <div className="grid gap-1.5">
              {accts.map((a) => (
                <label
                  key={a.id}
                  className={cn(
                    "flex cursor-pointer items-center justify-between rounded-md border bg-background/40 p-2 text-sm transition-colors",
                    acctId === a.id ? "ring-1 ring-primary" : "",
                  )}
                >
                  <span>
                    <Wallet className="mr-1 inline size-3.5 text-muted-foreground" />
                    {a.display_name}{" "}
                    <span className="font-mono text-xs text-muted-foreground">
                      {a.masked_number}
                    </span>
                  </span>
                  <input
                    type="radio"
                    className="ml-2"
                    checked={acctId === a.id}
                    onChange={() => setAcctId(a.id)}
                  />
                </label>
              ))}
            </div>
          </div>
        </div>
        <DialogFooter>
          <Button variant="ghost" onClick={onClose} disabled={busy}>
            {t("common.cancel")}
          </Button>
          <Button
            variant="gradient"
            onClick={submit}
            disabled={busy || !acctId || cnyToFen(monthly) <= 0}
          >
            {busy ? <Loader2 className="size-4 animate-spin" /> : <CheckCircle2 className="size-4" />}
            {t("wealth.goals.rebalance")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
