"use client";

// Shared "user-facing state" primitives.
//
// Everything a normal user sees on an empty / loading / error page goes
// through these components so we don't have a different message tone
// (or a different icon) on every page. They also make screen readers
// happy: each has a `role` and a sensible `aria-*` set.

import { motion } from "framer-motion";
import {
  Inbox,
  AlertTriangle,
  Loader2,
  type LucideIcon,
} from "lucide-react";
import { cn } from "@/lib/utils";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import type { ReactNode } from "react";

export function PageHeader({
  eyebrow,
  title,
  description,
  actions,
  className,
}: {
  /** Tiny uppercase label rendered above the title. Optional. */
  eyebrow?: string;
  title: string;
  description?: string;
  /** Right-aligned slot for a primary action button. */
  actions?: ReactNode;
  className?: string;
}) {
  return (
    <div
      className={cn(
        "flex flex-col gap-3 sm:flex-row sm:items-end sm:justify-between",
        className,
      )}
    >
      <div className="min-w-0">
        {eyebrow && (
          <p className="text-[11px] font-medium uppercase tracking-[0.18em] text-muted-foreground">
            {eyebrow}
          </p>
        )}
        <h1 className="mt-1 font-display text-2xl font-semibold tracking-tight sm:text-3xl">
          {title}
        </h1>
        {description && (
          <p className="mt-1.5 max-w-2xl text-sm text-muted-foreground">
            {description}
          </p>
        )}
      </div>
      {actions && (
        <div className="flex flex-wrap items-center gap-2 sm:justify-end">
          {actions}
        </div>
      )}
    </div>
  );
}

export function EmptyState({
  icon: Icon = Inbox,
  title,
  body,
  action,
  className,
}: {
  icon?: LucideIcon;
  title: string;
  body?: string;
  action?: ReactNode;
  className?: string;
}) {
  return (
    <div
      role="status"
      className={cn(
        "flex flex-col items-center justify-center gap-3 rounded-2xl border border-dashed border-white/10 bg-card/30 px-6 py-12 text-center",
        className,
      )}
    >
      <div className="grid size-12 place-items-center rounded-xl bg-primary/10 text-primary ring-1 ring-primary/30">
        <Icon className="size-5" aria-hidden />
      </div>
      <div>
        <p className="text-sm font-semibold text-foreground">{title}</p>
        {body && (
          <p className="mt-1 max-w-sm text-xs text-muted-foreground">{body}</p>
        )}
      </div>
      {action && <div className="mt-1">{action}</div>}
    </div>
  );
}

export function LoadingState({
  label = "Loading…",
  body,
  className,
}: {
  label?: string;
  body?: string;
  className?: string;
}) {
  return (
    <div
      role="status"
      aria-live="polite"
      className={cn(
        "flex flex-col items-center justify-center gap-2 rounded-2xl border border-white/5 bg-card/30 px-6 py-10 text-center",
        className,
      )}
    >
      <Loader2 className="size-5 animate-spin text-primary" aria-hidden />
      <p className="text-sm font-medium text-foreground">{label}</p>
      {body && <p className="text-xs text-muted-foreground">{body}</p>}
    </div>
  );
}

export function ErrorState({
  title = "Something went wrong",
  body,
  onRetry,
  retryLabel = "Try again",
  className,
}: {
  title?: string;
  body?: string;
  onRetry?: () => void;
  retryLabel?: string;
  className?: string;
}) {
  return (
    <div
      role="alert"
      className={cn(
        "flex flex-col items-center justify-center gap-3 rounded-2xl border border-tier-red/30 bg-tier-red/5 px-6 py-10 text-center",
        className,
      )}
    >
      <div className="grid size-12 place-items-center rounded-xl bg-tier-red/10 text-tier-red ring-1 ring-tier-red/30">
        <AlertTriangle className="size-5" aria-hidden />
      </div>
      <div>
        <p className="text-sm font-semibold text-foreground">{title}</p>
        {body && (
          <p className="mt-1 max-w-sm text-xs text-muted-foreground">{body}</p>
        )}
      </div>
      {onRetry && (
        <Button size="sm" variant="outline" onClick={onRetry}>
          {retryLabel}
        </Button>
      )}
    </div>
  );
}

export function StatCard({
  label,
  value,
  hint,
  icon: Icon,
  tone = "neutral",
  loading,
}: {
  label: string;
  value: ReactNode;
  hint?: string;
  icon?: LucideIcon;
  tone?: "neutral" | "up" | "down" | "warning";
  loading?: boolean;
}) {
  const toneRing = {
    neutral: "ring-white/10",
    up: "ring-tier-green/30",
    down: "ring-tier-yellow/30",
    warning: "ring-tier-red/30",
  }[tone];
  const toneIcon = {
    neutral: "text-primary",
    up: "text-tier-green",
    down: "text-tier-yellow",
    warning: "text-tier-red",
  }[tone];

  return (
    <Card className={cn("ring-1 ring-inset", toneRing)}>
      <CardContent className="flex items-start gap-3 p-5">
        {Icon && (
          <div className={cn("grid size-9 shrink-0 place-items-center rounded-lg bg-white/[0.04] ring-1 ring-white/10", toneIcon)}>
            <Icon className="size-4" aria-hidden />
          </div>
        )}
        <div className="min-w-0 flex-1">
          <p className="text-[10px] font-medium uppercase tracking-wider text-muted-foreground">
            {label}
          </p>
          {loading ? (
            <div className="mt-1.5 h-7 w-24 animate-pulse rounded-md bg-white/10" />
          ) : (
            <div className="mt-1 font-display text-xl font-semibold tracking-tight">
              {value}
            </div>
          )}
          {hint && (
            <p className="mt-1 text-[11px] text-muted-foreground">{hint}</p>
          )}
        </div>
      </CardContent>
    </Card>
  );
}

export function KpiGrid({
  children,
  className,
}: {
  children: ReactNode;
  className?: string;
}) {
  return (
    <div
      className={cn(
        "grid gap-3 sm:grid-cols-2 lg:grid-cols-4",
        className,
      )}
    >
      {children}
    </div>
  );
}

export function SectionCard({
  title,
  description,
  actions,
  children,
  className,
  bodyClassName,
}: {
  title?: ReactNode;
  description?: ReactNode;
  actions?: ReactNode;
  children: ReactNode;
  className?: string;
  bodyClassName?: string;
}) {
  return (
    <Card className={className}>
      {(title || description || actions) && (
        <CardHeader>
          <div className="flex items-start justify-between gap-3">
            <div className="min-w-0">
              {title && (
                <CardTitle className="text-base sm:text-lg">
                  {title}
                </CardTitle>
              )}
              {description && (
                <CardDescription className="mt-0.5">{description}</CardDescription>
              )}
            </div>
            {actions && <div className="flex shrink-0 gap-2">{actions}</div>}
          </div>
        </CardHeader>
      )}
      <CardContent className={cn(bodyClassName)}>{children}</CardContent>
    </Card>
  );
}

export function Toolbar({
  children,
  className,
}: {
  children: ReactNode;
  className?: string;
}) {
  return (
    <div
      className={cn(
        "flex flex-wrap items-center justify-between gap-2 rounded-xl border border-white/5 bg-card/40 p-2 backdrop-blur",
        className,
      )}
    >
      {children}
    </div>
  );
}

// Skeleton row used by per-route loading.tsx segments. Animates a soft
// pulse so the user feels the page is being prepared, not stuck.
export function Skeleton({
  className,
  delay = 0,
}: {
  className?: string;
  delay?: number;
}) {
  return (
    <motion.div
      aria-hidden
      className={cn("rounded-md bg-white/8", className)}
      style={{
        background:
          "linear-gradient(90deg, hsl(200 30% 22% / 0.6) 0%, hsl(200 30% 30% / 0.8) 50%, hsl(200 30% 22% / 0.6) 100%)",
        backgroundSize: "200% 100%",
      }}
      animate={{ backgroundPosition: ["0% 0%", "100% 0%"] }}
      transition={{
        duration: 1.4,
        repeat: Infinity,
        ease: "linear",
        delay,
      }}
    />
  );
}
