"use client";

import { useEffect, useMemo, useState } from "react";
import Link from "next/link";
import { motion } from "framer-motion";
import {
  Users as UsersIcon,
  Search,
  RefreshCw,
  Loader2,
  Lock,
  Unlock,
  ShieldAlert,
  ArrowUpRight,
} from "lucide-react";
import { AdminShell } from "@/components/admin/admin-shell";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { adminApi, auditApi, type AdminUser } from "@/lib/api";
import { FadeIn, Stagger, StaggerItem } from "@/components/motion/aurora";
import { useToast } from "@/components/ui/toast";
import { cn, formatDateTime } from "@/lib/utils";

export default function AdminUsersPage() {
  return (
    <AdminShell>
      <Body />
    </AdminShell>
  );
}

function Body() {
  const [users, setUsers] = useState<AdminUser[]>([]);
  const [q, setQ] = useState("");
  const [roleFilter, setRoleFilter] = useState<"all" | "customer" | "admin">("all");
  const [lockFilter, setLockFilter] = useState<"all" | "locked" | "unlocked">("all");
  const [busy, setBusy] = useState(false);
  const { toast } = useToast();

  const refresh = async () => {
    setBusy(true);
    try {
      setUsers(await adminApi.users());
    } catch (err) {
      const msg = err instanceof Error ? err.message : "Failed to load";
      toast({ variant: "error", title: "Failed to load users", description: msg });
    } finally {
      setBusy(false);
    }
  };

  useEffect(() => {
    refresh();
  }, []);

  const filtered = useMemo(() => {
    const needle = q.trim().toLowerCase();
    return users.filter((u) => {
      if (roleFilter !== "all" && u.role !== roleFilter) return false;
      if (lockFilter === "locked" && u.safety_lock_state !== "locked") return false;
      if (lockFilter === "unlocked" && u.safety_lock_state === "locked") return false;
      if (!needle) return true;
      return (
        u.display_name.toLowerCase().includes(needle) ||
        u.username.toLowerCase().includes(needle) ||
        (u.safety_lock_reason ?? "").toLowerCase().includes(needle)
      );
    });
  }, [users, q, roleFilter, lockFilter]);

  const unlock = async (user_id: string) => {
    try {
      await auditApi.unlock(user_id, "Admin unlock from users list");
      toast({ variant: "success", title: "User unlocked" });
      refresh();
    } catch {
      toast({ variant: "error", title: "Unlock failed" });
    }
  };

  return (
    <div className="space-y-6">
      <FadeIn>
        <div className="flex flex-wrap items-end justify-between gap-3">
          <div>
            <p className="text-[11px] font-medium uppercase tracking-widest text-muted-foreground">
              Admin · users
            </p>
            <h1 className="mt-1 font-display text-3xl font-semibold tracking-tight">
              All workspace accounts
            </h1>
            <p className="mt-1 max-w-2xl text-sm text-muted-foreground">
              Inspect every account in the simulation, see their balances,
              transaction volume and any active safety lock. Click a row to
              drill into the full user detail.
            </p>
          </div>
          <Button variant="outline" onClick={refresh} disabled={busy}>
            {busy ? <Loader2 className="size-4 animate-spin" /> : <RefreshCw className="size-4" />}
            Refresh
          </Button>
        </div>
      </FadeIn>

      <FadeIn delay={0.05}>
        <Card>
          <CardContent className="flex flex-wrap items-center gap-3 p-4">
            <div className="relative min-w-[220px] flex-1">
              <Search className="absolute left-2.5 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
              <Input
                placeholder="Search by name, username or lock reason…"
                value={q}
                onChange={(e) => setQ(e.target.value)}
                className="pl-8"
              />
            </div>
            <FilterPills
              label="Role"
              options={[
                { v: "all", label: "All" },
                { v: "customer", label: "Customers" },
                { v: "admin", label: "Admins" },
              ]}
              value={roleFilter}
              onChange={(v) => setRoleFilter(v as "all" | "customer" | "admin")}
            />
            <FilterPills
              label="Status"
              options={[
                { v: "all", label: "All" },
                { v: "unlocked", label: "Unlocked" },
                { v: "locked", label: "Locked" },
              ]}
              value={lockFilter}
              onChange={(v) => setLockFilter(v as "all" | "locked" | "unlocked")}
            />
            <div className="text-xs text-muted-foreground">
              Showing {filtered.length} of {users.length}
            </div>
          </CardContent>
        </Card>
      </FadeIn>

      <FadeIn delay={0.1}>
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <UsersIcon className="size-4 text-primary" />
              Accounts ({filtered.length})
            </CardTitle>
            <CardDescription>Click a row to open user detail</CardDescription>
          </CardHeader>
          <CardContent className="p-0">
            <div className="overflow-x-auto">
              <table className="w-full text-sm">
                <thead className="bg-white/[0.02] text-left text-[10px] uppercase tracking-wider text-muted-foreground">
                  <tr>
                    <th className="px-4 py-2 font-medium">User</th>
                    <th className="px-4 py-2 font-medium">Role</th>
                    <th className="px-4 py-2 font-medium">Accounts</th>
                    <th className="px-4 py-2 font-medium">Transactions</th>
                    <th className="px-4 py-2 font-medium">Safety</th>
                    <th className="px-4 py-2 font-medium">Risk</th>
                    <th className="px-4 py-2 font-medium">Created</th>
                    <th className="px-4 py-2" />
                  </tr>
                </thead>
                <tbody className="divide-y divide-white/5">
                  {filtered.map((u) => (
                    <tr
                      key={u.id}
                      className={cn(
                        "group transition-colors hover:bg-white/[0.03]",
                        u.safety_lock_state === "locked" && "bg-tier-red/[0.03]",
                      )}
                    >
                      <td className="px-4 py-3">
                        <Link
                          href={`/admin/users/${u.id}`}
                          className="flex items-center gap-2 hover:text-foreground"
                        >
                          <div className="grid size-8 place-items-center rounded-full bg-gradient-to-br from-primary/30 to-indigo-500/30 text-xs font-semibold">
                            {u.display_name.slice(0, 1)}
                          </div>
                          <div className="min-w-0">
                            <div className="truncate font-medium">{u.display_name}</div>
                            <div className="truncate text-[10px] text-muted-foreground">
                              {u.username} · {u.id.slice(0, 12)}…
                            </div>
                          </div>
                        </Link>
                      </td>
                      <td className="px-4 py-3">
                        <Badge variant={u.role === "admin" ? "muted" : "muted"}>
                          {u.role}
                        </Badge>
                      </td>
                      <td className="px-4 py-3 tabular-nums">{u.account_count}</td>
                      <td className="px-4 py-3 tabular-nums">{u.transaction_count}</td>
                      <td className="px-4 py-3">
                        {u.safety_lock_state === "locked" ? (
                          <div className="flex flex-col">
                            <Badge variant="red">
                              <Lock className="size-2.5" />
                              locked
                            </Badge>
                            {u.safety_lock_reason && (
                              <span className="mt-0.5 text-[10px] text-muted-foreground">
                                {u.safety_lock_reason}
                              </span>
                            )}
                          </div>
                        ) : (
                          <Badge variant="green">unlocked</Badge>
                        )}
                      </td>
                      <td className="px-4 py-3 text-xs text-muted-foreground">
                        {u.risk_tolerance} · {u.horizon}
                      </td>
                      <td className="px-4 py-3 text-xs text-muted-foreground">
                        {formatDateTime(u.created_at)}
                      </td>
                      <td className="px-4 py-3 text-right">
                        <div className="flex items-center justify-end gap-2">
                          {u.safety_lock_state === "locked" && (
                            <Button
                              size="sm"
                              variant="outline"
                              onClick={() => unlock(u.id)}
                            >
                              <Unlock className="size-3.5" />
                              Unlock
                            </Button>
                          )}
                          <Link
                            href={`/admin/users/${u.id}`}
                            className="inline-flex items-center gap-1 rounded-md border border-white/10 px-2 py-1 text-[11px] text-muted-foreground transition-colors hover:border-primary/30 hover:text-foreground"
                          >
                            Open <ArrowUpRight className="size-3" />
                          </Link>
                        </div>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
              {filtered.length === 0 && (
                <div className="grid h-32 place-items-center text-sm text-muted-foreground">
                  No users match your filters.
                </div>
              )}
            </div>
          </CardContent>
        </Card>
      </FadeIn>
    </div>
  );
}

function FilterPills({
  label,
  options,
  value,
  onChange,
}: {
  label: string;
  options: { v: string; label: string }[];
  value: string;
  onChange: (v: string) => void;
}) {
  return (
    <div className="flex items-center gap-2 rounded-md border border-white/10 bg-white/5 px-2 py-1 text-xs">
      <span className="text-[10px] uppercase tracking-wider text-muted-foreground">
        {label}
      </span>
      <div className="flex items-center gap-0.5">
        {options.map((o) => {
          const active = o.v === value;
          return (
            <button
              key={o.v}
              type="button"
              onClick={() => onChange(o.v)}
              className={cn(
                "rounded-md px-2 py-0.5 transition-colors",
                active
                  ? "bg-primary/20 text-foreground ring-1 ring-primary/30"
                  : "text-muted-foreground hover:text-foreground",
              )}
            >
              {o.label}
            </button>
          );
        })}
      </div>
    </div>
  );
}
