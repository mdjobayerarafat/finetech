"use client";

import { useEffect, useState, useCallback } from "react";
import { motion } from "framer-motion";
import {
  TrendingUp,
  RefreshCcw,
  AlertOctagon,
  PiggyBank,
} from "lucide-react";
import { AppShell } from "@/components/layout/app-shell";
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
import { accounts, wealth, ApiError, type Account, type WealthProduct, type WealthHolding } from "@/lib/api";
import { FadeIn, Stagger, StaggerItem } from "@/components/motion/aurora";
import { TierBadge } from "@/components/motion/tier-badge";
import { useToast } from "@/components/ui/toast";
import { fenToCny, formatDate, cnyToFen, cn } from "@/lib/utils";
import { accountsBusApi } from "@/lib/store";
import { GoalsPanel } from "@/components/wealth/goals-panel";
import {
  PageHeader,
  EmptyState,
  LoadingState,
  ErrorState,
  SectionCard,
} from "@/components/ui/feedback";
import { useT } from "@/i18n/LanguageProvider";
import { usePageTitle } from "@/lib/use-page-title";

export default function WealthPage() {
  return (
    <AppShell>
      <Body />
    </AppShell>
  );
}

function Body() {
  const t = useT();
  usePageTitle("meta.wealth.title", "Investments");
  const [products, setProducts] = useState<WealthProduct[] | null>(null);
  const [holdings, setHoldings] = useState<WealthHolding[]>([]);
  const [accts, setAccts] = useState<Account[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [subscribing, setSubscribing] = useState<WealthProduct | null>(null);
  const [redeeming, setRedeeming] = useState<WealthHolding | null>(null);
  const [amount, setAmount] = useState("200");
  const [acctId, setAcctId] = useState<string | undefined>();
  const [busy, setBusy] = useState(false);
  const { toast } = useToast();

  const refreshAll = useCallback(() => {
    setError(null);
    Promise.all([
      wealth.products(),
      wealth.holdings(),
      accounts.list(),
    ])
      .then(([p, h, a]) => {
        setProducts(p);
        setHoldings(h);
        setAccts(a);
        if (!acctId && a[0]?.id) setAcctId(a[0].id);
      })
      .catch((e) => {
        setError(e instanceof ApiError ? e.message : "Could not load wealth");
        setProducts([]);
      });
  }, [acctId]);

  useEffect(() => {
    refreshAll();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const [goalsTick, setGoalsTick] = useState(0);

  const subscribe = async () => {
    if (!subscribing || !acctId) return;
    setBusy(true);
    try {
      await wealth.subscribe({
        product_id: subscribing.id,
        amount,
        account_id: acctId,
      });
      toast({
        variant: "success",
        title: t("wealth.toast.subscribed"),
        description: t("wealth.toast.subscribed.body", { name: subscribing.name }),
      });
      accountsBusApi.bump("wealth.subscribe");
      setSubscribing(null);
      setGoalsTick((g) => g + 1);
      refreshAll();
    } catch (err) {
      toast({
        variant: "error",
        title: t("wealth.toast.subFail"),
        description: err instanceof ApiError ? err.message : undefined,
      });
    } finally {
      setBusy(false);
    }
  };

  const redeem = async () => {
    if (!redeeming || !acctId) return;
    setBusy(true);
    try {
      const out = await wealth.redeem({
        holding_id: redeeming.id,
        account_id: acctId,
      });
      toast({
        variant: "success",
        title: t("wealth.toast.redeemed"),
        description: t("wealth.toast.redeemed.body", {
          amount: fenToCny(out.amount_fen),
        }),
      });
      accountsBusApi.bump("wealth.redeem");
      setRedeeming(null);
      refreshAll();
    } catch (err) {
      toast({
        variant: "error",
        title: t("wealth.toast.redeemFail"),
        description: err instanceof ApiError ? err.message : undefined,
      });
    } finally {
      setBusy(false);
    }
  };

  const isLoading = products === null && error === null;

  return (
    <div className="space-y-6">
      <FadeIn>
        <PageHeader
          eyebrow={t("wealth.page.eyebrow")}
          title={t("wealth.page.title")}
          description={t("wealth.page.subtitle")}
          actions={
            <Button
              variant="outline"
              size="sm"
              onClick={refreshAll}
              disabled={isLoading}
            >
              <RefreshCcw className="size-3.5" />
              {t("common.retry")}
            </Button>
          }
        />
      </FadeIn>

      <FadeIn>
        <GoalsPanel refreshKey={goalsTick} />
      </FadeIn>

      {isLoading ? (
        <LoadingState />
      ) : error ? (
        <ErrorState
          title={t("common.error.title")}
          body={error}
          onRetry={refreshAll}
          retryLabel={t("common.retry")}
        />
      ) : (
        <>
          {holdings.length > 0 && (
            <FadeIn>
              <SectionCard
                title={t("wealth.holdings.title")}
                description={t("wealth.holdings.body")}
              >
                <ul className="grid gap-3 sm:grid-cols-2">
                  {holdings.map((h) => {
                    const p = products?.find((x) => x.id === h.product_id);
                    return (
                      <li
                        key={h.id}
                        className="flex items-center justify-between rounded-lg border bg-background/30 p-3"
                      >
                        <div>
                          <div className="font-medium">
                            {p?.name ?? h.product_id}
                          </div>
                          <div className="text-[10px] uppercase tracking-wider text-muted-foreground">
                            {t("wealth.holdings.since", {
                              date: formatDate(h.subscribed_at),
                            })}
                          </div>
                        </div>
                        <div className="flex items-center gap-2">
                          <div className="text-right">
                            <div className="font-semibold">
                              {fenToCny(h.amount_fen)}
                            </div>
                            <Badge variant={h.status === "active" ? "muted" : "green"}>
                              {h.status}
                            </Badge>
                          </div>
                          {h.status === "active" && (
                            <Button
                              size="sm"
                              variant="outline"
                              onClick={() => setRedeeming(h)}
                            >
                              {t("wealth.action.redeem")}
                            </Button>
                          )}
                        </div>
                      </li>
                    );
                  })}
                </ul>
              </SectionCard>
            </FadeIn>
          )}

          {products && products.length === 0 ? (
            <EmptyState
              icon={PiggyBank}
              title={t("wealth.empty.title")}
              body={t("wealth.empty.body")}
            />
          ) : (
            <Stagger>
              <div className="grid gap-4 md:grid-cols-3">
                {(products ?? []).map((p) => (
                  <StaggerItem key={p.id}>
                    <ProductCard
                      p={p}
                      onSubscribe={() => {
                        setAmount(
                          String(Math.round(p.min_subscription_fen / 100)),
                        );
                        setSubscribing(p);
                      }}
                    />
                  </StaggerItem>
                ))}
              </div>
            </Stagger>
          )}
        </>
      )}

      <SubscribeDialog
        product={subscribing}
        onClose={() => setSubscribing(null)}
        accts={accts}
        acctId={acctId}
        setAcctId={setAcctId}
        amount={amount}
        setAmount={setAmount}
        busy={busy}
        onSubmit={subscribe}
      />

      <RedeemDialog
        holding={redeeming}
        onClose={() => setRedeeming(null)}
        busy={busy}
        onSubmit={redeem}
      />
    </div>
  );
}

function ProductCard({
  p,
  onSubscribe,
}: {
  p: WealthProduct;
  onSubscribe: () => void;
}) {
  const t = useT();
  const tier = p.risk_level === "low" ? "Green" : p.risk_level === "medium" ? "Yellow" : "Red";
  return (
    <motion.div
      whileHover={{ y: -2 }}
      transition={{ type: "spring", damping: 18, stiffness: 200 }}
    >
      <Card>
        <CardContent className="p-5">
          <div className="flex items-start justify-between">
            <div>
              <p className="text-[10px] uppercase tracking-widest text-muted-foreground">
                {p.category.replace("_", " ")}
              </p>
              <h3 className="mt-1 font-display text-lg font-semibold tracking-tight">
                {p.name}
              </h3>
            </div>
            <TierBadge tier={tier} />
          </div>
          <div className="mt-4 flex items-end gap-3">
            <div>
              <p className="text-[10px] uppercase tracking-wider text-muted-foreground">
                {t("wealth.product.expectedReturn")}
              </p>
              <p className="font-display text-xl font-semibold tracking-tight">
                {p.expected_annual_return_pct.toFixed(1)}%
              </p>
            </div>
            <div className="flex-1">
              <p className="text-[10px] uppercase tracking-wider text-muted-foreground">
                {t("wealth.product.minimum")}
              </p>
              <p className="text-sm font-medium">
                {fenToCny(p.min_subscription_fen)}
              </p>
            </div>
            <TrendingUp
              className={cn(
                "size-4",
                tier === "Green"
                  ? "text-tier-green"
                  : tier === "Yellow"
                    ? "text-tier-yellow"
                    : "text-tier-red",
              )}
            />
          </div>
          <div className="mt-3 rounded-md border bg-background/40 p-2 text-[11px]">
            <span className="text-muted-foreground">
              {t("wealth.product.suitability")}:
            </span>{" "}
            <span className="font-mono">
              {p.suitability_required || t("wealth.product.suitability.open")}
            </span>
          </div>
          <Button className="mt-4 w-full" variant="gradient" onClick={onSubscribe}>
            {t("wealth.product.subscribe")}
          </Button>
        </CardContent>
      </Card>
    </motion.div>
  );
}

function SubscribeDialog({
  product,
  onClose,
  accts,
  acctId,
  setAcctId,
  amount,
  setAmount,
  busy,
  onSubmit,
}: {
  product: WealthProduct | null;
  onClose: () => void;
  accts: Account[];
  acctId?: string;
  setAcctId: (v?: string) => void;
  amount: string;
  setAmount: (v: string) => void;
  busy: boolean;
  onSubmit: () => void;
}) {
  const t = useT();
  const open = !!product;
  return (
    <Dialog open={open} onOpenChange={(o) => !o && onClose()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>
            {t("wealth.dialog.subscribe.title", { name: product?.name ?? "" })}
          </DialogTitle>
          <DialogDescription>{t("wealth.dialog.subscribe.body")}</DialogDescription>
        </DialogHeader>
        <div className="space-y-3">
          <div className="space-y-1.5">
            <Label htmlFor="amount">{t("wealth.dialog.subscribe.amount")}</Label>
            <Input
              id="amount"
              inputMode="decimal"
              value={amount}
              onChange={(e) => setAmount(e.target.value)}
            />
            <p className="text-[11px] text-muted-foreground">
              {t("wealth.dialog.subscribe.minimum", {
                amount: (product?.min_subscription_fen ?? 0) / 100,
              })}
            </p>
          </div>
          <div className="space-y-1.5">
            <Label>{t("wealth.dialog.subscribe.fromAccount")}</Label>
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
                  <span className="font-medium">
                    {fenToCny(a.available_balance_fen)}
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
            onClick={onSubmit}
            disabled={
              busy ||
              !acctId ||
              cnyToFen(amount) < (product?.min_subscription_fen ?? 0)
            }
          >
            {busy ? t("wealth.dialog.subscribe.busy") : t("wealth.dialog.subscribe.cta")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function RedeemDialog({
  holding,
  onClose,
  busy,
  onSubmit,
}: {
  holding: WealthHolding | null;
  onClose: () => void;
  busy: boolean;
  onSubmit: () => void;
}) {
  const t = useT();
  return (
    <Dialog open={!!holding} onOpenChange={(o) => !o && onClose()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t("wealth.dialog.redeem.title")}</DialogTitle>
          <DialogDescription>{t("wealth.dialog.redeem.body")}</DialogDescription>
        </DialogHeader>
        <div className="rounded-lg border bg-background/40 p-3 text-sm">
          <div className="flex justify-between">
            <span className="text-muted-foreground">
              {t("wealth.dialog.redeem.amount")}
            </span>
            <span className="font-medium">
              {holding ? fenToCny(holding.amount_fen) : "—"}
            </span>
          </div>
          <div className="mt-1 flex justify-between">
            <span className="text-muted-foreground">
              {t("wealth.dialog.redeem.subscribed")}
            </span>
            <span className="font-medium">
              {holding ? formatDate(holding.subscribed_at) : "—"}
            </span>
          </div>
        </div>
        <DialogFooter>
          <Button variant="ghost" onClick={onClose} disabled={busy}>
            {t("common.cancel")}
          </Button>
          <Button variant="gradient" onClick={onSubmit} disabled={busy}>
            {busy ? t("wealth.dialog.redeem.busy") : t("wealth.dialog.redeem.cta")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}