"use client";

// Root 404 page.
//
// Rendered when a user hits a route that doesn't exist or that no
// segment could match. Distinct from `error.tsx` (which is for
// unhandled throws) and from the framework's default not-found
// screen. We give the user a clear way back so the dead end doesn't
// feel like a dead end.

import Link from "next/link";
import { Compass, Home, Map } from "lucide-react";
import { AppShell } from "@/components/layout/app-shell";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { FadeIn } from "@/components/motion/aurora";
import { useT } from "@/i18n/LanguageProvider";

export default function GlobalNotFound() {
  const t = useT();
  return (
    <AppShell>
      <div className="px-4 py-12 md:px-8">
        <FadeIn>
          <div className="mx-auto max-w-xl">
            <Card>
              <CardHeader>
                <div className="flex items-center gap-2 text-[10px] font-medium uppercase tracking-[0.22em] text-muted-foreground">
                  <Compass className="size-3.5" />
                  {t("notfound.eyebrow")}
                </div>
                <CardTitle className="mt-1 text-2xl">
                  {t("notfound.title")}
                </CardTitle>
                <CardDescription>{t("notfound.body")}</CardDescription>
              </CardHeader>
              <CardContent className="space-y-4">
                <div className="flex flex-wrap gap-2">
                  <Link href="/dashboard">
                    <Button className="gap-1.5">
                      <Home className="size-4" />
                      {t("notfound.dashboard")}
                    </Button>
                  </Link>
                  <Link href="/docs">
                    <Button variant="outline" className="gap-1.5">
                      <Map className="size-4" />
                      {t("notfound.docs")}
                    </Button>
                  </Link>
                </div>
                <Badge variant="muted" className="text-[10px]">
                  404 · not-found
                </Badge>
              </CardContent>
            </Card>
          </div>
        </FadeIn>
      </div>
    </AppShell>
  );
}
