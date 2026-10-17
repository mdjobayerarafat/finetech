"use client";

import { useEffect, useState } from "react";
import { motion } from "framer-motion";
import { CreditCard, Snowflake, Unlock, RotateCcw, AlertOctagon, Plus } from "lucide-react";
import { AppShell } from "@/components/layout/app-shell";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { cards, ApiError, type Card as VirtualCard } from "@/lib/api";
import { FadeIn, Stagger, StaggerItem } from "@/components/motion/aurora";
import { useToast } from "@/components/ui/toast";
import { fenToCny } from "@/lib/utils";
import { cn } from "@/lib/utils";

export default function CardsPage() {
  return (
    <AppShell>
      <Body />
    </AppShell>
  );
}

function Body() {
  const [list, setList] = useState<VirtualCard[]>([]);
  const [confirming, setConfirming] = useState<{ card: VirtualCard; action: "lost" | "cancel" } | null>(null);
  const [busy, setBusy] = useState(false);
  const [showIssue, setShowIssue] = useState(false);
  const [issueName, setIssueName] = useState("Travel card");
  const { toast } = useToast();

  const refresh = () => cards.list().then(setList).catch(() => {});

  useEffect(() => {
    refresh();
  }, []);

  const act = async (c: VirtualCard, action: "freeze" | "unfreeze" | "cancel") => {
    try {
      const next =
        action === "freeze"
          ? await cards.freeze(c.id)
          : action === "unfreeze"
          ? await cards.unfreeze(c.id)
          : await cards.cancel(c.id);
      setList((prev) => prev.map((x) => (x.id === next.id ? next : x)));
      toast({
        variant: "success",
        title: `Card ${action === "unfreeze" ? "unfrozen" : action + "d"}`,
        description: next.masked_number,
      });
    } catch (err) {
      const msg = err instanceof ApiError ? err.message : "Action failed";
      toast({ variant: "error", title: "Action failed", description: msg });
    }
  };

  const reportLost = async () => {
    if (!confirming) return;
    setBusy(true);
    try {
      const next = await cards.reportLost(confirming.card.id);
      setList((prev) => prev.map((x) => (x.id === next.id ? next : x)));
      toast({ variant: "success", title: "Card reported lost" });
      setConfirming(null);
    } catch (err) {
      toast({ variant: "error", title: "Failed" });
    } finally {
      setBusy(false);
    }
  };

  const issue = async () => {
    setBusy(true);
    try {
      const next = await cards.issue({ display_name: issueName });
      setList((prev) => [...prev, next]);
      toast({ variant: "success", title: "Card issued", description: next.masked_number });
      setShowIssue(false);
      setIssueName("Travel card");
    } catch (err) {
      toast({ variant: "error", title: "Issue failed" });
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="space-y-6">
      <FadeIn>
        <div className="flex items-end justify-between gap-3">
          <div>
            <p className="text-[11px] font-medium uppercase tracking-widest text-muted-foreground">
              Virtual cards
            </p>
            <h1 className="mt-1 font-display text-3xl font-semibold tracking-tight">Cards</h1>
            <p className="mt-1 max-w-2xl text-sm text-muted-foreground">
              Lifecycle in simulation. Issue a card, freeze it while
              travelling, cancel when retired.
            </p>
          </div>
          <Button variant="gradient" onClick={() => setShowIssue(true)}>
            <Plus className="size-4" />
            Issue new card
          </Button>
        </div>
      </FadeIn>

      <Stagger>
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {list.map((c) => (
            <StaggerItem key={c.id}>
              <CardVisual card={c} />
              <div className="mt-3 grid grid-cols-3 gap-1.5">
                {c.status === "active" && (
                  <Button size="sm" variant="outline" onClick={() => act(c, "freeze")}>
                    <Snowflake className="size-3.5" /> Freeze
                  </Button>
                )}
                {c.status === "frozen" && (
                  <Button size="sm" variant="outline" onClick={() => act(c, "unfreeze")}>
                    <Unlock className="size-3.5" /> Unfreeze
                  </Button>
                )}
                <Button
                  size="sm"
                  variant="destructive"
                  onClick={() => setConfirming({ card: c, action: "lost" })}
                >
                  <AlertOctagon className="size-3.5" /> Lost
                </Button>
                <Button
                  size="sm"
                  variant="ghost"
                  onClick={() => act(c, "cancel")}
                  disabled={c.status === "cancelled"}
                >
                  <RotateCcw className="size-3.5" /> Cancel
                </Button>
              </div>
            </StaggerItem>
          ))}
          {list.length === 0 && (
            <Card className="sm:col-span-2 lg:col-span-3">
              <CardContent className="grid h-40 place-items-center text-sm text-muted-foreground">
                No virtual cards yet. Click <strong className="mx-1">Issue new card</strong> to start.
              </CardContent>
            </Card>
          )}
        </div>
      </Stagger>

      <Dialog open={showIssue} onOpenChange={setShowIssue}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Issue a new virtual card</DialogTitle>
            <DialogDescription>
              Yellow-tier action. A new card row is created in simulation
              with the chosen display name.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-2">
            <Label htmlFor="name">Display name</Label>
            <Input
              id="name"
              value={issueName}
              onChange={(e) => setIssueName(e.target.value)}
              placeholder="Travel card"
            />
          </div>
          <DialogFooter>
            <Button variant="ghost" onClick={() => setShowIssue(false)}>
              Cancel
            </Button>
            <Button variant="gradient" onClick={issue} disabled={busy || !issueName.trim()}>
              {busy ? "Issuing…" : "Issue"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog open={!!confirming} onOpenChange={(o) => !o && setConfirming(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Report card as lost?</DialogTitle>
            <DialogDescription>
              This is a Red-tier action. The card will be marked lost and
              recorded in the audit log.
            </DialogDescription>
          </DialogHeader>
          <div className="rounded-lg border bg-background/40 p-3 text-sm">
            <div className="flex justify-between">
              <span className="text-muted-foreground">Card</span>
              <span className="font-medium">{confirming?.card.masked_number}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-muted-foreground">Display name</span>
              <span className="font-medium">{confirming?.card.display_name}</span>
            </div>
          </div>
          <DialogFooter>
            <Button variant="ghost" onClick={() => setConfirming(null)} disabled={busy}>
              Keep card
            </Button>
            <Button variant="destructive" onClick={reportLost} disabled={busy}>
              {busy ? "Reporting…" : "Report lost"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}

function CardVisual({ card }: { card: VirtualCard }) {
  return (
    <motion.div whileHover={{ y: -4, rotate: -0.5 }} transition={{ type: "spring", damping: 18, stiffness: 200 }}>
      <div
        className={cn(
          "relative h-44 overflow-hidden rounded-2xl p-5 text-white shadow-[0_24px_60px_-20px_rgba(0,0,0,0.5)]",
          "bg-[conic-gradient(at_top_left,hsl(263_80%_55%),hsl(220_80%_45%),hsl(199_89%_50%),hsl(263_80%_55%))]"
        )}
      >
        <div className="absolute inset-0 -z-0 opacity-50 mix-blend-overlay bg-noise" />
        <div className="relative z-10 flex h-full flex-col justify-between">
          <div className="flex items-start justify-between">
            <div>
              <div className="text-[10px] uppercase tracking-widest opacity-80">
                {`FinTech Pioneers · Virtual`}
              </div>
              <div className="mt-1 font-display text-base font-semibold leading-tight">
                {card.display_name}
              </div>
            </div>
            <CreditCard className="size-7 opacity-80" />
          </div>
          <div>
            <div className="font-mono text-xl tracking-widest">{card.masked_number}</div>
            <div className="mt-2 flex items-end justify-between">
              <div className="font-mono text-sm opacity-80">
                EXP {String(card.expiry_month).padStart(2, "0")}/{String(card.expiry_year).slice(-2)}
              </div>
              <Badge
                variant={
                  card.status === "active"
                    ? "green"
                    : card.status === "frozen"
                    ? "yellow"
                    : card.status === "cancelled" || card.status === "lost"
                    ? "red"
                    : "muted"
                }
              >
                {card.status}
              </Badge>
            </div>
          </div>
        </div>
      </div>
      <div className="mt-3 grid grid-cols-2 gap-2 rounded-xl border bg-card/60 p-3 text-[11px]">
        <div>
          <div className="text-[10px] uppercase tracking-wider text-muted-foreground">
            Single-tx limit
          </div>
          <div className="font-medium">{fenToCny(card.single_tx_limit_fen ?? 0)}</div>
        </div>
        <div>
          <div className="text-[10px] uppercase tracking-wider text-muted-foreground">
            Monthly limit
          </div>
          <div className="font-medium">{fenToCny(card.monthly_limit_fen ?? 0)}</div>
        </div>
      </div>
    </motion.div>
  );
}
