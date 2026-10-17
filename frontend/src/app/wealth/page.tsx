"use client";

import { useEffect, useState } from "react";
import { motion } from "framer-motion";
import { AppShell } from "@/components/layout/app-shell";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
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
import { accounts, wealth, type Account, type WealthProduct, type WealthHolding } from "@/lib/api";
import { FadeIn, Stagger, StaggerItem } from "@/components/motion/aurora";
import { TierBadge } from "@/components/motion/tier-badge";
import { useToast } from "@/components/ui/toast";
import { fenToCny, formatDate, cnyToFen } from "@/lib/utils";
import { accountsBusApi } from "@/lib/store";
import { TrendingUp } from "lucide-react";

export default function WealthPage() {
  return (
    <AppShell>
      <Body />
    </AppShell>
  );
}

function Body() {
  const [products, setProducts] = useState<WealthProduct[]>([]);
  const [holdings, setHoldings] = useState<WealthHolding[]>([]);
  const [accts, setAccts] = useState<Account[]>([]);
  const [subscribing, setSubscribing] = useState<WealthProduct | null>(null);
  const [redeeming, setRedeeming] = useState<WealthHolding | null>(null);
  const [amount, setAmount] = useState("200");
  const [acctId, setAcctId] = useState<string | undefined>();
  const [busy, setBusy] = useState(false);
  const { toast } = useToast();

  const refreshAll = () => {
    wealth.products().then(setProducts).catch(() => {});
    wealth.holdings().then(setHoldings).catch(() => {});
    accounts.list().then((a) => {
      setAccts(a);
      setAcctId(a[0]?.id);
    });
  };

  useEffect(refreshAll, []);

  const subscribe = async () => {
    if (!subscribing || !acctId) return;
    setBusy(true);
    try {
      await wealth.subscribe({ product_id: subscribing.id, amount, account_id: acctId });
      toast({ variant: "success", title: "Subscribed", description: subscribing.name });
      accountsBusApi.bump("wealth.subscribe");
      setSubscribing(null);
      refreshAll();
    } catch (err) {
      toast({ variant: "error", title: "Subscription failed" });
    } finally {
      setBusy(false);
    }
  };
  const redeem = async () => {
    if (!redeeming || !acctId) return;
    setBusy(true);
    try {
      await wealth.redeem({ holding_id: redeeming.id, account_id: acctId });
      toast({ variant: "success", title: "Redeemed", description: fenToCny(redeeming.amount_fen) });
      accountsBusApi.bump("wealth.redeem");
      setRedeeming(null);
      refreshAll();
    } catch {
      toast({ variant: "error", title: "Redemption failed" });
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="space-y-6">
      <FadeIn>
        <div>
          <p className="text-[11px] font-medium uppercase tracking-widest text-muted-foreground">
            Wealth (simulation)
          </p>
          <h1 className="mt-1 font-display text-3xl font-semibold tracking-tight">Investments</h1>
          <p className="mt-1 max-w-2xl text-sm text-muted-foreground">
            Catalog is read-only. Subscribe / redeem always run through the
            policy engine; suitability is decided server-side, never by the LLM.
          </p>
        </div>
      </FadeIn>

      {holdings.length > 0 && (
        <FadeIn>
          <Card>
            <CardHeader>
              <CardTitle>Your holdings</CardTitle>
              <CardDescription>Active simulated positions</CardDescription>
            </CardHeader>
            <CardContent>
              <ul className="grid gap-3 sm:grid-cols-2">
                {holdings.map((h) => {
                  const p = products.find((x) => x.id === h.product_id);
                  return (
                    <li
                      key={h.id}
                      className="flex items-center justify-between rounded-lg border bg-background/30 p-3"
                    >
                      <div>
                        <div className="font-medium">{p?.name ?? h.product_id}</div>
                        <div className="text-[10px] uppercase tracking-wider text-muted-foreground">
                          since {formatDate(h.subscribed_at)}
                        </div>
                      </div>
                      <div className="flex items-center gap-2">
                        <div className="text-right">
                          <div className="font-semibold">{fenToCny(h.amount_fen)}</div>
                          <Badge variant={h.status === "active" ? "muted" : "green"}>{h.status}</Badge>
                        </div>
                        {h.status === "active" && (
                          <Button size="sm" variant="outline" onClick={() => setRedeeming(h)}>
                            Redeem
                          </Button>
                        )}
                      </div>
                    </li>
                  );
                })}
              </ul>
            </CardContent>
          </Card>
        </FadeIn>
      )}

      <Stagger>
        <div className="grid gap-4 md:grid-cols-3">
          {products.map((p) => (
            <StaggerItem key={p.id}>
              <motion.div whileHover={{ y: -2 }} transition={{ type: "spring", damping: 18, stiffness: 200 }}>
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
                      <TierBadge tier={p.risk_level === "low" ? "Green" : p.risk_level === "medium" ? "Yellow" : "Red"} />
                    </div>
                    <div className="mt-4 flex items-end gap-3">
                      <div>
                        <p className="text-[10px] uppercase tracking-wider text-muted-foreground">
                          Expected return
                        </p>
                        <p className="font-display text-xl font-semibold tracking-tight">
                          {p.expected_annual_return_pct.toFixed(1)}%
                        </p>
                      </div>
                      <div className="flex-1">
                        <p className="text-[10px] uppercase tracking-wider text-muted-foreground">
                          Minimum
                        </p>
                        <p className="text-sm font-medium">
                          {fenToCny(p.min_subscription_fen)}
                        </p>
                      </div>
                      <TrendingUp className="size-4 text-tier-green" />
                    </div>
                    <div className="mt-3 rounded-md border bg-background/40 p-2 text-[11px]">
                      <span className="text-muted-foreground">Suitability:</span>{" "}
                      <span className="font-mono">{p.suitability_required || "open"}</span>
                    </div>
                    <Button
                      className="mt-4 w-full"
                      variant="gradient"
                      onClick={() => {
                        setAmount(String(Math.round((p.min_subscription_fen / 100))));
                        setSubscribing(p);
                      }}
                    >
                      Subscribe (simulate)
                    </Button>
                  </CardContent>
                </Card>
              </motion.div>
            </StaggerItem>
          ))}
        </div>
      </Stagger>

      <Dialog open={!!subscribing} onOpenChange={(o) => !o && setSubscribing(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Subscribe to {subscribing?.name}</DialogTitle>
            <DialogDescription>
              Suitability is decided by the backend. High-risk products require
              an additional MFA challenge.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-3">
            <div className="space-y-1.5">
              <Label htmlFor="amount">Amount (CNY)</Label>
              <Input
                id="amount"
                inputMode="decimal"
                value={amount}
                onChange={(e) => setAmount(e.target.value)}
              />
              <p className="text-[11px] text-muted-foreground">
                Minimum: ¥{(subscribing?.min_subscription_fen ?? 0) / 100}
              </p>
            </div>
            <div className="space-y-1.5">
              <Label>From account</Label>
              <div className="grid gap-1.5">
                {accts.map((a) => (
                  <label
                    key={a.id}
                    className={
                      "flex cursor-pointer items-center justify-between rounded-md border bg-background/40 p-2 text-sm transition-colors " +
                      (acctId === a.id ? "ring-1 ring-primary" : "")
                    }
                  >
                    <span>
                      {a.display_name} <span className="font-mono text-xs text-muted-foreground">{a.masked_number}</span>
                    </span>
                    <span className="font-medium">{fenToCny(a.available_balance_fen)}</span>
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
            <Button variant="ghost" onClick={() => setSubscribing(null)}>
              Cancel
            </Button>
            <Button
              variant="gradient"
              onClick={subscribe}
              disabled={busy || !acctId || cnyToFen(amount) < (subscribing?.min_subscription_fen ?? 0)}
            >
              {busy ? "Subscribing…" : "Confirm"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog open={!!redeeming} onOpenChange={(o) => !o && setRedeeming(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Redeem holding?</DialogTitle>
            <DialogDescription>
              The principal returns to your account as a simulated credit.
            </DialogDescription>
          </DialogHeader>
          <div className="rounded-lg border bg-background/40 p-3 text-sm">
            <div className="flex justify-between">
              <span className="text-muted-foreground">Amount</span>
              <span className="font-medium">
                {redeeming ? fenToCny(redeeming.amount_fen) : "—"}
              </span>
            </div>
            <div className="mt-1 flex justify-between">
              <span className="text-muted-foreground">Subscribed</span>
              <span className="font-medium">
                {redeeming ? formatDate(redeeming.subscribed_at) : "—"}
              </span>
            </div>
          </div>
          <DialogFooter>
            <Button variant="ghost" onClick={() => setRedeeming(null)}>
              Cancel
            </Button>
            <Button variant="gradient" onClick={redeem} disabled={busy}>
              {busy ? "Redeeming…" : "Redeem"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
