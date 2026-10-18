"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import { motion } from "framer-motion";
import {
  ChevronRight,
  Hourglass,
  ListChecks,
  Loader2,
  Pause,
  PlayCircle,
  RefreshCcw,
} from "lucide-react";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { actions, ApiError, friendlyError, workflows, type PendingWorkflow } from "@/lib/api";
import { useToast } from "@/components/ui/toast";
import { useT } from "@/i18n/LanguageProvider";
import { cn } from "@/lib/utils";

type ResumeResult = {
  workflow_id: string;
  status: string;
  next_pending_action: { id: string; tool: string; expires_at: string } | null;
};

type Props = {
  /** Called when the user taps "Open in chat" so the host can scroll to
   *  the conversation or focus the input. */
  onOpenInChat?: (workflowId: string) => void;
  /** Bump to force a re-fetch (e.g. after the user sends a new agent
   *  message and may have started a fresh workflow). */
  refreshKey?: number;
};

/**
 * Side panel under the chat input that lists the user's pending workflows.
 *
 * - Pulls `workflows.pending()` on mount and whenever the parent bumps
 *   `refreshKey` (we expose this so the chat page can wire it to "agent
 *   just sent a reply").
 * - Per row: short id, status badge, "Resume" (calls
 *   `workflows.resume(id)`), and a contextual "Defer 30 min" / "Open in
 *   chat" pair that appears only after Resume returns a
 *   `next_pending_action`.
 * - Every error toast routes through `friendlyError()` for Layer 5.5.
 */
export function PendingWorkflowsPanel({ onOpenInChat, refreshKey = 0 }: Props) {
  const t = useT();
  const { toast } = useToast();
  const [items, setItems] = useState<PendingWorkflow[]>([]);
  const [loading, setLoading] = useState(false);
  const [busyId, setBusyId] = useState<string | null>(null);
  // Per-workflow resume state so the row can stay open after the user
  // resumes and we still know what to defer/open.
  const [resumed, setResumed] = useState<Record<string, ResumeResult | null>>({});

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const res = await workflows.pending();
      setItems(res.items);
    } catch (err) {
      const f = err instanceof ApiError ? friendlyError(err) : null;
      toast({
        variant: "error",
        title: f?.title ?? t("agent.pending.refresh") + " failed",
        description: f?.body,
      });
    } finally {
      setLoading(false);
    }
  }, [toast, t]);

  useEffect(() => {
    refresh();
  }, [refresh, refreshKey]);

  const resume = useCallback(
    async (id: string) => {
      setBusyId(id);
      try {
        const res = await workflows.resume(id);
        setResumed((r) => ({ ...r, [id]: res }));
        toast({
          variant: "success",
          title: t("agent.pending.resumed"),
          description: t("agent.pending.resumedBody", { id: res.workflow_id.slice(0, 12) }),
        });
      } catch (err) {
        const f = err instanceof ApiError ? friendlyError(err) : null;
        toast({
          variant: "error",
          title: f?.title ?? t("agent.pending.resume") + " failed",
          description: f?.body,
        });
      } finally {
        setBusyId(null);
      }
    },
    [toast, t],
  );

  const defer = useCallback(
    async (workflowId: string, actionId: string) => {
      setBusyId(workflowId);
      try {
        const res = await actions.defer(actionId, 30);
        toast({
          variant: "success",
          title: t("agent.pending.deferred"),
          description: t("agent.pending.deferredBody", {
            expires: new Date(res.new_expires_at).toLocaleString(),
          }),
        });
        // Optimistically mark the resumed state as "no next action" so
        // the defer UI disappears.
        setResumed((r) => ({
          ...r,
          [workflowId]: r[workflowId] ? { ...r[workflowId]!, next_pending_action: null } : null,
        }));
      } catch (err) {
        const f = err instanceof ApiError ? friendlyError(err) : null;
        toast({
          variant: "error",
          title: f?.title ?? t("agent.pending.defer") + " failed",
          description: f?.body,
        });
      } finally {
        setBusyId(null);
      }
    },
    [toast, t],
  );

  const statusVariant = useCallback(
    (s: string): "green" | "yellow" | "muted" => {
      if (s === "active") return "green";
      if (s === "paused") return "yellow";
      return "muted";
    },
    [],
  );

  const statusLabel = useCallback(
    (s: string): string => {
      const key =
        s === "active"
          ? "agent.pending.status.active"
          : s === "paused"
            ? "agent.pending.status.paused"
            : "agent.pending.status.completed";
      return t(key);
    },
    [t],
  );

  const hasItems = useMemo(() => items.length > 0, [items]);

  return (
    <Card className="overflow-hidden">
      <CardContent className="p-0">
        <div className="flex items-center justify-between border-b bg-background/40 px-4 py-3">
          <div className="flex items-center gap-2">
            <span className="grid size-7 place-items-center rounded-md bg-primary/10 text-primary">
              <ListChecks className="size-3.5" />
            </span>
            <div>
              <div className="text-sm font-semibold leading-tight">
                {t("agent.pending.title")}
              </div>
              <div className="text-[11px] text-muted-foreground">
                {t("agent.pending.subtitle")}
              </div>
            </div>
          </div>
          <div className="flex items-center gap-2">
            {loading && <Loader2 className="size-3.5 animate-spin text-muted-foreground" />}
            <Badge variant="muted">{items.length}</Badge>
            <Button
              size="sm"
              variant="ghost"
              onClick={refresh}
              disabled={loading}
              title={t("agent.pending.refresh")}
            >
              <RefreshCcw className="size-3.5" />
            </Button>
          </div>
        </div>

        {!hasItems ? (
          <div className="px-4 py-6 text-center text-[12px] text-muted-foreground">
            {t("agent.pending.empty")}
          </div>
        ) : (
          <ul className="divide-y">
            {items.map((w) => {
              const resumedState = resumed[w.workflow_id];
              const next = resumedState?.next_pending_action ?? null;
              return (
                <li key={w.workflow_id} className="px-4 py-3">
                  <div className="flex flex-wrap items-center justify-between gap-2">
                    <div className="min-w-0">
                      <div className="flex flex-wrap items-center gap-2">
                        <code className="rounded bg-muted px-1.5 py-0.5 font-mono text-[10px] text-muted-foreground">
                          {w.workflow_id.slice(0, 16)}…
                        </code>
                        <Badge variant={statusVariant(w.status)}>{statusLabel(w.status)}</Badge>
                        <span className="text-[11px] text-muted-foreground">
                          {new Date(w.created_at).toLocaleString()}
                        </span>
                      </div>
                      <div className="mt-1 text-[12px] text-muted-foreground">
                        {w.title}
                      </div>
                      {next && (
                        <div
                          className={cn(
                            "mt-2 flex items-center gap-1.5 rounded-md border p-2 text-[11px]",
                            "border-primary/30 bg-primary/[0.04] text-foreground/80",
                          )}
                        >
                          <Hourglass className="size-3 shrink-0 text-primary" />
                          <span>{t("agent.pending.nextAction", { tool: next.tool })}</span>
                        </div>
                      )}
                    </div>
                    <div className="flex flex-wrap items-center gap-1.5">
                      <Button
                        size="sm"
                        variant="outline"
                        onClick={() => resume(w.workflow_id)}
                        disabled={busyId === w.workflow_id}
                      >
                        {busyId === w.workflow_id ? (
                          <Loader2 className="size-3.5 animate-spin" />
                        ) : (
                          <PlayCircle className="size-3.5" />
                        )}
                        {t("agent.pending.resume")}
                      </Button>
                      {next && (
                        <>
                          <Button
                            size="sm"
                            variant="ghost"
                            onClick={() => defer(w.workflow_id, next.id)}
                            disabled={busyId === w.workflow_id}
                          >
                            <Pause className="size-3.5" />
                            {t("agent.pending.defer")}
                          </Button>
                          <Button
                            size="sm"
                            variant="gradient"
                            onClick={() => onOpenInChat?.(w.workflow_id)}
                          >
                            <ChevronRight className="size-3.5" />
                            {t("agent.pending.openChat")}
                          </Button>
                        </>
                      )}
                    </div>
                  </div>
                </li>
              );
            })}
          </ul>
        )}
      </CardContent>
    </Card>
  );
}
