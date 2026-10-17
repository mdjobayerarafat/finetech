"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { cn } from "@/lib/utils";
import {
  LayoutDashboard,
  Users,
  Bug,
  ShieldCheck,
  Activity,
  ShieldAlert,
} from "lucide-react";
import { motion } from "framer-motion";

export const ADMIN_NAV: Array<{
  href: string;
  label: string;
  icon: React.ComponentType<{ className?: string }>;
  exact?: boolean;
}> = [
  { href: "/admin", label: "Dashboard", icon: LayoutDashboard, exact: true },
  { href: "/admin/users", label: "Users", icon: Users },
  { href: "/admin/redteam", label: "Red-team", icon: Bug },
  { href: "/admin/rules", label: "Agent rules", icon: ShieldCheck },
  { href: "/admin/audit", label: "Audit ledger", icon: Activity },
  { href: "/admin/safety", label: "Safety events", icon: ShieldAlert },
];

export function AdminNav() {
  const pathname = usePathname();
  return (
    <div className="mb-6 flex flex-wrap items-center gap-1 rounded-xl border border-white/5 bg-card/40 p-1.5 backdrop-blur">
      {ADMIN_NAV.map((item) => {
        const active = item.exact
          ? pathname === item.href
          : pathname === item.href || pathname.startsWith(item.href + "/");
        return (
          <Link
            key={item.href}
            href={item.href}
            className={cn(
              "relative inline-flex items-center gap-2 rounded-lg px-3 py-1.5 text-sm transition-colors",
              active
                ? "text-foreground"
                : "text-muted-foreground hover:text-foreground",
            )}
          >
            {active && (
              <motion.span
                layoutId="admin-tab"
                className="absolute inset-0 -z-0 rounded-lg bg-gradient-to-r from-primary/25 via-primary/10 to-transparent ring-1 ring-primary/30"
                transition={{ type: "spring", damping: 22, stiffness: 240 }}
              />
            )}
            <item.icon
              className={cn(
                "relative z-10 size-3.5",
                active && "text-primary",
              )}
            />
            <span className="relative z-10">{item.label}</span>
          </Link>
        );
      })}
    </div>
  );
}
