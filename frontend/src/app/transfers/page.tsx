"use client";

import { useEffect, useMemo, useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import {
  Bot,
  Calendar,
  CheckCircle2,
  ChevronRight,
  CircleDollarSign,
  Clock,
  ListChecks as ActivityIcon,
  Loader2,
  Plus,
  Receipt,
  Search,
  Send,
  Shield,
  ShieldAlert,
  Sparkles,
  Split,
  Trash2,
  Undo2,
  UserPlus,
  Users,
  Wallet,
  X,
} from "lucide-react";
import { AppShell } from "@/components/layout/app-shell";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { useToast } from "@/components/ui/toast";
import {
  smart,
  splitBills,
  accounts,
  transfers,
  friendlyError,
  type Account,
  type ApiError,
  type Payee,
  type PayeeSuggestion,
  type SecurityReview,
  type SmartParseOut,
  type SmartPreviewOut,
  type SplitBill,
} from "@/lib/api";
import { FadeIn, Stagger, StaggerItem, AuroraBackdrop } from "@/components/motion/aurora";
import { TierBadge } from "@/components/motion/tier-badge";
import { cn, cnyToFen, fenToCny, formatDateTime, relativeTime } from "@/lib/utils";
import { accountsBusApi } from "@/lib/store";
import { useT } from "@/i18n/LanguageProvider";
import { usePageTitle } from "@/lib/use-page-title";
import { ConfirmationCard } from "@/components/confirmation-card";

type Tab = "pay" | "schedule" | "splits" | "activity";

export default function SmartTransfersPage() {
  return (
    <AppShell>
      <Body />
    </AppShell>
  );
}

function Body() {
  const [tab, setTab] = useState<Tab>("pay");
  return (
    <div className="space-y-6">
      <AuroraBackdrop />
      <FadeIn>
        <div>
          <p className="text-[11px] font-medium uppercase tracking-widest text-muted-foreground">
            Smart transfers · AI agent
          </p>
          <h1 className="mt-1 font-display text-3xl font-semibold tracking-tight">
            Beneficial accounts & smart transfers.
          </h1>
          <p className="mt-1 max-w-2xl text-sm text-muted-foreground">
            Save the people you pay by name, phone, or note. The assistant
            resolves who you mean, narrates the security check, schedules for
            later, and helps you collect money back with split-bills.
          </p>
        </div>
      </FadeIn>

      <div className="flex flex-wrap items-center gap-1.5 rounded-lg border border-white/5 bg-white/[0.03] p-1">
        <TabButton active={tab === "pay"} onClick={() => setTab("pay")} icon={<Send className="size-3.5" />}>
          Pay someone
        </TabButton>
        <TabButton active={tab === "schedule"} onClick={() => setTab("schedule")} icon={<Calendar className="size-3.5" />}>
          Scheduled
        </TabButton>
        <TabButton active={tab === "activity"} onClick={() => setTab("activity")} icon={<ActivityIcon className="size-3.5" />}>
          Activity
        </TabButton>
        <TabButton active={tab === "splits"} onClick={() => setTab("splits")} icon={<Split className="size-3.5" />}>
          Split a bill
        </TabButton>
      </div>

      {tab === "pay" ? <PayTab /> : tab === "schedule" ? <ScheduleTab /> : tab === "activity" ? <ActivityTab /> : <SplitsTab />}
    </div>
  );
}

function TabButton({
  active,
  onClick,
  icon,
  children,
}: {
  active: boolean;
  onClick: () => void;
  icon: React.ReactNode;
  children: React.ReactNode;
}) {
  return (
    <button
      onClick={onClick}
      className={cn(
        "inline-flex items-center gap-2 rounded-md px-3 py-1.5 text-xs font-medium transition-colors",
        active
          ? "bg-primary/15 text-foreground ring-1 ring-primary/30"
          : "text-muted-foreground hover:bg-accent hover:text-foreground"
      )}
    >
      {icon}
      {children}
    </button>
  );
}

// =====================================================================
// Pay tab
// =====================================================================

function PayTab() {
  const [payees, setPayees] = useState<Payee[]>([]);
  const [addOpen, setAddOpen] = useState(false);
  const [nlText, setNlText] = useState<string>("Pay Li Wei 200 yuan for Friday lunch");
  const [parsed, setParsed] = useState<SmartParseOut | null>(null);
  const [preview, setPreview] = useState<SmartPreviewOut | null>(null);
  const [accounts_, setAccounts] = useState<Account[]>([]);
  const [accountId, setAccountId] = useState<string | null>(null);
  const [parseBusy, setParseBusy] = useState(false);
  const [previewBusy, setPreviewBusy] = useState(false);
  const [confirmBusy, setConfirmBusy] = useState(false);
  const [scheduleIso, setScheduleIso] = useState<string>("");
  // Pending action surfaced by a Yellow/Red-tier execute. Rendered
  // inline as a `ConfirmationCard` so the user can drive the existing
  // mock-MFA + confirm flow without bouncing off the page.
  const [pending, setPending] = useState<{
    id: string;
    tool: string;
    parameters: Record<string, unknown>;
    riskTier: "Green" | "Yellow" | "Red";
  } | null>(null);
  const t = useT();
  usePageTitle("meta.transfers.title", "Transfers");
  const { toast } = useToast();

  const refresh = () => {
    smart.payees.list().then((d) => {
      if (Array.isArray(d)) setPayees(d);
      else setPayees(d.suggestions.map((s) => s.payee));
    }).catch(() => {});
  };
  useEffect(() => { refresh(); }, []);
  useEffect(() => {
    accounts.list().then((a) => {
      setAccounts(a);
      if (a.length) setAccountId(a[0].id);
    });
  }, []);

  const parse = async () => {
    if (!nlText.trim()) return;
    setParseBusy(true);
    setPreview(null);
    try {
      const r = await smart.parse({ text: nlText });
      setParsed(r);
      if (!r.needs_clarification && r.resolved_payee && r.amount_fen > 0) {
        // Auto-build preview to keep the demo snappy.
        await runPreview(r);
      } else {
        toast({
          variant: r.needs_clarification ? "warning" : "default",
          title: r.needs_clarification ? "Need a bit more" : "Drafted",
          description: r.questions.join(" / ") || "Drafted from your message.",
        });
      }
    } catch (e) {
      const msg = e instanceof Error ? e.message : "Parse failed";
      toast({ variant: "error", title: "Parse failed", description: msg });
    } finally {
      setParseBusy(false);
    }
  };

  const runPreview = async (source?: SmartParseOut) => {
    const p = source ?? parsed;
    if (!p) return;
    const payee = p.resolved_payee ?? p.candidates?.[0]?.payee;
    if (!payee) {
      toast({ variant: "error", title: "No payee resolved" });
      return;
    }
    if (p.amount_fen <= 0) {
      toast({ variant: "error", title: "No amount detected" });
      return;
    }
    setPreviewBusy(true);
    try {
      const r = await smart.preview({
        payee_id: payee.id,
        amount_fen: p.amount_fen,
        memo: p.memo || undefined,
        schedule_iso: scheduleIso || undefined,
        account_id: accountId ?? undefined,
      });
      setPreview(r);
    } catch (e) {
      const msg = e instanceof Error ? e.message : "Preview failed";
      toast({ variant: "error", title: "Preview failed", description: msg });
    } finally {
      setPreviewBusy(false);
    }
  };

  const confirm = async () => {
    if (!preview) return;
    setConfirmBusy(true);
    try {
      if (scheduleIso) {
        // Scheduled path: still no money movement today. Just create a
        // `scheduled` transfer row; a future job would post it.
        const r = await smart.schedule({
          payee_id: preview.payee.id,
          amount_fen: preview.amount_fen,
          memo: preview.memo,
          schedule_iso: scheduleIso,
          account_id: accountId ?? undefined,
        });
        toast({
          variant: "success",
          title: t("transfers.scheduled"),
          description: `${fenToCny(r.amount_fen)} to ${preview.payee.display_name} on ${new Date(r.schedule_iso).toLocaleString()}.`,
        });
        accountsBusApi.bump("transfer.scheduled");
        setPreview(null);
        setParsed(null);
        setScheduleIso("");
        return;
      }
      // Pay-now path. Green debits the account immediately; Yellow/Red
      // comes back as a 403 with a `pending_action_id` that the
      // ConfirmationCard can drive through the existing confirm + mock
      // MFA flow.
      let res;
      try {
        res = await smart.execute({
          payee_id: preview.payee.id,
          amount_fen: preview.amount_fen,
          memo: preview.memo,
          account_id: accountId ?? undefined,
        });
      } catch (e: any) {
        const code = e?.code as string | undefined;
        const details = (e?.details as any) ?? {};
        const pa = details.pending_action_id as string | undefined;
        const tier = (details.tier as string | undefined) ?? "Yellow";
        if (code === "approval_required" && pa) {
          setPending({
            id: pa,
            tool: "transfer.create_pending",
            parameters: {
              payee_id: preview.payee.id,
              amount_fen: preview.amount_fen,
              memo: preview.memo,
              account_id: accountId,
            },
            riskTier: (tier as "Green" | "Yellow" | "Red") ?? "Yellow",
          });
          toast({
            variant: "warning",
            title:
              tier === "Red"
                ? t("transfer.execute.red")
                : t("transfer.execute.yellow"),
          });
          return;
        }
        throw e;
      }
      const amount = fenToCny(res.amount_fen);
      const who = preview.payee.display_name;
      toast({
        variant: "success",
        title: t("transfer.execute.success"),
        description: res.replay
          ? t("transfer.execute.replay")
          : t("transfer.execute.successBody", {
              amount,
              who,
            }),
      });
      accountsBusApi.bump("transfer.execute");
      accounts
        .list()
        .then((a) => setAccounts(a))
        .catch(() => {});
      setPreview(null);
      setParsed(null);
      setScheduleIso("");
    } catch (e) {
      const msg = e instanceof Error ? e.message : "Confirm failed";
      toast({ variant: "error", title: t("transfer.execute.failed"), description: msg });
    } finally {
      setConfirmBusy(false);
    }
  };

  const onActionDone = () => {
    accountsBusApi.bump("transfer.execute");
    accounts
      .list()
      .then((a) => setAccounts(a))
      .catch(() => {});
    setPending(null);
    setPreview(null);
    setParsed(null);
    setScheduleIso("");
  };

  return (
    <div className="grid gap-4 lg:grid-cols-[1fr_400px]">
      <div className="space-y-4">
        <FadeIn delay={0.05}>
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Bot className="size-4 text-primary" />
                Tell me what to do
              </CardTitle>
              <CardDescription>
                English, 中文, or mixed — the agent extracts amount, recipient,
                memo, and (optional) schedule.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-3">
              <Textarea
                value={nlText}
                onChange={(e) => setNlText(e.target.value)}
                rows={3}
                placeholder='e.g. "转 200 给 Li Wei，午饭钱，明天" or "Pay Anna $50 split with Mei"'
                className="text-sm"
              />
              <div className="flex flex-wrap items-center justify-between gap-2">
                <p className="text-[11px] text-muted-foreground">
                  <Sparkles className="mr-1 inline size-3 text-primary" />
                  Hint: include a day (&ldquo;tomorrow&rdquo;, &ldquo;next Monday&rdquo;) for scheduling.
                </p>
                <Button onClick={parse} disabled={parseBusy || !nlText.trim()}>
                  {parseBusy ? <Loader2 className="size-4 animate-spin" /> : <Sparkles className="size-4" />}
                  Parse & review
                </Button>
              </div>
            </CardContent>
          </Card>
        </FadeIn>

        <FadeIn delay={0.1}>
          <BeneficialAccounts
            payees={payees}
            onAddOpen={() => setAddOpen(true)}
            onSelect={async (p) => {
              setParsed({
                amount_fen: 0,
                recipient_query: p.display_name,
                memo: "",
                schedule_iso: null,
                split_participants: null,
                needs_clarification: true,
                questions: ["How much (in CNY)?"],
                resolved_payee: p,
                resolve_status: "resolved",
                candidates: [{ payee: p, match_kind: "exact_name", confidence: 1, note: "selected" }],
                resolve_note: "selected from your book",
              });
              setPreview(null);
            }}
            onRefresh={refresh}
          />
        </FadeIn>

        <AnimatePresence>
          {parsed && (
            <FadeIn delay={0.05}>
              <ParseCard
                parsed={parsed}
                onRunPreview={() => runPreview()}
                previewBusy={previewBusy}
                onPickCandidate={async (s) => {
                  setParsed({ ...parsed, resolved_payee: s.payee, resolve_status: "resolved" });
                  await runPreview({ ...parsed, resolved_payee: s.payee, resolve_status: "resolved" });
                }}
              />
            </FadeIn>
          )}
        </AnimatePresence>
      </div>

      <div className="space-y-4">
        <FadeIn delay={0.05}>
          <AccountSelector accounts={accounts_} selected={accountId} onSelect={setAccountId} />
        </FadeIn>

        <FadeIn delay={0.1}>
          <ScheduleToggle value={scheduleIso} onChange={setScheduleIso} />
        </FadeIn>

        <AnimatePresence>
          {preview && (
            <FadeIn delay={0.05}>
              <ReviewPanel
                preview={preview}
                onConfirm={confirm}
                confirmBusy={confirmBusy}
                scheduleNote={preview.review.schedule_note}
              />
            </FadeIn>
          )}
        </AnimatePresence>

        <AnimatePresence>
          {pending && (
            <FadeIn delay={0.05}>
              <ConfirmationCard
                pendingActionId={pending.id}
                tool={pending.tool}
                parameters={pending.parameters}
                riskTier={pending.riskTier}
                onDone={onActionDone}
              />
            </FadeIn>
          )}
        </AnimatePresence>
      </div>

      <AddPayeeDialog
        open={addOpen}
        onOpenChange={setAddOpen}
        onCreated={() => {
          setAddOpen(false);
          refresh();
        }}
      />
    </div>
  );
}

function BeneficialAccounts({
  payees,
  onAddOpen,
  onSelect,
  onRefresh,
}: {
  payees: Payee[];
  onAddOpen: () => void;
  onSelect: (p: Payee) => void;
  onRefresh: () => void;
}) {
  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between gap-2">
          <div>
            <CardTitle className="flex items-center gap-2">
              <Users className="size-4 text-primary" />
              Beneficial accounts
            </CardTitle>
            <CardDescription>
              People and merchants you can pay with one tap.
            </CardDescription>
          </div>
          <div className="flex items-center gap-2">
            <Button size="sm" variant="ghost" onClick={onRefresh}>
              Refresh
            </Button>
            <Button size="sm" onClick={onAddOpen}>
              <UserPlus className="size-3.5" />
              Add
            </Button>
          </div>
        </div>
      </CardHeader>
      <CardContent>
        {payees.length === 0 ? (
          <p className="text-sm text-muted-foreground">
            No saved payees yet. Add a beneficial account to start.
          </p>
        ) : (
          <Stagger>
            <ul className="grid gap-2 sm:grid-cols-2">
              {payees.map((p) => (
                <StaggerItem key={p.id}>
                  <motion.li
                    layout
                    onClick={() => onSelect(p)}
                    className="group cursor-pointer rounded-md border border-white/5 bg-white/[0.03] p-3 transition-colors hover:border-primary/30 hover:bg-primary/[0.05]"
                  >
                    <div className="flex items-center justify-between">
                      <div className="min-w-0">
                        <div className="flex items-center gap-2">
                          <div className="truncate text-sm font-medium">{p.display_name}</div>
                          {p.verified ? (
                            <Badge variant="green">verified</Badge>
                          ) : (
                            <Badge variant="muted">new</Badge>
                          )}
                        </div>
                        <div className="mt-0.5 truncate text-[11px] text-muted-foreground">
                          {p.phone_tail ? `phone tail ${p.phone_tail}` : "no phone"}
                          {p.first_used_at ? ` · paid before` : " · first time"}
                        </div>
                      </div>
                      <ChevronRight className="size-4 text-muted-foreground group-hover:text-primary" />
                    </div>
                  </motion.li>
                </StaggerItem>
              ))}
            </ul>
          </Stagger>
        )}
      </CardContent>
    </Card>
  );
}

function AddPayeeDialog({
  open,
  onOpenChange,
  onCreated,
}: {
  open: boolean;
  onOpenChange: (v: boolean) => void;
  onCreated: () => void;
}) {
  const [name, setName] = useState("");
  const [phone, setPhone] = useState("");
  const [note, setNote] = useState("");
  const [busy, setBusy] = useState(false);
  const { toast } = useToast();

  const submit = async () => {
    if (!name.trim()) {
      toast({ variant: "error", title: "Name is required" });
      return;
    }
    setBusy(true);
    try {
      await smart.payees.create({ display_name: name, phone_tail: phone || undefined, note: note || undefined });
      toast({ variant: "success", title: "Saved", description: `${name} added to your beneficial accounts.` });
      setName("");
      setPhone("");
      setNote("");
      onCreated();
    } catch (e) {
      const msg = e instanceof Error ? e.message : "Save failed";
      toast({ variant: "error", title: "Save failed", description: msg });
    } finally {
      setBusy(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Add a beneficial account</DialogTitle>
          <DialogDescription>
            A payee you trust. The agent will use it to resolve &ldquo;Pay X&rdquo; automatically.
          </DialogDescription>
        </DialogHeader>
        <div className="space-y-3">
          <div>
            <Label htmlFor="payee-name">Display name</Label>
            <Input id="payee-name" value={name} onChange={(e) => setName(e.target.value)} placeholder="Mom, Li Wei, Acme Books…" />
          </div>
          <div>
            <Label htmlFor="payee-phone">Phone tail (optional, 4+ digits)</Label>
            <Input id="payee-phone" value={phone} onChange={(e) => setPhone(e.target.value)} placeholder="1234" />
          </div>
          <div>
            <Label htmlFor="payee-note">Note (optional)</Label>
            <Input id="payee-note" value={note} onChange={(e) => setNote(e.target.value)} placeholder="birthday gift, monthly rent, …" />
          </div>
          <div className="flex justify-end">
            <Button onClick={submit} disabled={busy}>
              {busy ? <Loader2 className="size-4 animate-spin" /> : <Plus className="size-4" />}
              Save payee
            </Button>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}

// =====================================================================
// Activity tab — close-the-loop on transfers (1.1)
// =====================================================================
//
// Shows the last 50 transfers for the user. Each row surfaces the
// current state (pending / posted / reversed / scheduled) and gives
// the user two close-the-loop actions:
//   • Cancel a pending transfer (Yellow / Red tier sits in pending_actions)
//   • Reverse a recently-posted transfer (within 5 minutes of posting)
//
// Both actions go through the v3 `transfers.getStatus` / `transfers.cancel`
// / `transfers.reverse` endpoints and surface Layer 5.5 friendly errors
// in a toast on failure.

type TransferRow = {
  id: string;
  source_account_id: string;
  payee_id: string | null;
  amount_fen: number;
  memo: string;
  schedule: string | null;
  status: string;
  created_at: string;
  posted_at: string | null;
};

function ActivityTab() {
  const t = useT();
  const { toast } = useToast();
  const [rows, setRows] = useState<TransferRow[]>([]);
  const [loading, setLoading] = useState(true);
  const [reverseOf, setReverseOf] = useState<TransferRow | null>(null);
  const [reverseReason, setReverseReason] = useState("");
  const [busy, setBusy] = useState<string | null>(null);
  const [statusOf, setStatusOf] = useState<Record<string, {
    status: string;
    reversible: boolean;
    posted_at: string | null;
  }>>({});

  // Localized status label. The `transfers.status.<lowercase>` keys are
  // pre-declared in dictionaries.ts; anything we don't translate (e.g. an
  // unexpected backend status) just shows the raw string.
  const statusLabel = (s: string) => {
    const k = `agent.transfers.status.${s.toLowerCase()}` as
      | "agent.transfers.status.pending"
      | "agent.transfers.status.posted"
      | "agent.transfers.status.reversed"
      | "agent.transfers.status.scheduled"
      | "agent.transfers.status.awaiting_confirmation"
      | "agent.transfers.status.awaiting_mfa"
      | "agent.transfers.status.cancelled";
    return t(k);
  };

  const refresh = () => {
    setLoading(true);
    transfers
      .list()
      .then((r) => setRows((r as TransferRow[]) ?? []))
      .catch((e: ApiError) => {
        const fe = friendlyError(e);
        toast({ variant: "error", title: fe.title, description: fe.body });
      })
      .finally(() => setLoading(false));
  };

  useEffect(() => { refresh(); }, []);

  const cancel = async (row: TransferRow) => {
    setBusy(row.id);
    try {
      const r = await transfers.cancel(row.id);
      toast({ variant: "success", title: t("agent.transfers.status.cancelled"), description: `Transfer ${r.transfer_id} → ${r.status}` });
      refresh();
    } catch (e) {
      const fe = friendlyError(e as ApiError);
      toast({ variant: "error", title: fe.title, description: fe.body });
    } finally {
      setBusy(null);
    }
  };

  const startReverse = async (row: TransferRow) => {
    setBusy(row.id);
    try {
      const s = await transfers.getStatus(row.id);
      setStatusOf((m) => ({ ...m, [row.id]: { status: s.status, reversible: s.reversible, posted_at: s.posted_at } }));
      if (!s.reversible) {
        toast({ variant: "warning", title: t("agent.transfers.reverse.expired"), description: t("agent.transfers.reverse.expired_note") });
        return;
      }
      setReverseOf(row);
    } catch (e) {
      const fe = friendlyError(e as ApiError);
      toast({ variant: "error", title: fe.title, description: fe.body });
    } finally {
      setBusy(null);
    }
  };

  const confirmReverse = async () => {
    if (!reverseOf) return;
    setBusy(reverseOf.id);
    try {
      const r = await transfers.reverse(reverseOf.id, reverseReason);
      toast({
        variant: "success",
        title: t("agent.transfers.reverse.confirmed"),
        description: `${t("agent.transfers.reverse.demo_code")} ${r.mock_mfa_code} — ${r.next_step}`,
      });
      setReverseOf(null);
      setReverseReason("");
      refresh();
    } catch (e) {
      const fe = friendlyError(e as ApiError);
      toast({ variant: "error", title: fe.title, description: fe.body });
    } finally {
      setBusy(null);
    }
  };

  return (
    <div className="space-y-4">
      <FadeIn>
        <Card>
          <CardHeader>
            <div className="flex flex-wrap items-end justify-between gap-3">
              <div>
                <CardTitle className="flex items-center gap-2">
                  <ActivityIcon className="size-4 text-primary" />
                  {t("agent.transfers.activity.title")}
                </CardTitle>
                <CardDescription>
                  {t("agent.transfers.activity.subtitle")}
                </CardDescription>
              </div>
              <Button variant="outline" size="sm" onClick={refresh} disabled={loading}>
                {loading ? <Loader2 className="size-4 animate-spin" /> : <Undo2 className="size-4" />}
                {t("agent.transfers.activity.refresh")}
              </Button>
            </div>
          </CardHeader>
          <CardContent>
            {loading ? (
              <div className="flex items-center gap-2 text-sm text-muted-foreground">
                <Loader2 className="size-4 animate-spin" />
                {t("agent.transfers.activity.loading")}
              </div>
            ) : rows.length === 0 ? (
              <p className="text-sm text-muted-foreground">
                {t("agent.transfers.activity.empty")}
              </p>
            ) : (
              <ul className="divide-y divide-white/5">
                {rows.map((r) => {
                  const sKey = `agent.transfers.status.${r.status.toLowerCase()}`;
                  const isPending = r.status === "pending" || r.status === "awaiting_confirmation" || r.status === "awaiting_mfa" || r.status === "scheduled";
                  const isPosted = r.status === "posted";
                  return (
                    <li key={r.id} className="flex flex-wrap items-center gap-3 py-3">
                      <div className="min-w-0 flex-1">
                        <div className="flex items-center gap-2">
                          <span className="font-medium tabular-nums">{fenToCny(Math.abs(r.amount_fen))}</span>
                          <Badge variant={isPosted ? "muted" : isPending ? "yellow" : r.status === "reversed" ? "muted" : "default"}>
                            {statusLabel(r.status)}
                          </Badge>
                          {r.schedule && (
                            <span className="text-xs text-muted-foreground">
                              {t("agent.transfers.activity.scheduled_for")} {formatDateTime(r.schedule)}
                            </span>
                          )}
                        </div>
                        {r.memo && (
                          <p className="mt-0.5 truncate text-xs text-muted-foreground">
                            {r.memo}
                          </p>
                        )}
                        <p className="mt-0.5 text-[10px] uppercase tracking-wider text-muted-foreground">
                          {relativeTime(r.created_at)} · {r.id}
                        </p>
                      </div>
                      <div className="flex flex-wrap items-center gap-1.5">
                        {isPending && (
                          <Button
                            size="sm"
                            variant="outline"
                            disabled={busy === r.id}
                            onClick={() => cancel(r)}
                          >
                            {busy === r.id ? <Loader2 className="size-4 animate-spin" /> : <X className="size-4" />}
                            {t("agent.transfers.cancel")}
                          </Button>
                        )}
                        {isPosted && (
                          <Button
                            size="sm"
                            variant="outline"
                            disabled={busy === r.id}
                            onClick={() => startReverse(r)}
                          >
                            {busy === r.id ? <Loader2 className="size-4 animate-spin" /> : <Undo2 className="size-4" />}
                            {t("agent.transfers.reverse")}
                          </Button>
                        )}
                        {statusOf[r.id] && (
                          <Badge variant={statusOf[r.id].reversible ? "muted" : "yellow"}>
                            {statusOf[r.id].reversible
                              ? t("agent.transfers.activity.reversible")
                              : t("agent.transfers.activity.locked")}
                          </Badge>
                        )}
                      </div>
                    </li>
                  );
                })}
              </ul>
            )}
          </CardContent>
        </Card>
      </FadeIn>

      {/* Reverse dialog */}
      <Dialog open={!!reverseOf} onOpenChange={(o) => { if (!o) { setReverseOf(null); setReverseReason(""); } }}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <Undo2 className="size-4 text-primary" />
              {t("agent.transfers.reverse")}
            </DialogTitle>
            <DialogDescription>
              {reverseOf && (
                <>
                  {fenToCny(Math.abs(reverseOf.amount_fen))} · {reverseOf.memo || t("agent.transfers.activity.no_memo")}
                </>
              )}
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-3">
            <Label htmlFor="reverse-reason">{t("agent.transfers.reverse.reason")}</Label>
            <Textarea
              id="reverse-reason"
              value={reverseReason}
              onChange={(e) => setReverseReason(e.target.value)}
              placeholder={t("agent.transfers.reverse.reason_ph")}
              rows={3}
            />
            <p className="text-xs text-muted-foreground">
              {t("agent.transfers.reverse.next")}
            </p>
            <div className="flex justify-end gap-2">
              <Button variant="ghost" onClick={() => { setReverseOf(null); setReverseReason(""); }} disabled={busy !== null}>
                {t("form.cancel")}
              </Button>
              <Button onClick={confirmReverse} disabled={busy !== null || !reverseReason.trim()}>
                {busy ? <Loader2 className="size-4 animate-spin" /> : <Undo2 className="size-4" />}
                {t("agent.transfers.reverse.confirm")}
              </Button>
            </div>
          </div>
        </DialogContent>
      </Dialog>
    </div>
  );
}

function ParseCard({
  parsed,
  previewBusy,
  onRunPreview,
  onPickCandidate,
}: {
  parsed: SmartParseOut;
  previewBusy: boolean;
  onRunPreview: () => void;
  onPickCandidate: (s: PayeeSuggestion) => void;
}) {
  return (
    <motion.div initial={{ opacity: 0, y: 6 }} animate={{ opacity: 1, y: 0 }}>
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2 text-base">
            <Sparkles className="size-4 text-primary" />
            Parsed draft
          </CardTitle>
          <CardDescription>
            Review what the agent extracted. You can adjust the recipient below.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-3">
          <div className="grid gap-3 sm:grid-cols-2">
            <KV label="Amount" value={parsed.amount_fen > 0 ? fenToCny(parsed.amount_fen) : "—"} />
            <KV label="Recipient" value={parsed.recipient_query || "—"} />
            <KV label="Memo" value={parsed.memo || "—"} />
            <KV
              label="Schedule"
              value={
                parsed.schedule_iso
                  ? new Date(parsed.schedule_iso).toLocaleString()
                  : "immediately"
              }
            />
          </div>
          {parsed.resolve_note && (
            <div className="flex items-center gap-2 rounded-md border border-white/5 bg-white/[0.03] p-2 text-xs">
              <Search className="size-3.5 text-muted-foreground" />
              <span className="text-muted-foreground">{parsed.resolve_note}</span>
            </div>
          )}
          {parsed.candidates && parsed.candidates.length > 0 && (
            <div>
              <p className="mb-1 text-[10px] font-medium uppercase tracking-wider text-muted-foreground">
                Candidate payees
              </p>
              <ul className="space-y-1">
                {parsed.candidates.map((c) => (
                  <li key={c.payee.id}>
                    <button
                      onClick={() => onPickCandidate(c)}
                      className="flex w-full items-center justify-between rounded-md border border-white/5 bg-white/[0.03] px-2 py-1.5 text-left text-xs transition-colors hover:bg-accent"
                    >
                      <div>
                        <span className="font-medium">{c.payee.display_name}</span>
                        <span className="ml-2 text-muted-foreground">{c.match_kind}</span>
                      </div>
                      <span className="font-mono text-[10px] text-muted-foreground">
                        {Math.round(c.confidence * 100)}%
                      </span>
                    </button>
                  </li>
                ))}
              </ul>
            </div>
          )}
          {parsed.needs_clarification && (
            <div className="rounded-md border border-tier-yellow/30 bg-tier-yellow/[0.06] p-2 text-xs text-tier-yellow">
              <div className="font-semibold">Need a bit more</div>
              <ul className="mt-1 list-disc pl-4">
                {parsed.questions.map((q) => <li key={q}>{q}</li>)}
              </ul>
            </div>
          )}
          <div className="flex justify-end">
            <Button
              onClick={onRunPreview}
              disabled={previewBusy || !parsed.resolved_payee || parsed.amount_fen <= 0}
            >
              {previewBusy ? <Loader2 className="size-4 animate-spin" /> : <Shield className="size-4" />}
              Security review
            </Button>
          </div>
        </CardContent>
      </Card>
    </motion.div>
  );
}

function AccountSelector({
  accounts,
  selected,
  onSelect,
}: {
  accounts: Account[];
  selected: string | null;
  onSelect: (id: string) => void;
}) {
  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2 text-base">
          <Wallet className="size-4 text-primary" />
          Source account
        </CardTitle>
      </CardHeader>
      <CardContent>
        {accounts.length === 0 ? (
          <p className="text-sm text-muted-foreground">No accounts on file.</p>
        ) : (
          <div className="grid gap-2">
            {accounts.map((a) => (
              <button
                key={a.id}
                onClick={() => onSelect(a.id)}
                className={cn(
                  "rounded-md border p-3 text-left transition-colors",
                  selected === a.id
                    ? "border-primary/40 bg-primary/[0.08]"
                    : "border-white/5 bg-white/[0.03] hover:bg-accent"
                )}
              >
                <div className="flex items-center justify-between">
                  <div>
                    <div className="text-sm font-medium">{a.display_name}</div>
                    <div className="text-[10px] text-muted-foreground">{a.masked_number}</div>
                  </div>
                  <div className="font-mono text-sm font-semibold">{fenToCny(a.available_balance_fen)}</div>
                </div>
              </button>
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  );
}

function ScheduleToggle({ value, onChange }: { value: string; onChange: (v: string) => void }) {
  const presets = [
    { label: "Now", iso: "" },
    { label: "Tomorrow", iso: tomorrowIso() },
    { label: "+3 days", iso: inDaysIso(3) },
    { label: "Next week", iso: inDaysIso(7) },
  ];
  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2 text-base">
          <Calendar className="size-4 text-primary" />
          Schedule
        </CardTitle>
        <CardDescription>Leave empty to send immediately.</CardDescription>
      </CardHeader>
      <CardContent className="space-y-2">
        <div className="flex flex-wrap gap-1.5">
          {presets.map((p) => (
            <button
              key={p.label}
              onClick={() => onChange(p.iso)}
              className={cn(
                "rounded-full border px-3 py-1 text-xs transition-colors",
                value === p.iso
                  ? "border-primary/40 bg-primary/[0.1] text-foreground"
                  : "border-white/10 bg-white/[0.03] text-muted-foreground hover:bg-accent"
              )}
            >
              {p.label}
            </button>
          ))}
        </div>
        {value && (
          <p className="text-[11px] text-muted-foreground">
            Will run on <span className="font-mono">{new Date(value).toLocaleString()}</span>.
          </p>
        )}
      </CardContent>
    </Card>
  );
}

function tomorrowIso(): string {
  const d = new Date();
  d.setDate(d.getDate() + 1);
  d.setHours(9, 0, 0, 0);
  return d.toISOString();
}
function inDaysIso(n: number): string {
  const d = new Date();
  d.setDate(d.getDate() + n);
  d.setHours(9, 0, 0, 0);
  return d.toISOString();
}

function ReviewPanel({
  preview,
  onConfirm,
  confirmBusy,
  scheduleNote,
}: {
  preview: SmartPreviewOut;
  onConfirm: () => void;
  confirmBusy: boolean;
  scheduleNote: string;
}) {
  const r = preview.review;
  return (
    <motion.div initial={{ opacity: 0, y: 6 }} animate={{ opacity: 1, y: 0 }}>
      <Card className={cn("border-l-4", r.recommended_tier === "Red" ? "border-l-tier-red" : "border-l-tier-yellow")}>
        <CardHeader>
          <CardTitle className="flex items-center gap-2 text-base">
            <ShieldAlert className="size-4 text-tier-yellow" />
            Security review
            <TierBadge tier={r.recommended_tier as "Green" | "Yellow" | "Red"} size="sm" />
          </CardTitle>
          <CardDescription>{scheduleNote}</CardDescription>
        </CardHeader>
        <CardContent className="space-y-3">
          <p className="text-sm leading-relaxed">{r.summary}</p>
          {r.flags.length > 0 && (
            <ul className="space-y-1 text-xs">
              {r.flags.map((f) => (
                <li
                  key={f.code}
                  className="flex items-start gap-2 rounded-md border border-white/5 bg-white/[0.03] p-2"
                >
                  <Shield className="mt-0.5 size-3.5 text-muted-foreground" />
                  <div>
                    <code className="font-mono text-[10px] text-muted-foreground">{f.code}</code>
                    <span className="ml-2">{f.message}</span>
                  </div>
                </li>
              ))}
            </ul>
          )}
          {preview.balance_check && (
            <div className="flex items-center justify-between rounded-md border border-white/5 bg-white/[0.03] p-2 text-xs">
              <span>After this transfer</span>
              <span
                className={cn(
                  "font-mono",
                  preview.balance_check.after_transfer_fen < 0 ? "text-tier-red" : "text-tier-green"
                )}
              >
                {fenToCny(preview.balance_check.after_transfer_fen)}
              </span>
            </div>
          )}
          <Button
            onClick={onConfirm}
            disabled={confirmBusy}
            variant={r.recommended_tier === "Red" ? "outline" : "gradient"}
            className="w-full"
          >
            {confirmBusy ? <Loader2 className="size-4 animate-spin" /> : <CheckCircle2 className="size-4" />}
            {r.recommended_tier === "Red" ? "Confirm anyway (Red tier)" : "Confirm transfer"}
          </Button>
        </CardContent>
      </Card>
    </motion.div>
  );
}

function KV({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-md border border-white/5 bg-white/[0.03] p-2.5">
      <p className="text-[10px] font-medium uppercase tracking-wider text-muted-foreground">{label}</p>
      <p className="mt-1 text-sm font-medium">{value}</p>
    </div>
  );
}

// =====================================================================
// Schedule tab
// =====================================================================

function ScheduleTab() {
  const [rows, setRows] = useState<
    Array<{
      id: string;
      payee_id: string;
      amount_fen: number;
      memo: string;
      schedule: string | null;
      status: string;
      created_at: string;
    }>
  >([]);
  const [busy, setBusy] = useState(false);
  const { toast } = useToast();

  const refresh = () => {
    setBusy(true);
    smart.scheduled().then(setRows).catch(() => toast({ variant: "error", title: "Failed to load" })).finally(() => setBusy(false));
  };
  useEffect(() => { refresh(); }, []);

  return (
    <FadeIn delay={0.05}>
      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle className="flex items-center gap-2">
                <Clock className="size-4 text-primary" />
                Scheduled transfers
              </CardTitle>
              <CardDescription>Drafts the agent will post at the scheduled time.</CardDescription>
            </div>
            <Button size="sm" variant="outline" onClick={refresh} disabled={busy}>
              {busy ? <Loader2 className="size-4 animate-spin" /> : <Refresh />}
              Refresh
            </Button>
          </div>
        </CardHeader>
        <CardContent>
          {rows.length === 0 ? (
            <p className="text-sm text-muted-foreground">
              No scheduled transfers. Switch to the &ldquo;Pay&rdquo; tab and pick a future date.
            </p>
          ) : (
            <Stagger>
              <ul className="space-y-2">
                {rows.map((r) => (
                  <StaggerItem key={r.id}>
                    <motion.li
                      layout
                      className="flex items-center justify-between rounded-md border border-white/5 bg-white/[0.03] p-3"
                    >
                      <div>
                        <div className="flex items-center gap-2">
                          <CircleDollarSign className="size-4 text-primary" />
                          <span className="font-mono text-sm">{fenToCny(r.amount_fen)}</span>
                          <span className="text-sm">→</span>
                          <code className="font-mono text-[11px] text-muted-foreground">{r.payee_id}</code>
                        </div>
                        <div className="mt-0.5 text-[11px] text-muted-foreground">
                          {r.memo || "(no memo)"} · {r.schedule ? `runs ${new Date(r.schedule).toLocaleString()}` : "no schedule"}
                        </div>
                      </div>
                      <Badge variant={r.status === "scheduled" ? "yellow" : "muted"}>{r.status}</Badge>
                    </motion.li>
                  </StaggerItem>
                ))}
              </ul>
            </Stagger>
          )}
        </CardContent>
      </Card>
    </FadeIn>
  );
}

const Refresh = ({ className = "" }: { className?: string }) => (
  <svg className={cn("size-4", className)} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
    <path d="M3 12a9 9 0 0 1 15-6.7L21 8" />
    <path d="M21 3v5h-5" />
    <path d="M21 12a9 9 0 0 1-15 6.7L3 16" />
    <path d="M3 21v-5h5" />
  </svg>
);

// =====================================================================
// Splits tab
// =====================================================================

function SplitsTab() {
  const [bills, setBills] = useState<SplitBill[]>([]);
  const [createOpen, setCreateOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const { toast } = useToast();

  const refresh = () => {
    setBusy(true);
    splitBills.list().then(setBills).catch(() => toast({ variant: "error", title: "Failed to load" })).finally(() => setBusy(false));
  };
  useEffect(() => { refresh(); }, []);

  return (
    <FadeIn delay={0.05}>
      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle className="flex items-center gap-2">
                <Split className="size-4 text-primary" />
                Split bills
              </CardTitle>
              <CardDescription>One tap per participant, no spreadsheets.</CardDescription>
            </div>
            <div className="flex items-center gap-2">
              <Button size="sm" variant="outline" onClick={refresh} disabled={busy}>
                Refresh
              </Button>
              <Button size="sm" onClick={() => setCreateOpen(true)}>
                <Plus className="size-3.5" />
                New split
              </Button>
            </div>
          </div>
        </CardHeader>
        <CardContent>
          {bills.length === 0 ? (
            <p className="text-sm text-muted-foreground">No splits yet.</p>
          ) : (
            <Stagger>
              <ul className="space-y-3">
                {bills.map((b) => (
                  <StaggerItem key={b.id}>
                    <motion.li layout className="rounded-lg border border-white/5 bg-white/[0.03] p-4">
                      <div className="flex items-center justify-between gap-2">
                        <div>
                          <div className="text-sm font-semibold">{b.title}</div>
                          <div className="text-[11px] text-muted-foreground">
                            {b.memo || "(no memo)"} · total {fenToCny(b.total_fen)} · {b.currency}
                          </div>
                        </div>
                        <div className="flex items-center gap-2">
                          <Badge variant={b.status === "settled" ? "green" : b.status === "partial" ? "yellow" : "muted"}>
                            {b.status}
                          </Badge>
                          <Badge variant="muted">
                            {b.remaining_fen && b.remaining_fen > 0
                              ? `${fenToCny(b.remaining_fen)} left`
                              : "fully paid"}
                          </Badge>
                        </div>
                      </div>
                      <ul className="mt-3 space-y-1.5">
                        {(b.parts ?? []).map((p) => (
                          <li
                            key={p.id}
                            className="flex items-center justify-between rounded-md border border-white/5 bg-background/30 px-2.5 py-1.5 text-xs"
                          >
                            <div>
                              <div className="font-medium">{p.participant_name}</div>
                              <div className="text-[10px] text-muted-foreground">
                                {p.participant_phone_tail ? `phone ${p.participant_phone_tail}` : "no phone"}
                              </div>
                            </div>
                            <div className="flex items-center gap-2">
                              <span className="font-mono">{fenToCny(p.share_fen)}</span>
                              <Badge variant={p.status === "paid" ? "green" : "muted"}>{p.status}</Badge>
                            </div>
                          </li>
                        ))}
                      </ul>
                    </motion.li>
                  </StaggerItem>
                ))}
              </ul>
            </Stagger>
          )}
        </CardContent>
      </Card>
      <CreateSplitDialog open={createOpen} onOpenChange={setCreateOpen} onCreated={refresh} />
    </FadeIn>
  );
}

function CreateSplitDialog({
  open,
  onOpenChange,
  onCreated,
}: {
  open: boolean;
  onOpenChange: (v: boolean) => void;
  onCreated: () => void;
}) {
  const [title, setTitle] = useState("Friday dinner");
  const [memo, setMemo] = useState("Pizza + drinks");
  const [total, setTotal] = useState("300");
  const [parts, setParts] = useState([
    { participant_name: "Me", share_fen: "100" },
    { participant_name: "Li Wei", share_fen: "100" },
    { participant_name: "Anna", share_fen: "100" },
  ]);
  const [busy, setBusy] = useState(false);
  const { toast } = useToast();

  const totalFen = useMemo(() => {
    const n = Number(total);
    if (isNaN(n)) return 0;
    return Math.round(n * 100);
  }, [total]);

  const partsSum = useMemo(
    () => parts.reduce((s, p) => s + (Number(p.share_fen) || 0) * 100, 0),
    [parts]
  );

  const submit = async () => {
    if (!title.trim()) {
      toast({ variant: "error", title: "Title is required" });
      return;
    }
    if (totalFen !== partsSum) {
      toast({ variant: "error", title: "Shares must add up", description: `${fenToCny(partsSum)} ≠ ${fenToCny(totalFen)}` });
      return;
    }
    setBusy(true);
    try {
      await splitBills.create({
        title,
        memo: memo || undefined,
        total_fen: totalFen,
        parts: parts.map((p) => ({
          participant_name: p.participant_name,
          share_fen: Math.round((Number(p.share_fen) || 0) * 100),
        })),
      });
      toast({ variant: "success", title: "Split created" });
      onCreated();
      onOpenChange(false);
    } catch (e) {
      const msg = e instanceof Error ? e.message : "Create failed";
      toast({ variant: "error", title: "Create failed", description: msg });
    } finally {
      setBusy(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>New split-bill</DialogTitle>
          <DialogDescription>Each share appears in the list with its own one-tap “pay”.</DialogDescription>
        </DialogHeader>
        <div className="space-y-3">
          <div>
            <Label>Title</Label>
            <Input value={title} onChange={(e) => setTitle(e.target.value)} />
          </div>
          <div>
            <Label>Memo</Label>
            <Input value={memo} onChange={(e) => setMemo(e.target.value)} />
          </div>
          <div>
            <Label>Total (CNY)</Label>
            <Input type="number" min={0} step="0.01" value={total} onChange={(e) => setTotal(e.target.value)} />
          </div>
          <div>
            <Label>Shares</Label>
            <ul className="space-y-1.5">
              {parts.map((p, i) => (
                <li key={i} className="flex items-center gap-2">
                  <Input
                    className="flex-1"
                    value={p.participant_name}
                    onChange={(e) => {
                      const v = e.target.value;
                      setParts((cur) => cur.map((c, j) => (j === i ? { ...c, participant_name: v } : c)));
                    }}
                  />
                  <Input
                    className="w-24"
                    type="number"
                    min={0}
                    step="0.01"
                    value={p.share_fen}
                    onChange={(e) => {
                      const v = e.target.value;
                      setParts((cur) => cur.map((c, j) => (j === i ? { ...c, share_fen: v } : c)));
                    }}
                  />
                  <button
                    onClick={() => setParts((cur) => cur.filter((_, j) => j !== i))}
                    className="grid size-9 place-items-center rounded-md border border-white/10 text-muted-foreground hover:bg-accent"
                    type="button"
                  >
                    <X className="size-3.5" />
                  </button>
                </li>
              ))}
            </ul>
            <Button
              size="sm"
              variant="ghost"
              onClick={() => setParts((cur) => [...cur, { participant_name: "Guest", share_fen: "0" }])}
              className="mt-2"
            >
              <Plus className="size-3.5" />
              Add participant
            </Button>
            <p className="mt-2 text-[11px] text-muted-foreground">
              Sum: {fenToCny(partsSum)} / total {fenToCny(totalFen)}
              {totalFen !== partsSum && (
                <span className="ml-2 text-tier-red">must add up to total</span>
              )}
            </p>
          </div>
          <div className="flex justify-end">
            <Button onClick={submit} disabled={busy}>
              {busy ? <Loader2 className="size-4 animate-spin" /> : <Receipt className="size-4" />}
              Create split
            </Button>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
