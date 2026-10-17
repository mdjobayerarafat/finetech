"use client";

import { useEffect } from "react";
import { useRouter } from "next/navigation";

/**
 * Old admin overview URL — the dashboard now lives at `/admin`. We
 * redirect so any bookmark or chat link keeps working.
 */
export default function AdminOverviewRedirect() {
  const router = useRouter();
  useEffect(() => {
    router.replace("/admin");
  }, [router]);
  return (
    <div className="grid min-h-[40dvh] place-items-center text-sm text-muted-foreground">
      Redirecting…
    </div>
  );
}
