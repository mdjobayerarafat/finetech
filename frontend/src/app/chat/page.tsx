"use client";

import { useEffect, useRef, useState } from "react";
import { motion } from "framer-motion";
import {
  Loader2,
  Send,
  Sparkles,
  User2,
  ShieldCheck,
  AlertTriangle,
  Download,
  FileText,
} from "lucide-react";
import { AppShell } from "@/components/layout/app-shell";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { Card, CardContent } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import {
  agent,
  ApiError,
  type AgentMessageOut,
  type AgentReadResult,
} from "@/lib/api";
import { useAuth } from "@/lib/store";
import { TierBadge } from "@/components/motion/tier-badge";
import { ConfirmationCard } from "@/components/confirmation-card";
import { FadeIn, AuroraBackdrop } from "@/components/motion/aurora";
import { useLanguage, useT } from "@/i18n/LanguageProvider";
import { useTranslated } from "@/i18n/use-translate";
import { cn, fenToCny } from "@/lib/utils";

type ChatMessage = {
  id: string;
  role: "user" | "assistant";
  text: string;
  ts: number;
  plan?: AgentMessageOut;
};

// Suggestion prompts are dictionary keys, not raw text. We resolve them
// through `t()` at render time so a single source of truth (the
// dictionaries file) drives both languages.
const SUGGESTIONS_EN = [
  "chat.suggestion.pay",
  "chat.suggestion.spending",
  "chat.suggestion.cancel",
  "chat.suggestion.list",
] as const;
const SUGGESTIONS_ZH = [
  "chat.suggestion.pay.zh",
  "chat.suggestion.spending.zh",
  "chat.suggestion.cancel.zh",
  "chat.suggestion.list.zh",
] as const;

export default function ChatPage() {
  return (
    <AppShell>
      <ChatBody />
    </AppShell>
  );
}

function ChatBody() {
  const { me } = useAuth();
  const { lang } = useLanguage();
  const t = useT();
  // The server should always answer in the user's selected language, so we
  // forward `lang` to every chat call. The backend's stub / LLM then uses
  // that hint to decide the response language.
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [input, setInput] = useState("");
  const [busy, setBusy] = useState(false);
  const scrollRef = useRef<HTMLDivElement>(null);
  const taRef = useRef<HTMLTextAreaElement>(null);

  // Re-greet when the user switches languages so the assistant's first
  // turn is in the language they just selected.
  useEffect(() => {
    if (me) {
      setMessages([
        {
          id: "welcome-" + lang,
          role: "assistant",
          text: t("chat.greeting", { name: me.display_name }),
          ts: Date.now(),
        },
      ]);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [lang, me?.id]);

  useEffect(() => {
    scrollRef.current?.scrollTo({
      top: scrollRef.current.scrollHeight,
      behavior: "smooth",
    });
  }, [messages]);

  const send = async (text: string) => {
    const trimmed = text.trim();
    if (!trimmed || busy) return;
    setInput("");
    const userMsg: ChatMessage = {
      id: crypto.randomUUID(),
      role: "user",
      text: trimmed,
      ts: Date.now(),
    };
    setMessages((m) => [...m, userMsg]);
    setBusy(true);
    try {
      const res = await agent.message(trimmed, lang);
      const assistantMsg: ChatMessage = {
        id: crypto.randomUUID(),
        role: "assistant",
        text: assistantText(res, t),
        ts: Date.now(),
        plan: res,
      };
      setMessages((m) => [...m, assistantMsg]);
    } catch (err) {
      const msg = err instanceof ApiError ? err.message : t("error.network");
      setMessages((m) => [
        ...m,
        {
          id: crypto.randomUUID(),
          role: "assistant",
          text: `⚠️ ${msg}`,
          ts: Date.now(),
        },
      ]);
    } finally {
      setBusy(false);
      requestAnimationFrame(() => taRef.current?.focus());
    }
  };

  const onSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    send(input);
  };

  const onKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      send(input);
    }
  };

  const onActionDone = () => {
    // re-fetch the latest plan result on next send; no-op here.
  };

  return (
    <div className="space-y-4">
      <AuroraBackdrop />
      <FadeIn>
        <div>
          <p className="text-[11px] font-medium uppercase tracking-widest text-muted-foreground">
            {t("chat.eyebrow")}
          </p>
          <h1 className="mt-1 font-display text-3xl font-semibold tracking-tight">
            {t("chat.title")}
          </h1>
          <p className="mt-1 max-w-2xl text-sm text-muted-foreground">
            {t("chat.subtitle")}
          </p>
        </div>
      </FadeIn>

      <Card className="overflow-hidden">
        <div
          ref={scrollRef}
          className="h-[520px] overflow-y-auto px-4 py-6 sm:px-6"
        >
          <ul className="space-y-4">
            {messages.map((m) => (
              <li
                key={m.id}
                className={cn(
                  "flex w-full gap-3",
                  m.role === "user" ? "justify-end" : "justify-start"
                )}
              >
                {m.role === "assistant" && (
                  <div className="grid size-8 shrink-0 place-items-center rounded-full bg-[linear-gradient(135deg,hsl(263_80%_60%),hsl(199_89%_60%))] text-white shadow">
                    <Sparkles className="size-3.5" />
                  </div>
                )}
                <div
                  className={cn(
                    "max-w-[80%] space-y-3",
                    m.role === "user" && "text-right"
                  )}
                >
                  <motion.div
                    initial={{ opacity: 0, y: 6 }}
                    animate={{ opacity: 1, y: 0 }}
                    transition={{ duration: 0.3 }}
                    className={cn(
                      "inline-block rounded-2xl px-4 py-2.5 text-sm leading-relaxed",
                      m.role === "user"
                        ? "bg-primary text-primary-foreground"
                        : "border bg-card/60"
                    )}
                  >
                    {m.text}
                  </motion.div>

                  {m.plan && (
                    <PlanSummary plan={m.plan} onActionDone={onActionDone} />
                  )}

                  {m.plan?.statement_url && (
                    <StatementDownloadCard
                      url={m.plan.statement_url}
                      filename={m.plan.statement_filename}
                    />
                  )}
                </div>
                {m.role === "user" && (
                  <div className="grid size-8 shrink-0 place-items-center rounded-full bg-muted text-foreground/80">
                    <User2 className="size-3.5" />
                  </div>
                )}
              </li>
            ))}
            {busy && (
              <li className="flex items-center gap-3">
                <div className="grid size-8 place-items-center rounded-full bg-[linear-gradient(135deg,hsl(263_80%_60%),hsl(199_89%_60%))] text-white shadow">
                  <Sparkles className="size-3.5" />
                </div>
                <div className="inline-flex items-center gap-2 rounded-2xl border bg-card/60 px-4 py-2.5 text-sm text-muted-foreground">
                  <Loader2 className="size-3.5 animate-spin" />
                  {t("common.thinking")}
                </div>
              </li>
            )}
          </ul>
        </div>

        <div className="border-t bg-background/40 p-3 sm:p-4">
          <div className="mb-2 flex flex-wrap gap-1.5">
            {(lang === "zh" ? SUGGESTIONS_ZH : SUGGESTIONS_EN).map((k) => (
              <button
                key={k}
                onClick={() => send(t(k))}
                disabled={busy}
                className="rounded-full border bg-background/40 px-3 py-1 text-xs text-muted-foreground transition-colors hover:bg-accent hover:text-foreground disabled:opacity-50"
              >
                {t(k)}
              </button>
            ))}
          </div>
          <form onSubmit={onSubmit} className="flex items-end gap-2">
            <Textarea
              ref={taRef}
              value={input}
              onChange={(e) => setInput(e.target.value)}
              onKeyDown={onKeyDown}
              placeholder={
                lang === "zh" ? t("chat.placeholder.zh") : t("chat.placeholder")
              }
              rows={1}
              className="min-h-[44px] flex-1 resize-none"
              disabled={busy}
            />
            <Button
              type="submit"
              variant="gradient"
              size="lg"
              disabled={busy || !input.trim()}
            >
              {busy ? (
                <Loader2 className="size-4 animate-spin" />
              ) : (
                <Send className="size-4" />
              )}
              <span className="hidden sm:inline">{t("common.send")}</span>
            </Button>
          </form>
        </div>
      </Card>

      <p className="text-center text-[11px] text-muted-foreground">
        <ShieldCheck className="mr-1 inline size-3" />
        {t("chat.disclaimer")}
      </p>
    </div>
  );
}

// Map a backend intent to a localized summary line. We do this on the
// client (instead of relying on the LLM to translate) so the message
// always matches the UI language even when the LLM is offline and the
// stub proposes only structured actions.
function assistantText(
  res: AgentMessageOut,
  t: (k: never, vars?: Record<string, string | number>) => string
): string {
  if (res.clarification_required) {
    if (res.questions.length) {
      return t("chat.clarifyHint" as never, {
        questions: res.questions.join(" / "),
      });
    }
    return t("chat.clarifyFallback" as never);
  }
  if (res.intent === "transfer") {
    const amt = (res.entities as { amount_fen?: number; amount_cny?: string | number })
      .amount_fen;
    const cny = (res.entities as { amount_cny?: string | number }).amount_cny;
    const who = (res.entities as { recipient_query?: string }).recipient_query;
    const a =
      amt != null ? fenToCny(amt) : cny != null ? `¥${cny}` : "";
    if (a && who) return t("chat.intent.transfer" as never, { amount: a, who });
    if (a) return t("chat.intent.transfer.noWho" as never, { amount: a });
    if (who) return t("chat.intent.transfer.noAmount" as never, { who });
    return t("chat.intent.transfer.bare" as never);
  }
  const key = (
    {
      report_spending: "chat.intent.report_spending",
      list_subscriptions: "chat.intent.list_subscriptions",
      cancel_subscription: "chat.intent.cancel_subscription",
      plan_birthday: "chat.intent.plan_birthday",
      balance: "chat.intent.balance",
      list_accounts: "chat.intent.list_accounts",
      list_cards: "chat.intent.list_cards",
      wealth_summary: "chat.intent.wealth_summary",
      recent_transactions: "chat.intent.recent_transactions",
    } as Record<string, string>
  )[res.intent];
  if (key) return t(key as never);
  return t("chat.intent.unknown" as never);
}

function PlanSummary({
  plan,
  onActionDone,
}: {
  plan: AgentMessageOut;
  onActionDone: () => void;
}) {
  const t = useT();
  if (plan.clarification_required) {
    return (
      <Card className="text-left">
        <CardContent className="space-y-2 p-4 text-sm">
          <div className="flex items-center gap-2 text-tier-yellow">
            <Badge variant="yellow">clarify</Badge>
            <span>{t("chat.clarify")}</span>
          </div>
          {plan.questions.length > 0 && (
            <ul className="list-disc pl-5 text-muted-foreground">
              {plan.questions.map((q) => (
                <li key={q}>{q}</li>
              ))}
            </ul>
          )}
        </CardContent>
      </Card>
    );
  }

  return (
    <div className="space-y-2 text-left">
      <div className="flex flex-wrap items-center gap-2">
        <Badge variant="muted">{plan.intent}</Badge>
        <Badge variant="muted">{plan.language}</Badge>
        {plan.plan.workflow_id && (
          <code className="rounded bg-muted px-1.5 py-0.5 font-mono text-[10px] text-muted-foreground">
            {plan.plan.workflow_id.slice(0, 16)}…
          </code>
        )}
      </div>

      {plan.read_results && plan.read_results.length > 0 && (
        <ReadResultsView results={plan.read_results} />
      )}

      {plan.plan.nodes.length > 0 && (
        <ul className="space-y-2">
          {plan.plan.nodes.map((n) => (
            <li key={n.id} className="rounded-lg border bg-background/30 p-3">
              <div className="flex flex-wrap items-center justify-between gap-2">
                <div className="flex items-center gap-2">
                  <TierBadge tier={n.risk_tier} size="sm" />
                  <code className="text-[11px] font-mono uppercase tracking-wider text-muted-foreground">
                    {n.tool}
                  </code>
                </div>
                {n.approval_required && (
                  <Badge variant="muted">{t("chat.approvalRequired")}</Badge>
                )}
              </div>
              {n.risk_tier === "Green" ? (
                <div className="mt-2 text-xs text-muted-foreground">
                  {t("chat.readOnly")}
                </div>
              ) : null}
              {n.fraud_assessment && (
                <FraudWarning assessment={n.fraud_assessment} />
              )}
            </li>
          ))}
        </ul>
      )}

      {plan.pending_action && (
        <ConfirmationCard
          pendingActionId={plan.pending_action.id}
          tool={plan.pending_action.tool}
          parameters={
            plan.plan.nodes.find(
              (n) => n.pending_action_id === plan.pending_action!.id
            )?.parameters ?? {}
          }
          riskTier={plan.pending_action.risk_tier as "Green" | "Yellow" | "Red"}
          workflowId={plan.plan.workflow_id}
          onDone={onActionDone}
        />
      )}
    </div>
  );
}

function FraudWarning({
  assessment,
}: {
  assessment: {
    flags: { code: string; message: string; weight: number }[];
    score: number;
    narrative: string | null;
    templated: string;
  };
}) {
  const t = useT();
  const source = assessment.narrative ?? assessment.templated;
  const { translated, isTranslating } = useTranslated(source);
  if (!assessment.flags || assessment.flags.length === 0) return null;
  const hard = assessment.flags.some(
    (f) =>
      f.code === "urgency_words" ||
      f.code === "payee_flagged" ||
      f.weight >= 5
  );
  const count = assessment.flags.length;
  return (
    <div
      className={cn(
        "mt-2 rounded-md border p-3 text-xs",
        hard
          ? "border-tier-red/40 bg-tier-red/[0.06] text-tier-red"
          : "border-tier-yellow/40 bg-tier-yellow/[0.06] text-tier-yellow"
      )}
    >
      <div className="flex items-center gap-2">
        <AlertTriangle className="size-3.5" />
        <span className="font-semibold">
          {t("chat.scamGuard")} ·{" "}
          {count === 1
            ? t("chat.scamGuardSignals" as never, { count })
            : t("chat.scamGuardSignalsPlural" as never, { count })}{" "}
          · {t("chat.scamGuardScore" as never, { score: assessment.score })}
        </span>
        {isTranslating && (
          <Loader2 className="size-3 animate-spin opacity-70" />
        )}
      </div>
      <p className="mt-1 leading-relaxed text-foreground/80">{translated}</p>
      <ul className="mt-2 list-disc pl-4 text-foreground/70">
        {assessment.flags.map((f) => (
          <li key={f.code}>
            <code className="font-mono text-[10px]">{f.code}</code> · {f.message}
          </li>
        ))}
      </ul>
      <p className="mt-2 text-[10px] uppercase tracking-wider text-muted-foreground">
        {t("chat.scamGuardNote")}
      </p>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Read-results renderer — shows the data the backend fetched inline.
// Tool names are mapped to friendly cards; arbitrary results fall back to JSON.
// ---------------------------------------------------------------------------

function ReadResultsView({ results }: { results: AgentReadResult[] }) {
  return (
    <div className="space-y-2">
      {results.map((r, i) => (
        <ReadCard key={`${r.tool}-${i}`} result={r} />
      ))}
    </div>
  );
}

function ReadCard({ result }: { result: AgentReadResult }) {
  const t = useT();
  if (result.error) {
    return (
      <div className="rounded-lg border border-tier-red/30 bg-tier-red/[0.05] p-3 text-sm">
        <div className="flex items-center gap-2 text-tier-red">
          <code className="font-mono text-[11px]">{result.tool}</code>
          <span className="text-xs">{t("chat.readFailed")}</span>
        </div>
        <p className="mt-1 text-xs">{result.error}</p>
      </div>
    );
  }
  const data = result.result ?? {};
  switch (result.tool) {
    case "subscriptions.list":
      return <SubscriptionsCard data={data} />;
    case "accounts.list":
      return <AccountsCard data={data} />;
    case "accounts.balance_summary":
      return <BalanceCard data={data} />;
    case "spending.report":
      return <SpendingCard data={data} />;
    case "transactions.recent":
      return <TransactionsCard data={data} />;
    case "cards.list":
      return <CardsCard data={data} />;
    case "payees.list":
      return <PayeesCard data={data} />;
    case "wealth.summary":
      return <WealthCard data={data} />;
    case "user.profile":
      return <ProfileCard data={data} />;
    default:
      return (
        <details className="rounded-lg border bg-background/30 p-3 text-sm">
          <summary className="cursor-pointer text-xs text-muted-foreground">
            <code className="font-mono text-[11px]">{result.tool}</code>
          </summary>
          <pre className="mt-2 max-h-72 overflow-auto rounded bg-muted/50 p-2 text-[10px]">
            {JSON.stringify(data, null, 2)}
          </pre>
        </details>
      );
  }
}

function SubscriptionsCard({ data }: { data: Record<string, unknown> }) {
  const t = useT();
  const subs = (data.subscriptions as Array<Record<string, unknown>>) ?? [];
  const monthly = Number(data.monthly_total_fen ?? 0);
  const count = Number(data.active_count ?? subs.length);
  return (
    <div className="rounded-lg border bg-background/30 p-3 text-sm">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <span className="font-medium">{t("read.subscriptions.title")}</span>
        </div>
        <div className="flex items-center gap-2 text-xs text-muted-foreground">
          <Badge variant="muted">
            {t("read.subscriptions.active", { count })}
          </Badge>
          {monthly > 0 && (
            <span>
              {t("read.subscriptions.monthly", { amount: fenToCny(monthly) })}
            </span>
          )}
        </div>
      </div>
      {subs.length === 0 ? (
        <p className="mt-2 text-xs text-muted-foreground">
          {t("read.subscriptions.empty")}
        </p>
      ) : (
        <ul className="mt-2 divide-y">
          {subs.map((s, i) => (
            <li
              key={String(s.id ?? i)}
              className="flex items-center justify-between py-2 text-xs"
            >
              <div>
                <div className="font-medium">{String(s.merchant ?? "—")}</div>
                <div className="text-[10px] text-muted-foreground">
                  every {String(s.interval_days ?? "?")}d ·{" "}
                  {String(s.cancellation_status ?? "active")}
                </div>
              </div>
              <div className="text-right">
                <div className="font-mono font-semibold">
                  {fenToCny(Number(s.estimated_amount_fen ?? 0))}
                </div>
              </div>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

function AccountsCard({ data }: { data: Record<string, unknown> }) {
  const t = useT();
  const accts = (data.accounts as Array<Record<string, unknown>>) ?? [];
  return (
    <div className="rounded-lg border bg-background/30 p-3 text-sm">
      <div className="flex items-center gap-2">
        <Sparkles className="size-3.5 text-primary" />
        <span className="font-medium">{t("read.accounts.title")}</span>
      </div>
      {accts.length === 0 ? (
        <p className="mt-2 text-xs text-muted-foreground">
          {t("read.accounts.empty")}
        </p>
      ) : (
        <ul className="mt-2 space-y-1.5">
          {accts.map((a, i) => (
            <li
              key={String(a.id ?? i)}
              className="flex items-center justify-between text-xs"
            >
              <div>
                <div className="font-medium">
                  {String(a.display_name ?? "—")}
                </div>
                <div className="text-[10px] font-mono text-muted-foreground">
                  {String(a.masked_number ?? "—")}
                </div>
              </div>
              <div className="text-right">
                <div className="font-mono font-semibold">
                  {fenToCny(Number(a.available_balance_fen ?? 0))}
                </div>
                {Number(a.reserved_balance_fen ?? 0) > 0 && (
                  <div className="text-[10px] text-muted-foreground">
                    {t("read.accounts.reserved", {
                      amount: fenToCny(Number(a.reserved_balance_fen ?? 0)),
                    })}
                  </div>
                )}
              </div>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

function BalanceCard({ data }: { data: Record<string, unknown> }) {
  const t = useT();
  return (
    <div className="rounded-lg border bg-background/30 p-3 text-sm">
      <div className="flex items-center gap-2">
        <span className="font-medium">{t("read.balance.title")}</span>
      </div>
      <div className="mt-2 font-display text-2xl font-semibold tracking-tight">
        {fenToCny(Number(data.total_fen ?? 0))}{" "}
        <span className="text-xs text-muted-foreground">
          {String(data.currency ?? "")}
        </span>
      </div>
      <div className="mt-1 text-[11px] text-muted-foreground">
        {t("read.balance.available", {
          amount: fenToCny(Number(data.available_balance_fen ?? 0)),
        })}{" "}
        ·{" "}
        {t("read.balance.reserved", {
          amount: fenToCny(Number(data.reserved_balance_fen ?? 0)),
        })}{" "}
        ·{" "}
        {t("read.balance.count", {
          count: String(data.account_count ?? 0),
        })}
      </div>
    </div>
  );
}

function SpendingCard({ data }: { data: Record<string, unknown> }) {
  const t = useT();
  const cats =
    (data.categories as Array<{ category: string; amount_fen: number }>) ?? [];
  const total = Number(data.total_spend_fen ?? 0);
  return (
    <div className="rounded-lg border bg-background/30 p-3 text-sm">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <span className="font-medium">
            {t("read.spending.title", { days: String(data.days ?? 30) })}
          </span>
        </div>
        <div className="font-mono font-semibold">{fenToCny(total)}</div>
      </div>
      {cats.length > 0 && (
        <ul className="mt-2 space-y-1">
          {cats.slice(0, 6).map((c) => (
            <li
              key={c.category}
              className="flex items-center justify-between text-xs"
            >
              <span className="capitalize text-muted-foreground">
                {c.category}
              </span>
              <span className="font-mono">{fenToCny(c.amount_fen)}</span>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

function TransactionsCard({ data }: { data: Record<string, unknown> }) {
  const t = useT();
  const txs = (data.transactions as Array<Record<string, unknown>>) ?? [];
  return (
    <div className="rounded-lg border bg-background/30 p-3 text-sm">
      <div className="flex items-center gap-2">
        <span className="font-medium">{t("read.transactions.title")}</span>
      </div>
      {txs.length === 0 ? (
        <p className="mt-2 text-xs text-muted-foreground">
          {t("read.transactions.empty")}
        </p>
      ) : (
        <ul className="mt-2 divide-y">
          {txs.slice(0, 10).map((tx, i) => (
            <li
              key={String(tx.id ?? i)}
              className="flex items-center justify-between py-1.5 text-xs"
            >
              <div>
                <div className="font-medium">{String(tx.merchant ?? "—")}</div>
                <div className="text-[10px] text-muted-foreground">
                  {String(tx.category ?? "")}
                </div>
              </div>
              <div
                className={cn(
                  "font-mono font-semibold",
                  tx.direction === "credit"
                    ? "text-tier-green"
                    : "text-foreground"
                )}
              >
                {tx.direction === "credit" ? "+" : "−"}
                {fenToCny(Math.abs(Number(tx.amount_fen ?? 0)))}
              </div>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

function CardsCard({ data }: { data: Record<string, unknown> }) {
  const t = useT();
  const cards = (data.cards as Array<Record<string, unknown>>) ?? [];
  return (
    <div className="rounded-lg border bg-background/30 p-3 text-sm">
      <div className="flex items-center gap-2">
        <span className="font-medium">{t("read.cards.title")}</span>
      </div>
      {cards.length === 0 ? (
        <p className="mt-2 text-xs text-muted-foreground">
          {t("read.cards.empty")}
        </p>
      ) : (
        <ul className="mt-2 space-y-1.5">
          {cards.map((c, i) => (
            <li
              key={String(c.id ?? i)}
              className="flex items-center justify-between text-xs"
            >
              <div>
                <div className="font-medium">
                  {String(c.display_name ?? "—")}
                </div>
                <div className="text-[10px] font-mono text-muted-foreground">
                  {String(c.masked_number ?? "—")}
                </div>
              </div>
              <Badge
                variant={
                  c.status === "active"
                    ? "green"
                    : c.status === "frozen"
                    ? "yellow"
                    : "muted"
                }
              >
                {String(c.status ?? "—")}
              </Badge>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

function PayeesCard({ data }: { data: Record<string, unknown> }) {
  const t = useT();
  const list = (data.payees as Array<Record<string, unknown>>) ?? [];
  return (
    <div className="rounded-lg border bg-background/30 p-3 text-sm">
      <div className="flex items-center gap-2">
        <span className="font-medium">{t("read.payees.title")}</span>
      </div>
      {list.length === 0 ? (
        <p className="mt-2 text-xs text-muted-foreground">
          {t("read.payees.empty")}
        </p>
      ) : (
        <ul className="mt-2 grid grid-cols-1 gap-1.5 sm:grid-cols-2">
          {list.map((p, i) => (
            <li
              key={String(p.id ?? i)}
              className="flex items-center justify-between rounded border border-white/5 bg-white/[0.03] px-2 py-1 text-xs"
            >
              <span className="font-medium">
                {String(p.display_name ?? "—")}
              </span>
              {p.verified ? (
                <Badge variant="green">{t("read.payees.verified")}</Badge>
              ) : (
                <Badge variant="yellow">{t("read.payees.unverified")}</Badge>
              )}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

function WealthCard({ data }: { data: Record<string, unknown> }) {
  const t = useT();
  const holdings = (data.holdings as Array<Record<string, unknown>>) ?? [];
  const total = Number(data.total_fen ?? 0);
  return (
    <div className="rounded-lg border bg-background/30 p-3 text-sm">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <span className="font-medium">{t("read.wealth.title")}</span>
        </div>
        <div className="font-mono font-semibold">{fenToCny(total)}</div>
      </div>
      {holdings.length === 0 ? (
        <p className="mt-2 text-xs text-muted-foreground">
          {t("read.wealth.empty")}
        </p>
      ) : (
        <ul className="mt-2 space-y-1.5">
          {holdings.map((h, i) => {
            const product = h.product as Record<string, unknown> | null;
            return (
              <li
                key={String(h.id ?? i)}
                className="flex items-center justify-between text-xs"
              >
                <div>
                  <div className="font-medium">
                    {String(product?.name ?? "—")}
                  </div>
                  <div className="text-[10px] text-muted-foreground">
                    {String(product?.category ?? "")} ·{" "}
                    {t("read.wealth.risk" as never, {
                      level: String(product?.risk_level ?? ""),
                    })}
                  </div>
                </div>
                <div className="font-mono">
                  {fenToCny(Number(h.amount_fen ?? 0))}
                </div>
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}

function ProfileCard({ data }: { data: Record<string, unknown> }) {
  const t = useT();
  return (
    <div className="rounded-lg border bg-background/30 p-3 text-sm">
      <div className="flex items-center gap-2">
        <span className="font-medium">{t("read.profile.title")}</span>
      </div>
      <dl className="mt-2 grid grid-cols-2 gap-x-3 gap-y-1 text-xs">
        <dt className="text-muted-foreground">{t("read.profile.name")}</dt>
        <dd>{String(data.display_name ?? "—")}</dd>
        <dt className="text-muted-foreground">{t("read.profile.language")}</dt>
        <dd>{String(data.preferred_language ?? "—")}</dd>
        <dt className="text-muted-foreground">{t("read.profile.risk")}</dt>
        <dd>{String(data.risk_tolerance ?? "—")}</dd>
        <dt className="text-muted-foreground">{t("read.profile.horizon")}</dt>
        <dd>{String(data.horizon ?? "—")}</dd>
        <dt className="text-muted-foreground">{t("read.profile.safety")}</dt>
        <dd>{String(data.safety_lock_state ?? "—")}</dd>
      </dl>
    </div>
  );
}

/**
 * Renders the "Download your statement" card under an assistant turn.
 * The backend already attaches `Content-Disposition: attachment`, so a
 * plain `<a download>` click triggers the browser's save dialog. We
 * include `download={filename}` as a stronger hint for cross-browser
 * compatibility (Firefox honours it; Chrome infers from the header).
 */
function StatementDownloadCard({
  url,
  filename,
}: {
  url: string;
  filename: string | null | undefined;
}) {
  const t = useT();
  // Backend path is "/api/v1/reports/statement?..." — relative, so it goes
  // through the Next.js proxy (`/api/v1/*` rewrites) and reuses the
  // HttpOnly session cookie.
  const href =
    url.startsWith("/api/") || url.startsWith("http")
      ? url
      : `/api/v1${url}`;
  const name = filename ?? href.split("/").pop() ?? "statement";
  return (
    <Card className="text-left">
      <CardContent className="flex items-start gap-3 p-4 text-sm">
        <div className="grid size-10 shrink-0 place-items-center rounded-lg bg-tier-green/15 text-tier-green">
          <FileText className="size-5" />
        </div>
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-2">
            <Badge variant="green">{t("chat.statement.title")}</Badge>
            <code className="rounded bg-muted px-1.5 py-0.5 font-mono text-[10px] text-muted-foreground">
              {name}
            </code>
          </div>
          <p className="mt-1 text-muted-foreground">
            {t("chat.statement.body")}
          </p>
          <a
            href={href}
            download={name}
            className={cn(
              "mt-3 inline-flex items-center gap-2 rounded-md border border-tier-green/40 bg-tier-green/10 px-3 py-1.5",
              "text-xs font-medium text-tier-green hover:bg-tier-green/20"
            )}
          >
            <Download className="size-3.5" />
            {t("common.download")} {name}
          </a>
        </div>
      </CardContent>
    </Card>
  );
}
