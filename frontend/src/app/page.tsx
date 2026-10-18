"use client";

// Public landing page.
//
// `/` used to immediately bounce visitors to /login. That made the
// product feel like a back-office tool, not a real product. Now we
// show a marketing page first and let the user opt in to sign in
// from a clear CTA. Authenticated users still get a tiny escape
// hatch — the brand mark on the page header links back to their
// dashboard, so a signed-in user landing on `/` from a stale link
// isn't trapped.

import { useRouter } from "next/navigation";
import { MarketingShell } from "@/components/layout/marketing-shell";
import {
  LandingHero,
  LandingFeatures,
  LandingHowItWorks,
  LandingSecurity,
  LandingTools,
  LandingFAQ,
  LandingCTA,
  LogoStrip,
} from "@/components/marketing/landing-sections";
import { useAuth } from "@/lib/store";

export default function Home() {
  const router = useRouter();
  const { me, loading } = useAuth();

  // Sign-in CTA targets /login. The chip in the nav (rendered when
  // signed in) targets the right place per role.
  const ctaHref = !loading && me
    ? me.role === "admin"
      ? "/admin"
      : "/dashboard"
    : "/login";

  return (
    <MarketingShell ctaHref={ctaHref} ctaLabelKey="marketing.nav.signin">
      <SignedInNotice onOpen={() =>
        me ? router.replace(me.role === "admin" ? "/admin" : "/dashboard") : null
      } />
      <LandingHero />
      <LogoStrip />
      <LandingFeatures />
      <LandingHowItWorks />
      <LandingSecurity />
      <LandingTools />
      <LandingFAQ />
      <LandingCTA />
    </MarketingShell>
  );
}

// Tiny "you're already signed in" chip — hidden until the auth store
// resolves so we don't flash during hydration.
function SignedInNotice({ onOpen }: { onOpen: () => void }) {
  const { me, loading } = useAuth();
  if (loading || !me) return null;
  return (
    <div className="border-b border-white/5 bg-background/40 backdrop-blur">
      <div className="mx-auto flex w-full max-w-6xl items-center justify-between gap-3 px-4 py-2 text-xs sm:px-6 lg:px-8">
        <p className="text-muted-foreground">
          Signed in as <span className="font-medium text-foreground">{me.display_name}</span>
        </p>
        <button
          type="button"
          onClick={onOpen}
          className="inline-flex items-center gap-1 rounded-md border border-primary/30 bg-primary/10 px-2.5 py-1 font-medium text-primary transition-colors hover:bg-primary/20"
        >
          Open dashboard →
        </button>
      </div>
    </div>
  );
}
