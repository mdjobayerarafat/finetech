"use client";

// Shared "skeleton shell" for a typical app page. Used by per-route
// `loading.tsx` segments so the user sees the page chrome (sidebar /
// header / page title placeholder / card placeholders) while the
// data is fetched. Reusing one component keeps the loading states
// consistent across the app.

import { AppShell } from "@/components/layout/app-shell";
import { Skeleton } from "@/components/ui/feedback";
import { cn } from "@/lib/utils";

export function PageSkeleton({
  /** Number of skeleton "card" rows in the body. */
  cards = 2,
  /** Tailwind grid column class, e.g. "md:grid-cols-2 lg:grid-cols-3". */
  gridCols,
  /** Show a row of 4 KPI placeholders above the cards. */
  kpis = false,
  className,
}: {
  cards?: number;
  gridCols?: string;
  kpis?: boolean;
  className?: string;
}) {
  return (
    <AppShell>
      <div className={cn("space-y-6", className)}>
        <div>
          <Skeleton className="h-3 w-20" />
          <Skeleton className="mt-2 h-7 w-1/2" />
          <Skeleton className="mt-2 h-3 w-2/3" />
        </div>

        {kpis && (
          <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
            {Array.from({ length: 4 }).map((_, i) => (
              <div
                key={i}
                className="rounded-2xl border border-white/5 bg-card/60 p-5"
              >
                <Skeleton className="h-3 w-20" />
                <Skeleton className="mt-3 h-7 w-24" />
                <Skeleton className="mt-2 h-3 w-32" />
              </div>
            ))}
          </div>
        )}

        <div
          className={cn(
            "grid gap-4",
            gridCols ?? "md:grid-cols-2 lg:grid-cols-3",
          )}
        >
          {Array.from({ length: cards }).map((_, i) => (
            <div
              key={i}
              className="rounded-2xl border border-white/5 bg-card/60 p-5"
            >
              <Skeleton className="h-3 w-16" />
              <Skeleton className="mt-3 h-5 w-3/4" />
              <Skeleton className="mt-3 h-3 w-full" />
              <Skeleton className="mt-2 h-3 w-5/6" />
              <div className="mt-4 flex gap-2">
                <Skeleton className="h-8 w-20" />
                <Skeleton className="h-8 w-20" />
              </div>
            </div>
          ))}
        </div>
      </div>
    </AppShell>
  );
}
