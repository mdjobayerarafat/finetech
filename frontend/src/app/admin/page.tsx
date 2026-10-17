"use client";

import { useEffect, useMemo, useState } from "react";
import Link from "next/link";
import { motion } from "framer-motion";
import {
  Users,
  ShieldAlert,
  ShieldCheck,
  Activity,
  Receipt,
  AlertTriangle,
  RefreshCw,
  Loader2,
  Lock,
  Unlock,
  Bug,
  ShieldCheck as ShieldCheckIcon,
  ServerCrash,
  ArrowUpRight,
} from "lucide-react";
import {
  AdminShell,
} from "@/components/admin/admin-shell";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  adminApi,
  agentRulesApi,
  auditApi,
  type AdminOverview,
  type AdminUser,
  type AgentRule,
  type AdminConversation,
} from "@/lib/api";
import { FadeIn, Stagger, StaggerItem } from "@/components/motion/aurora";
import { useToast } from "@/components/ui/toast";
import { cn, relativeTime } from "@/lib/utils";

export default function AdminDashboardPage() {
  return (
    <AdminShell>
      <Body />
    </AdminShell>
  );
}

function Body() {
  const [overview, setOverview] = useState<AdminOverview | null>(null);
  const [users, setUsers] = useState<AdminUser[]>([]);
  const [rules, setRules] = useState<AgentRule[]>([]);
  const [conversations, setConversations] = useState<AdminConversation[]>([]);
  const [busy, setBusy] = useState(false);
  const { toast } = useToast();

  const refresh = async () => {
    setBusy(true);
    try {
      const [o, u, r, c] = await Promise.all([
        adminApi.overview(),
        adminApi.users(),
        agentRulesApi.list(),
        adminConversationsSafe(),
      ]);
      setOverview(o);
      setUsers(u);
      setRules(r.rules);
      setConversations(c);
    } catch (err) {
      const msg = err instanceof Error ? err.message : "Failed to load";
      toast({ variant: "error", title: "Failed to load admin overview", description: msg });
    } finally {
      setBusy(false);
    }
  };

  useEffect(() => {
    refresh();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const unlock = async (user_id: string) => {
    try {
      await auditApi.unlock(user_id, "Admin unlock from dashboard");
      toast({ variant: "success", title: "User unlocked" });
      refresh();
    } catch {
      toast({ variant: "error", title: "Unlock failed" });
    }
  };

  const enabledRules = useMemo(() => rules.filter((r) => r.enabled), [rules]);
  const blockedRules = useMemo(
    () => enabledRules.filter((r) => r.severity === "block"),
    [enabledRules],
  );

  return (
    <div className="space-y-6">
      <FadeIn>
        <div className="flex flex-wrap items-end justify-between gap-3">
          <div>
            <p className="text-[11px] font-medium uppercase tracking-widest text-muted-foreground">
              Admin · dashboard
            </p>
            <h1 className="mt-1 font-display text-3xl font-semibold tracking-tight">
              Workspace control centre
            </h1>
            <p className="mt-1 max-w-2xl text-sm text-muted-foreground">
              Live snapshot of users, accounts, audit, safety and the agent
              rules engine. All numbers come from the database; the audit
              hash chain is verified by the backend on every refresh.
            </p>
          </div>
          <Button variant="outline" onClick={refresh} disabled={busy}>
            {busy ? <Loader2 className="size-4 animate-spin" /> : <RefreshCw className="size-4" />}
            Refresh
          </Button>
        </div>
      </FadeIn>

      {/* KPI strip */}
      <KpiStrip overview={overview} />

      <div className="grid gap-4 lg:grid-cols-3">
        {/* Hash chain card */}
        <FadeIn delay={0.05}>
          <Card>
            <CardHeader>
              <CardTitle>Hash chain</CardTitle>
              <CardDescription>SHA-256 over the canonical event</CardDescription>
            </CardHeader>
            <CardContent>
              {!overview ? (
                <p className="text-sm text-muted-foreground">Loading…</p>
              ) : (
                <div className="flex items-start gap-3">
                  {overview.chain.ok ? (
                    <ShieldCheck className="size-6 text-tier-green" />
                  ) : (
                    <ShieldAlert className="size-6 text-tier-red" />
                  )}
                  <div className="space-y-1">
                    <div className="font-display text-lg font-semibold tracking-tight">
                      {overview.chain.ok ? "Chain intact" : "Chain broken"}
                    </div>
                    <div className="text-xs text-muted-foreground">
                      {overview.chain.events} event(s) walked
                      {overview.chain.break_at && (
                        <span className="ml-2 text-tier-red">
                          break at <code className="font-mono">{overview.chain.break_at.slice(0, 16)}…</code>
                        </span>
                      )}
                    </div>
                    {overview.chain.reason && (
                      <div className="text-xs text-tier-red">{overview.chain.reason}</div>
                    )}
                  </div>
                </div>
              )}
            </CardContent>
          </Card>
        </FadeIn>

        {/* Locked accounts card */}
        <FadeIn delay={0.08}>
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center justify-between">
                <span>Locked accounts</span>
                {overview && overview.locked_users.length > 0 && (
                  <Badge variant="red">{overview.locked_users.length}</Badge>
                )}
              </CardTitle>
              <CardDescription>Require admin unlock before any Yellow/Red action</CardDescription>
            </CardHeader>
            <CardContent>
              {!overview ? (
                <p className="text-sm text-muted-foreground">Loading…</p>
              ) : overview.locked_users.length === 0 ? (
                <div className="flex items-center gap-2 text-sm text-muted-foreground">
                  <ShieldCheck className="size-4 text-tier-green" />
                  No locked accounts. Everything is calm.
                </div>
              ) : (
                <ul className="space-y-2">
                  {overview.locked_users.slice(0, 5).map((u) => (
                    <motion.li
                      key={u.id}
                      layout
                      className="flex items-center justify-between gap-3 rounded-md border border-tier-red/30 bg-tier-red/[0.04] p-3"
                    >
                      <div className="min-w-0">
                        <div className="flex items-center gap-2">
                          <Lock className="size-3.5 text-tier-red" />
                          <div className="truncate text-sm font-medium">{u.display_name}</div>
                          <Badge variant="muted">{u.username}</Badge>
                        </div>
                        <div className="mt-0.5 truncate text-[11px] text-muted-foreground">
                          {u.safety_lock_reason ?? "No reason recorded."}
                        </div>
                      </div>
                      <Button size="sm" variant="outline" onClick={() => unlock(u.id)}>
                        <Unlock className="size-3.5" />
                        Unlock
                      </Button>
                    </motion.li>
                  ))}
                </ul>
              )}
            </CardContent>
          </Card>
        </FadeIn>

        {/* Agent rules summary */}
        <FadeIn delay={0.11}>
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center justify-between">
                <span className="flex items-center gap-2">
                  <ShieldCheckIcon className="size-4 text-primary" />
                  Agent rules
                </span>
                <Link href="/admin/rules" className="inline-flex items-center gap-1 text-[11px] text-muted-foreground hover:text-foreground">
                  Manage <ArrowUpRight className="size-3" />
                </Link>
              </CardTitle>
              <CardDescription>Active constraints on what the agent can do</CardDescription>
            </CardHeader>
            <CardContent>
              <div className="grid grid-cols-3 gap-3 text-center">
                <div>
                  <div className="font-display text-2xl font-semibold">{enabledRules.length}</div>
                  <div className="text-[10px] uppercase tracking-wider text-muted-foreground">enabled</div>
                </div>
                <div>
                  <div className="font-display text-2xl font-semibold text-tier-red">{blockedRules.length}</div>
                  <div className="text-[10px] uppercase tracking-wider text-muted-foreground">blocking</div>
                </div>
                <div>
                  <div className="font-display text-2xl font-semibold text-tier-yellow">
                    {enabledRules.filter((r) => r.severity === "warn").length}
                  </div>
                  <div className="text-[10px] uppercase tracking-wider text-muted-foreground">warnings</div>
                </div>
              </div>
              {enabledRules.length > 0 ? (
                <ul className="mt-4 space-y-1.5">
                  {enabledRules.slice(0, 4).map((r) => (
                    <li
                      key={r.id}
                      className="flex items-center justify-between gap-2 rounded-md border border-white/5 bg-white/[0.02] px-2.5 py-1.5 text-xs"
                    >
                      <div className="flex min-w-0 items-center gap-2">
                        <Badge
                          variant={
                            r.severity === "block" ? "red" : r.severity === "warn" ? "yellow" : "muted"
                          }
                        >
                          {r.severity}
                        </Badge>
                        <code className="truncate font-mono text-[10px] text-muted-foreground">
                          {r.kind}
                        </code>
                        <span className="truncate text-foreground/80">{r.name}</span>
                      </div>
                    </li>
                  ))}
                </ul>
              ) : (
                <p className="mt-3 text-xs text-muted-foreground">
                  No rules enabled — agents operate with no extra constraints.
                </p>
              )}
            </CardContent>
          </Card>
        </FadeIn>
      </div>

      <div className="grid gap-4 lg:grid-cols-2">
        {/* Recent safety events */}
        <FadeIn delay={0.15}>
          <Card>
            <CardHeader>
              <CardTitle>Recent safety events</CardTitle>
              <CardDescription>Failed MFA · denied high-risk · rapid high-value · admin unlocks</CardDescription>
            </CardHeader>
            <CardContent>
              {!overview ? (
                <p className="text-sm text-muted-foreground">Loading…</p>
              ) : overview.recent_safety.length === 0 ? (
                <p className="text-sm text-muted-foreground">No safety events.</p>
              ) : (
                <Stagger>
                  <ul className="divide-y">
                    {overview.recent_safety.slice(0, 8).map((ev) => (
                      <StaggerItem key={ev.id}>
                        <motion.li
                          layout
                          className="flex items-start justify-between gap-3 py-3"
                        >
                          <div>
                            <div className="flex items-center gap-2">
                              <Badge
                                variant={
                                  ev.kind === "denied_high_risk"
                                    ? "red"
                                    : ev.kind === "injection_pattern"
                                      ? "red"
                                      : ev.kind === "admin_unlock"
                                        ? "green"
                                        : "yellow"
                                }
                              >
                                {ev.kind === "denied_high_risk" && (
                                  <AlertTriangle className="size-2.5" />
                                )}
                                {ev.kind}
                              </Badge>
                              <code className="font-mono text-[10px] text-muted-foreground">
                                {ev.user_id.slice(0, 16)}…
                              </code>
                            </div>
                            <pre className="mt-1 text-[10px] text-muted-foreground line-clamp-2">
                              {JSON.stringify(ev.metadata, null, 2)}
                            </pre>
                          </div>
                          <div className="text-[10px] uppercase tracking-wider text-muted-foreground">
                            {relativeTime(ev.created_at)}
                          </div>
                        </motion.li>
                      </StaggerItem>
                    ))}
                  </ul>
                </Stagger>
              )}
            </CardContent>
          </Card>
        </FadeIn>

        {/* Recent conversations */}
        <FadeIn delay={0.2}>
          <Card>
            <CardHeader>
              <CardTitle>Recent conversations</CardTitle>
              <CardDescription>Latest chat sessions across the workspace</CardDescription>
            </CardHeader>
            <CardContent>
              {conversations.length === 0 ? (
                <p className="text-sm text-muted-foreground">No conversations yet.</p>
              ) : (
                <Stagger>
                  <ul className="divide-y">
                    {conversations.slice(0, 8).map((c) => {
                      const user = users.find((u) => u.id === c.user_id);
                      return (
                        <StaggerItem key={c.id}>
                          <motion.li layout className="flex items-center justify-between gap-3 py-2.5 text-sm">
                            <div className="flex min-w-0 items-center gap-2">
                              <Activity className="size-3.5 text-muted-foreground" />
                              <code className="font-mono text-[10px] text-muted-foreground">
                                {c.id.slice(0, 18)}…
                              </code>
                              <span className="truncate text-foreground/80">
                                {user ? user.display_name : c.user_id.slice(0, 12)}
                              </span>
                            </div>
                            <div className="text-[10px] uppercase tracking-wider text-muted-foreground">
                              {relativeTime(c.last_message_at)}
                            </div>
                          </motion.li>
                        </StaggerItem>
                      );
                    })}
                  </ul>
                </Stagger>
              )}
            </CardContent>
          </Card>
        </FadeIn>
      </div>

      {/* Users table */}
      <FadeIn delay={0.25}>
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center justify-between">
              <span>Workspace accounts</span>
              <Link href="/admin/users" className="inline-flex items-center gap-1 text-[11px] text-muted-foreground hover:text-foreground">
                See all <ArrowUpRight className="size-3" />
              </Link>
            </CardTitle>
            <CardDescription>Seeded demo + admin identities</CardDescription>
          </CardHeader>
          <CardContent className="space-y-6">
            <UserGroup
              title="Customers"
              users={users.filter((u) => u.role === "customer")}
              onUnlock={unlock}
            />
            <UserGroup
              title="Admins / auditors"
              users={users.filter((u) => u.role === "admin")}
              onUnlock={unlock}
            />
          </CardContent>
        </Card>
      </FadeIn>

      {/* Quick links */}
      <FadeIn delay={0.3}>
        <Card>
          <CardHeader>
            <CardTitle>Quick actions</CardTitle>
            <CardDescription>Open the dedicated admin tools</CardDescription>
          </CardHeader>
          <CardContent>
            <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
              <QuickLink href="/admin/users" icon={<Users className="size-4" />} label="Manage users" hint="Inspect accounts, transactions and conversations" />
              <QuickLink href="/admin/redteam" icon={<Bug className="size-4" />} label="Live red-team" hint="Probe prompt injection patterns" />
              <QuickLink href="/admin/rules" icon={<ShieldCheckIcon className="size-4" />} label="Agent rules" hint="Restrict what the agent can do" />
              <QuickLink href="/admin/audit" icon={<ServerCrash className="size-4" />} label="Audit ledger" hint="Append-only event log" />
            </div>
          </CardContent>
        </Card>
      </FadeIn>
    </div>
  );
}

function KpiStrip({ overview }: { overview: AdminOverview | null }) {
  const kpi = overview?.counts;
  const chain = overview?.chain;
  return (
    <Stagger>
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
        <StaggerItem>
          <KpiTile
            icon={<Users className="size-4" />}
            label="Users"
            value={kpi?.users ?? 0}
            hint={`${kpi?.customers ?? 0} customers · ${kpi?.admins ?? 0} admins`}
            tone="primary"
          />
        </StaggerItem>
        <StaggerItem>
          <KpiTile
            icon={<ShieldAlert className="size-4" />}
            label="Locked accounts"
            value={kpi?.locked ?? 0}
            hint="safety_lock_state = locked"
            tone={(kpi?.locked ?? 0) > 0 ? "red" : "muted"}
          />
        </StaggerItem>
        <StaggerItem>
          <KpiTile
            icon={<Receipt className="size-4" />}
            label="Transactions"
            value={kpi?.transactions ?? 0}
            hint="across all demo users"
            tone="muted"
          />
        </StaggerItem>
        <StaggerItem>
          <KpiTile
            icon={<Activity className="size-4" />}
            label="Audit events"
            value={kpi?.audit_events ?? 0}
            hint={chain ? `${chain.events} walked` : "—"}
            tone={chain?.ok ? "green" : "red"}
          />
        </StaggerItem>
      </div>
    </Stagger>
  );
}

function KpiTile({
  icon,
  label,
  value,
  hint,
  tone,
}: {
  icon: React.ReactNode;
  label: string;
  value: number;
  hint: string;
  tone: "primary" | "red" | "green" | "muted";
}) {
  const toneCls =
    tone === "primary"
      ? "text-primary"
      : tone === "red"
        ? "text-tier-red"
        : tone === "green"
          ? "text-tier-green"
          : "text-muted-foreground";
  return (
    <Card>
      <CardContent className="p-5">
        <div className="flex items-center justify-between">
          <p className="text-[10px] font-medium uppercase tracking-[0.18em] text-muted-foreground">
            {label}
          </p>
          <span className={cn("grid size-7 place-items-center rounded-md bg-white/5 ring-1 ring-white/10", toneCls)}>
            {icon}
          </span>
        </div>
        <div className="mt-2 font-display text-2xl font-semibold tracking-tight">{value}</div>
        <div className="mt-1 text-[11px] text-muted-foreground">{hint}</div>
      </CardContent>
    </Card>
  );
}

function UserGroup({
  title,
  users,
  onUnlock,
}: {
  title: string;
  users: AdminUser[];
  onUnlock: (id: string) => void;
}) {
  if (users.length === 0) return null;
  return (
    <div>
      <p className="mb-2 text-[10px] font-medium uppercase tracking-[0.18em] text-muted-foreground">
        {title} · {users.length}
      </p>
      <Stagger>
        <ul className="grid gap-2 sm:grid-cols-2">
          {users.map((u) => (
            <StaggerItem key={u.id}>
              <motion.li
                layout
                className="flex items-center justify-between gap-3 rounded-md border border-white/5 bg-white/[0.03] p-3"
              >
                <Link
                  href={`/admin/users/${u.id}`}
                  className="min-w-0 flex-1 transition-colors hover:text-foreground"
                >
                  <div className="flex items-center gap-2">
                    <div className="truncate text-sm font-medium">{u.display_name}</div>
                    <Badge variant="muted">{u.username}</Badge>
                    {u.safety_lock_state === "locked" && (
                      <Badge variant="red">
                        <Lock className="size-2.5" />
                        locked
                      </Badge>
                    )}
                  </div>
                  <div className="mt-0.5 truncate text-[11px] text-muted-foreground">
                    {u.account_count} account{u.account_count === 1 ? "" : "s"} ·{" "}
                    {u.transaction_count} txns · risk {u.risk_tolerance} · horizon {u.horizon}
                    {u.locked_event_count > 0 && (
                      <span className="ml-2 text-tier-yellow">
                        · {u.locked_event_count} safety event
                        {u.locked_event_count === 1 ? "" : "s"}
                      </span>
                    )}
                  </div>
                  {u.safety_lock_reason && (
                    <div className="mt-0.5 truncate text-[11px] text-tier-red">
                      {u.safety_lock_reason}
                    </div>
                  )}
                </Link>
                {u.safety_lock_state === "locked" && (
                  <Button size="sm" variant="outline" onClick={() => onUnlock(u.id)}>
                    <Unlock className="size-3.5" />
                    Unlock
                  </Button>
                )}
              </motion.li>
            </StaggerItem>
          ))}
        </ul>
      </Stagger>
    </div>
  );
}

function QuickLink({
  href,
  icon,
  label,
  hint,
}: {
  href: string;
  icon: React.ReactNode;
  label: string;
  hint: string;
}) {
  return (
    <Link
      href={href}
      className="group flex items-center gap-3 rounded-lg border border-white/5 bg-white/[0.02] p-3 transition-colors hover:border-primary/30 hover:bg-white/[0.05]"
    >
      <span className="grid size-9 place-items-center rounded-md bg-primary/15 text-primary ring-1 ring-primary/30">
        {icon}
      </span>
      <div className="min-w-0">
        <div className="flex items-center gap-1 text-sm font-medium">
          {label}
          <ArrowUpRight className="size-3.5 text-muted-foreground transition-transform group-hover:translate-x-0.5 group-hover:-translate-y-0.5" />
        </div>
        <div className="truncate text-[11px] text-muted-foreground">{hint}</div>
      </div>
    </Link>
  );
}

// Safe wrapper: if the admin conversations endpoint errors out (network
// blip, role-mismatch race during hot-reload), we don't want the whole
// dashboard to fail — degrade to "no conversations" instead.
async function adminConversationsSafe(): Promise<AdminConversation[]> {
  try {
    const res = await (await import("@/lib/api")).adminConversationsApi.list();
    return res.conversations;
  } catch {
    return [];
  }
}
