"use client";

// Public-facing marketing layout.
//
// Used for the landing page, the login page, and any future marketing
// routes (/about, /pricing, /security, …). It gives us a single place
// to keep the top navigation, footer, and language switch consistent
// across all of those surfaces, instead of each page re-inventing its
// own header and stripping the sidebar.

import Link from "next/link";
import { useEffect, useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import {
  Menu,
  X,
  Sparkles,
  ShieldCheck,
  Activity,
  Link as LinkIcon,
  Mail,
  ChevronRight,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { LanguageButton } from "@/components/language-button";
import { AuroraBackdrop, FadeIn } from "@/components/motion/aurora";
import { useT } from "@/i18n/LanguageProvider";
import { cn } from "@/lib/utils";

function BrandMark({ className }: { className?: string }) {
  return (
    <span className={cn("inline-flex items-center gap-2.5", className)}>
      <span className="relative grid size-9 place-items-center overflow-hidden rounded-xl bg-[linear-gradient(135deg,hsl(186_95%_55%),hsl(220_85%_65%))] text-white shadow-glow">
        <svg
          viewBox="0 0 24 24"
          className="size-4.5"
          width={18}
          height={18}
          fill="none"
          stroke="currentColor"
          strokeWidth="2.2"
          strokeLinecap="round"
          strokeLinejoin="round"
          aria-hidden
        >
          <path d="M3 19 L8 11 L12 15 L16 8 L21 13" />
          <path d="M3 21 L21 21" opacity="0.6" />
        </svg>
        <span className="absolute inset-0 bg-[radial-gradient(circle_at_30%_20%,white/40,transparent_60%)]" />
      </span>
      <span className="font-display text-base font-semibold tracking-tight text-foreground">
        FinTech Pioneers
      </span>
    </span>
  );
}

export type NavLink = {
  href: string;
  labelKey:
    | "marketing.nav.features"
    | "marketing.nav.security"
    | "marketing.nav.how"
    | "marketing.nav.faq"
    | "marketing.nav.docs";
};

const DEFAULT_LINKS: NavLink[] = [
  { href: "#features", labelKey: "marketing.nav.features" },
  { href: "#how", labelKey: "marketing.nav.how" },
  { href: "#security", labelKey: "marketing.nav.security" },
  { href: "#faq", labelKey: "marketing.nav.faq" },
];

export function MarketingNav({
  links = DEFAULT_LINKS,
  ctaHref = "/login",
  ctaLabelKey = "marketing.nav.signin",
  /** When true, render a transparent header that turns into a glass
   *  panel as the user scrolls. Defaults to true. */
  sticky = true,
}: {
  links?: NavLink[];
  ctaHref?: string;
  ctaLabelKey?: "marketing.nav.signin" | "marketing.nav.getstarted";
  sticky?: boolean;
}) {
  const t = useT();
  const [scrolled, setScrolled] = useState(false);
  const [mobileOpen, setMobileOpen] = useState(false);

  useEffect(() => {
    if (!sticky) return;
    const onScroll = () => setScrolled(window.scrollY > 12);
    onScroll();
    window.addEventListener("scroll", onScroll, { passive: true });
    return () => window.removeEventListener("scroll", onScroll);
  }, [sticky]);

  return (
    <header
      className={cn(
        "sticky top-0 z-40 w-full transition-all duration-300",
        scrolled
          ? "border-b border-white/10 bg-background/80 backdrop-blur-xl"
          : "border-b border-transparent bg-background/20 backdrop-blur-sm",
      )}
    >
      <div className="mx-auto flex h-16 w-full max-w-6xl items-center justify-between gap-3 px-4 sm:px-6 lg:px-8">
        <Link
          href="/"
          className="rounded-md focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
          aria-label="FinTech Pioneers home"
        >
          <BrandMark />
        </Link>

        <nav className="hidden items-center gap-1 md:flex" aria-label="Primary">
          {links.map((l) => (
            <a
              key={l.href}
              href={l.href}
              className="rounded-md px-3 py-1.5 text-sm text-muted-foreground transition-colors hover:bg-white/[0.04] hover:text-foreground"
            >
              {t(l.labelKey)}
            </a>
          ))}
        </nav>

        <div className="hidden items-center gap-2 md:flex">
          <LanguageButton />
          <Link href={ctaHref}>
            <Button variant="gradient" size="sm" className="gap-1.5">
              {t(ctaLabelKey)}
              <ChevronRight className="size-3.5" />
            </Button>
          </Link>
        </div>

        <button
          type="button"
          className="grid size-9 place-items-center rounded-md border border-white/10 bg-card/40 text-foreground/80 backdrop-blur md:hidden"
          aria-label="Open menu"
          aria-expanded={mobileOpen}
          onClick={() => setMobileOpen((o) => !o)}
        >
          {mobileOpen ? <X className="size-4" /> : <Menu className="size-4" />}
        </button>
      </div>

      <AnimatePresence>
        {mobileOpen && (
          <motion.div
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: "auto", opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            transition={{ duration: 0.2, ease: [0.16, 1, 0.3, 1] }}
            className="overflow-hidden border-t border-white/5 bg-background/95 backdrop-blur-xl md:hidden"
          >
            <nav className="mx-auto flex w-full max-w-6xl flex-col gap-1 px-4 py-4 sm:px-6">
              {links.map((l) => (
                <a
                  key={l.href}
                  href={l.href}
                  onClick={() => setMobileOpen(false)}
                  className="rounded-md px-3 py-2.5 text-sm text-muted-foreground transition-colors hover:bg-white/[0.05] hover:text-foreground"
                >
                  {t(l.labelKey)}
                </a>
              ))}
              <div className="mt-2 flex items-center justify-between gap-2 border-t border-white/5 pt-3">
                <LanguageButton />
                <Link href={ctaHref} className="flex-1">
                  <Button variant="gradient" size="sm" className="w-full">
                    {t(ctaLabelKey)}
                  </Button>
                </Link>
              </div>
            </nav>
          </motion.div>
        )}
      </AnimatePresence>
    </header>
  );
}

function MarketingFooter() {
  const t = useT();
  const year = new Date().getFullYear();

  return (
    <footer className="relative border-t border-white/5 bg-background/40 backdrop-blur">
      <div className="mx-auto w-full max-w-6xl px-4 py-12 sm:px-6 lg:px-8">
        <div className="grid gap-10 md:grid-cols-4">
          <div className="md:col-span-2">
            <BrandMark />
            <p className="mt-3 max-w-sm text-sm text-muted-foreground">
              {t("marketing.footer.tagline")}
            </p>
            <p className="mt-4 inline-flex items-center gap-1.5 rounded-full border border-tier-green/30 bg-tier-green/10 px-2.5 py-1 text-[11px] font-medium text-tier-green">
              <ShieldCheck className="size-3" />
              {t("marketing.footer.simulationOnly")}
            </p>
          </div>

          <div>
            <p className="text-[11px] font-medium uppercase tracking-[0.18em] text-muted-foreground">
              {t("marketing.footer.product")}
            </p>
            <ul className="mt-3 space-y-2 text-sm">
              <li>
                <a href="#features" className="text-muted-foreground hover:text-foreground">
                  {t("marketing.nav.features")}
                </a>
              </li>
              <li>
                <a href="#security" className="text-muted-foreground hover:text-foreground">
                  {t("marketing.nav.security")}
                </a>
              </li>
              <li>
                <Link href="/docs" className="text-muted-foreground hover:text-foreground">
                  {t("marketing.footer.docs")}
                </Link>
              </li>
            </ul>
          </div>

          <div>
            <p className="text-[11px] font-medium uppercase tracking-[0.18em] text-muted-foreground">
              {t("marketing.footer.account")}
            </p>
            <ul className="mt-3 space-y-2 text-sm">
              <li>
                <Link href="/login" className="text-muted-foreground hover:text-foreground">
                  {t("marketing.nav.signin")}
                </Link>
              </li>
              <li className="flex items-center gap-2 text-muted-foreground/80">
                <Activity className="size-3.5" />
                <span>{t("marketing.footer.demoNotice")}</span>
              </li>
            </ul>
            <div className="mt-4 flex items-center gap-2 text-muted-foreground/70">
              <a
                href="https://github.com"
                target="_blank"
                rel="noopener noreferrer"
                aria-label="GitHub"
                className="grid size-8 place-items-center rounded-md border border-white/10 transition-colors hover:border-primary/40 hover:text-foreground"
              >
                <GithubMark className="size-3.5" />
              </a>
              <a
                href="https://twitter.com"
                target="_blank"
                rel="noopener noreferrer"
                aria-label="Twitter"
                className="grid size-8 place-items-center rounded-md border border-white/10 transition-colors hover:border-primary/40 hover:text-foreground"
              >
                <LinkIcon className="size-3.5" />
              </a>
              <a
                href="mailto:hello@example.com"
                aria-label="Email"
                className="grid size-8 place-items-center rounded-md border border-white/10 transition-colors hover:border-primary/40 hover:text-foreground"
              >
                <Mail className="size-3.5" />
              </a>
            </div>
          </div>
        </div>

        <div className="mt-10 flex flex-col items-start justify-between gap-3 border-t border-white/5 pt-6 text-[11px] text-muted-foreground/70 sm:flex-row sm:items-center">
          <p>
            © {year} FinTech Pioneers. {t("marketing.footer.rights")}
          </p>
          <p className="inline-flex items-center gap-1.5">
            <Sparkles className="size-3 text-primary" />
            {t("marketing.footer.motto")}
          </p>
        </div>
      </div>
    </footer>
  );
}

export function MarketingShell({
  children,
  navLinks,
  ctaHref,
  ctaLabelKey,
  showFooter = true,
}: {
  children: React.ReactNode;
  navLinks?: NavLink[];
  ctaHref?: string;
  ctaLabelKey?: "marketing.nav.signin" | "marketing.nav.getstarted";
  showFooter?: boolean;
}) {
  return (
    <div className="relative flex min-h-dvh flex-col">
      <AuroraBackdrop />
      <MarketingNav
        links={navLinks}
        ctaHref={ctaHref}
        ctaLabelKey={ctaLabelKey}
      />
      <FadeIn y={0} className="flex-1">
        {children}
      </FadeIn>
      {showFooter && <MarketingFooter />}
    </div>
  );
}

// Tiny inline GitHub mark (avoids pulling a 1MB icon lib for one glyph).
function GithubMark({ className }: { className?: string }) {
  return (
    <svg
      viewBox="0 0 24 24"
      aria-hidden
      className={className}
      fill="currentColor"
    >
      <path
        fillRule="evenodd"
        clipRule="evenodd"
        d="M12 2C6.48 2 2 6.58 2 12.24c0 4.52 2.87 8.36 6.84 9.72.5.1.68-.22.68-.49 0-.24-.01-.88-.01-1.72-2.78.62-3.37-1.36-3.37-1.36-.46-1.18-1.11-1.49-1.11-1.49-.91-.63.07-.62.07-.62 1 .07 1.53 1.05 1.53 1.05.89 1.56 2.34 1.11 2.91.85.09-.66.35-1.11.63-1.37-2.22-.26-4.55-1.14-4.55-5.06 0-1.12.39-2.04 1.03-2.75-.1-.26-.45-1.31.1-2.72 0 0 .84-.27 2.75 1.05.8-.23 1.65-.34 2.5-.34s1.7.11 2.5.34c1.91-1.32 2.75-1.05 2.75-1.05.55 1.41.2 2.46.1 2.72.64.71 1.03 1.63 1.03 2.75 0 3.93-2.34 4.8-4.57 5.05.36.32.68.95.68 1.92 0 1.38-.01 2.5-.01 2.84 0 .27.18.6.69.49C19.13 20.59 22 16.75 22 12.24 22 6.58 17.52 2 12 2Z"
      />
    </svg>
  );
}
