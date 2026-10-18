"use client";

// All public landing-page sections.
//
// These are intentionally split out of the route file so each section
// can be tweaked, reordered, or reused (e.g. the hero block in /login)
// without re-reading the entire landing page. Every section takes its
// copy from the i18n dictionary, never from inline strings, so the
// English/中文 toggle keeps every page in lock-step.

import Link from "next/link";
import { motion } from "framer-motion";
import {
  ShieldCheck,
  Lock,
  Eye,
  Activity,
  Languages,
  AlertTriangle,
  CheckCircle2,
  Mic,
  Camera,
  Calculator,
  PiggyBank,
  CreditCard,
  Coins,
  Receipt,
  MessageSquare,
  Workflow,
  Send,
  ArrowLeftRight,
  FileSearch,
  ChevronRight,
  Sparkles,
  type LucideIcon,
} from "lucide-react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { FadeIn, Stagger, StaggerItem } from "@/components/motion/aurora";
import { useT } from "@/i18n/LanguageProvider";
import { cn } from "@/lib/utils";

// --- Hero ----------------------------------------------------------------

export function LandingHero() {
  const t = useT();
  return (
    <section
      id="hero"
      className="relative isolate overflow-hidden pb-20 pt-16 sm:pt-24 lg:pt-28"
    >
      {/* additional spotlights behind the hero text */}
      <div
        aria-hidden
        className="pointer-events-none absolute -top-24 left-1/2 h-[480px] w-[820px] -translate-x-1/2 rounded-full opacity-70 blur-3xl"
        style={{
          background:
            "radial-gradient(ellipse at center, hsl(186 95% 55% / 0.35), transparent 60%)",
        }}
      />
      <div
        aria-hidden
        className="pointer-events-none absolute right-[-10%] top-32 h-[360px] w-[560px] rounded-full opacity-50 blur-3xl"
        style={{
          background:
            "radial-gradient(ellipse at center, hsl(220 80% 60% / 0.30), transparent 60%)",
        }}
      />

      <div className="relative mx-auto grid w-full max-w-6xl items-center gap-12 px-4 sm:px-6 lg:grid-cols-[1.1fr_0.9fr] lg:gap-16 lg:px-8">
        <div>
          <FadeIn>
            <span className="inline-flex items-center gap-2 rounded-full border border-white/10 bg-card/60 px-3 py-1 text-[11px] font-medium text-muted-foreground backdrop-blur">
              <Sparkles className="size-3 text-primary" />
              {t("marketing.hero.eyebrow")}
            </span>
          </FadeIn>

          <FadeIn delay={0.05}>
            <h1 className="mt-5 font-display text-4xl font-semibold leading-[1.05] tracking-tight text-foreground sm:text-5xl lg:text-6xl">
              {t("marketing.hero.title.1")}{" "}
              <span className="text-brand">{t("marketing.hero.title.2")}</span>
              <br />
              {t("marketing.hero.title.3")}
            </h1>
          </FadeIn>

          <FadeIn delay={0.1}>
            <p className="mt-5 max-w-xl text-base text-muted-foreground sm:text-lg">
              {t("marketing.hero.subtitle")}
            </p>
          </FadeIn>

          <FadeIn delay={0.18}>
            <div className="mt-7 flex flex-wrap items-center gap-3">
              <Link href="/login">
                <Button variant="gradient" size="lg" className="gap-1.5">
                  {t("marketing.hero.ctaPrimary")}
                  <ChevronRight className="size-4" />
                </Button>
              </Link>
              <a href="#features">
                <Button variant="outline" size="lg">
                  {t("marketing.hero.ctaSecondary")}
                </Button>
              </a>
            </div>
          </FadeIn>

          <FadeIn delay={0.25}>
            <ul className="mt-8 flex flex-wrap items-center gap-x-6 gap-y-2 text-xs text-muted-foreground">
              {(
                [
                  t("marketing.hero.bullet.1"),
                  t("marketing.hero.bullet.2"),
                  t("marketing.hero.bullet.3"),
                ] as const
              ).map((label) => (
                <li key={label} className="inline-flex items-center gap-1.5">
                  <CheckCircle2 className="size-3.5 text-tier-green" />
                  {label}
                </li>
              ))}
            </ul>
          </FadeIn>
        </div>

        <FadeIn delay={0.15} y={16}>
          <HeroPreviewCard />
        </FadeIn>
      </div>
    </section>
  );
}

function HeroPreviewCard() {
  const t = useT();
  return (
    <div className="relative">
      <div
        aria-hidden
        className="absolute -inset-6 -z-10 rounded-[2.5rem] opacity-70 blur-2xl"
        style={{
          background:
            "radial-gradient(ellipse at 50% 0%, hsl(186 95% 55% / 0.35), transparent 65%)",
        }}
      />
      <Card className="overflow-hidden">
        <CardHeader className="border-b border-white/5 pb-3">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <span className="size-2 rounded-full bg-tier-red" />
              <span className="size-2 rounded-full bg-tier-yellow" />
              <span className="size-2 rounded-full bg-tier-green" />
            </div>
            <Badge variant="muted" className="text-[10px] uppercase tracking-wider">
              {t("marketing.hero.preview.eyebrow")}
            </Badge>
          </div>
          <div className="mt-3 flex items-start gap-3">
            <div className="grid size-9 shrink-0 place-items-center rounded-full bg-primary/15 text-primary ring-1 ring-primary/30">
              <MessageSquare className="size-4" />
            </div>
            <div>
              <p className="text-[11px] font-medium uppercase tracking-wider text-muted-foreground">
                {t("marketing.hero.preview.userLabel")}
              </p>
              <p className="mt-0.5 text-sm">{t("marketing.hero.preview.userText")}</p>
            </div>
          </div>
          <div className="mt-3 flex items-start gap-3">
            <div className="grid size-9 shrink-0 place-items-center rounded-full bg-tier-green/15 text-tier-green ring-1 ring-tier-green/30">
              <ShieldCheck className="size-4" />
            </div>
            <div>
              <p className="text-[11px] font-medium uppercase tracking-wider text-muted-foreground">
                {t("marketing.hero.preview.assistantLabel")}
              </p>
              <p className="mt-0.5 text-sm text-foreground/90">
                {t("marketing.hero.preview.assistantText")}
              </p>
              <div className="mt-3 grid grid-cols-3 gap-2 text-[11px]">
                <PreviewPill
                  icon={Send}
                  label={t("marketing.hero.preview.pill.to")}
                  value="Li Wei · *0421"
                />
                <PreviewPill
                  icon={Coins}
                  label={t("marketing.hero.preview.pill.amount")}
                  value="¥120.00"
                />
                <PreviewPill
                  icon={ShieldCheck}
                  label={t("marketing.hero.preview.pill.tier")}
                  value="Yellow"
                />
              </div>
            </div>
          </div>
        </CardHeader>
        <CardContent className="space-y-3 pt-4">
          <div className="flex items-center justify-between text-xs text-muted-foreground">
            <span className="inline-flex items-center gap-1.5">
              <Lock className="size-3" />
              {t("marketing.hero.preview.tip")}
            </span>
            <span className="font-mono text-[10px] text-muted-foreground/70">
              audit · 7a3f…
            </span>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

function PreviewPill({
  icon: Icon,
  label,
  value,
}: {
  icon: LucideIcon;
  label: string;
  value: string;
}) {
  return (
    <div className="rounded-md border border-white/10 bg-white/[0.03] p-2">
      <p className="inline-flex items-center gap-1 text-[10px] uppercase tracking-wider text-muted-foreground">
        <Icon className="size-3" />
        {label}
      </p>
      <p className="mt-0.5 truncate font-mono text-[11px] text-foreground">{value}</p>
    </div>
  );
}

// --- Logo strip / social proof ------------------------------------------

export function LogoStrip() {
  const t = useT();
  const labels = [
    t("marketing.logos.press1"),
    t("marketing.logos.press2"),
    t("marketing.logos.press3"),
    t("marketing.logos.press4"),
  ];
  return (
    <section className="border-y border-white/5 bg-background/40 py-6 backdrop-blur">
      <div className="mx-auto w-full max-w-6xl px-4 sm:px-6 lg:px-8">
        <p className="text-center text-[11px] font-medium uppercase tracking-[0.18em] text-muted-foreground">
          {t("marketing.logos.label")}
        </p>
        <div className="mt-3 grid grid-cols-2 items-center gap-x-6 gap-y-2 text-center text-sm font-display text-foreground/60 sm:grid-cols-4">
          {labels.map((l) => (
            <span key={l} className="truncate">
              {l}
            </span>
          ))}
        </div>
      </div>
    </section>
  );
}

// --- Features grid -------------------------------------------------------

type FeatureItem = {
  icon: LucideIcon;
  titleKey:
    | "marketing.features.smart.title"
    | "marketing.features.bills.title"
    | "marketing.features.subs.title"
    | "marketing.features.wealth.title"
    | "marketing.features.cards.title"
    | "marketing.features.voice.title";
  bodyKey:
    | "marketing.features.smart.body"
    | "marketing.features.bills.body"
    | "marketing.features.subs.body"
    | "marketing.features.wealth.body"
    | "marketing.features.cards.body"
    | "marketing.features.voice.body";
  accent: string;
};

const FEATURES: FeatureItem[] = [
  {
    icon: MessageSquare,
    titleKey: "marketing.features.smart.title",
    bodyKey: "marketing.features.smart.body",
    accent: "from-primary/30 to-cyan-500/30",
  },
  {
    icon: Receipt,
    titleKey: "marketing.features.bills.title",
    bodyKey: "marketing.features.bills.body",
    accent: "from-indigo-500/30 to-fuchsia-500/30",
  },
  {
    icon: ArrowLeftRight,
    titleKey: "marketing.features.subs.title",
    bodyKey: "marketing.features.subs.body",
    accent: "from-amber-500/30 to-rose-500/30",
  },
  {
    icon: Coins,
    titleKey: "marketing.features.wealth.title",
    bodyKey: "marketing.features.wealth.body",
    accent: "from-emerald-500/30 to-teal-500/30",
  },
  {
    icon: CreditCard,
    titleKey: "marketing.features.cards.title",
    bodyKey: "marketing.features.cards.body",
    accent: "from-sky-500/30 to-indigo-500/30",
  },
  {
    icon: Mic,
    titleKey: "marketing.features.voice.title",
    bodyKey: "marketing.features.voice.body",
    accent: "from-fuchsia-500/30 to-pink-500/30",
  },
];

export function LandingFeatures() {
  const t = useT();
  return (
    <section id="features" className="relative py-20 sm:py-24">
      <div className="mx-auto w-full max-w-6xl px-4 sm:px-6 lg:px-8">
        <SectionHeader
          eyebrow={t("marketing.features.eyebrow")}
          title={t("marketing.features.title")}
          subtitle={t("marketing.features.subtitle")}
        />

        <Stagger delayChildren={0.05} staggerChildren={0.06}>
          <div className="mt-10 grid gap-5 sm:grid-cols-2 lg:grid-cols-3">
            {FEATURES.map((f) => (
              <StaggerItem key={f.titleKey}>
                <Card className="h-full">
                  <CardHeader>
                    <div
                      className={cn(
                        "mb-3 grid size-10 place-items-center rounded-xl bg-gradient-to-br text-foreground ring-1 ring-white/10",
                        f.accent,
                      )}
                    >
                      <f.icon className="size-4.5" />
                    </div>
                    <CardTitle className="text-lg">{t(f.titleKey)}</CardTitle>
                  </CardHeader>
                  <CardContent>
                    <p className="text-sm text-muted-foreground">{t(f.bodyKey)}</p>
                  </CardContent>
                </Card>
              </StaggerItem>
            ))}
          </div>
        </Stagger>
      </div>
    </section>
  );
}

// --- How it works --------------------------------------------------------

const STEPS: { icon: LucideIcon; titleKey: "marketing.how.step1.title" | "marketing.how.step2.title" | "marketing.how.step3.title"; bodyKey: "marketing.how.step1.body" | "marketing.how.step2.body" | "marketing.how.step3.body" }[] = [
  {
    icon: MessageSquare,
    titleKey: "marketing.how.step1.title",
    bodyKey: "marketing.how.step1.body",
  },
  {
    icon: Workflow,
    titleKey: "marketing.how.step2.title",
    bodyKey: "marketing.how.step2.body",
  },
  {
    icon: ShieldCheck,
    titleKey: "marketing.how.step3.title",
    bodyKey: "marketing.how.step3.body",
  },
];

export function LandingHowItWorks() {
  const t = useT();
  return (
    <section
      id="how"
      className="relative border-y border-white/5 bg-background/40 py-20 sm:py-24 backdrop-blur"
    >
      <div className="mx-auto w-full max-w-6xl px-4 sm:px-6 lg:px-8">
        <SectionHeader
          eyebrow={t("marketing.how.eyebrow")}
          title={t("marketing.how.title")}
          subtitle={t("marketing.how.subtitle")}
        />

        <ol className="mt-12 grid gap-6 md:grid-cols-3">
          {STEPS.map((s, i) => (
            <FadeIn key={s.titleKey} delay={0.05 + i * 0.08}>
              <li className="relative h-full">
                <span className="absolute -top-3 left-4 grid size-8 place-items-center rounded-full border border-white/10 bg-card text-sm font-semibold text-foreground">
                  {i + 1}
                </span>
                <Card className="h-full pt-8">
                  <CardHeader>
                    <s.icon className="size-5 text-primary" />
                    <CardTitle className="mt-3 text-lg">{t(s.titleKey)}</CardTitle>
                  </CardHeader>
                  <CardContent>
                    <p className="text-sm text-muted-foreground">{t(s.bodyKey)}</p>
                  </CardContent>
                </Card>
              </li>
            </FadeIn>
          ))}
        </ol>
      </div>
    </section>
  );
}

// --- Security section ----------------------------------------------------

const PILLARS: { icon: LucideIcon; titleKey: "marketing.security.pillars.tier.title" | "marketing.security.pillars.audit.title" | "marketing.security.pillars.lock.title" | "marketing.security.pillars.bilingual.title"; bodyKey: "marketing.security.pillars.tier.body" | "marketing.security.pillars.audit.body" | "marketing.security.pillars.lock.body" | "marketing.security.pillars.bilingual.body" }[] = [
  {
    icon: ShieldCheck,
    titleKey: "marketing.security.pillars.tier.title",
    bodyKey: "marketing.security.pillars.tier.body",
  },
  {
    icon: FileSearch,
    titleKey: "marketing.security.pillars.audit.title",
    bodyKey: "marketing.security.pillars.audit.body",
  },
  {
    icon: Lock,
    titleKey: "marketing.security.pillars.lock.title",
    bodyKey: "marketing.security.pillars.lock.body",
  },
  {
    icon: Languages,
    titleKey: "marketing.security.pillars.bilingual.title",
    bodyKey: "marketing.security.pillars.bilingual.body",
  },
];

export function LandingSecurity() {
  const t = useT();
  return (
    <section id="security" className="relative py-20 sm:py-24">
      <div className="mx-auto grid w-full max-w-6xl items-start gap-12 px-4 sm:px-6 lg:grid-cols-[0.9fr_1.1fr] lg:px-8">
        <div>
          <span className="inline-flex items-center gap-1.5 rounded-full border border-tier-green/30 bg-tier-green/10 px-2.5 py-1 text-[11px] font-medium text-tier-green">
            <ShieldCheck className="size-3" />
            {t("marketing.security.eyebrow")}
          </span>
          <h2 className="mt-4 font-display text-3xl font-semibold tracking-tight sm:text-4xl">
            {t("marketing.security.title")}
          </h2>
          <p className="mt-3 text-muted-foreground">
            {t("marketing.security.subtitle")}
          </p>
          <ul className="mt-6 space-y-3 text-sm text-foreground/90">
            {(
              [
                t("marketing.security.check.1"),
                t("marketing.security.check.2"),
                t("marketing.security.check.3"),
                t("marketing.security.check.4"),
              ] as const
            ).map((line) => (
              <li key={line} className="flex items-start gap-2">
                <CheckCircle2 className="mt-0.5 size-4 shrink-0 text-tier-green" />
                <span>{line}</span>
              </li>
            ))}
          </ul>
        </div>

        <Stagger delayChildren={0.05} staggerChildren={0.08}>
          <div className="grid gap-4 sm:grid-cols-2">
            {PILLARS.map((p) => (
              <StaggerItem key={p.titleKey}>
                <Card className="h-full">
                  <CardHeader>
                    <div className="grid size-9 place-items-center rounded-lg bg-tier-green/10 text-tier-green ring-1 ring-tier-green/30">
                      <p.icon className="size-4" />
                    </div>
                    <CardTitle className="mt-3 text-base">{t(p.titleKey)}</CardTitle>
                  </CardHeader>
                  <CardContent>
                    <p className="text-sm text-muted-foreground">{t(p.bodyKey)}</p>
                  </CardContent>
                </Card>
              </StaggerItem>
            ))}
          </div>
        </Stagger>
      </div>
    </section>
  );
}

// --- Tools grid (deep links to /login) ----------------------------------

const TOOL_GROUPS: { icon: LucideIcon; titleKey: "marketing.tools.assistant.title" | "marketing.tools.forecast.title" | "marketing.tools.receipt.title" | "marketing.tools.budgets.title" | "marketing.tools.workflow.title" | "marketing.tools.alerts.title"; bodyKey: "marketing.tools.assistant.body" | "marketing.tools.forecast.body" | "marketing.tools.receipt.body" | "marketing.tools.budgets.body" | "marketing.tools.workflow.body" | "marketing.tools.alerts.body" }[] = [
  { icon: MessageSquare, titleKey: "marketing.tools.assistant.title", bodyKey: "marketing.tools.assistant.body" },
  { icon: Calculator, titleKey: "marketing.tools.forecast.title", bodyKey: "marketing.tools.forecast.body" },
  { icon: Camera, titleKey: "marketing.tools.receipt.title", bodyKey: "marketing.tools.receipt.body" },
  { icon: PiggyBank, titleKey: "marketing.tools.budgets.title", bodyKey: "marketing.tools.budgets.body" },
  { icon: Workflow, titleKey: "marketing.tools.workflow.title", bodyKey: "marketing.tools.workflow.body" },
  { icon: AlertTriangle, titleKey: "marketing.tools.alerts.title", bodyKey: "marketing.tools.alerts.body" },
];

export function LandingTools() {
  const t = useT();
  return (
    <section className="relative border-t border-white/5 bg-background/40 py-20 sm:py-24 backdrop-blur">
      <div className="mx-auto w-full max-w-6xl px-4 sm:px-6 lg:px-8">
        <SectionHeader
          eyebrow={t("marketing.tools.eyebrow")}
          title={t("marketing.tools.title")}
          subtitle={t("marketing.tools.subtitle")}
        />
        <Stagger delayChildren={0.05} staggerChildren={0.06}>
          <div className="mt-10 grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
            {TOOL_GROUPS.map((g) => (
              <StaggerItem key={g.titleKey}>
                <div className="group flex h-full items-start gap-3 rounded-2xl border border-white/5 bg-card/60 p-5 backdrop-blur transition-all duration-300 hover:border-primary/30 hover:bg-card/80">
                  <div className="grid size-10 shrink-0 place-items-center rounded-lg bg-primary/10 text-primary ring-1 ring-primary/30">
                    <g.icon className="size-4" />
                  </div>
                  <div>
                    <p className="font-display text-sm font-semibold tracking-tight">
                      {t(g.titleKey)}
                    </p>
                    <p className="mt-1 text-xs text-muted-foreground">{t(g.bodyKey)}</p>
                  </div>
                </div>
              </StaggerItem>
            ))}
          </div>
        </Stagger>
      </div>
    </section>
  );
}

// --- FAQ -----------------------------------------------------------------

const FAQ: { qKey: "marketing.faq.q1" | "marketing.faq.q2" | "marketing.faq.q3" | "marketing.faq.q4"; aKey: "marketing.faq.a1" | "marketing.faq.a2" | "marketing.faq.a3" | "marketing.faq.a4" }[] = [
  { qKey: "marketing.faq.q1", aKey: "marketing.faq.a1" },
  { qKey: "marketing.faq.q2", aKey: "marketing.faq.a2" },
  { qKey: "marketing.faq.q3", aKey: "marketing.faq.a3" },
  { qKey: "marketing.faq.q4", aKey: "marketing.faq.a4" },
];

export function LandingFAQ() {
  const t = useT();
  return (
    <section id="faq" className="relative py-20 sm:py-24">
      <div className="mx-auto w-full max-w-3xl px-4 sm:px-6 lg:px-8">
        <SectionHeader
          eyebrow={t("marketing.faq.eyebrow")}
          title={t("marketing.faq.title")}
          subtitle={t("marketing.faq.subtitle")}
          align="center"
        />
        <div className="mt-10 space-y-3">
          {FAQ.map((row, i) => (
            <FadeIn key={row.qKey} delay={0.04 * i}>
              <details
                className={cn(
                  "group rounded-2xl border border-white/10 bg-card/60 p-5 backdrop-blur transition-colors",
                  "open:border-primary/30 open:bg-card/80",
                )}
              >
                <summary className="flex cursor-pointer list-none items-center justify-between gap-4 text-sm font-medium text-foreground">
                  <span className="inline-flex items-center gap-2">
                    <Eye className="size-3.5 text-primary" />
                    {t(row.qKey)}
                  </span>
                  <ChevronRight className="size-4 text-muted-foreground transition-transform group-open:rotate-90" />
                </summary>
                <p className="mt-3 text-sm leading-relaxed text-muted-foreground">
                  {t(row.aKey)}
                </p>
              </details>
            </FadeIn>
          ))}
        </div>
      </div>
    </section>
  );
}

// --- Final CTA -----------------------------------------------------------

export function LandingCTA() {
  const t = useT();
  return (
    <section className="relative py-20 sm:py-24">
      <div className="mx-auto w-full max-w-5xl px-4 sm:px-6 lg:px-8">
        <motion.div
          initial={{ opacity: 0, y: 16 }}
          whileInView={{ opacity: 1, y: 0 }}
          viewport={{ once: true, margin: "-80px" }}
          transition={{ duration: 0.5, ease: [0.16, 1, 0.3, 1] }}
          className="relative overflow-hidden rounded-3xl border border-white/10 bg-card/60 p-8 backdrop-blur-md sm:p-12"
        >
          <div
            aria-hidden
            className="pointer-events-none absolute -top-24 right-[-10%] h-72 w-72 rounded-full opacity-60 blur-3xl"
            style={{
              background:
                "radial-gradient(circle, hsl(186 95% 55% / 0.45), transparent 70%)",
            }}
          />
          <div
            aria-hidden
            className="pointer-events-none absolute -bottom-24 left-[-10%] h-72 w-72 rounded-full opacity-50 blur-3xl"
            style={{
              background:
                "radial-gradient(circle, hsl(220 80% 60% / 0.35), transparent 70%)",
            }}
          />
          <div className="relative grid items-center gap-8 lg:grid-cols-[1.4fr_1fr]">
            <div>
              <h2 className="font-display text-3xl font-semibold tracking-tight sm:text-4xl">
                {t("marketing.cta.title")}
              </h2>
              <p className="mt-3 max-w-xl text-muted-foreground">
                {t("marketing.cta.subtitle")}
              </p>
            </div>
            <div className="flex flex-col items-stretch gap-3 sm:flex-row lg:items-end lg:justify-end">
              <Link href="/login" className="sm:flex-1 lg:flex-none">
                <Button variant="gradient" size="lg" className="w-full gap-1.5">
                  {t("marketing.cta.primary")}
                  <ChevronRight className="size-4" />
                </Button>
              </Link>
              <Link href="/docs" className="sm:flex-1 lg:flex-none">
                <Button variant="outline" size="lg" className="w-full">
                  {t("marketing.cta.secondary")}
                </Button>
              </Link>
            </div>
          </div>
        </motion.div>
      </div>
    </section>
  );
}

// --- Helpers -------------------------------------------------------------

function SectionHeader({
  eyebrow,
  title,
  subtitle,
  align = "left",
}: {
  eyebrow?: string;
  title: string;
  subtitle?: string;
  align?: "left" | "center";
}) {
  return (
    <div
      className={cn(
        "max-w-2xl",
        align === "center" && "mx-auto text-center",
      )}
    >
      {eyebrow && (
        <p className="text-[11px] font-medium uppercase tracking-[0.18em] text-primary">
          {eyebrow}
        </p>
      )}
      <h2 className="mt-2 font-display text-3xl font-semibold tracking-tight sm:text-4xl">
        {title}
      </h2>
      {subtitle && (
        <p className="mt-3 text-base text-muted-foreground">{subtitle}</p>
      )}
    </div>
  );
}

// (No re-exports needed — sections are imported by name in app/page.tsx.)
