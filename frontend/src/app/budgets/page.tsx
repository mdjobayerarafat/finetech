"use client";

import { Suspense, useEffect, useMemo, useState } from "react";
import { motion } from "framer-motion";
import {
  AlertTriangle,
  PiggyBank,
  Plus,
  Sparkles,
  Trash2,
  TrendingUp,
} from "lucide-react";
import { useSearchParams } from "next/navigation";

import { AppShell } from "@/components/layout/app-shell";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useToast } from "@/components/ui/toast";
import {
  budgets,
  accounts,
  type Account,
  type Budget,
  type BudgetReservation,
  type BudgetSummary,
} from "@/lib/api";
import { FadeIn, Stagger, StaggerItem } from "@/components/motion/aurora";
import { fenToCny } from "@/lib/utils";
import { useT } from "@/i18n/LanguageProvider";

const CATEGORIES = [
  "food",
  "transport",
  "shopping",
  "subscription",
  "entertainment",
  "travel",
  "utilities",
  "health",
  "education",
  "other",
] as const;

export default function BudgetsPage() {
  return (
    <AppShell>
      <Suspense fallback={null}>
        <Body />
      </Suspense>
    </AppShell>
  );
}

function Body() {
  const params = useSearchParams();
  const focusBudgetId = params.get("focus");
  const prefillCategory = params.get("category");
  const prefillLimitFen = params.get("limit_fen");

  const [list, setList] = useState<Budget[]>([]);
  const [summary, setSummary] = useState<BudgetSummary | null>(null);
  const [reservations, setReservations] = useState<BudgetReservation[]>([]);
  const [accts, setAccts] = useState<Account[]>([]);
  const [open, setOpen] = useState(false);
  const [category, setCategory] = useState<string>(prefillCategory ?? "food");
  const [amount, setAmount] = useState<string>(
    prefillLimitFen ? String(Math.round(Number(prefillLimitFen) / 100)) : ""
  );
  const [accountId, setAccountId] = useState<string>("");
  const [note, setNote] = useState<string>("");
  const [saving, setSaving] = useState(false);
  const { toast } = useToast();
  const t = useT();

  // Open the create modal if the user came from a deep-link (?category=…).
  useEffect(() => {
    if (prefillCategory || prefillLimitFen) {
      setOpen(true);
    }
  }, [prefillCategory, prefillLimitFen]);

  const refresh = async () => {
    const [b, s, r, a] = await Promise.all([
      budgets.list().catch(() => ({ budgets: [] })),
      budgets.summary().catch(() => null),
      budgets.reservations().catch(() => ({ reservations: [], total_held_fen: 0 })),
      accounts.list().catch(() => []),
    ]);
    setList(b.budgets ?? []);
    setSummary(s);
    setReservations(r.reservations ?? []);
    setAccts(a);
  };

  useEffect(() => {
    refresh();
  }, []);

  const onSave = async () => {
    if (!category || !amount) {
      toast({
        title: "Pick a category and amount",
        description: "Both are required to set a budget.",
        variant: "destructive",
      });
      return;
    }
    setSaving(true);
    try {
      await budgets.set({
        category,
        amount: Number(amount) || amount,
        account_id: accountId || undefined,
        note: note || undefined,
      });
      toast({ title: "Budget set", description: `${category} capped at ${amount}` });
      setOpen(false);
      setAmount("");
      setNote("");
      await refresh();
    } catch (e) {
      toast({
        title: "Couldn't save budget",
        description: (e as Error).message ?? "Try again.",
        variant: "destructive",
      });
    } finally {
      setSaving(false);
    }
  };

  const onDelete = async (id: string) => {
    try {
      await budgets.remove(id);
      toast({ title: "Budget removed" });
      await refresh();
    } catch (e) {
      toast({
        title: "Couldn't delete",
        description: (e as Error).message,
        variant: "destructive",
      });
    }
  };

  const focus = useMemo(
    () => list.find((b) => b.id === focusBudgetId) ?? null,
    [focusBudgetId, list]
  );

  const totalLimit = summary?.total_limit_fen ?? 0;
  const totalSpent = summary?.total_spent_fen ?? 0;
  const util = totalLimit > 0 ? (totalSpent / totalLimit) * 100 : 0;
  const overAll = totalLimit > 0 && totalSpent > totalLimit;

  return (
    <div className="space-y-6 px-4 md:px-8 pt-4 pb-16">
      <FadeIn>
        <div className="flex flex-wrap items-end justify-between gap-4">
          <div>
            <h1 className="text-3xl font-semibold tracking-tight">Budgets</h1>
            <p className="text-muted-foreground mt-1">
              Cap how much you spend on each category, get a heads-up before
              you blow the cap, and let the forecast respect your caps.
            </p>
          </div>
          <Button onClick={() => setOpen(true)} className="gap-2">
            <Plus className="h-4 w-4" /> Set a budget
          </Button>
        </div>
      </FadeIn>

      {/* --- Rollup ----------------------------------------------------- */}
      <Stagger className="grid gap-4 md:grid-cols-3">
        <StaggerItem>
          <Card>
            <CardHeader className="pb-2">
              <CardDescription>Total cap</CardDescription>
              <CardTitle className="text-3xl">
                {summary ? summary.display.limit : "—"}
              </CardTitle>
            </CardHeader>
            <CardContent className="text-xs text-muted-foreground">
              Across {summary?.budget_count ?? 0} categor
              {(summary?.budget_count ?? 0) === 1 ? "y" : "ies"}
            </CardContent>
          </Card>
        </StaggerItem>
        <StaggerItem>
          <Card>
            <CardHeader className="pb-2">
              <CardDescription>Spent this period</CardDescription>
              <CardTitle className="text-3xl">
                {summary ? summary.display.spent : "—"}
              </CardTitle>
            </CardHeader>
            <CardContent>
              <div className="h-2 w-full bg-muted rounded">
                <div
                  className={`h-2 rounded ${overAll ? "bg-rose-500" : "bg-emerald-500"}`}
                  style={{ width: `${Math.min(util, 100)}%` }}
                />
              </div>
              <div className="text-xs text-muted-foreground mt-1">
                {util.toFixed(0)}% of cap used
              </div>
            </CardContent>
          </Card>
        </StaggerItem>
        <StaggerItem>
          <Card>
            <CardHeader className="pb-2">
              <CardDescription>Held for upcoming events</CardDescription>
              <CardTitle className="text-3xl">
                {fenToCny(summary?.held_reservations_fen ?? 0)}
              </CardTitle>
            </CardHeader>
            <CardContent className="text-xs text-muted-foreground">
              {reservations.length} active reservation
              {reservations.length === 1 ? "" : "s"} (birthday, trips, …)
            </CardContent>
          </Card>
        </StaggerItem>
      </Stagger>

      {/* --- Breach / near-limit callouts ------------------------------- */}
      {(summary?.breached.length ?? 0) > 0 && (
        <FadeIn>
          <Card className="border-rose-500/40 bg-rose-500/5">
            <CardHeader>
              <div className="flex items-center gap-2 text-rose-600">
                <AlertTriangle className="h-4 w-4" />
                <CardTitle>Over budget</CardTitle>
              </div>
              <CardDescription>
                Slow down on these categories or raise the cap.
              </CardDescription>
            </CardHeader>
            <CardContent className="grid gap-2 text-sm">
              {summary!.breached.map((b) => (
                <div
                  key={b.category}
                  className="flex items-center justify-between rounded-md border border-rose-500/30 px-3 py-2"
                >
                  <span className="font-medium">{b.category}</span>
                  <span className="text-rose-600">
                    {fenToCny(b.spent_fen)} / {fenToCny(b.limit_fen)} (
                    {b.utilization_pct.toFixed(0)}%)
                  </span>
                </div>
              ))}
            </CardContent>
          </Card>
        </FadeIn>
      )}

      {(summary?.near_limit.length ?? 0) > 0 && (
        <FadeIn>
          <Card className="border-amber-500/40 bg-amber-500/5">
            <CardHeader>
              <div className="flex items-center gap-2 text-amber-600">
                <TrendingUp className="h-4 w-4" />
                <CardTitle>Close to your cap</CardTitle>
              </div>
            </CardHeader>
            <CardContent className="grid gap-2 text-sm">
              {summary!.near_limit.map((b) => (
                <div
                  key={b.category}
                  className="flex items-center justify-between rounded-md border border-amber-500/30 px-3 py-2"
                >
                  <span className="font-medium">{b.category}</span>
                  <span>
                    {fenToCny(b.spent_fen)} / {fenToCny(b.limit_fen)}
                  </span>
                </div>
              ))}
            </CardContent>
          </Card>
        </FadeIn>
      )}

      {/* --- Active budgets list --------------------------------------- */}
      <FadeIn>
        <Card>
          <CardHeader>
            <CardTitle>Active budgets</CardTitle>
            <CardDescription>
              Trailing 30 days. Caps are soft — the chat agent uses them to
              warn before you transfer, and the forecaster deflates your
              liquid balance by any reservations.
            </CardDescription>
          </CardHeader>
          <CardContent className="grid gap-3">
            {list.length === 0 && (
              <div className="text-sm text-muted-foreground">
                No budgets yet. Click <strong>Set a budget</strong> to add one.
              </div>
            )}
            {list.map((b) => {
              const isFocus = focus?.id === b.id;
              return (
                <motion.div
                  key={b.id}
                  layout
                  className={`rounded-lg border p-3 ${
                    isFocus ? "border-primary bg-primary/5" : ""
                  } ${b.breached ? "border-rose-500/30" : ""}`}
                >
                  <div className="flex items-center justify-between gap-3">
                    <div className="flex items-center gap-2">
                      <PiggyBank className="h-4 w-4 text-muted-foreground" />
                      <div>
                        <div className="font-medium capitalize">
                          {b.category}
                        </div>
                        {b.note && (
                          <div className="text-xs text-muted-foreground">
                            {b.note}
                          </div>
                        )}
                      </div>
                    </div>
                    <div className="flex items-center gap-2">
                      {b.breached ? (
                        <Badge variant="destructive">Over</Badge>
                      ) : b.utilization_pct >= 80 ? (
                        <Badge className="bg-amber-500/20 text-amber-700">
                          Near cap
                        </Badge>
                      ) : (
                        <Badge variant="secondary">
                          {b.utilization_pct.toFixed(0)}% used
                        </Badge>
                      )}
                      <Button
                        variant="ghost"
                        size="icon"
                        onClick={() => onDelete(b.id)}
                        title="Remove budget"
                      >
                        <Trash2 className="h-4 w-4" />
                      </Button>
                    </div>
                  </div>
                  <div className="mt-2">
                    <div className="flex items-center justify-between text-xs text-muted-foreground">
                      <span>{fenToCny(b.spent_fen)} spent</span>
                      <span>{fenToCny(b.limit_fen)} cap</span>
                    </div>
                    <div className="h-2 w-full bg-muted rounded mt-1">
                      <div
                        className={`h-2 rounded ${
                          b.breached
                            ? "bg-rose-500"
                            : b.utilization_pct >= 80
                              ? "bg-amber-500"
                              : "bg-emerald-500"
                        }`}
                        style={{ width: `${Math.min(b.utilization_pct, 100)}%` }}
                      />
                    </div>
                  </div>
                </motion.div>
              );
            })}
          </CardContent>
        </Card>
      </FadeIn>

      {/* --- Reservations ---------------------------------------------- */}
      {reservations.length > 0 && (
        <FadeIn>
          <Card>
            <CardHeader>
              <div className="flex items-center gap-2">
                <Sparkles className="h-4 w-4 text-violet-500" />
                <CardTitle>Upcoming reservations</CardTitle>
              </div>
              <CardDescription>
                Cash the agent has already earmarked for a future event. The
                forecast treats this as committed and your forecast answers
                respect it.
              </CardDescription>
            </CardHeader>
            <CardContent className="grid gap-2 text-sm">
              {reservations.map((r) => (
                <div
                  key={r.id}
                  className="flex items-center justify-between rounded-md border px-3 py-2"
                >
                  <div>
                    <div className="font-medium">{r.label}</div>
                    {r.due_at && (
                      <div className="text-xs text-muted-foreground">
                        Due {new Date(r.due_at).toLocaleDateString()}
                      </div>
                    )}
                  </div>
                  <span className="font-mono">{fenToCny(r.amount_fen)}</span>
                </div>
              ))}
            </CardContent>
          </Card>
        </FadeIn>
      )}

      {/* --- Set-a-budget dialog --------------------------------------- */}
      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Set a budget</DialogTitle>
            <DialogDescription>
              Soft cap per category per 30 days. The chat agent uses it to
              warn before transfers, and the forecaster accounts for it.
            </DialogDescription>
          </DialogHeader>
          <div className="grid gap-3">
            <div>
              <Label>Category</Label>
              <Select value={category} onValueChange={setCategory}>
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {CATEGORIES.map((c) => (
                    <SelectItem key={c} value={c}>
                      {c}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            <div>
              <Label>Monthly cap (CNY)</Label>
              <Input
                type="number"
                min={1}
                value={amount}
                onChange={(e) => setAmount(e.target.value)}
                placeholder="e.g. 2000"
              />
            </div>
            <div>
              <Label>Account (optional)</Label>
              <Select value={accountId} onValueChange={setAccountId}>
                <SelectTrigger>
                  <SelectValue placeholder="All accounts" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="">All accounts</SelectItem>
                  {accts.map((a) => (
                    <SelectItem key={a.id} value={a.id}>
                      {a.display_name} ···{a.account_number_tail}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            <div>
              <Label>Note (optional)</Label>
              <Input
                value={note}
                onChange={(e) => setNote(e.target.value)}
                placeholder="e.g. cutting back before trip"
              />
            </div>
          </div>
          <DialogFooter>
            <Button variant="ghost" onClick={() => setOpen(false)}>
              Cancel
            </Button>
            <Button onClick={onSave} disabled={saving}>
              {saving ? "Saving…" : "Save budget"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
