"use client";

import { useEffect, useState, useCallback } from "react";
import { motion } from "framer-motion";
import {
  CreditCard,
  Snowflake,
  Unlock,
  AlertOctagon,
  RotateCcw,
  Plus,
  type LucideIcon,
} from "lucide-react";
import { AppShell } from "@/components/layout/app-shell";
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
import { cards, ApiError, type Card as VirtualCard } from "@/lib/api";
import { FadeIn, Stagger, StaggerItem } from "@/components/motion/aurora";
import { useToast } from "@/components/ui/toast";
import { fenToCny } from "@/lib/utils";
import { cn } from "@/lib/utils";
import {
  PageHeader,
  EmptyState,
  LoadingState,
  ErrorState,
  Toolbar,
} from "@/components/ui/feedback";
import { useT } from "@/i18n/LanguageProvider";
import { usePageTitle } from "@/lib/use-page-title";

type ConfirmingAction = "lost" | "cancel";

export default function CardsPage() {
  return (
    <AppShell>
      <CardsBody />
    </AppShell>
  );
}

function CardsBody() {
  const t = useT();
  usePageTitle("meta.cards.title", "Cards");
  const [list, setList] = useState<VirtualCard[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [confirming, setConfirming] = useState<{
    card: VirtualCard;
    action: ConfirmingAction;
  } | null>(null);
  const [showIssue, setShowIssue] = useState(false);
  const [busy, setBusy] = useState(false);
  const { toast } = useToast();

  const refresh = useCallback(() => {
    setList(null);
    setError(null);
    cards
      .list()
      .then((rows) => setList(rows))
      .catch((e) => {
        setError(e instanceof ApiError ? e.message : "Could not load cards");
        setList([]);
      });
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  const act = async (c: VirtualCard, action: "freeze" | "unfreeze" | "cancel") => {
    try {
      const next =
        action === "freeze"
          ? await cards.freeze(c.id)
          : action === "unfreeze"
            ? await cards.unfreeze(c.id)
            : await cards.cancel(c.id);
      setList((prev) =>
        prev ? prev.map((x) => (x.id === next.id ? next : x)) : prev,
      );
      const title =
        action === "freeze"
          ? t("cards.frozen.title")
          : action === "unfreeze"
            ? t("cards.unfrozen.title")
            : t("cards.cancelled.title");
      toast({ variant: "success", title, description: next.masked_number });
    } catch (err) {
      const msg = err instanceof ApiError ? err.message : t("cards.actionFailed");
      toast({ variant: "error", title: t("cards.actionFailed"), description: msg });
    }
  };

  const reportLost = async () => {
    if (!confirming) return;
    setBusy(true);
    try {
      const next = await cards.reportLost(confirming.card.id);
      setList((prev) =>
        prev ? prev.map((x) => (x.id === next.id ? next : x)) : prev,
      );
      toast({ variant: "success", title: t("cards.lost.title") });
      setConfirming(null);
    } catch (err) {
      const msg = err instanceof ApiError ? err.message : t("cards.actionFailed");
      toast({ variant: "error", title: t("cards.actionFailed"), description: msg });
    } finally {
      setBusy(false);
    }
  };

  const isLoading = list === null && error === null;

  return (
    <div className="space-y-6">
      <FadeIn>
        <PageHeader
          eyebrow={t("cards.page.eyebrow")}
          title={t("cards.page.title")}
          description={t("cards.page.subtitle")}
          actions={
            <Button
              variant="gradient"
              onClick={() => setShowIssue(true)}
              disabled={isLoading}
            >
              <Plus className="size-4" />
              {t("cards.page.issue")}
            </Button>
          }
        />
      </FadeIn>

      {isLoading ? (
        <LoadingState
          label={t("common.loading", { defaultValue: "Loading…" })}
        />
      ) : error ? (
        <ErrorState
          title={t("common.error.title")}
          body={error}
          onRetry={refresh}
          retryLabel={t("common.retry")}
        />
      ) : list && list.length === 0 ? (
        <EmptyState
          icon={CreditCard}
          title={t("cards.empty.title")}
          body={t("cards.empty.body")}
          action={
            <Button variant="gradient" onClick={() => setShowIssue(true)}>
              <Plus className="size-4" />
              {t("cards.page.issue")}
            </Button>
          }
        />
      ) : list ? (
        <Stagger>
          <div className="grid gap-5 sm:grid-cols-2 lg:grid-cols-3">
            {list.map((c) => (
              <StaggerItem key={c.id}>
                <CardVisual card={c} />
                <CardActions
                  card={c}
                  onFreeze={() => act(c, "freeze")}
                  onUnfreeze={() => act(c, "unfreeze")}
                  onCancel={() => act(c, "cancel")}
                  onLost={() => setConfirming({ card: c, action: "lost" })}
                />
              </StaggerItem>
            ))}
          </div>
        </Stagger>
      ) : null}

      <IssueCardDialog
        open={showIssue}
        onOpenChange={setShowIssue}
        onCreated={(next) => {
          setList((prev) => (prev ? [...prev, next] : [next]));
          toast({
            variant: "success",
            title: t("cards.issued.title"),
            description: next.masked_number,
          });
        }}
        busy={busy}
        setBusy={setBusy}
      />

      <Dialog
        open={!!confirming}
        onOpenChange={(o) => !o && setConfirming(null)}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t("cards.dialog.lost.title")}</DialogTitle>
            <DialogDescription>{t("cards.dialog.lost.body")}</DialogDescription>
          </DialogHeader>
          <div className="rounded-lg border bg-background/40 p-3 text-sm">
            <div className="flex justify-between">
              <span className="text-muted-foreground">Card</span>
              <span className="font-medium">{confirming?.card.masked_number}</span>
            </div>
            <div className="mt-1 flex justify-between">
              <span className="text-muted-foreground">Display name</span>
              <span className="font-medium">
                {confirming?.card.display_name}
              </span>
            </div>
          </div>
          <DialogFooter>
            <Button
              variant="ghost"
              onClick={() => setConfirming(null)}
              disabled={busy}
            >
              {t("common.keep")}
            </Button>
            <Button variant="destructive" onClick={reportLost} disabled={busy}>
              {busy ? t("cards.dialog.lost.busy") : t("cards.dialog.lost.cta")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}

// --- Subcomponents --------------------------------------------------------

function CardActions({
  card,
  onFreeze,
  onUnfreeze,
  onCancel,
  onLost,
}: {
  card: VirtualCard;
  onFreeze: () => void;
  onUnfreeze: () => void;
  onCancel: () => void;
  onLost: () => void;
}) {
  const t = useT();
  return (
    <Toolbar className="mt-3 grid grid-cols-3 gap-1.5 p-1.5">
      {card.status === "active" && (
        <ActionButton
          icon={Snowflake}
          label={t("cards.action.freeze")}
          onClick={onFreeze}
        />
      )}
      {card.status === "frozen" && (
        <ActionButton
          icon={Unlock}
          label={t("cards.action.unfreeze")}
          onClick={onUnfreeze}
        />
      )}
      <ActionButton
        icon={AlertOctagon}
        label={t("cards.action.lost")}
        tone="destructive"
        onClick={onLost}
        disabled={card.status === "cancelled" || card.status === "lost"}
      />
      <ActionButton
        icon={RotateCcw}
        label={t("cards.action.cancel")}
        tone="ghost"
        onClick={onCancel}
        disabled={card.status === "cancelled"}
      />
    </Toolbar>
  );
}

function ActionButton({
  icon: Icon,
  label,
  onClick,
  tone = "outline",
  disabled,
}: {
  icon: LucideIcon;
  label: string;
  onClick: () => void;
  tone?: "outline" | "destructive" | "ghost";
  disabled?: boolean;
}) {
  return (
    <Button
      size="sm"
      variant={tone}
      onClick={onClick}
      disabled={disabled}
      className="w-full"
    >
      <Icon className="size-3.5" />
      {label}
    </Button>
  );
}

function CardVisual({ card }: { card: VirtualCard }) {
  const t = useT();
  return (
    <motion.div
      whileHover={{ y: -4, rotate: -0.5 }}
      transition={{ type: "spring", damping: 18, stiffness: 200 }}
    >
      <div
        className={cn(
          "relative h-44 overflow-hidden rounded-2xl p-5 text-white shadow-[0_24px_60px_-20px_rgba(0,0,0,0.5)]",
          "bg-[conic-gradient(at_top_left,hsl(263_80%_55%),hsl(220_80%_45%),hsl(199_89%_50%),hsl(263_80%_55%))]",
        )}
      >
        <div className="relative z-10 flex h-full flex-col justify-between">
          <div className="flex items-start justify-between">
            <div>
              <div className="text-[10px] uppercase tracking-widest opacity-80">
                {t("cards.virtual")}
              </div>
              <div className="mt-1 font-display text-base font-semibold leading-tight">
                {card.display_name}
              </div>
            </div>
            <CreditCard className="size-7 opacity-80" />
          </div>
          <div>
            <div className="font-mono text-xl tracking-widest">
              {card.masked_number}
            </div>
            <div className="mt-2 flex items-end justify-between">
              <div className="font-mono text-sm opacity-80">
                EXP {String(card.expiry_month).padStart(2, "0")}/
                {String(card.expiry_year).slice(-2)}
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
                {t(`cards.status.${card.status}` as never)}
              </Badge>
            </div>
          </div>
        </div>
      </div>
      <div className="mt-3 grid grid-cols-2 gap-2 rounded-xl border bg-card/60 p-3 text-[11px]">
        <div>
          <div className="text-[10px] uppercase tracking-wider text-muted-foreground">
            {t("cards.fields.singleLimit")}
          </div>
          <div className="font-medium">
            {fenToCny(card.single_tx_limit_fen ?? 0)}
          </div>
        </div>
        <div>
          <div className="text-[10px] uppercase tracking-wider text-muted-foreground">
            {t("cards.fields.monthlyLimit")}
          </div>
          <div className="font-medium">
            {fenToCny(card.monthly_limit_fen ?? 0)}
          </div>
        </div>
      </div>
    </motion.div>
  );
}

function IssueCardDialog({
  open,
  onOpenChange,
  onCreated,
  busy,
  setBusy,
}: {
  open: boolean;
  onOpenChange: (v: boolean) => void;
  onCreated: (card: VirtualCard) => void;
  busy: boolean;
  setBusy: (b: boolean) => void;
}) {
  const t = useT();
  const [name, setName] = useState("Travel card");
  const { toast } = useToast();

  const issue = async () => {
    if (!name.trim()) return;
    setBusy(true);
    try {
      const next = await cards.issue({ display_name: name });
      onCreated(next);
      onOpenChange(false);
      setName("Travel card");
    } catch (err) {
      const msg = err instanceof ApiError ? err.message : t("cards.actionFailed");
      toast({
        variant: "error",
        title: t("cards.actionFailed"),
        description: msg,
      });
    } finally {
      setBusy(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t("cards.dialog.issue.title")}</DialogTitle>
          <DialogDescription>{t("cards.dialog.issue.body")}</DialogDescription>
        </DialogHeader>
        <div className="space-y-2">
          <Label htmlFor="card-name">{t("cards.dialog.issue.name")}</Label>
          <Input
            id="card-name"
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder={t("cards.dialog.issue.namePh")}
          />
        </div>
        <DialogFooter>
          <Button variant="ghost" onClick={() => onOpenChange(false)}>
            {t("common.cancel")}
          </Button>
          <Button
            variant="gradient"
            onClick={issue}
            disabled={busy || !name.trim()}
          >
            {busy ? t("cards.dialog.issue.busy") : t("cards.dialog.issue.cta")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
