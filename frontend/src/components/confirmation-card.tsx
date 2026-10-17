"use client";

import { useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import {
  AlertTriangle,
  CheckCircle2,
  Loader2,
  Lock,
  Shield,
  Sparkles,
} from "lucide-react";
import { TierBadge } from "@/components/motion/tier-badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { agent, ApiError } from "@/lib/api";
import { useToast } from "@/components/ui/toast";
import { fenToCny, cn } from "@/lib/utils";
import { useAuth } from "@/lib/store";

type Props = {
  pendingActionId: string;
  tool: string;
  parameters: Record<string, unknown>;
  riskTier: "Green" | "Yellow" | "Red";
  workflowId?: string | null;
  onDone?: () => void;
};

const FRIENDLY_PARAM: Record<string, (v: unknown) => string> = {
  amount_fen: (v) => fenToCny(Number(v)),
  amount_cny: (v) => `¥${v}`,
  memo: (v) => String(v ?? "—"),
  account_id: (v) => String(v ?? "—"),
  payee_id: (v) => String(v ?? "—"),
  recipient_query: (v) => String(v ?? "—"),
  phone_tail: (v) => String(v ?? "—"),
  days_before: (v) => `${v} day(s) before`,
  goal_id: (v) => String(v ?? "—"),
  reservation_id: (v) => String(v ?? "—"),
  product_id: (v) => String(v ?? "—"),
  holding_id: (v) => String(v ?? "—"),
  merchant: (v) => String(v ?? "—"),
  schedule: (v) => String(v ?? "—"),
  display_name: (v) => String(v ?? "—"),
  deliver_at: (v) => String(v ?? "—"),
  subscription_id: (v) => String(v ?? "—"),
  subject: (v) => String(v ?? "—"),
  due_at: (v) => String(v ?? "—"),
};

const HIDDEN = new Set(["tier"]);

export function ConfirmationCard({
  pendingActionId,
  tool,
  parameters,
  riskTier,
  workflowId,
  onDone,
}: Props) {
  const { me } = useAuth();
  const { toast } = useToast();
  const [mfaCode, setMfaCode] = useState("");
  const [phase, setPhase] = useState<"idle" | "verifying" | "confirming" | "done">("idle");
  const [result, setResult] = useState<Record<string, unknown> | null>(null);

  const needsMfa = riskTier === "Red";

  const verify = async () => {
    setPhase("verifying");
    try {
      await agent.verify(pendingActionId, mfaCode || "000000");
      setPhase("idle");
      toast({ variant: "success", title: "MFA verified" });
    } catch (err) {
      const msg = err instanceof ApiError ? err.message : "MFA failed";
      toast({ variant: "error", title: "MFA failed", description: msg });
      setPhase("idle");
    }
  };

  const confirm = async () => {
    setPhase("confirming");
    try {
      const res = await agent.confirm(pendingActionId);
      setResult((res.result as Record<string, unknown>) ?? null);
      setPhase("done");
      toast({ variant: "success", title: "Action confirmed", description: `${tool} executed.` });
      onDone?.();
    } catch (err) {
      const msg = err instanceof ApiError ? err.message : "Confirmation failed";
      toast({ variant: "error", title: "Confirmation failed", description: msg });
      setPhase("idle");
    }
  };

  return (
    <motion.div
      layout
      initial={{ opacity: 0, y: 8, scale: 0.98 }}
      animate={{ opacity: 1, y: 0, scale: 1 }}
      transition={{ type: "spring", damping: 22, stiffness: 240 }}
      className={cn(
        "rounded-2xl border bg-card/70 p-5 shadow-[0_24px_60px_-30px_rgba(0,0,0,0.5)] backdrop-blur",
        riskTier === "Red" && "ring-1 ring-tier-red/30",
        riskTier === "Yellow" && "ring-1 ring-tier-yellow/30"
      )}
    >
      <div className="flex items-start justify-between gap-4">
        <div>
          <div className="flex items-center gap-2">
            <TierBadge tier={riskTier} />
            <span className="text-[11px] font-mono uppercase tracking-wider text-muted-foreground">
              {tool}
            </span>
          </div>
          <h3 className="mt-2 font-display text-lg font-semibold tracking-tight">
            Approve this action
          </h3>
          <p className="mt-1 text-xs text-muted-foreground">
            <Shield className="mr-1 inline size-3" />
            Bound to a parameter hash. Any change invalidates this approval.
          </p>
        </div>
        <div className="hidden text-right text-[10px] font-mono uppercase tracking-wider text-muted-foreground md:block">
          <div>action</div>
          <div className="font-medium text-foreground/70">{pendingActionId.slice(0, 16)}…</div>
        </div>
      </div>

      <ul className="mt-4 grid gap-1.5 text-sm">
        {Object.entries(parameters)
          .filter(([k]) => !HIDDEN.has(k))
          .map(([k, v]) => (
            <li
              key={k}
              className="grid grid-cols-[140px_1fr] items-center gap-3 rounded-md border bg-background/30 px-3 py-2"
            >
              <span className="text-[10px] font-mono uppercase tracking-wider text-muted-foreground">
                {k}
              </span>
              <span className="truncate font-medium">
                {FRIENDLY_PARAM[k] ? FRIENDLY_PARAM[k](v) : String(v ?? "—")}
              </span>
            </li>
          ))}
      </ul>

      <AnimatePresence>
        {phase === "done" ? (
          <motion.div
            initial={{ opacity: 0, y: 8 }}
            animate={{ opacity: 1, y: 0 }}
            className="mt-4 flex items-start gap-2 rounded-lg border border-tier-green/30 bg-tier-green/10 p-3 text-sm text-tier-green"
          >
            <CheckCircle2 className="mt-0.5 size-4" />
            <div>
              <div className="font-medium">Action completed</div>
              {result && (
                <pre className="mt-1 overflow-x-auto text-[11px] text-foreground/80">
                  {JSON.stringify(result, null, 2)}
                </pre>
              )}
            </div>
          </motion.div>
        ) : needsMfa ? (
          <motion.div
            key="mfa"
            initial={{ opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: "auto" }}
            exit={{ opacity: 0, height: 0 }}
            className="mt-4 rounded-lg border bg-background/40 p-3"
          >
            <div className="flex items-start gap-2">
              <Lock className="mt-0.5 size-4 text-tier-red" />
              <div className="flex-1">
                <div className="text-sm font-medium">Mock MFA required</div>
                <p className="mt-0.5 text-xs text-muted-foreground">
                  Demo code: <code className="rounded bg-muted px-1.5 py-0.5 font-mono">000000</code>
                </p>
                <div className="mt-3 flex items-end gap-2">
                  <div className="flex-1">
                    <Label htmlFor="mfa">One-time code</Label>
                    <Input
                      id="mfa"
                      inputMode="numeric"
                      pattern="[0-9]*"
                      maxLength={6}
                      value={mfaCode}
                      onChange={(e) => setMfaCode(e.target.value.replace(/\D/g, ""))}
                      placeholder="000000"
                    />
                  </div>
                  <Button
                    variant="outline"
                    onClick={verify}
                    disabled={phase !== "idle" || mfaCode.length !== 6}
                  >
                    {phase === "verifying" ? <Loader2 className="size-4 animate-spin" /> : "Verify"}
                  </Button>
                </div>
              </div>
            </div>
          </motion.div>
        ) : null}
      </AnimatePresence>

      {phase !== "done" && (
        <div className="mt-4 flex items-center justify-end gap-2">
          <Button
            variant="gradient"
            onClick={confirm}
            disabled={
              phase === "confirming" ||
              phase === "verifying" ||
              (needsMfa && false /* mfa checked inside tool flow */)
            }
          >
            {phase === "confirming" ? (
              <Loader2 className="size-4 animate-spin" />
            ) : (
              <Sparkles className="size-4" />
            )}
            Confirm {riskTier === "Green" ? "and run" : needsMfa ? "after MFA" : "with one tap"}
          </Button>
        </div>
      )}

      {me?.safety_lock_state === "locked" && (
        <div className="mt-3 flex items-start gap-2 rounded-md border border-tier-red/30 bg-tier-red/10 p-2 text-xs text-tier-red">
          <AlertTriangle className="mt-0.5 size-3.5" />
          Your account is locked. Contact an admin to unlock.
        </div>
      )}
    </motion.div>
  );
}
