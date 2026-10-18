"use client";

// Dismissible "first-run" banner shown on the dashboard. Helps a new
// user understand what to do first: open the assistant, scan a
// receipt, or check a forecast. The dismissal state is kept in
// localStorage so it stays out of the way on subsequent visits.

import Link from "next/link";
import { useEffect, useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import {
  MessageSquare,
  X,
  Sparkles,
  Calculator,
  Camera,
  ArrowRight,
} from "lucide-react";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { useT } from "@/i18n/LanguageProvider";
import { cn } from "@/lib/utils";

const STORAGE_KEY = "ftp.dashboard.quickstart.dismissed.v1";

const STEPS: {
  icon: typeof MessageSquare;
  titleKey: "quickstart.step1.title" | "quickstart.step2.title" | "quickstart.step3.title";
  bodyKey: "quickstart.step1.body" | "quickstart.step2.body" | "quickstart.step3.body";
  href: string;
  ctaKey: "quickstart.step1.cta" | "quickstart.step2.cta" | "quickstart.step3.cta";
}[] = [
  {
    icon: MessageSquare,
    titleKey: "quickstart.step1.title",
    bodyKey: "quickstart.step1.body",
    href: "/chat",
    ctaKey: "quickstart.step1.cta",
  },
  {
    icon: Calculator,
    titleKey: "quickstart.step2.title",
    bodyKey: "quickstart.step2.body",
    href: "/forecast",
    ctaKey: "quickstart.step2.cta",
  },
  {
    icon: Camera,
    titleKey: "quickstart.step3.title",
    bodyKey: "quickstart.step3.body",
    href: "/receipt",
    ctaKey: "quickstart.step3.cta",
  },
];

export function QuickstartBanner() {
  const t = useT();
  const [open, setOpen] = useState(false);

  useEffect(() => {
    if (typeof window === "undefined") return;
    try {
      if (window.localStorage.getItem(STORAGE_KEY) !== "1") {
        setOpen(true);
      }
    } catch {
      setOpen(true);
    }
  }, []);

  const dismiss = () => {
    setOpen(false);
    try {
      window.localStorage.setItem(STORAGE_KEY, "1");
    } catch {
      /* private mode / quota — fine to ignore */
    }
  };

  return (
    <AnimatePresence>
      {open && (
        <motion.div
          initial={{ opacity: 0, y: -8 }}
          animate={{ opacity: 1, y: 0 }}
          exit={{ opacity: 0, y: -8 }}
          transition={{ duration: 0.3, ease: [0.16, 1, 0.3, 1] }}
        >
          <Card className="relative overflow-hidden ring-1 ring-primary/30">
            <div
              aria-hidden
              className="pointer-events-none absolute -right-12 -top-12 size-40 rounded-full opacity-50 blur-3xl"
              style={{
                background:
                  "radial-gradient(circle, hsl(186 95% 55% / 0.45), transparent 70%)",
              }}
            />
            <button
              type="button"
              onClick={dismiss}
              aria-label={t("common.close")}
              className="absolute right-2 top-2 z-10 grid size-7 place-items-center rounded-md text-muted-foreground transition-colors hover:bg-white/5 hover:text-foreground"
            >
              <X className="size-3.5" />
            </button>
            <CardContent className="p-5">
              <div className="flex items-center gap-2 text-[11px] font-medium uppercase tracking-[0.18em] text-primary">
                <Sparkles className="size-3" />
                {t("quickstart.eyebrow")}
              </div>
              <h2 className="mt-1 font-display text-lg font-semibold tracking-tight sm:text-xl">
                {t("quickstart.title")}
              </h2>
              <p className="mt-1 text-sm text-muted-foreground">
                {t("quickstart.body")}
              </p>
              <div className="mt-4 grid gap-3 sm:grid-cols-3">
                {STEPS.map((s, i) => (
                  <Link
                    key={s.href}
                    href={s.href}
                    onClick={dismiss}
                    className={cn(
                      "group flex h-full flex-col gap-2 rounded-xl border border-white/5 bg-white/[0.03] p-3 text-left transition-colors",
                      "hover:border-primary/40 hover:bg-white/[0.06]",
                    )}
                  >
                    <div className="grid size-8 place-items-center rounded-lg bg-primary/15 text-primary ring-1 ring-primary/30">
                      <s.icon className="size-3.5" />
                    </div>
                    <p className="text-[10px] font-medium uppercase tracking-wider text-muted-foreground">
                      {t("quickstart.stepLabel", { n: i + 1 })}
                    </p>
                    <p className="text-sm font-medium text-foreground">
                      {t(s.titleKey)}
                    </p>
                    <p className="text-xs text-muted-foreground">
                      {t(s.bodyKey)}
                    </p>
                    <span className="mt-auto inline-flex items-center gap-1 text-xs font-medium text-primary">
                      {t(s.ctaKey)}
                      <ArrowRight className="size-3 transition-transform group-hover:translate-x-0.5" />
                    </span>
                  </Link>
                ))}
              </div>
            </CardContent>
          </Card>
        </motion.div>
      )}
    </AnimatePresence>
  );
}
