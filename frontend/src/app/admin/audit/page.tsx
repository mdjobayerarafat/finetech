"use client";

import { useEffect, useMemo, useState } from "react";
import { motion } from "framer-motion";
import { AdminShell } from "@/components/admin/admin-shell";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { adminApi, auditApi, type AdminUser, type AuditEvent, type ChainVerify } from "@/lib/api";
import { FadeIn, Stagger, StaggerItem } from "@/components/motion/aurora";
import { ShieldCheck, ShieldAlert, Loader2, RefreshCw, Filter } from "lucide-react";
import { useToast } from "@/components/ui/toast";
import { cn, formatDateTime } from "@/lib/utils";

export default function AdminAuditPage() {
  return (
    <AdminShell>
      <Body />
    </AdminShell>
  );
}

function Body() {
  const [events, setEvents] = useState<AuditEvent[]>([]);
  const [chain, setChain] = useState<ChainVerify | null>(null);
  const [users, setUsers] = useState<AdminUser[]>([]);
  const [actor, setActor] = useState<string>("all");
  const [busy, setBusy] = useState(false);
  const { toast } = useToast();

  const refresh = () => {
    setBusy(true);
    Promise.all([auditApi.list(200), auditApi.verifyChain(), adminApi.users()])
      .then(([e, c, u]) => {
        setEvents(e);
        setChain(c);
        setUsers(u);
      })
      .catch(() => {
        toast({ variant: "error", title: "Failed to load audit" });
      })
      .finally(() => setBusy(false));
  };

  useEffect(() => {
    refresh();
  }, []);

  const usersById = useMemo(() => {
    const m: Record<string, AdminUser> = {};
    for (const u of users) m[u.id] = u;
    return m;
  }, [users]);

  const filtered = useMemo(() => {
    if (actor === "all") return events;
    return events.filter((e) => e.actor_user_id === actor);
  }, [events, actor]);

  return (
    <div className="space-y-6">
      <FadeIn>
        <div className="flex flex-wrap items-end justify-between gap-3">
          <div>
            <p className="text-[11px] font-medium uppercase tracking-widest text-muted-foreground">
              Admin · audit
            </p>
            <h1 className="mt-1 font-display text-3xl font-semibold tracking-tight">
              Append-only ledger
            </h1>
            <p className="mt-1 max-w-2xl text-sm text-muted-foreground">
              Every state change is recorded with a SHA-256 chain. The verify
              endpoint walks the chain and reports the first break.
            </p>
          </div>
          <div className="flex items-center gap-2">
            <div className="flex items-center gap-2 rounded-md border border-white/10 bg-white/5 px-2 py-1 text-xs">
              <Filter className="size-3.5 text-muted-foreground" />
              <select
                value={actor}
                onChange={(e) => setActor(e.target.value)}
                className="bg-transparent text-xs outline-none"
              >
                <option value="all">All actors</option>
                {users.map((u) => (
                  <option key={u.id} value={u.id}>
                    {u.display_name} ({u.username})
                  </option>
                ))}
              </select>
            </div>
            <Button variant="outline" onClick={refresh} disabled={busy}>
              {busy ? <Loader2 className="size-4 animate-spin" /> : <RefreshCw className="size-4" />}
              Refresh
            </Button>
          </div>
        </div>
      </FadeIn>

      <FadeIn delay={0.1}>
        <Card>
          <CardContent className="flex flex-wrap items-center justify-between gap-3 p-5">
            <div className="flex items-center gap-3">
              {chain?.ok ? (
                <ShieldCheck className="size-6 text-tier-green" />
              ) : (
                <ShieldAlert className="size-6 text-tier-red" />
              )}
              <div>
                <div className="font-display text-lg font-semibold tracking-tight">
                  Hash chain {chain?.ok ? "intact" : "BROKEN"}
                </div>
                <div className="text-xs text-muted-foreground">
                  {chain?.events ?? 0} events walked
                  {chain?.break_at && (
                    <span className="ml-2 text-tier-red">
                      break at <code className="font-mono">{chain.break_at.slice(0, 16)}…</code>
                      {chain.reason ? ` · ${chain.reason}` : ""}
                    </span>
                  )}
                </div>
              </div>
            </div>
            <div className="flex flex-wrap items-center gap-2">
              <Badge variant="muted">{filtered.length} shown</Badge>
              <Badge variant={chain?.ok ? "green" : "red"}>
                {chain?.ok ? "verified" : "tampered"}
              </Badge>
            </div>
          </CardContent>
        </Card>
      </FadeIn>

      <FadeIn delay={0.2}>
        <Card>
          <CardHeader>
            <CardTitle>Recent events</CardTitle>
            <CardDescription>
              Newest first {actor !== "all" && "· filtered to selected actor"}
            </CardDescription>
          </CardHeader>
          <CardContent>
            {filtered.length === 0 ? (
              <div className="grid h-40 place-items-center text-sm text-muted-foreground">
                {events.length === 0
                  ? "No audit events yet. Run a chat or transfer to start the chain."
                  : "No events for the selected actor."}
              </div>
            ) : (
              <Stagger>
                <ul className="space-y-2">
                  {filtered.map((e) => {
                    const actorUser = e.actor_user_id ? usersById[e.actor_user_id] : undefined;
                    return (
                      <StaggerItem key={e.id}>
                        <motion.div
                          layout
                          className="rounded-lg border bg-background/30 p-3"
                        >
                          <div className="flex flex-wrap items-center justify-between gap-2">
                            <div className="flex flex-wrap items-center gap-2">
                              <code className="rounded bg-muted px-1.5 py-0.5 font-mono text-[10px] text-muted-foreground">
                                {e.action}
                              </code>
                              {e.tool && (
                                <code className="font-mono text-[10px] text-muted-foreground">
                                  {e.tool}
                                </code>
                              )}
                              {e.risk_tier && (
                                <Badge
                                  variant={
                                    e.risk_tier === "Green"
                                      ? "green"
                                      : e.risk_tier === "Yellow"
                                      ? "yellow"
                                      : "red"
                                  }
                                >
                                  {e.risk_tier}
                                </Badge>
                              )}
                              <Badge
                                variant={
                                  e.result === "ok"
                                    ? "green"
                                    : e.result === "denied"
                                    ? "yellow"
                                    : e.result === "blocked"
                                    ? "red"
                                    : "muted"
                                }
                              >
                                {e.result}
                              </Badge>
                              {actorUser ? (
                                <Badge variant="muted">{actorUser.display_name}</Badge>
                              ) : e.actor_user_id ? (
                                <Badge variant="muted">
                                  <code className="font-mono text-[10px]">
                                    {e.actor_user_id.slice(0, 12)}…
                                  </code>
                                </Badge>
                              ) : null}
                            </div>
                            <div className="text-[10px] uppercase tracking-wider text-muted-foreground">
                              {formatDateTime(e.created_at)}
                            </div>
                          </div>
                          {e.policy_reason && (
                            <p className="mt-1 text-[11px] text-muted-foreground">
                              <span className="font-mono">reason</span> {e.policy_reason}
                            </p>
                          )}
                          {e.error_code && (
                            <p className={cn("mt-1 text-[11px]", "text-tier-red")}>
                              <span className="font-mono">error</span> {e.error_code}
                            </p>
                          )}
                          <pre className="mt-2 max-h-32 overflow-auto rounded bg-muted/50 p-2 text-[10px] leading-relaxed text-foreground/80">
                            {JSON.stringify(e.parameters_masked, null, 2)}
                          </pre>
                        </motion.div>
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
  );
}