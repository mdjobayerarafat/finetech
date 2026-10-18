"use client";

import { useEffect, useState, useCallback } from "react";
import { motion } from "framer-motion";
import {
  Bell,
  BellOff,
  Trash2,
  RefreshCcw,
  Clock,
  Sparkles,
} from "lucide-react";
import { AppShell } from "@/components/layout/app-shell";
import { Card, CardContent } from "@/components/ui/card";
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
import {
  subscriptions,
  ApiError,
  type Subscription,
} from "@/lib/api";
import { FadeIn, Stagger, StaggerItem } from "@/components/motion/aurora";
import { fenToCny, formatDate } from "@/lib/utils";
import { accountsBusApi } from "@/lib/store";
import { useToast } from "@/components/ui/toast";
import {
  PageHeader,
  EmptyState,
  LoadingState,
  ErrorState,
  SectionCard,
} from "@/components/ui/feedback";
import { useT } from "@/i18n/LanguageProvider";
import { usePageTitle } from "@/lib/use-page-title";

export default function SubscriptionsPage() {
  return (
    <AppShell>
      <Body />
    </AppShell>
  );
}

function Body() {
  usePageTitle("meta.subscriptions.title", "Subscriptions");
  const [list, setList] = useState<Subscription[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [reminded, setReminded] = useState<Record<string, boolean>>({});
  const [confirming, setConfirming] = useState<Subscription | null>(null);
  const [busy, setBusy] = useState(false);
  const { toast } = useToast();
  const t = useT();

  const refresh = useCallback(() => {
    setError(null);
    subscriptions
      .list()
      .then((rows) => setList(rows))
      .catch((e) => {
        setError(
          e instanceof ApiError ? e.message : "Could not load subscriptions",
        );
        setList([]);
      });
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  const onRemind = async (s: Subscription) => {
    try {
      await subscriptions.remind(s.id);
      setReminded((r) => ({ ...r, [s.id]: true }));
      toast({
        variant: "success",
        title: t("subs.toast.remind"),
        description: t("subs.toast.remind.body", { merchant: s.merchant }),
      });
    } catch {
      toast({ variant: "error", title: t("common.error.title") });
    }
  };

  const onSnooze = async (s: Subscription, days: number) => {
    try {
      const out = await subscriptions.snooze(s.id, days);
      toast({
        variant: "success",
        title: t("subs.toast.snooze"),
        description: t("subs.toast.snooze.body", {
          merchant: s.merchant,
          date: out.next_expected_at.slice(0, 10),
        }),
      });
      refresh();
    } catch (e) {
      toast({
        variant: "error",
        title: t("subs.toast.snoozeFail"),
        description: e instanceof ApiError ? e.message : undefined,
      });
    }
  };

  const onDefer = async (s: Subscription, reason: string) => {
    try {
      const out = await subscriptions.defer(s.id, reason);
      toast({
        variant: "success",
        title: t("subs.toast.defer"),
        description: t("subs.toast.defer.body", {
          merchant: s.merchant,
          date: out.new_next_expected_at.slice(0, 10),
        }),
      });
      refresh();
    } catch (e) {
      toast({
        variant: "error",
        title: t("subs.toast.deferFail"),
        description: e instanceof ApiError ? e.message : undefined,
      });
    }
  };

  const onUncancel = async (s: Subscription) => {
    try {
      await subscriptions.uncancel(s.id);
      toast({
        variant: "success",
        title: t("subs.toast.uncancel"),
        description: t("subs.toast.uncancel.body", { merchant: s.merchant }),
      });
      refresh();
    } catch {
      toast({ variant: "error", title: t("subs.toast.uncancelFail") });
    }
  };

  const onCancel = async () => {
    if (!confirming) return;
    setBusy(true);
    try {
      await subscriptions.cancel(confirming.id);
      toast({
        variant: "success",
        title: t("subs.toast.cancel"),
        description: t("subs.toast.cancel.body", { merchant: confirming.merchant }),
      });
      accountsBusApi.bump("subscription.cancel");
      setConfirming(null);
      refresh();
    } catch {
      toast({ variant: "error", title: t("subs.toast.cancelFail") });
    } finally {
      setBusy(false);
    }
  };

  const isLoading = list === null && error === null;
  const active = (list ?? []).filter(
    (s) => s.cancellation_status === "active",
  );
  const cancelled = (list ?? []).filter(
    (s) => s.cancellation_status !== "active",
  );

  return (
    <div className="space-y-6">
      <FadeIn>
        <PageHeader
          eyebrow={t("subs.page.eyebrow")}
          title={t("subs.page.title")}
          description={t("subs.page.subtitle")}
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
      ) : active.length === 0 ? (
        <EmptyState
          icon={Sparkles}
          title={t("subs.empty.title")}
          body={t("subs.empty.body")}
        />
      ) : (
        <Stagger>
          <div className="grid gap-4 md:grid-cols-2">
            {active.map((s) => (
              <StaggerItem key={s.id}>
                <SubscriptionCard
                  s={s}
                  onRemind={() => onRemind(s)}
                  onCancel={() => setConfirming(s)}
                  onSnooze={(days) => onSnooze(s, days)}
                  onDefer={(reason) => onDefer(s, reason)}
                  onUncancel={() => onUncancel(s)}
                  reminded={!!reminded[s.id]}
                />
              </StaggerItem>
            ))}
          </div>
        </Stagger>
      )}

      {!isLoading && !error && cancelled.length > 0 && (
        <FadeIn delay={0.2}>
          <SectionCard
            title={t("subs.page.cancelled.title")}
            description={t("subs.page.cancelled.body")}
          >
            <ul className="grid gap-2 sm:grid-cols-2">
              {cancelled.map((s) => (
                <li
                  key={s.id}
                  className="flex items-center justify-between rounded-md border bg-background/30 p-3 text-sm"
                >
                  <div>
                    <div className="font-medium">{s.merchant}</div>
                    <div className="text-[10px] uppercase tracking-wider text-muted-foreground">
                      {t(`subs.status.${s.cancellation_status}` as never)}
                    </div>
                  </div>
                  <div className="font-semibold">
                    {fenToCny(s.estimated_amount_fen)}
                  </div>
                </li>
              ))}
            </ul>
          </SectionCard>
        </FadeIn>
      )}

      <Dialog open={!!confirming} onOpenChange={(o) => !o && setConfirming(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>
              {t("subs.dialog.cancel.title", { merchant: confirming?.merchant ?? "" })}
            </DialogTitle>
            <DialogDescription>{t("subs.dialog.cancel.body")}</DialogDescription>
          </DialogHeader>
          <div className="rounded-lg border bg-background/40 p-3 text-sm">
            <div className="flex justify-between">
              <span className="text-muted-foreground">
                {t("subs.dialog.cancel.merchant")}
              </span>
              <span className="font-medium">{confirming?.merchant}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-muted-foreground">
                {t("subs.dialog.cancel.charge")}
              </span>
              <span className="font-medium">
                {confirming ? fenToCny(confirming.estimated_amount_fen) : "—"}
              </span>
            </div>
            <div className="flex justify-between">
              <span className="text-muted-foreground">
                {t("subs.dialog.cancel.next")}
              </span>
              <span className="font-medium">
                {confirming ? formatDate(confirming.next_expected_at) : "—"}
              </span>
            </div>
          </div>
          <DialogFooter>
            <Button variant="ghost" onClick={() => setConfirming(null)} disabled={busy}>
              {t("common.keep")}
            </Button>
            <Button variant="destructive" onClick={onCancel} disabled={busy}>
              {busy
                ? t("subs.dialog.cancel.busy")
                : t("subs.dialog.cancel.cta")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}

function SubscriptionCard({
  s,
  onRemind,
  onCancel,
  onSnooze,
  onDefer,
  onUncancel,
  reminded,
}: {
  s: Subscription;
  onRemind: () => void;
  onCancel: () => void;
  onSnooze: (days: number) => void;
  onDefer: (reason: string) => void;
  onUncancel: () => void;
  reminded: boolean;
}) {
  const t = useT();
  return (
    <motion.div whileHover={{ y: -2 }} transition={{ type: "spring", damping: 18, stiffness: 200 }}>
      <Card>
        <CardContent className="p-5">
          <div className="flex items-start justify-between">
            <div>
              <h3 className="font-display text-lg font-semibold tracking-tight">
                {s.merchant}
              </h3>
              <p className="mt-0.5 inline-flex items-center gap-1 text-xs text-muted-foreground">
                <Clock className="size-3" />
                {t("subs.card.interval", { n: s.interval_days })}
                <span className="text-muted-foreground/40">·</span>
                {t("subs.card.next", { date: formatDate(s.next_expected_at) })}
              </p>
            </div>
            <Badge variant={s.cancellation_status === "active" ? "muted" : "green"}>
              {t(`subs.status.${s.cancellation_status}` as never)}
            </Badge>
          </div>
          <div className="mt-4 flex items-end justify-between">
            <div className="font-display text-2xl font-semibold tracking-tight">
              {fenToCny(s.estimated_amount_fen)}
            </div>
            <div className="text-[10px] uppercase tracking-wider text-muted-foreground">
              {s.last_charge_at
                ? t("subs.card.last", { date: formatDate(s.last_charge_at) })
                : t("subs.card.never")}
            </div>
          </div>
          <div className="mt-4 grid grid-cols-2 gap-2 sm:flex sm:flex-wrap">
            <Button variant="outline" size="sm" onClick={onRemind} className="w-full sm:flex-1">
              {reminded ? <BellOff className="size-3.5" /> : <Bell className="size-3.5" />}
              {reminded ? t("subs.action.reminded") : t("subs.action.remind")}
            </Button>
            {s.cancellation_status === "active" ? (
              <>
                <Button
                  variant="secondary"
                  size="sm"
                  onClick={() => onSnooze(7)}
                  className="w-full sm:flex-1"
                >
                  {t("subs.action.snooze7")}
                </Button>
                <Button
                  variant="secondary"
                  size="sm"
                  onClick={() => onDefer("")}
                  className="w-full sm:flex-1"
                >
                  {t("subs.action.skipNext")}
                </Button>
                <Button
                  variant="destructive"
                  size="sm"
                  onClick={onCancel}
                  className="w-full sm:flex-1"
                >
                  <Trash2 className="size-3.5" />
                  {t("subs.action.cancel")}
                </Button>
              </>
            ) : (
              <Button
                variant="secondary"
                size="sm"
                onClick={onUncancel}
                className="w-full sm:flex-1"
              >
                {t("subs.action.uncancel")}
              </Button>
            )}
          </div>
        </CardContent>
      </Card>
    </motion.div>
  );
}