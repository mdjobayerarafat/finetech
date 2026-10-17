"use client";

import Link from "next/link";
import { usePathname, useRouter } from "next/navigation";
import { motion } from "framer-motion";
import {
  LayoutDashboard,
  MessageSquare,
  ArrowLeftRight,
  Receipt,
  CreditCard,
  Coins,
  Shield,
  ShieldCheck,
  LogOut,
  AlertTriangle,
  Activity,
  Bug,
  Calculator,
  Camera,
  Mic,
  Send,
  Users,
  PiggyBank,
} from "lucide-react";
import { useEffect } from "react";
import { useAuth } from "@/lib/store";
import { auth } from "@/lib/api";
import { useToast } from "@/components/ui/toast";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { LanguageButton } from "@/components/language-button";
import { useT } from "@/i18n/LanguageProvider";
import { cn } from "@/lib/utils";

type NavItem = { href: string; label: string; icon: React.ComponentType<{ className?: string }>; admin?: boolean; adminSection?: "primary" | "secondary" };
// Note: nav item `label` is a dictionary key (e.g. "nav.dashboard") that
// gets resolved through `t()` at render time. Keep them in sync with
// `frontend/src/i18n/dictionaries.ts`.

const CUSTOMER_NAV: NavItem[] = [
  { href: "/dashboard", label: "nav.dashboard", icon: LayoutDashboard },
  { href: "/chat", label: "nav.assistant", icon: MessageSquare },
  { href: "/transfers", label: "nav.transfers", icon: Send },
  { href: "/accounts", label: "nav.accounts", icon: Receipt },
  { href: "/budgets", label: "nav.budgets", icon: PiggyBank },
  { href: "/subscriptions", label: "nav.subscriptions", icon: ArrowLeftRight },
  { href: "/cards", label: "nav.cards", icon: CreditCard },
  { href: "/wealth", label: "nav.wealth", icon: Coins },
  { href: "/forecast", label: "nav.forecast", icon: Calculator },
  { href: "/receipt", label: "nav.receipt", icon: Camera },
  { href: "/voice", label: "nav.voice", icon: Mic },
];

const ADMIN_NAV: NavItem[] = [
  { href: "/admin", label: "nav.dashboard", icon: LayoutDashboard, adminSection: "primary" },
  { href: "/admin/users", label: "nav.users", icon: Users, adminSection: "primary" },
  { href: "/admin/redteam", label: "nav.redteam", icon: Bug, adminSection: "primary" },
  { href: "/admin/rules", label: "nav.rules", icon: ShieldCheck, adminSection: "primary" },
  { href: "/admin/audit", label: "nav.audit", icon: Activity, adminSection: "secondary" },
  { href: "/admin/safety", label: "nav.safety", icon: Shield, adminSection: "secondary" },
];

export function AppShell({ children }: { children: React.ReactNode }) {
  const { me, loading, reset } = useAuth();
  const router = useRouter();
  const pathname = usePathname();
  const { toast } = useToast();
  const t = useT();

  useEffect(() => {
    if (loading) return;
    if (!me) {
      router.replace("/login");
    }
  }, [me, loading, router]);

  if (loading || !me) {
    return (
      <div className="grid min-h-dvh place-items-center text-sm text-muted-foreground">
        <div className="flex items-center gap-3">
          <span className="size-2 animate-pulse rounded-full bg-primary" />
          {t("common.loading")}
        </div>
      </div>
    );
  }

  const onLogout = async () => {
    try {
      await auth.logout();
    } catch {}
    reset();
    router.replace("/login");
  };

  return (
    <div className="grid min-h-dvh grid-cols-1 lg:grid-cols-[260px_1fr]">
      {/* Sidebar */}
      <aside className="relative hidden border-r border-white/5 bg-sidebar/80 backdrop-blur-xl lg:block">
        {/* ambient glow inside the sidebar */}
        <div
          aria-hidden
          className="pointer-events-none absolute -top-20 left-1/2 h-56 w-72 -translate-x-1/2 rounded-full opacity-60 blur-3xl"
          style={{
            background: "radial-gradient(ellipse at center, hsl(186 95% 55% / 0.35), transparent 60%)",
          }}
        />

        <div className="sticky top-0 flex h-dvh flex-col">
          <div className="px-6 pt-6">
            <Link
              href={me.role === "admin" ? "/admin" : "/dashboard"}
              className="group inline-flex items-center gap-2.5"
            >
              <motion.div
                initial={{ rotate: -10, scale: 0.9, opacity: 0 }}
                animate={{ rotate: 0, scale: 1, opacity: 1 }}
                transition={{ type: "spring", damping: 14, stiffness: 200 }}
                className={cn(
                  "relative grid size-10 place-items-center overflow-hidden rounded-xl text-white shadow-glow",
                  me.role === "admin"
                    ? "bg-[linear-gradient(135deg,hsl(38_95%_60%),hsl(15_90%_60%))]"
                    : "bg-[linear-gradient(135deg,hsl(186_95%_55%),hsl(220_85%_65%))]",
                )}
              >
                <svg viewBox="0 0 24 24" className="size-5" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round">
                  <path d="M3 19 L8 11 L12 15 L16 8 L21 13" />
                  <path d="M3 21 L21 21" opacity="0.6" />
                </svg>
                <span className="absolute inset-0 bg-[radial-gradient(circle_at_30%_20%,white/40,transparent_60%)]" />
              </motion.div>
              <div className="leading-tight">
                <div className="font-display text-base font-semibold tracking-tight text-foreground">
                  {me.role === "admin" ? t("brand.admin") : t("brand.name")}
                </div>
                <div className="text-[10px] uppercase tracking-[0.18em] text-muted-foreground">
                  {me.role === "admin" ? t("brand.tagline.admin") : t("brand.tagline.customer")}
                </div>
              </div>
            </Link>
          </div>

          <nav className="mt-6 flex-1 overflow-y-auto px-3">
            {me.role === "admin" ? (
              <>
                <p className="px-3 pb-2 text-[10px] font-medium uppercase tracking-[0.18em] text-muted-foreground/70">
                  {t("shell.admin")}
                </p>
                <ul className="space-y-0.5">
                  {ADMIN_NAV.filter((n) => n.adminSection === "primary").map((n) => {
                    const active =
                      n.href === "/admin"
                        ? pathname === "/admin"
                        : pathname === n.href || pathname.startsWith(n.href + "/");
                    return (
                      <li key={n.href}>
                        <Link
                          href={n.href}
                          className={cn(
                            "group relative flex items-center gap-3 rounded-lg px-3 py-2 text-sm transition-colors",
                            active ? "text-foreground" : "text-muted-foreground hover:text-foreground",
                          )}
                        >
                          {active && (
                            <motion.span
                              layoutId="nav-active"
                              className="absolute inset-0 -z-0 rounded-lg bg-gradient-to-r from-amber-500/25 via-amber-500/10 to-transparent ring-1 ring-amber-500/30"
                              transition={{ type: "spring", damping: 22, stiffness: 240 }}
                            />
                          )}
                          <n.icon className={cn("relative z-10 size-4 transition-colors", active && "text-amber-400")} />
                          <span className="relative z-10">{t(n.label as never)}</span>
                          {active && (
                            <motion.span
                              layoutId="nav-dot"
                              className="ml-auto size-1.5 rounded-full bg-amber-400 shadow-[0_0_10px_hsl(38_95%_60%/0.8)]"
                            />
                          )}
                        </Link>
                      </li>
                    );
                  })}
                </ul>
                <p className="mt-4 px-3 pb-2 text-[10px] font-medium uppercase tracking-[0.18em] text-muted-foreground/70">
                  {t("shell.logs")}
                </p>
                <ul className="space-y-0.5">
                  {ADMIN_NAV.filter((n) => n.adminSection === "secondary").map((n) => {
                    const active = pathname === n.href || pathname.startsWith(n.href + "/");
                    return (
                      <li key={n.href}>
                        <Link
                          href={n.href}
                          className={cn(
                            "group relative flex items-center gap-3 rounded-lg px-3 py-2 text-sm transition-colors",
                            active ? "text-foreground" : "text-muted-foreground hover:text-foreground",
                          )}
                        >
                          {active && (
                            <motion.span
                              layoutId="nav-active-2"
                              className="absolute inset-0 -z-0 rounded-lg bg-gradient-to-r from-amber-500/25 via-amber-500/10 to-transparent ring-1 ring-amber-500/30"
                              transition={{ type: "spring", damping: 22, stiffness: 240 }}
                            />
                          )}
                          <n.icon className={cn("relative z-10 size-4 transition-colors", active && "text-amber-400")} />
                          <span className="relative z-10">{t(n.label as never)}</span>
                        </Link>
                      </li>
                    );
                  })}
                </ul>
              </>
            ) : (
              <>
                <p className="px-3 pb-2 text-[10px] font-medium uppercase tracking-[0.18em] text-muted-foreground/70">
                  {t("shell.workspace")}
                </p>
                <ul className="space-y-0.5">
                  {CUSTOMER_NAV.map((n) => {
                    const active = pathname === n.href || pathname.startsWith(n.href + "/");
                    return (
                      <li key={n.href}>
                        <Link
                          href={n.href}
                          className={cn(
                            "group relative flex items-center gap-3 rounded-lg px-3 py-2 text-sm transition-colors",
                            active ? "text-foreground" : "text-muted-foreground hover:text-foreground",
                          )}
                        >
                          {active && (
                            <motion.span
                              layoutId="nav-active-customer"
                              className="absolute inset-0 -z-0 rounded-lg bg-gradient-to-r from-primary/20 via-primary/10 to-transparent ring-1 ring-primary/30"
                              transition={{ type: "spring", damping: 22, stiffness: 240 }}
                            />
                          )}
                          <n.icon className={cn("relative z-10 size-4 transition-colors", active && "text-primary")} />
                          <span className="relative z-10">{t(n.label as never)}</span>
                        </Link>
                      </li>
                    );
                  })}
                </ul>
              </>
            )}
          </nav>

          <div className="m-3 rounded-xl border border-white/5 bg-card/40 p-3 backdrop-blur">
            <div className="flex items-center gap-2">
              <div
                className={cn(
                  "grid size-8 place-items-center rounded-full text-sm font-semibold text-foreground",
                  me.role === "admin"
                    ? "bg-gradient-to-br from-amber-500/40 to-rose-500/40"
                    : "bg-gradient-to-br from-primary/40 to-indigo-500/40",
                )}
              >
                {me.display_name.slice(0, 1)}
              </div>
              <div className="min-w-0 leading-tight">
                <div className="truncate text-sm font-medium">{me.display_name}</div>
                <div className="truncate text-[10px] uppercase tracking-[0.18em] text-muted-foreground">
                  {me.role}
                </div>
              </div>
            </div>
            {me.safety_lock_state === "locked" && (
              <div className="mt-2 flex items-start gap-2 rounded-md border border-tier-red/30 bg-tier-red/10 p-2 text-[11px] text-tier-red">
                <AlertTriangle className="mt-0.5 size-3.5" />
                <div>
                  <div className="font-semibold">{t("shell.locked.title")}</div>
                  <div className="opacity-80">{me.safety_lock_reason ?? t("shell.locked.body")}</div>
                </div>
              </div>
            )}
            <Button
              variant="ghost"
              size="sm"
              className="mt-3 w-full justify-start"
              onClick={onLogout}
            >
              <LogOut className="size-3.5" />
              {t("shell.signout")}
            </Button>
          </div>
        </div>
      </aside>

      <main className="relative min-h-dvh">
        {/* desktop topbar — shows the language switcher so the user can
            switch UI + AI reply language at any time. */}
        <div className="sticky top-0 z-20 hidden h-14 items-center justify-end gap-2 border-b border-white/5 bg-background/60 px-6 backdrop-blur-xl lg:flex">
          <LanguageButton />
        </div>
        {/* mobile topbar */}
        <div className="sticky top-0 z-20 flex h-14 items-center justify-between border-b border-white/5 bg-background/60 px-4 backdrop-blur-xl lg:hidden">
          <Link href="/dashboard" className="flex items-center gap-2">
            <div className="grid size-7 place-items-center rounded-md bg-[linear-gradient(135deg,hsl(186_95%_55%),hsl(220_85%_65%))] text-white shadow-glow-soft">
              <span className="text-xs font-bold">F</span>
            </div>
            <span className="font-display text-sm font-semibold">{t("brand.name")}</span>
          </Link>
          <div className="flex items-center gap-2">
            <LanguageButton />
            <Badge variant={me.safety_lock_state === "locked" ? "red" : "muted"}>
              {me.role}
            </Badge>
          </div>
        </div>

        <div className="mx-auto w-full max-w-6xl px-4 py-6 lg:px-8 lg:py-10">
          {children}
        </div>
      </main>
    </div>
  );
}
