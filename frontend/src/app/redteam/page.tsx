"use client";

import { useEffect } from "react";
import { useRouter } from "next/navigation";
import { useAuth } from "@/lib/store";

/**
 * The red-team panel moved to `/admin/redteam` (admin-only). Anyone hitting
 * the old URL is bounced — admins go to the new home, customers go to
 * their dashboard.
 */
export default function RedTeamRedirect() {
  const router = useRouter();
  const { me, loading } = useAuth();
  useEffect(() => {
    if (loading) return;
    if (!me) {
      router.replace("/login");
    } else if (me.role === "admin") {
      router.replace("/admin/redteam");
    } else {
      router.replace("/dashboard");
    }
  }, [me, loading, router]);
  return (
    <div className="grid min-h-[40dvh] place-items-center text-sm text-muted-foreground">
      Redirecting…
    </div>
  );
}
