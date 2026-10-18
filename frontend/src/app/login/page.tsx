"use client";

import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { motion } from "framer-motion";
import { useAuth } from "@/lib/store";
import { auth, ApiError } from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { MarketingShell } from "@/components/layout/marketing-shell";
import {
  FadeIn,
  MountainHero,
  Stagger,
  StaggerItem,
} from "@/components/motion/aurora";
import { useToast } from "@/components/ui/toast";
import { Loader2, ShieldCheck, Sparkles, Eye, EyeOff, Lock, ChevronLeft } from "lucide-react";
import { useT } from "@/i18n/LanguageProvider";
import Link from "next/link";

type DemoAccount = {
  labelKey:
    | "demo.customer.alex.label"
    | "demo.customer.mei.label"
    | "demo.customer.vera.label"
    | "demo.admin.label"
    | "demo.auditor.label"
    | "demo.locked.label";
  subKey:
    | "demo.customer.alex.sub"
    | "demo.customer.mei.sub"
    | "demo.customer.vera.sub"
    | "demo.admin.sub"
    | "demo.auditor.sub"
    | "demo.locked.sub";
  username: string;
  password: string;
  accent: string;
  disabled?: boolean;
};

const DEMO_ACCOUNTS: DemoAccount[] = [
  {
    labelKey: "demo.customer.alex.label",
    subKey: "demo.customer.alex.sub",
    username: "demo",
    password: "demo1234",
    accent: "from-primary/20 to-cyan-500/20",
  },
  {
    labelKey: "demo.customer.mei.label",
    subKey: "demo.customer.mei.sub",
    username: "intl_student",
    password: "demo1234",
    accent: "from-indigo-500/20 to-fuchsia-500/20",
  },
  {
    labelKey: "demo.customer.vera.label",
    subKey: "demo.customer.vera.sub",
    username: "vip_demo",
    password: "demo1234",
    accent: "from-emerald-500/20 to-amber-500/20",
  },
  {
    labelKey: "demo.admin.label",
    subKey: "demo.admin.sub",
    username: "admin",
    password: "admin1234",
    accent: "from-amber-500/20 to-rose-500/20",
  },
  {
    labelKey: "demo.auditor.label",
    subKey: "demo.auditor.sub",
    username: "auditor",
    password: "demo1234",
    accent: "from-slate-500/20 to-slate-500/5",
  },
  {
    labelKey: "demo.locked.label",
    subKey: "demo.locked.sub",
    username: "locked_vip",
    password: "demo1234",
    accent: "from-rose-500/20 to-rose-500/5",
    disabled: true,
  },
];

export default function LoginPage() {
  const router = useRouter();
  const { setMe, me, loading } = useAuth();
  const { toast } = useToast();
  const t = useT();
  const [username, setUsername] = useState("demo");
  const [password, setPassword] = useState("demo1234");
  const [submitting, setSubmitting] = useState(false);
  const [show, setShow] = useState(false);

  useEffect(() => {
    if (!loading && me) router.replace("/dashboard");
  }, [me, loading, router]);

  const onSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setSubmitting(true);
    try {
      await auth.login(username, password);
      const meData = await auth.me();
      setMe(meData);
      router.replace("/dashboard");
    } catch (err) {
      const msg =
        err instanceof ApiError ? err.message : t("error.network");
      toast({ variant: "error", title: t("login.signInFailed"), description: msg });
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <MarketingShell showFooter={false}>
      <div className="relative grid min-h-[calc(100dvh-4rem)] grid-cols-1 lg:grid-cols-2">
        {/* Left: hero */}
        <div className="relative hidden flex-col justify-between overflow-hidden p-10 lg:flex lg:p-12">
          <FadeIn>
            <Link
              href="/"
              className="relative z-10 inline-flex items-center gap-1.5 self-start rounded-md border border-white/10 bg-card/40 px-2.5 py-1 text-xs text-muted-foreground backdrop-blur transition-colors hover:border-primary/40 hover:text-foreground"
            >
              <ChevronLeft className="size-3.5" />
              {t("login.backHome")}
            </Link>
          </FadeIn>

          <div className="relative z-10">
            <FadeIn delay={0.05}>
              <h1 className="font-display text-4xl font-semibold leading-[1.05] tracking-tight md:text-5xl">
                {t("login.hero.heading.1")}
                <br />
                <span className="text-brand">{t("login.hero.heading.2")}</span>
              </h1>
            </FadeIn>
            <FadeIn delay={0.12}>
              <p className="mt-5 max-w-md text-base text-muted-foreground">
                {t("login.hero.body")}
              </p>
            </FadeIn>

            <Stagger delayChildren={0.2} staggerChildren={0.08}>
              <ul className="mt-10 grid gap-3 text-sm">
                {[
                  { icon: ShieldCheck, label: t("login.hero.bullet.1") },
                  { icon: Sparkles, label: t("login.hero.bullet.2") },
                  { icon: Eye, label: t("login.hero.bullet.3") },
                ].map((it) => (
                  <StaggerItem key={it.label}>
                    <li className="flex items-center gap-3 rounded-lg border border-white/5 bg-white/[0.03] p-3 backdrop-blur">
                      <span className="grid size-8 place-items-center rounded-md bg-primary/15 text-primary ring-1 ring-primary/30">
                        <it.icon className="size-4" />
                      </span>
                      {it.label}
                    </li>
                  </StaggerItem>
                ))}
              </ul>
            </Stagger>
          </div>

          <FadeIn delay={0.35}>
            <p className="relative z-10 text-xs text-muted-foreground">
              {t("login.hero.disclaimer")}
            </p>
          </FadeIn>

          <MountainHero />
        </div>

        {/* Right: form */}
        <div className="relative flex items-center justify-center p-6 sm:p-10">
          {/* mobile-only back link */}
          <Link
            href="/"
            className="absolute left-4 top-4 inline-flex items-center gap-1 rounded-md border border-white/10 bg-card/40 px-2.5 py-1 text-xs text-muted-foreground backdrop-blur transition-colors hover:border-primary/40 hover:text-foreground lg:hidden"
          >
            <ChevronLeft className="size-3.5" />
            {t("login.backHome")}
          </Link>

          <FadeIn y={12}>
            <Card className="w-full max-w-sm">
              <CardHeader className="space-y-1.5">
                <p className="text-[10px] font-medium uppercase tracking-[0.18em] text-primary">
                  {t("login.eyebrow")}
                </p>
                <CardTitle className="text-2xl">{t("login.title")}</CardTitle>
                <p className="text-sm text-muted-foreground">{t("login.subtitle")}</p>
              </CardHeader>
              <CardContent>
                <form onSubmit={onSubmit} className="space-y-3">
                  <div className="space-y-1.5">
                    <Label htmlFor="username">{t("login.username")}</Label>
                    <Input
                      id="username"
                      autoComplete="username"
                      value={username}
                      onChange={(e) => setUsername(e.target.value)}
                      required
                    />
                  </div>
                  <div className="space-y-1.5">
                    <Label htmlFor="password">{t("login.password")}</Label>
                    <div className="relative">
                      <Input
                        id="password"
                        type={show ? "text" : "password"}
                        autoComplete="current-password"
                        value={password}
                        onChange={(e) => setPassword(e.target.value)}
                        required
                        className="pr-9"
                      />
                      <button
                        type="button"
                        onClick={() => setShow((s) => !s)}
                        className="absolute right-2 top-1/2 -translate-y-1/2 rounded p-1 text-muted-foreground hover:text-foreground"
                        aria-label={show ? t("login.toggle.hide") : t("login.toggle.show")}
                      >
                        {show ? <EyeOff className="size-4" /> : <Eye className="size-4" />}
                      </button>
                    </div>
                  </div>

                  <Button
                    type="submit"
                    variant="gradient"
                    size="lg"
                    className="mt-2 w-full"
                    disabled={submitting}
                  >
                    {submitting && <Loader2 className="size-4 animate-spin" />}
                    {submitting ? t("login.signingIn") : t("login.submit")}
                  </Button>
                </form>

                <div className="mt-7">
                  <p className="text-[11px] font-medium uppercase tracking-wider text-muted-foreground">
                    {t("login.demo.title")}
                  </p>
                  <p className="mt-1 text-[11px] text-muted-foreground/70">
                    {t("login.demo.hint")} (<code className="font-mono">demo1234</code> / <code className="font-mono">admin1234</code>)
                  </p>
                  <div className="mt-2 grid gap-1.5">
                    {DEMO_ACCOUNTS.map((a, i) => (
                      <motion.button
                        key={a.username}
                        type="button"
                        onClick={() => {
                          if (a.disabled) return;
                          setUsername(a.username);
                          setPassword(a.password);
                        }}
                        initial={{ opacity: 0, y: 6 }}
                        animate={{ opacity: 1, y: 0 }}
                        transition={{ delay: 0.1 + i * 0.05 }}
                        aria-disabled={a.disabled}
                        className={
                          "group relative flex items-center justify-between gap-3 overflow-hidden rounded-md border border-white/5 bg-white/[0.03] px-3 py-2 text-left text-xs transition-colors " +
                          (a.disabled
                            ? "cursor-not-allowed opacity-70"
                            : "hover:border-primary/30 hover:bg-white/[0.08]")
                        }
                      >
                        <span
                          aria-hidden
                          className={
                            "pointer-events-none absolute inset-0 bg-gradient-to-r opacity-60 " +
                            a.accent
                          }
                        />
                        <span className="relative flex min-w-0 flex-col">
                          <span className="flex items-center gap-1.5 font-medium">
                            {a.disabled && <Lock className="size-3 text-tier-red" />}
                            {t(a.labelKey)}
                          </span>
                          <span className="truncate text-[10px] text-muted-foreground">
                            {t(a.subKey)}
                          </span>
                        </span>
                        <code className="relative shrink-0 rounded bg-muted/80 px-1.5 py-0.5 font-mono text-[10px] text-muted-foreground">
                          {a.username} / {a.password}
                        </code>
                      </motion.button>
                    ))}
                  </div>
                </div>
              </CardContent>
            </Card>
          </FadeIn>
        </div>
      </div>
    </MarketingShell>
  );
}
