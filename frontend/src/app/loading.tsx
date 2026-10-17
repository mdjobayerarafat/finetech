"use client";

// Root loading UI.
//
// Shown while a server component is being rendered for a navigable
// route. We keep this very lightweight on purpose: it's the first
// thing the user sees on a slow network and we want it to feel
// immediate. A few skeleton bars + a brand-tinted copy block is
// enough — no spinning loaders, no progress percentages.

import { motion } from "framer-motion";
import { Loader2 } from "lucide-react";
import { AppShell } from "@/components/layout/app-shell";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import { FadeIn } from "@/components/motion/aurora";
import { useT } from "@/i18n/LanguageProvider";

export default function GlobalLoading() {
  const t = useT();
  return (
    <AppShell>
      <div className="px-4 py-12 md:px-8">
        <FadeIn>
          <div className="mx-auto max-w-xl space-y-3">
            <div className="flex items-center gap-2 text-[10px] font-medium uppercase tracking-[0.22em] text-primary">
              <Loader2 className="size-3.5 animate-spin" />
              {t("loading.eyebrow")}
            </div>
            <Card>
              <CardHeader className="space-y-3">
                <motion.div
                  className="h-7 w-1/2 rounded-md bg-muted/60"
                  animate={{ opacity: [0.4, 0.8, 0.4] }}
                  transition={{ duration: 1.6, repeat: Infinity }}
                />
                <motion.div
                  className="h-3 w-2/3 rounded-md bg-muted/40"
                  animate={{ opacity: [0.4, 0.8, 0.4] }}
                  transition={{ duration: 1.6, repeat: Infinity, delay: 0.2 }}
                />
              </CardHeader>
              <CardContent className="space-y-2">
                <motion.div
                  className="h-3 w-full rounded-md bg-muted/40"
                  animate={{ opacity: [0.4, 0.8, 0.4] }}
                  transition={{ duration: 1.6, repeat: Infinity, delay: 0.3 }}
                />
                <motion.div
                  className="h-3 w-5/6 rounded-md bg-muted/40"
                  animate={{ opacity: [0.4, 0.8, 0.4] }}
                  transition={{ duration: 1.6, repeat: Infinity, delay: 0.4 }}
                />
                <motion.div
                  className="h-3 w-3/4 rounded-md bg-muted/40"
                  animate={{ opacity: [0.4, 0.8, 0.4] }}
                  transition={{ duration: 1.6, repeat: Infinity, delay: 0.5 }}
                />
              </CardContent>
            </Card>
            <p className="text-center text-xs text-muted-foreground">
              {t("loading.title")}
            </p>
            <p className="text-center text-[11px] text-muted-foreground/70">
              {t("loading.body")}
            </p>
          </div>
        </FadeIn>
      </div>
    </AppShell>
  );
}
