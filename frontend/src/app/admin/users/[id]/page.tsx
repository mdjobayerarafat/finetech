"use client";

import { useEffect, useMemo, useState } from "react";
import Link from "next/link";
import { useParams, useRouter } from "next/navigation";
import { motion, AnimatePresence } from "framer-motion";
import {
  ArrowLeft,
  RefreshCw,
  Loader2,
  Lock,
  Unlock,
  Receipt,
  Wallet,
  MessageSquare,
  ShieldAlert,
  Activity,
  ChevronRight,
  Eye,
  X,
  TrendingUp,
  TrendingDown,
} from "lucide-react";
import { AdminShell } from "@/components/admin/admin-shell";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  adminUserApi,
  adminConversationsApi,
  auditApi,
  type AdminUserDetail,
  type AdminTransaction,
  type AdminConversation,
  type AdminConversationDetail,
  type SafetyEvent,
} from "@/lib/api";
import { FadeIn, Stagger, StaggerItem } from "@/components/motion/aurora";
import { useToast } from "@/components/ui/toast";
import { cn, formatDateTime, formatFen, relativeTime } from "@/lib/utils";

type Tab = "overview" | "accounts" | "transactions" | "conversations" | "safety";

export default function AdminUserDetailPage() {
  return (
    <AdminShell>
      <Body />
    </AdminShell>
  );
}

function Body() {
  const params = useParams<{ id: string }>();
  const router = useRouter();
  const userId = params.id;
  const [detail, setDetail] = useState<AdminUserDetail | null>(null);
  const [txns, setTxns] = useState<AdminTransaction[]>([]);
  const [conversations, setConversations] = useState<AdminConversation[]>([]);
  const [safety, setSafety] = useState<SafetyEvent[]>([]);
  const [tab, setTab] = useState<Tab>("overview");
  const [busy, setBusy] = useState(false);
  const [activeConv, setActiveConv] = useState<AdminConversationDetail | null>(null);
  const { toast } = useToast();

  const refresh = async () => {
    if (!userId) return;
    setBusy(true);
    try {
      const [d, t, c, s] = await Promise.all([
        adminUserApi.detail(userId),
        adminUserApi.transactions(userId, 200),
        adminUserApi.conversations(userId),
        adminUserApi.safetyEvents(userId),
      ]);
      setDetail(d);
      setTxns(t.transactions);
      setConversations(c.conversations);
      setSafety(s.events);
    } catch (err) {
      const msg = err instanceof Error ? err.message : "Failed to load user";
      toast({ variant: "error", title: "Failed to load user", description: msg });
    } finally {
      setBusy(false);
    }
  };

  useEffect(() => {
    refresh();
  }, [userId]);

  const unlock = async () => {
    if (!detail) return;
    try {
      await auditApi.unlock(detail.id, "Admin unlock from user detail");
      toast({ variant: "success", title: "User unlocked" });
      refresh();
    } catch {
      toast({ variant: "error", title: "Unlock failed" });
    }
  };

  const openConversation = async (id: string) => {
    try {
      setActiveConv(await adminConversationsApi.detail(id));
    } catch {
      toast({ variant: "error", title: "Failed to load conversation" });
    }
  };

  const totals = useMemo(() => {
    const inflow = txns
      .filter((t) => t.direction === "credit")
      .reduce((a, t) => a + t.amount_fen, 0);
    const outflow = txns
      .filter((t) => t.direction === "debit")
      .reduce((a, t) => a + t.amount_fen, 0);
    return { inflow, outflow, count: txns.length };
  }, [txns]);

  if (!detail) {
    return (
      <div className="grid min-h-[40dvh] place-items-center text-sm text-muted-foreground">
        <div className="flex items-center gap-3">
          <Loader2 className="size-4 animate-spin" />
          Loading user…
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <FadeIn>
        <div className="flex flex-wrap items-end justify-between gap-3">
          <div className="space-y-2">
            <button
              onClick={() => router.push("/admin/users")}
              className="inline-flex items-center gap-1 text-[11px] text-muted-foreground hover:text-foreground"
            >
              <ArrowLeft className="size-3" />
              Back to users
            </button>
            <div className="flex items-center gap-3">
              <div className="grid size-12 place-items-center rounded-full bg-gradient-to-br from-primary/30 to-indigo-500/30 font-display text-lg font-semibold">
                {detail.display_name.slice(0, 1)}
              </div>
              <div>
                <h1 className="font-display text-3xl font-semibold tracking-tight">
                  {detail.display_name}
                </h1>
                <div className="mt-0.5 flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
                  <code className="font-mono text-[10px]">{detail.id}</code>
                  <span>·</span>
                  <span>{detail.username}</span>
                  <span>·</span>
                  <Badge variant="muted">{detail.role}</Badge>
                  {detail.safety_lock_state === "locked" ? (
                    <Badge variant="red">
                      <Lock className="size-2.5" />
                      locked
                    </Badge>
                  ) : (
                    <Badge variant="green">unlocked</Badge>
                  )}
                </div>
              </div>
            </div>
          </div>
          <div className="flex items-center gap-2">
            {detail.safety_lock_state === "locked" && (
              <Button variant="outline" onClick={unlock}>
                <Unlock className="size-4" />
                Unlock
              </Button>
            )}
            <Button variant="outline" onClick={refresh} disabled={busy}>
              {busy ? <Loader2 className="size-4 animate-spin" /> : <RefreshCw className="size-4" />}
              Refresh
            </Button>
          </div>
        </div>
      </FadeIn>

      {/* Tab strip */}
      <FadeIn delay={0.05}>
        <div className="flex flex-wrap items-center gap-1 rounded-xl border border-white/5 bg-card/40 p-1.5 backdrop-blur">
          {(
            [
              { v: "overview", label: "Overview", icon: Eye },
              { v: "accounts", label: `Accounts (${detail.accounts.length})`, icon: Wallet },
              { v: "transactions", label: `Transactions (${txns.length})`, icon: Receipt },
              { v: "conversations", label: `Conversations (${conversations.length})`, icon: MessageSquare },
              { v: "safety", label: `Safety (${safety.length})`, icon: ShieldAlert },
            ] as { v: Tab; label: string; icon: React.ComponentType<{ className?: string }> }[]
          ).map((t) => {
            const active = tab === t.v;
            return (
              <button
                key={t.v}
                onClick={() => setTab(t.v)}
                className={cn(
                  "relative inline-flex items-center gap-2 rounded-lg px-3 py-1.5 text-sm transition-colors",
                  active ? "text-foreground" : "text-muted-foreground hover:text-foreground",
                )}
              >
                {active && (
                  <motion.span
                    layoutId="user-tab"
                    className="absolute inset-0 -z-0 rounded-lg bg-gradient-to-r from-primary/25 via-primary/10 to-transparent ring-1 ring-primary/30"
                    transition={{ type: "spring", damping: 22, stiffness: 240 }}
                  />
                )}
                <t.icon
                  className={cn(
                    "relative z-10 size-3.5",
                    active && "text-primary",
                  )}
                />
                <span className="relative z-10">{t.label}</span>
              </button>
            );
          })}
        </div>
      </FadeIn>

      {tab === "overview" && (
        <OverviewTab
          detail={detail}
          totals={totals}
          recentTxns={txns.slice(0, 8)}
          recentConvs={conversations.slice(0, 5)}
          onOpenConversation={openConversation}
        />
      )}
      {tab === "accounts" && <AccountsTab detail={detail} />}
      {tab === "transactions" && <TransactionsTab txns={txns} />}
      {tab === "conversations" && (
        <ConversationsTab
          conversations={conversations}
          onOpen={openConversation}
        />
      )}
      {tab === "safety" && <SafetyTab events={safety} />}

      <AnimatePresence>
        {activeConv && (
          <ConversationModal
            detail={activeConv}
            onClose={() => setActiveConv(null)}
          />
        )}
      </AnimatePresence>
    </div>
  );
}

function OverviewTab({
  detail,
  totals,
  recentTxns,
  recentConvs,
  onOpenConversation,
}: {
  detail: AdminUserDetail;
  totals: { inflow: number; outflow: number; count: number };
  recentTxns: AdminTransaction[];
  recentConvs: AdminConversation[];
  onOpenConversation: (id: string) => void;
}) {
  return (
    <div className="grid gap-4 lg:grid-cols-3">
      <FadeIn delay={0.1} className="lg:col-span-2">
        <Card>
          <CardHeader>
            <CardTitle>Activity</CardTitle>
            <CardDescription>Most recent transactions and conversations</CardDescription>
          </CardHeader>
          <CardContent className="space-y-6">
            <div className="grid gap-3 sm:grid-cols-3">
              <Stat
                icon={<TrendingUp className="size-4 text-tier-green" />}
                label="Inflow (90d)"
                value={formatFen(totals.inflow)}
                tone="green"
              />
              <Stat
                icon={<TrendingDown className="size-4 text-tier-red" />}
                label="Outflow (90d)"
                value={formatFen(totals.outflow)}
                tone="red"
              />
              <Stat
                icon={<Receipt className="size-4 text-primary" />}
                label="Transactions"
                value={String(totals.count)}
                tone="primary"
              />
            </div>
            <div>
              <h3 className="mb-2 text-[10px] font-medium uppercase tracking-[0.18em] text-muted-foreground">
                Recent transactions
              </h3>
              {recentTxns.length === 0 ? (
                <p className="text-sm text-muted-foreground">None.</p>
              ) : (
                <ul className="divide-y">
                  {recentTxns.map((t) => (
                    <li
                      key={t.id}
                      className="flex items-center justify-between gap-3 py-2 text-sm"
                    >
                      <div className="flex min-w-0 items-center gap-2">
                        <div
                          className={cn(
                            "grid size-7 place-items-center rounded-md",
                            t.direction === "credit"
                              ? "bg-tier-green/10 text-tier-green"
                              : "bg-tier-red/10 text-tier-red",
                          )}
                        >
                          {t.direction === "credit" ? (
                            <TrendingUp className="size-3.5" />
                          ) : (
                            <TrendingDown className="size-3.5" />
                          )}
                        </div>
                        <div className="min-w-0">
                          <div className="truncate">{t.merchant || t.category}</div>
                          <div className="truncate text-[10px] text-muted-foreground">
                            {t.category} · {relativeTime(t.occurred_at)}
                          </div>
                        </div>
                      </div>
                      <div
                        className={cn(
                          "tabular-nums",
                          t.direction === "credit" ? "text-tier-green" : "text-tier-red",
                        )}
                      >
                        {t.direction === "credit" ? "+" : "−"}
                        {formatFen(t.amount_fen)}
                      </div>
                    </li>
                  ))}
                </ul>
              )}
            </div>
            <div>
              <h3 className="mb-2 text-[10px] font-medium uppercase tracking-[0.18em] text-muted-foreground">
                Recent conversations
              </h3>
              {recentConvs.length === 0 ? (
                <p className="text-sm text-muted-foreground">No conversations yet.</p>
              ) : (
                <ul className="divide-y">
                  {recentConvs.map((c) => (
                    <li
                      key={c.id}
                      className="flex items-center justify-between gap-3 py-2 text-sm"
                    >
                      <button
                        onClick={() => onOpenConversation(c.id)}
                        className="flex min-w-0 items-center gap-2 hover:text-foreground"
                      >
                        <MessageSquare className="size-3.5 text-muted-foreground" />
                        <code className="font-mono text-[10px] text-muted-foreground">
                          {c.id.slice(0, 18)}…
                        </code>
                        <span className="truncate text-foreground/80">
                          {c.title ?? "Untitled"}
                        </span>
                      </button>
                      <span className="text-[10px] uppercase tracking-wider text-muted-foreground">
                        {relativeTime(c.last_message_at)}
                      </span>
                    </li>
                  ))}
                </ul>
              )}
            </div>
          </CardContent>
        </Card>
      </FadeIn>

      <FadeIn delay={0.15}>
        <Card>
          <CardHeader>
            <CardTitle>Profile</CardTitle>
            <CardDescription>Account metadata and lock state</CardDescription>
          </CardHeader>
          <CardContent className="space-y-3 text-sm">
            <Row k="Username" v={<code className="font-mono text-xs">{detail.username}</code>} />
            <Row k="Role" v={<Badge variant="muted">{detail.role}</Badge>} />
            <Row
              k="Language"
              v={<Badge variant="muted">{detail.preferred_language}</Badge>}
            />
            <Row k="Risk tolerance" v={detail.risk_tolerance} />
            <Row k="Horizon" v={detail.horizon} />
            <Row k="Created" v={formatDateTime(detail.created_at)} />
            <hr className="border-white/5" />
            <Row
              k="Safety state"
              v={
                detail.safety_lock_state === "locked" ? (
                  <Badge variant="red">
                    <Lock className="size-2.5" />
                    locked
                  </Badge>
                ) : (
                  <Badge variant="green">unlocked</Badge>
                )
              }
            />
            {detail.safety_lock_reason && (
              <Row k="Lock reason" v={detail.safety_lock_reason} />
            )}
            <Row k="Failed MFA" v={String(detail.failed_mfa_count)} />
            <hr className="border-white/5" />
            <Row k="Accounts" v={String(detail.accounts.length)} />
            <Row k="Transactions" v={String(detail.transaction_count)} />
            <Row k="Conversations" v={String(detail.conversation_count)} />
            <Row k="Safety events" v={String(detail.safety_event_count)} />
            <Row
              k="Total balance"
              v={<span className="font-display font-semibold">{formatFen(detail.balance_total_fen)}</span>}
            />
          </CardContent>
        </Card>
      </FadeIn>
    </div>
  );
}

function AccountsTab({ detail }: { detail: AdminUserDetail }) {
  if (detail.accounts.length === 0) {
    return (
      <Card>
        <CardContent className="grid h-40 place-items-center text-sm text-muted-foreground">
          No accounts for this user.
        </CardContent>
      </Card>
    );
  }
  return (
    <Stagger>
      <div className="grid gap-3 sm:grid-cols-2">
        {detail.accounts.map((a) => (
          <StaggerItem key={a.id}>
            <motion.div layout>
              <Card>
                <CardContent className="p-5">
                  <div className="flex items-start justify-between gap-2">
                    <div>
                      <p className="text-[10px] font-medium uppercase tracking-[0.18em] text-muted-foreground">
                        {a.currency}
                      </p>
                      <p className="mt-0.5 font-display text-base font-semibold">
                        {a.display_name}
                      </p>
                      <p className="font-mono text-xs text-muted-foreground">
                        {a.masked_number}
                      </p>
                    </div>
                    <Wallet className="size-5 text-primary" />
                  </div>
                  <div className="mt-4">
                    <p className="text-[10px] uppercase tracking-wider text-muted-foreground">
                      Available
                    </p>
                    <p className="mt-0.5 font-display text-2xl font-semibold tracking-tight">
                      {formatFen(a.available_balance_fen)}
                    </p>
                    {a.reserved_balance_fen > 0 && (
                      <p className="mt-0.5 text-[11px] text-muted-foreground">
                        Reserved: {formatFen(a.reserved_balance_fen)}
                      </p>
                    )}
                  </div>
                </CardContent>
              </Card>
            </motion.div>
          </StaggerItem>
        ))}
      </div>
    </Stagger>
  );
}

function TransactionsTab({ txns }: { txns: AdminTransaction[] }) {
  if (txns.length === 0) {
    return (
      <Card>
        <CardContent className="grid h-40 place-items-center text-sm text-muted-foreground">
          No transactions for this user.
        </CardContent>
      </Card>
    );
  }
  return (
    <Card>
      <CardContent className="p-0">
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead className="bg-white/[0.02] text-left text-[10px] uppercase tracking-wider text-muted-foreground">
              <tr>
                <th className="px-4 py-2 font-medium">Date</th>
                <th className="px-4 py-2 font-medium">Merchant</th>
                <th className="px-4 py-2 font-medium">Category</th>
                <th className="px-4 py-2 font-medium">Memo</th>
                <th className="px-4 py-2 font-medium text-right">Amount</th>
                <th className="px-4 py-2 font-medium">Status</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-white/5">
              {txns.map((t) => (
                <tr key={t.id} className="hover:bg-white/[0.02]">
                  <td className="px-4 py-2 text-xs text-muted-foreground">
                    {formatDateTime(t.occurred_at)}
                  </td>
                  <td className="px-4 py-2">{t.merchant || "—"}</td>
                  <td className="px-4 py-2">
                    <Badge variant="muted">{t.category}</Badge>
                  </td>
                  <td className="px-4 py-2 max-w-[280px] truncate text-muted-foreground">
                    {t.memo || "—"}
                  </td>
                  <td
                    className={cn(
                      "px-4 py-2 text-right tabular-nums",
                      t.direction === "credit" ? "text-tier-green" : "text-tier-red",
                    )}
                  >
                    {t.direction === "credit" ? "+" : "−"}
                    {formatFen(t.amount_fen)}
                  </td>
                  <td className="px-4 py-2">
                    <Badge variant={t.status === "posted" ? "green" : "muted"}>
                      {t.status}
                    </Badge>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </CardContent>
    </Card>
  );
}

function ConversationsTab({
  conversations,
  onOpen,
}: {
  conversations: AdminConversation[];
  onOpen: (id: string) => void;
}) {
  if (conversations.length === 0) {
    return (
      <Card>
        <CardContent className="grid h-40 place-items-center text-sm text-muted-foreground">
          No conversations for this user.
        </CardContent>
      </Card>
    );
  }
  return (
    <Stagger>
      <ul className="space-y-2">
        {conversations.map((c) => (
          <StaggerItem key={c.id}>
            <motion.li layout>
              <button
                onClick={() => onOpen(c.id)}
                className="group flex w-full items-center justify-between gap-3 rounded-lg border border-white/5 bg-white/[0.02] p-3 text-left transition-colors hover:border-primary/30 hover:bg-white/[0.04]"
              >
                <div className="flex min-w-0 items-center gap-2">
                  <MessageSquare className="size-4 text-muted-foreground" />
                  <code className="font-mono text-[10px] text-muted-foreground">
                    {c.id}
                  </code>
                  {c.advisor_mode && <Badge variant="muted">advisor</Badge>}
                  <span className="truncate font-medium">
                    {c.title ?? "Untitled"}
                  </span>
                </div>
                <div className="flex items-center gap-3">
                  <span className="text-[10px] uppercase tracking-wider text-muted-foreground">
                    {relativeTime(c.last_message_at)}
                  </span>
                  <ChevronRight className="size-3.5 text-muted-foreground transition-transform group-hover:translate-x-0.5" />
                </div>
              </button>
            </motion.li>
          </StaggerItem>
        ))}
      </ul>
    </Stagger>
  );
}

function SafetyTab({ events }: { events: SafetyEvent[] }) {
  if (events.length === 0) {
    return (
      <Card>
        <CardContent className="grid h-40 place-items-center text-sm text-muted-foreground">
          No safety events for this user.
        </CardContent>
      </Card>
    );
  }
  return (
    <Stagger>
      <ul className="space-y-2">
        {events.map((e) => (
          <StaggerItem key={e.id}>
            <motion.li layout>
              <div className="rounded-lg border border-white/5 bg-white/[0.02] p-3">
                <div className="flex items-center justify-between gap-2">
                  <div className="flex items-center gap-2">
                    <Badge
                      variant={
                        e.kind === "admin_unlock"
                          ? "green"
                          : e.kind === "denied_high_risk"
                            ? "red"
                            : e.kind === "injection_pattern"
                              ? "red"
                              : "yellow"
                      }
                    >
                      {e.kind}
                    </Badge>
                  </div>
                  <span className="text-[10px] uppercase tracking-wider text-muted-foreground">
                    {formatDateTime(e.created_at)}
                  </span>
                </div>
                <pre className="mt-2 overflow-x-auto rounded bg-muted/50 p-2 text-[10px] leading-relaxed text-foreground/80">
                  {JSON.stringify(e.metadata, null, 2)}
                </pre>
              </div>
            </motion.li>
          </StaggerItem>
        ))}
      </ul>
    </Stagger>
  );
}

function ConversationModal({
  detail,
  onClose,
}: {
  detail: AdminConversationDetail;
  onClose: () => void;
}) {
  return (
    <motion.div
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      className="fixed inset-0 z-50 grid place-items-center bg-background/80 p-4 backdrop-blur"
      onClick={onClose}
    >
      <motion.div
        initial={{ y: 12, opacity: 0 }}
        animate={{ y: 0, opacity: 1 }}
        exit={{ y: 12, opacity: 0 }}
        onClick={(e: React.MouseEvent) => e.stopPropagation()}
        className="glass-strong relative grid max-h-[80dvh] w-full max-w-3xl grid-rows-[auto_1fr] overflow-hidden rounded-2xl"
      >
        <div className="flex items-center justify-between border-b border-white/5 p-4">
          <div>
            <p className="text-[10px] font-medium uppercase tracking-wider text-muted-foreground">
              Conversation
            </p>
            <p className="font-mono text-xs text-foreground/80">
              {detail.conversation.id}
            </p>
          </div>
          <Button size="sm" variant="ghost" onClick={onClose}>
            <X className="size-4" />
          </Button>
        </div>
        <div className="overflow-y-auto p-4">
          {detail.messages.length === 0 ? (
            <p className="text-sm text-muted-foreground">
              This conversation has no messages.
            </p>
          ) : (
            <ul className="space-y-3">
              {detail.messages.map((m) => (
                <li
                  key={m.id}
                  className={cn(
                    "rounded-md border p-3",
                    m.role === "user"
                      ? "border-primary/20 bg-primary/[0.04]"
                      : m.role === "assistant"
                        ? "border-white/10 bg-white/[0.03]"
                        : "border-white/5 bg-white/[0.01]",
                  )}
                >
                  <div className="flex items-center justify-between gap-2">
                    <Badge
                      variant={
                        m.role === "user"
                          ? "muted"
                          : m.role === "assistant"
                            ? "muted"
                            : "muted"
                      }
                    >
                      {m.role}
                    </Badge>
                    <span className="text-[10px] uppercase tracking-wider text-muted-foreground">
                      {formatDateTime(m.created_at)}
                    </span>
                  </div>
                  <p className="mt-2 whitespace-pre-wrap text-sm">{m.content}</p>
                  {m.proposal_json && (
                    <pre className="mt-2 overflow-x-auto rounded bg-muted/50 p-2 text-[10px]">
                      {JSON.stringify(JSON.parse(m.proposal_json), null, 2)}
                    </pre>
                  )}
                  {m.tool_calls_json && (
                    <pre className="mt-2 overflow-x-auto rounded bg-muted/50 p-2 text-[10px]">
                      {JSON.stringify(JSON.parse(m.tool_calls_json), null, 2)}
                    </pre>
                  )}
                </li>
              ))}
            </ul>
          )}
        </div>
      </motion.div>
    </motion.div>
  );
}

function Stat({
  icon,
  label,
  value,
  tone,
}: {
  icon: React.ReactNode;
  label: string;
  value: string;
  tone: "primary" | "green" | "red";
}) {
  return (
    <div className="rounded-md border border-white/5 bg-white/[0.02] p-3">
      <div className="flex items-center gap-2">
        {icon}
        <span className="text-[10px] uppercase tracking-wider text-muted-foreground">
          {label}
        </span>
      </div>
      <div className="mt-1 font-display text-lg font-semibold tabular-nums">{value}</div>
    </div>
  );
}

function Row({ k, v }: { k: string; v: React.ReactNode }) {
  return (
    <div className="flex items-center justify-between gap-3 text-sm">
      <span className="text-xs text-muted-foreground">{k}</span>
      <span className="text-right">{v}</span>
    </div>
  );
}
