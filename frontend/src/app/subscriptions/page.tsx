"use client";

import { useEffect, useState } from "react";
import { motion } from "framer-motion";
import { AppShell } from "@/components/layout/app-shell";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { subscriptions, type Subscription } from "@/lib/api";
import { FadeIn, Stagger, StaggerItem } from "@/components/motion/aurora";
import { fenToCny, formatDate } from "@/lib/utils";
import { accountsBusApi } from "@/lib/store";
import { Bell, BellOff, Trash2 } from "lucide-react";
import { useToast } from "@/components/ui/toast";

export default function SubscriptionsPage() {
  return (
    <AppShell>
      <Body />
    </AppShell>
  );
}

function Body() {
  const [list, setList] = useState<Subscription[]>([]);
  const [reminded, setReminded] = useState<Record<string, boolean>>({});
  const [confirming, setConfirming] = useState<Subscription | null>(null);
  const [busy, setBusy] = useState(false);
  const { toast } = useToast();

  const refresh = () => subscriptions.list().then(setList).catch(() => {});

  useEffect(() => {
    refresh();
  }, []);

  const onRemind = async (s: Subscription) => {
    try {
      await subscriptions.remind(s.id);
      setReminded((r) => ({ ...r, [s.id]: true }));
      toast({ variant: "success", title: "Reminder set", description: `${s.merchant} renews soon.` });
    } catch (err) {
      toast({ variant: "error", title: "Failed to set reminder" });
    }
  };

  const onCancel = async () => {
    if (!confirming) return;
    setBusy(true);
    try {
      await subscriptions.cancel(confirming.id);
      toast({ variant: "success", title: "Cancellation sent", description: `${confirming.merchant} → mock merchant.` });
      accountsBusApi.bump("subscription.cancel");
      setConfirming(null);
      refresh();
    } catch (err) {
      toast({ variant: "error", title: "Cancellation failed" });
    } finally {
      setBusy(false);
    }
  };

  const active = list.filter((s) => s.cancellation_status === "active");
  const cancelled = list.filter((s) => s.cancellation_status !== "active");

  return (
    <div className="space-y-6">
      <FadeIn>
        <div>
          <p className="text-[11px] font-medium uppercase tracking-widest text-muted-foreground">
            Subscriptions
          </p>
          <h1 className="mt-1 font-display text-3xl font-semibold tracking-tight">
            What am I paying for?
          </h1>
          <p className="mt-1 max-w-2xl text-sm text-muted-foreground">
            Detected by recurring charge patterns. Cancellations go through a
            mock merchant connector — they are simulations, not real bank API
            calls.
          </p>
        </div>
      </FadeIn>

      <Stagger>
        <div className="grid gap-4 md:grid-cols-2">
          {active.map((s) => (
            <StaggerItem key={s.id}>
              <SubscriptionCard
                s={s}
                onRemind={() => onRemind(s)}
                onCancel={() => setConfirming(s)}
                reminded={!!reminded[s.id]}
              />
            </StaggerItem>
          ))}
        </div>
        {active.length === 0 && (
          <FadeIn delay={0.1}>
            <Card>
              <CardContent className="grid h-32 place-items-center text-sm text-muted-foreground">
                No active subscriptions.
              </CardContent>
            </Card>
          </FadeIn>
        )}
      </Stagger>

      {cancelled.length > 0 && (
        <FadeIn delay={0.2}>
          <Card>
            <CardHeader>
              <CardTitle>Cancelled</CardTitle>
              <CardDescription>Already sent to the mock merchant connector.</CardDescription>
            </CardHeader>
            <CardContent>
              <ul className="grid gap-2 sm:grid-cols-2">
                {cancelled.map((s) => (
                  <li
                    key={s.id}
                    className="flex items-center justify-between rounded-md border bg-background/30 p-3 text-sm"
                  >
                    <div>
                      <div className="font-medium">{s.merchant}</div>
                      <div className="text-[10px] uppercase tracking-wider text-muted-foreground">
                        {s.cancellation_status}
                      </div>
                    </div>
                    <div className="font-semibold">{fenToCny(s.estimated_amount_fen)}</div>
                  </li>
                ))}
              </ul>
            </CardContent>
          </Card>
        </FadeIn>
      )}

      <Dialog open={!!confirming} onOpenChange={(o) => !o && setConfirming(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Cancel {confirming?.merchant}?</DialogTitle>
            <DialogDescription>
              This will run a Yellow-tier action through the mock merchant
              connector. The next charge should not occur after this completes.
            </DialogDescription>
          </DialogHeader>
          <div className="rounded-lg border bg-background/40 p-3 text-sm">
            <div className="flex justify-between">
              <span className="text-muted-foreground">Merchant</span>
              <span className="font-medium">{confirming?.merchant}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-muted-foreground">Charge</span>
              <span className="font-medium">
                {confirming ? fenToCny(confirming.estimated_amount_fen) : "—"}
              </span>
            </div>
            <div className="flex justify-between">
              <span className="text-muted-foreground">Next expected</span>
              <span className="font-medium">{confirming ? formatDate(confirming.next_expected_at) : "—"}</span>
            </div>
          </div>
          <DialogFooter>
            <Button variant="ghost" onClick={() => setConfirming(null)} disabled={busy}>
              Keep
            </Button>
            <Button variant="destructive" onClick={onCancel} disabled={busy}>
              {busy ? "Cancelling…" : "Cancel subscription"}
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
  reminded,
}: {
  s: Subscription;
  onRemind: () => void;
  onCancel: () => void;
  reminded: boolean;
}) {
  return (
    <motion.div whileHover={{ y: -2 }} transition={{ type: "spring", damping: 18, stiffness: 200 }}>
      <Card>
        <CardContent className="p-5">
          <div className="flex items-start justify-between">
            <div>
              <h3 className="font-display text-lg font-semibold tracking-tight">{s.merchant}</h3>
              <p className="mt-0.5 text-xs text-muted-foreground">
                every {s.interval_days} days · next {formatDate(s.next_expected_at)}
              </p>
            </div>
            <Badge variant={s.cancellation_status === "active" ? "muted" : "green"}>
              {s.cancellation_status}
            </Badge>
          </div>
          <div className="mt-4 flex items-end justify-between">
            <div className="font-display text-2xl font-semibold tracking-tight">
              {fenToCny(s.estimated_amount_fen)}
            </div>
            <div className="text-[10px] uppercase tracking-wider text-muted-foreground">
              {s.last_charge_at ? `last ${formatDate(s.last_charge_at)}` : "no charges yet"}
            </div>
          </div>
          <div className="mt-4 flex gap-2">
            <Button variant="outline" size="sm" onClick={onRemind} className="flex-1">
              {reminded ? <BellOff className="size-3.5" /> : <Bell className="size-3.5" />}
              {reminded ? "Reminder set" : "Remind me"}
            </Button>
            <Button variant="destructive" size="sm" onClick={onCancel}>
              <Trash2 className="size-3.5" />
              Cancel
            </Button>
          </div>
        </CardContent>
      </Card>
    </motion.div>
  );
}
