"use client";

// Docs segment error boundary.
//
// Scoped to the `/docs` route. If a client component under the docs
// tree throws (e.g. an effect that runs before its DOM is mounted, a
// bad import, a clipboard API that throws on a hostile embed), this
// catches the throw and shows a recovery card IN PLACE rather than
// tearing down the whole app shell.
//
// Note: Next.js requires the segment `error.tsx` to be a client
// component, and to NOT import the AppShell itself at this level —
// AppShell lives in the layout above, and re-importing it would loop
// the boundary.

import { useEffect } from "react";
import Link from "next/link";
import { AlertTriangle, ArrowLeft, BookOpen, Home, RotateCw } from "lucide-react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { FadeIn } from "@/components/motion/aurora";
import { useT } from "@/i18n/LanguageProvider";

export default function DocsError({
  error,
  reset,
}: {
  error: Error & { digest?: string };
  reset: () => void;
}) {
  const t = useT();

  useEffect(() => {
    // eslint-disable-next-line no-console
    console.error("Docs error boundary caught:", error);
  }, [error]);

  const goBack = () => {
    if (typeof window !== "undefined") {
      if (window.history.length > 1) window.history.back();
      else window.location.href = "/dashboard";
    }
  };

  return (
    <div className="px-4 py-12 md:px-8">
      <FadeIn>
        <div className="mx-auto max-w-xl">
          <Card>
            <CardHeader>
              <div className="flex items-center gap-2 text-[10px] font-medium uppercase tracking-[0.22em] text-rose-500">
                <AlertTriangle className="size-3.5" />
                {t("error.boundary.eyebrowDocs")}
              </div>
              <CardTitle className="mt-1 text-2xl">
                {t("error.boundary.titleDocs")}
              </CardTitle>
              <CardDescription>{t("error.boundary.bodyDocs")}</CardDescription>
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
                <Link href="/docs">
                  <Button variant="ghost" className="gap-1.5">
                    <BookOpen className="size-4" />
                    {t("error.boundary.docs")}
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
  );
}
