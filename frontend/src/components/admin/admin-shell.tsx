"use client";

import { useEffect } from "react";
import { useRouter } from "next/navigation";
import { AppShell } from "@/components/layout/app-shell";
import { AdminNav } from "@/components/admin/admin-nav";
import { useAuth } from "@/lib/store";

/**
 * Wraps any `/admin/*` page with:
 *   1. The standard app shell (sidebar, top bar, sign-out).
 *   2. The admin sub-tab nav (Dashboard / Users / Red-team / Rules / Audit / Safety).
 *   3. A role guard: if the logged-in user is not an admin, the user is
 *      silently redirected to `/dashboard` — we do *not* render any of the
 *      admin chrome first (prevents flicker / flash-of-content).
 */
export function AdminShell({ children }: { children: React.ReactNode }) {
  const { me, loading } = useAuth();
  const router = useRouter();

  useEffect(() => {
    if (loading) return;
    if (!me) {
      router.replace("/login");
    } else if (me.role !== "admin") {
      router.replace("/dashboard");
    }
  }, [me, loading, router]);

  if (loading || !me || me.role !== "admin") {
    return (
      <AppShell>
        <div className="grid min-h-[40dvh] place-items-center text-sm text-muted-foreground">
          <div className="flex items-center gap-3">
            <span className="size-2 animate-pulse rounded-full bg-primary" />
            Verifying admin access…
          </div>
        </div>
      </AppShell>
    );
  }

  return (
    <AppShell>
      <AdminNav />
      {children}
    </AppShell>
  );
}
