"use client";

import { useEffect, useMemo, useState } from "react";
import { motion } from "framer-motion";
import {
  ArrowRight,
  BookOpen,
  Copy,
  MessageSquare,
  Search,
  Sparkles,
} from "lucide-react";

import { AppShell } from "@/components/layout/app-shell";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { FadeIn } from "@/components/motion/aurora";
import { useT } from "@/i18n/LanguageProvider";
import { cn } from "@/lib/utils";
import { useToast } from "@/components/ui/toast";
import Link from "next/link";

// One section = one anchor in the docs page. Sections are intentionally
// listed in display order; the same order is used for both the sidebar
// TOC and the on-page render.
type Section = {
  id: string;
  // `i18nKey` points to a `docs.<id>.eyebrow` and `docs.<id>.title`
  // pair in the dictionaries so the sidebar and the section heading
  // stay in sync.
  i18nKey: string;
  // `bodyKey` is the top-level paragraph under the title.
  bodyKey: string;
  // A list of feature bullets or sub-headings. We keep this typed and
  // small instead of a free-form node list — every section is shaped
  // identically so the page reads like real product docs (Stripe /
  // Linear style), not a CMS dump.
  bullets: Array<{ key: string } | { subKey: string; bodyKey: string; bullets: Array<{ key: string }> }>;
};

const SECTIONS: Section[] = [
  {
    id: "getting-started",
    i18nKey: "gs",
    bodyKey: "docs.gs.body",
    bullets: [
      { key: "docs.gs.step1.title" },
      { key: "docs.gs.step1.body" },
      { key: "docs.gs.step2.title" },
      { key: "docs.gs.step2.body" },
      { key: "docs.gs.step3.title" },
      { key: "docs.gs.step3.body" },
      { key: "docs.gs.step4.title" },
      { key: "docs.gs.step4.body" },
    ],
  },
  {
    id: "accounts",
    i18nKey: "acct",
    bodyKey: "docs.acct.body",
    bullets: [
      { key: "docs.acct.feature1" },
      { key: "docs.acct.feature2" },
      { key: "docs.acct.feature3" },
      { key: "docs.acct.feature4" },
    ],
  },
  {
    id: "beneficial-accounts",
    i18nKey: "ba",
    bodyKey: "docs.ba.body",
    bullets: [
      { key: "docs.ba.f1" },
      { key: "docs.ba.f2" },
      { key: "docs.ba.f3" },
      { key: "docs.ba.f4" },
    ],
  },
  {
    id: "transfers",
    i18nKey: "xfer",
    bodyKey: "docs.xfer.body",
    bullets: [
      {
        subKey: "docs.xfer.tiers.title",
        bodyKey: "docs.xfer.tiers.body",
        bullets: [],
      },
      {
        subKey: "docs.xfer.schedule.title",
        bodyKey: "docs.xfer.schedule.body",
        bullets: [],
      },
      {
        subKey: "docs.xfer.splits.title",
        bodyKey: "docs.xfer.splits.body",
        bullets: [],
      },
    ],
  },
  {
    id: "budgets",
    i18nKey: "bg",
    bodyKey: "docs.bg.body",
    bullets: [
      { key: "docs.bg.f1" },
      { key: "docs.bg.f2" },
      { key: "docs.bg.f3" },
      { key: "docs.bg.f4" },
    ],
  },
  {
    id: "subscriptions",
    i18nKey: "sub",
    bodyKey: "docs.sub.body",
    bullets: [],
  },
  {
    id: "cards",
    i18nKey: "card",
    bodyKey: "docs.card.body",
    bullets: [],
  },
  {
    id: "wealth",
    i18nKey: "wl",
    bodyKey: "docs.wl.body",
    bullets: [],
  },
  {
    id: "forecast",
    i18nKey: "fc",
    bodyKey: "docs.fc.body",
    bullets: [],
  },
  {
    id: "receipt",
    i18nKey: "rc",
    bodyKey: "docs.rc.body",
    bullets: [],
  },
  {
    id: "voice",
    i18nKey: "vc",
    bodyKey: "docs.vc.body",
    bullets: [],
  },
  {
    id: "assistant",
    i18nKey: "as",
    bodyKey: "docs.as.body",
    bullets: [],
  },
  {
    id: "admin",
    i18nKey: "ad",
    bodyKey: "docs.ad.body",
    bullets: [],
  },
  {
    id: "safety",
    i18nKey: "sf",
    bodyKey: "docs.sf.body",
    bullets: [],
  },
  {
    id: "faq",
    i18nKey: "faq",
    bodyKey: "docs.faq.title",
    bullets: [
      { key: "docs.faq.q1" },
      { key: "docs.faq.a1" },
      { key: "docs.faq.q2" },
      { key: "docs.faq.a2" },
      { key: "docs.faq.q3" },
      { key: "docs.faq.a3" },
      { key: "docs.faq.q4" },
      { key: "docs.faq.a4" },
      { key: "docs.faq.q5" },
      { key: "docs.faq.a5" },
      { key: "docs.faq.q6" },
      { key: "docs.faq.a6" },
    ],
  },
  {
    id: "glossary",
    i18nKey: "gloss",
    bodyKey: "docs.gloss.title",
    bullets: [
      { key: "docs.gloss.fen" },
      { key: "docs.gloss.tier" },
      { key: "docs.gloss.reserved" },
      { key: "docs.gloss.bus" },
      { key: "docs.gloss.audit" },
      { key: "docs.gloss.safety" },
      { key: "docs.gloss.workflow" },
      { key: "docs.gloss.pending" },
    ],
  },
];

// Last-updated stamp. We hard-code the build date here so the page
// reads as a real manual; the actual data behind it is just the
// version of the manual content in this commit.
const LAST_UPDATED = "2025-01";

export default function DocsPage() {
  return (
    <AppShell>
      <Body />
    </AppShell>
  );
}

function Body() {
  const t = useT();
  const { toast } = useToast();
  const [filter, setFilter] = useState("");
  const [activeId, setActiveId] = useState<string | null>(null);

  // Filter the visible sections by a case-insensitive substring match
  // against the resolved title + body + bullets, so a user typing
  // "mfa" surfaces both the Safety section and any FAQ row mentioning
  // it. Empty filter = show everything.
  const filtered = useMemo(() => {
    const q = filter.trim().toLowerCase();
    if (!q) return SECTIONS;
    return SECTIONS.filter((s) => {
      const eyebrow = t(`docs.${s.i18nKey}.eyebrow` as never).toLowerCase();
      const title = t(`docs.${s.i18nKey}.title` as never).toLowerCase();
      const body = t(s.bodyKey as never).toLowerCase();
      const bulletBlob = s.bullets
        .map((b) => {
          if ("key" in b) return t(b.key as never).toLowerCase();
          return `${t(b.subKey as never).toLowerCase()} ${t(b.bodyKey as never).toLowerCase()}`;
        })
        .join(" ");
      return (
        eyebrow.includes(q) ||
        title.includes(q) ||
        body.includes(q) ||
        bulletBlob.includes(q)
      );
    });
  }, [filter, t]);

  // Set up an intersection observer so the sidebar entry for the
  // currently-visible section gets a visual highlight as the user
  // scrolls. We don't auto-scroll; the observer only updates state.
  useEffect(() => {
    const headings = SECTIONS.map((s) => document.getElementById(s.id)).filter(
      (n): n is HTMLElement => n !== null,
    );
    if (headings.length === 0) return;
    const io = new IntersectionObserver(
      (entries) => {
        // Pick the entry closest to the top that's still on screen.
        const visible = entries
          .filter((e) => e.isIntersecting)
          .sort((a, b) => a.boundingClientRect.top - b.boundingClientRect.top);
        if (visible[0]) setActiveId(visible[0].target.id);
      },
      { rootMargin: "-30% 0px -55% 0px", threshold: 0 },
    );
    for (const h of headings) io.observe(h);
    return () => io.disconnect();
  }, []);

  // Click a sidebar entry: smooth-scroll the heading into view and
  // replace the URL hash so the section is deep-linkable.
  const goTo = (id: string) => {
    const el = document.getElementById(id);
    if (!el) return;
    el.scrollIntoView({ behavior: "smooth", block: "start" });
    history.replaceState(null, "", `#${id}`);
    setActiveId(id);
  };

  // Copy-link: write the absolute URL for a section to the clipboard
  // and surface a confirmation toast. Used in real docs so a reader
  // can share a single answer.
  const copyLink = async (id: string) => {
    const url = `${window.location.origin}/docs#${id}`;
    try {
      await navigator.clipboard.writeText(url);
      toast({ variant: "success", title: t("docs.copied") });
    } catch {
      // Fall back to a hidden textarea + execCommand for older browsers
      // — clipboard API can be blocked in some embeds.
      const ta = document.createElement("textarea");
      ta.value = url;
      document.body.appendChild(ta);
      ta.select();
      try {
        document.execCommand("copy");
        toast({ variant: "success", title: t("docs.copied") });
      } catch {
        toast({ variant: "error", title: url });
      } finally {
        document.body.removeChild(ta);
      }
    }
  };

  return (
    <div className="space-y-8 px-4 pb-16 pt-4 md:px-8">
      {/* --- HERO ----------------------------------------------------- */}
      <FadeIn>
        <section className="relative overflow-hidden rounded-3xl border border-white/5 bg-card/40 p-8 backdrop-blur-md md:p-12">
          <div className="pointer-events-none absolute -right-24 -top-24 size-72 rounded-full bg-primary/15 blur-3xl" />
          <div className="pointer-events-none absolute -bottom-24 -left-24 size-72 rounded-full bg-indigo-500/10 blur-3xl" />
          <div className="relative z-10 max-w-3xl space-y-4">
            <motion.p
              initial={{ opacity: 0, y: 6 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: 0.05 }}
              className="text-[11px] font-medium uppercase tracking-[0.22em] text-primary"
            >
              <BookOpen className="mr-1.5 inline-block size-3.5" />
              {t("docs.eyebrow")}
            </motion.p>
            <h1 className="font-display text-3xl font-semibold leading-tight tracking-tight md:text-4xl">
              {t("docs.title")}
            </h1>
            <p className="max-w-2xl text-sm text-muted-foreground md:text-base">
              {t("docs.subtitle")}
            </p>
            <div className="flex flex-wrap items-center gap-3 pt-2 text-xs text-muted-foreground">
              <Badge variant="muted">
                {t("docs.lastUpdated", { date: LAST_UPDATED })}
              </Badge>
              <Badge variant="green">v1.0</Badge>
            </div>
          </div>
        </section>
      </FadeIn>

      {/* --- BODY: TOC + content -------------------------------------- */}
      <div className="grid gap-8 lg:grid-cols-[260px_1fr]">
        {/* Sidebar — sticky on large screens. */}
        <aside className="hidden lg:block">
          <div className="sticky top-20 space-y-4">
            <div>
              <Label className="mb-2 block text-[10px] font-medium uppercase tracking-[0.18em] text-muted-foreground">
                {t("docs.searchPh")}
              </Label>
              <div className="relative">
                <Search className="pointer-events-none absolute left-2.5 top-2.5 size-3.5 text-muted-foreground" />
                <Input
                  value={filter}
                  onChange={(e) => setFilter(e.target.value)}
                  placeholder={t("docs.searchPh")}
                  className="pl-8"
                />
              </div>
            </div>
            <div>
              <p className="mb-2 text-[10px] font-medium uppercase tracking-[0.18em] text-muted-foreground">
                {t("docs.tocTitle")}
              </p>
              <ul className="space-y-0.5">
                {filtered.map((s) => {
                  const title = t(`docs.${s.i18nKey}.title` as never);
                  const eyebrow = t(`docs.${s.i18nKey}.eyebrow` as never);
                  const active = activeId === s.id;
                  return (
                    <li key={s.id}>
                      <button
                        type="button"
                        onClick={() => goTo(s.id)}
                        className={cn(
                          "group block w-full rounded-lg px-3 py-2 text-left text-sm transition-colors",
                          active
                            ? "bg-primary/10 text-foreground ring-1 ring-primary/30"
                            : "text-muted-foreground hover:bg-accent/50 hover:text-foreground",
                        )}
                      >
                        <div className="text-[10px] uppercase tracking-[0.18em] text-muted-foreground/80">
                          {eyebrow}
                        </div>
                        <div className="truncate font-medium">{title}</div>
                      </button>
                    </li>
                  );
                })}
                {filtered.length === 0 && (
                  <li className="px-3 py-2 text-xs text-muted-foreground">
                    {t("docs.empty")}
                  </li>
                )}
              </ul>
            </div>
          </div>
        </aside>

        {/* Main content */}
        <main className="min-w-0 space-y-8">
          {/* Mobile-only filter, mirrors the sidebar one so the page
              is usable without the desktop rail. */}
          <div className="lg:hidden">
            <div className="relative">
              <Search className="pointer-events-none absolute left-2.5 top-2.5 size-3.5 text-muted-foreground" />
              <Input
                value={filter}
                onChange={(e) => setFilter(e.target.value)}
                placeholder={t("docs.searchPh")}
                className="pl-8"
              />
            </div>
          </div>

          {filtered.map((s, idx) => {
            const eyebrow = t(`docs.${s.i18nKey}.eyebrow` as never);
            const title = t(`docs.${s.i18nKey}.title` as never);
            const body = t(s.bodyKey as never);
            return (
              <FadeIn key={s.id} delay={0.04 * Math.min(idx, 4)}>
                <section
                  id={s.id}
                  className="scroll-mt-20"
                  aria-labelledby={`${s.id}-h`}
                >
                  <Card>
                    <CardHeader>
                      <div className="flex items-start justify-between gap-3">
                        <div className="min-w-0">
                          <div className="text-[10px] font-medium uppercase tracking-[0.22em] text-primary">
                            {eyebrow}
                          </div>
                          <CardTitle
                            id={`${s.id}-h`}
                            className="mt-1 text-xl"
                          >
                            {title}
                          </CardTitle>
                        </div>
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() => copyLink(s.id)}
                          className="text-muted-foreground hover:text-foreground"
                          aria-label={t("docs.copyLink")}
                          title={t("docs.copyLink")}
                        >
                          <Copy className="size-3.5" />
                        </Button>
                      </div>
                    </CardHeader>
                    <CardContent className="space-y-3">
                      <CardDescription className="text-sm leading-relaxed">
                        {body}
                      </CardDescription>
                      {s.bullets.length > 0 && (
                        <ul className="mt-2 space-y-2 text-sm">
                          {s.bullets.map((b, i) => {
                            if ("key" in b) {
                              const isHead =
                                b.key.endsWith(".title") ||
                                b.key.endsWith(".q1") ||
                                b.key.endsWith(".q2") ||
                                b.key.endsWith(".q3") ||
                                b.key.endsWith(".q4") ||
                                b.key.endsWith(".q5") ||
                                b.key.endsWith(".q6") ||
                                b.key.includes("step");
                              return (
                                <li
                                  key={`${s.id}-b-${i}`}
                                  className={cn(
                                    "flex gap-2",
                                    isHead
                                      ? "mt-3 font-medium text-foreground"
                                      : "text-muted-foreground",
                                  )}
                                >
                                  <span
                                    className={cn(
                                      "mt-2 size-1.5 shrink-0 rounded-full",
                                      isHead ? "bg-primary" : "bg-muted-foreground/50",
                                    )}
                                  />
                                  <span>{t(b.key as never)}</span>
                                </li>
                              );
                            }
                            // Sub-heading + body pair (used by Transfers)
                            return (
                              <li
                                key={`${s.id}-sub-${i}`}
                                className="mt-3 space-y-1"
                              >
                                <div className="font-medium text-foreground">
                                  {t(b.subKey as never)}
                                </div>
                                <div className="text-muted-foreground">
                                  {t(b.bodyKey as never)}
                                </div>
                              </li>
                            );
                          })}
                        </ul>
                      )}
                    </CardContent>
                  </Card>
                </section>
              </FadeIn>
            );
          })}

          {filtered.length === 0 && (
            <div className="grid h-32 place-items-center rounded-2xl border bg-card/40 text-sm text-muted-foreground">
              {t("docs.empty")}
            </div>
          )}

          {/* CTA card — keeps the manual honest about its limits. */}
          <FadeIn>
            <Card className="bg-gradient-to-br from-primary/10 via-card/60 to-indigo-500/10">
              <CardHeader>
                <div className="flex items-center gap-2 text-[10px] font-medium uppercase tracking-[0.22em] text-primary">
                  <Sparkles className="size-3" />
                  {t("docs.cta.title")}
                </div>
                <CardTitle className="text-lg">{t("docs.cta.body")}</CardTitle>
              </CardHeader>
              <CardContent>
                <Link href="/chat">
                  <Button className="gap-1.5">
                    <MessageSquare className="size-4" />
                    {t("docs.cta.cta")}
                    <ArrowRight className="size-4" />
                  </Button>
                </Link>
              </CardContent>
            </Card>
          </FadeIn>
        </main>
      </div>
    </div>
  );
}

// Local Label — small wrapper to match the visual style used elsewhere
// in the app without pulling in the shared component, which would add a
// prop-drilling dependency for one tiny string.
function Label({
  className,
  ...props
}: React.HTMLAttributes<HTMLSpanElement>) {
  return (
    <span
      className={cn("text-xs text-muted-foreground", className)}
      {...props}
    />
  );
}
