"use client";

// Root error boundary.
//
// In Next.js App Router, an unhandled throw inside any client component
// (or a server component error) bubbles up to the nearest `error.tsx`
// boundary. If none is defined, the framework falls back to its default
// overlay which renders "missing required error components, refreshing…"
// in dev. We always want a branded fallback instead, so the user lands
// on a clear, recoverable screen rather than a hard refresh loop.
//
// This is the LAST line of defense — every other page is expected to
// handle its own errors gracefully. We deliberately keep the markup
// minimal and free of feature state so a re-render of the error UI
// itself can never throw.

import { useEffect } from "react";
import Link from "next/link";
import { AlertTriangle, ArrowLeft, Home, RotateCw } from "lucide-react";
import { AppShell } from "@/components/layout/app-shell";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { FadeIn } from "@/components/motion/aurora";
import { useT } from "@/i18n/LanguageProvider";

export default function GlobalError({
  error,
  reset,
}: {
  error: Error & { digest?: string };
  reset: () => void;
}) {
  const t = useT();

  // Best-effort log so the issue shows up in the dev console. We don't
  // ship a real telemetry pipe in this app; the digest is enough for
  // the user to copy and send to support.
  useEffect(() => {
    // eslint-disable-next-line no-console
    console.error("App error boundary caught:", error);
  }, [error]);

  const goBack = () => {
    if (typeof window !== "undefined") {
      if (window.history.length > 1) window.history.back();
      else window.location.href = "/dashboard";
    }
  };

  return (
    <AppShell>
      <div className="px-4 py-12 md:px-8">
        <FadeIn>
          <div className="mx-auto max-w-xl">
            <Card>
              <CardHeader>
                <div className="flex items-center gap-2 text-[10px] font-medium uppercase tracking-[0.22em] text-rose-500">
                  <AlertTriangle className="size-3.5" />
                  {t("error.boundary.eyebrow")}
                </div>
                <CardTitle className="mt-1 text-2xl">
                  {t("error.boundary.title")}
                </CardTitle>
                <CardDescription>{t("error.boundary.body")}</CardDescription>
              </CardHeader>
              <CardContent className="space-y-4">
                {error.digest && (
                  <div className="rounded-lg border bg-muted/40 p-3 text-xs">
                    <div className="font-medium uppercase tracking-wider text-muted-foreground">
                      {t("error.boundary.reference")}
                    </div>
                    <code className="mt-1 block break-all font-mono text-[11px] text-foreground/80">
                      {error.digest}
                    </code>
                  </div>
                )}
                <div className="flex flex-wrap gap-2">
                  <Button onClick={reset} className="gap-1.5">
                    <RotateCw className="size-4" />
                    {t("error.boundary.retry")}
                  </Button>
                  <Button variant="outline" onClick={goBack} className="gap-1.5">
                    <ArrowLeft className="size-4" />
                    {t("error.boundary.back")}
                  </Button>
                  <Link href="/dashboard">
                    <Button variant="ghost" className="gap-1.5">
                      <Home className="size-4" />
                      {t("error.boundary.dashboard")}
                    </Button>
                  </Link>
                </div>
                <Badge variant="muted" className="text-[10px]">
                  {t("error.boundary.help")}
                </Badge>
              </CardContent>
            </Card>
          </div>
        </FadeIn>
      </div>
    </AppShell>
  );
}
