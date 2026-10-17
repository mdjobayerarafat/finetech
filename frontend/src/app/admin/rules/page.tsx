"use client";

import { useEffect, useMemo, useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import {
  ShieldCheck,
  Plus,
  Edit2,
  Trash2,
  Loader2,
  X,
  AlertTriangle,
  Lock,
  PowerOff,
  Power,
  Eye,
  EyeOff,
  Beaker,
} from "lucide-react";
import { AdminShell } from "@/components/admin/admin-shell";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Label } from "@/components/ui/label";
import { useToast } from "@/components/ui/toast";
import { agentRulesApi, type AgentRule, type SystemPromptPreview } from "@/lib/api";
import { FadeIn, Stagger, StaggerItem } from "@/components/motion/aurora";
import { cn, formatDateTime } from "@/lib/utils";

const KIND_LABELS: Record<AgentRule["kind"], { label: string; desc: string; needsInt?: boolean }> = {
  forbid_tool: {
    label: "Forbid tool",
    desc: "Remove a tool from the agent's allowlist. Severity must be block.",
  },
  restrict_payee: {
    label: "Restrict payee",
    desc: "Block transfers to a payee whose name or phone tail matches the pattern.",
  },
  restrict_memo: {
    label: "Restrict memo",
    desc: "Block transfers whose memo text contains the pattern (case-insensitive).",
  },
  restrict_amount_fen: {
    label: "Restrict single amount",
    desc: "Block any single transfer whose amount exceeds the cap (pattern = integer fen).",
    needsInt: true,
  },
  restrict_daily_fen: {
    label: "Restrict daily total",
    desc: "Block when today's transfers + this one would exceed the cap.",
    needsInt: true,
  },
  system_prompt_prefix: {
    label: "System prompt prefix",
    desc: "Inject the pattern text into the LLM system prompt as an admin instruction.",
  },
};

export default function AdminRulesPage() {
  return (
    <AdminShell>
      <Body />
    </AdminShell>
  );
}

function Body() {
  const [rules, setRules] = useState<AgentRule[]>([]);
  const [busy, setBusy] = useState(false);
  const [editing, setEditing] = useState<AgentRule | null>(null);
  const [creating, setCreating] = useState(false);
  const [preview, setPreview] = useState<SystemPromptPreview | null>(null);
  const [showPreview, setShowPreview] = useState(false);
  const { toast } = useToast();

  const refresh = async () => {
    setBusy(true);
    try {
      const r = await agentRulesApi.list();
      setRules(r.rules);
    } catch (err) {
      const msg = err instanceof Error ? err.message : "Failed to load";
      toast({ variant: "error", title: "Failed to load rules", description: msg });
    } finally {
      setBusy(false);
    }
  };

  useEffect(() => {
    refresh();
  }, []);

  const toggle = async (rule: AgentRule) => {
    try {
      await agentRulesApi.update(rule.id, { enabled: !rule.enabled });
      toast({ variant: "success", title: rule.enabled ? "Rule disabled" : "Rule enabled" });
      refresh();
    } catch (err) {
      const msg = err instanceof Error ? err.message : "Update failed";
      toast({ variant: "error", title: "Failed to update rule", description: msg });
    }
  };

  const remove = async (rule: AgentRule) => {
    if (!confirm(`Delete rule "${rule.name}"? This cannot be undone.`)) return;
    try {
      await agentRulesApi.remove(rule.id);
      toast({ variant: "success", title: "Rule deleted" });
      refresh();
    } catch (err) {
      const msg = err instanceof Error ? err.message : "Delete failed";
      toast({ variant: "error", title: "Failed to delete rule", description: msg });
    }
  };

  const counts = useMemo(() => {
    const enabled = rules.filter((r) => r.enabled);
    return {
      total: rules.length,
      enabled: enabled.length,
      blocking: enabled.filter((r) => r.severity === "block").length,
      warns: enabled.filter((r) => r.severity === "warn").length,
      forbiddenTools: enabled.filter((r) => r.kind === "forbid_tool").length,
      promptPrefixes: enabled.filter((r) => r.kind === "system_prompt_prefix").length,
    };
  }, [rules]);

  const openPreview = async (user_id?: string) => {
    try {
      setPreview(await agentRulesApi.systemPromptPreview(user_id));
      setShowPreview(true);
    } catch {
      toast({ variant: "error", title: "Failed to load preview" });
    }
  };

  return (
    <div className="space-y-6">
      <FadeIn>
        <div className="flex flex-wrap items-end justify-between gap-3">
          <div>
            <p className="text-[11px] font-medium uppercase tracking-widest text-muted-foreground">
              Admin · agent rules
            </p>
            <h1 className="mt-1 font-display text-3xl font-semibold tracking-tight">
              Rules engine
            </h1>
            <p className="mt-1 max-w-2xl text-sm text-muted-foreground">
              Configure what the agent is allowed to do. Rules are evaluated
              server-side in the planner, so they are enforced even if the LLM
              is tricked. Every match is recorded in the audit log.
            </p>
          </div>
          <div className="flex flex-wrap items-center gap-2">
            <Button variant="outline" onClick={() => openPreview()}>
              <Beaker className="size-4" />
              Preview system prompt
            </Button>
            <Button onClick={() => setCreating(true)}>
              <Plus className="size-4" />
              New rule
            </Button>
          </div>
        </div>
      </FadeIn>

      {/* KPI strip */}
      <FadeIn delay={0.05}>
        <Stagger>
          <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
            <StaggerItem>
              <Stat label="Total rules" value={counts.total} icon={<ShieldCheck className="size-4" />} />
            </StaggerItem>
            <StaggerItem>
              <Stat label="Enabled" value={counts.enabled} icon={<Power className="size-4 text-tier-green" />} />
            </StaggerItem>
            <StaggerItem>
              <Stat label="Blocking" value={counts.blocking} icon={<Lock className="size-4 text-tier-red" />} />
            </StaggerItem>
            <StaggerItem>
              <Stat label="Forbidden tools" value={counts.forbiddenTools} icon={<AlertTriangle className="size-4 text-tier-yellow" />} />
            </StaggerItem>
          </div>
        </Stagger>
      </FadeIn>

      <FadeIn delay={0.1}>
        <Card>
          <CardHeader>
            <CardTitle>Configured rules</CardTitle>
            <CardDescription>
              {rules.length === 0
                ? "No rules yet. Add one to start constraining the agent."
                : `${rules.length} rule${rules.length === 1 ? "" : "s"} · sorted by kind`}
            </CardDescription>
          </CardHeader>
          <CardContent>
            {rules.length === 0 ? (
              <div className="grid h-32 place-items-center text-sm text-muted-foreground">
                Click <span className="mx-1 font-medium text-foreground">New rule</span> to add the first one.
              </div>
            ) : (
              <Stagger>
                <ul className="space-y-2">
                  {rules.map((r) => (
                    <StaggerItem key={r.id}>
                      <RuleRow
                        rule={r}
                        onEdit={() => setEditing(r)}
                        onToggle={() => toggle(r)}
                        onDelete={() => remove(r)}
                      />
                    </StaggerItem>
                  ))}
                </ul>
              </Stagger>
            )}
          </CardContent>
        </Card>
      </FadeIn>

      <AnimatePresence>
        {(creating || editing) && (
          <RuleEditorModal
            rule={editing}
            onClose={() => {
              setCreating(false);
              setEditing(null);
            }}
            onSaved={() => {
              setCreating(false);
              setEditing(null);
              refresh();
            }}
          />
        )}
        {showPreview && preview && (
          <PreviewModal preview={preview} onClose={() => setShowPreview(false)} />
        )}
      </AnimatePresence>
    </div>
  );
}

function RuleRow({
  rule,
  onEdit,
  onToggle,
  onDelete,
}: {
  rule: AgentRule;
  onEdit: () => void;
  onToggle: () => void;
  onDelete: () => void;
}) {
  return (
    <motion.li
      layout
      className={cn(
        "rounded-lg border p-3",
        rule.enabled
          ? "border-white/10 bg-white/[0.02]"
          : "border-white/5 bg-white/[0.01] opacity-70",
      )}
    >
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-2">
            <code className="rounded bg-muted px-1.5 py-0.5 font-mono text-[10px] text-muted-foreground">
              {rule.kind}
            </code>
            <Badge
              variant={
                rule.severity === "block"
                  ? "red"
                  : rule.severity === "warn"
                    ? "yellow"
                    : "muted"
              }
            >
              {rule.severity}
            </Badge>
            {rule.applies_to !== "all" && (
              <Badge variant="muted">user: {rule.applies_to.slice(0, 10)}…</Badge>
            )}
            <span className="font-medium">{rule.name}</span>
          </div>
          <pre className="mt-1 max-h-24 overflow-auto whitespace-pre-wrap rounded bg-muted/40 p-2 font-mono text-[11px] text-foreground/80">
            {rule.pattern}
          </pre>
          {rule.notes && (
            <p className="mt-1 text-[11px] text-muted-foreground">
              {rule.notes}
            </p>
          )}
          <p className="mt-1 text-[10px] uppercase tracking-wider text-muted-foreground">
            {KIND_LABELS[rule.kind].desc}
          </p>
        </div>
        <div className="flex items-center gap-1.5">
          <Button size="sm" variant="ghost" onClick={onEdit}>
            <Edit2 className="size-3.5" />
            Edit
          </Button>
          <Button size="sm" variant="ghost" onClick={onToggle}>
            {rule.enabled ? (
              <>
                <PowerOff className="size-3.5" />
                Disable
              </>
            ) : (
              <>
                <Power className="size-3.5" />
                Enable
              </>
            )}
          </Button>
          <Button
            size="sm"
            variant="ghost"
            onClick={onDelete}
            className="text-tier-red hover:text-tier-red"
          >
            <Trash2 className="size-3.5" />
          </Button>
        </div>
      </div>
    </motion.li>
  );
}

function RuleEditorModal({
  rule,
  onClose,
  onSaved,
}: {
  rule: AgentRule | null;
  onClose: () => void;
  onSaved: () => void;
}) {
  const [kind, setKind] = useState<AgentRule["kind"]>(rule?.kind ?? "forbid_tool");
  const [name, setName] = useState(rule?.name ?? "");
  const [pattern, setPattern] = useState(rule?.pattern ?? "");
  const [severity, setSeverity] = useState<"block" | "warn" | "log">(
    rule?.severity ?? "block",
  );
  const [appliesTo, setAppliesTo] = useState(rule?.applies_to ?? "all");
  const [enabled, setEnabled] = useState(rule?.enabled ?? true);
  const [notes, setNotes] = useState(rule?.notes ?? "");
  const [busy, setBusy] = useState(false);
  const { toast } = useToast();

  const isInt = KIND_LABELS[kind].needsInt;
  const needsSeverity = kind !== "forbid_tool"; // forbid_tool is always block

  const save = async () => {
    if (!name.trim() || !pattern.trim()) {
      toast({ variant: "error", title: "Name and pattern are required" });
      return;
    }
    if (isInt && !/^\d+$/.test(pattern.trim())) {
      toast({ variant: "error", title: "Pattern must be an integer (fen)" });
      return;
    }
    setBusy(true);
    try {
      if (rule) {
        await agentRulesApi.update(rule.id, {
          name,
          pattern,
          severity,
          applies_to: appliesTo,
          enabled,
          notes: notes || undefined,
        });
        toast({ variant: "success", title: "Rule updated" });
      } else {
        await agentRulesApi.create({
          kind,
          name,
          pattern,
          severity,
          applies_to: appliesTo,
          enabled,
          notes: notes || undefined,
        });
        toast({ variant: "success", title: "Rule created" });
      }
      onSaved();
    } catch (err) {
      const msg = err instanceof Error ? err.message : "Save failed";
      toast({ variant: "error", title: "Save failed", description: msg });
    } finally {
      setBusy(false);
    }
  };

  return (
    <motion.div
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      className="fixed inset-0 z-50 grid place-items-center bg-background/80 p-4 backdrop-blur"
      onClick={onClose}
    >
      <motion.div
        initial={{ y: 12, opacity: 0 }}
        animate={{ y: 0, opacity: 1 }}
        exit={{ y: 12, opacity: 0 }}
        onClick={(e: React.MouseEvent) => e.stopPropagation()}
        className="glass-strong grid w-full max-w-2xl grid-rows-[auto_1fr_auto] overflow-hidden rounded-2xl"
      >
        <div className="flex items-center justify-between border-b border-white/5 p-4">
          <div>
            <p className="text-[10px] font-medium uppercase tracking-wider text-muted-foreground">
              {rule ? "Edit rule" : "New rule"}
            </p>
            <p className="font-display text-base font-semibold">
              {rule ? rule.name : "Configure a new constraint"}
            </p>
          </div>
          <Button size="sm" variant="ghost" onClick={onClose}>
            <X className="size-4" />
          </Button>
        </div>
        <div className="space-y-4 overflow-y-auto p-4">
          <div className="grid gap-3 sm:grid-cols-2">
            <div className="space-y-1.5">
              <Label>Kind</Label>
              <select
                value={kind}
                onChange={(e) => setKind(e.target.value as AgentRule["kind"])}
                disabled={!!rule}
                className="w-full rounded-md border border-white/10 bg-white/5 px-3 py-2 text-sm"
              >
                {(Object.keys(KIND_LABELS) as AgentRule["kind"][]).map((k) => (
                  <option key={k} value={k}>
                    {KIND_LABELS[k].label}
                  </option>
                ))}
              </select>
              <p className="text-[10px] text-muted-foreground">
                {KIND_LABELS[kind].desc}
              </p>
            </div>
            <div className="space-y-1.5">
              <Label>Severity</Label>
              <select
                value={severity}
                onChange={(e) => setSeverity(e.target.value as "block" | "warn" | "log")}
                disabled={kind === "forbid_tool"}
                className="w-full rounded-md border border-white/10 bg-white/5 px-3 py-2 text-sm"
              >
                <option value="block">block</option>
                <option value="warn">warn</option>
                <option value="log">log</option>
              </select>
              {kind === "forbid_tool" && (
                <p className="text-[10px] text-muted-foreground">
                  forbid_tool is always severity=block.
                </p>
              )}
            </div>
          </div>
          <div className="space-y-1.5">
            <Label>Name</Label>
            <Input
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="Block transfers to offshore accounts"
            />
          </div>
          <div className="space-y-1.5">
            <Label>
              Pattern{" "}
              {isInt && (
                <span className="text-[10px] uppercase tracking-wider text-muted-foreground">
                  · integer fen
                </span>
              )}
            </Label>
            {kind === "system_prompt_prefix" || pattern.length > 80 ? (
              <Textarea
                value={pattern}
                onChange={(e) => setPattern(e.target.value)}
                rows={4}
                className="font-mono text-xs"
              />
            ) : (
              <Input
                value={pattern}
                onChange={(e) => setPattern(e.target.value)}
                placeholder={
                  isInt
                    ? "5000000"
                    : kind === "forbid_tool"
                      ? "transfer.create_pending"
                      : kind === "restrict_payee"
                        ? "Anna"
                        : kind === "restrict_memo"
                          ? "wire only"
                          : "pattern"
                }
                className="font-mono"
              />
            )}
          </div>
          <div className="grid gap-3 sm:grid-cols-2">
            <div className="space-y-1.5">
              <Label>Applies to</Label>
              <Input
                value={appliesTo}
                onChange={(e) => setAppliesTo(e.target.value)}
                placeholder="all or a user id (e.g. user_xxx)"
              />
              <p className="text-[10px] text-muted-foreground">
                <code>all</code> for every user, or a specific user_id.
              </p>
            </div>
            <div className="space-y-1.5">
              <Label>Enabled</Label>
              <div className="mt-1 flex items-center gap-2">
                <button
                  type="button"
                  onClick={() => setEnabled(!enabled)}
                  className={cn(
                    "inline-flex items-center gap-2 rounded-md border px-3 py-2 text-sm transition-colors",
                    enabled
                      ? "border-tier-green/30 bg-tier-green/10 text-tier-green"
                      : "border-white/10 bg-white/5 text-muted-foreground",
                  )}
                >
                  {enabled ? <Power className="size-3.5" /> : <PowerOff className="size-3.5" />}
                  {enabled ? "Enabled" : "Disabled"}
                </button>
              </div>
            </div>
          </div>
          <div className="space-y-1.5">
            <Label>Notes (optional)</Label>
            <Textarea
              value={notes ?? ""}
              onChange={(e) => setNotes(e.target.value)}
              rows={2}
              placeholder="Why does this rule exist?"
            />
          </div>
        </div>
        <div className="flex items-center justify-end gap-2 border-t border-white/5 p-4">
          <Button variant="outline" onClick={onClose}>
            Cancel
          </Button>
          <Button onClick={save} disabled={busy}>
            {busy && <Loader2 className="size-4 animate-spin" />}
            {rule ? "Save changes" : "Create rule"}
          </Button>
        </div>
      </motion.div>
    </motion.div>
  );
}

function PreviewModal({
  preview,
  onClose,
}: {
  preview: SystemPromptPreview;
  onClose: () => void;
}) {
  const [showRaw, setShowRaw] = useState(false);
  return (
    <motion.div
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      className="fixed inset-0 z-50 grid place-items-center bg-background/80 p-4 backdrop-blur"
      onClick={onClose}
    >
      <motion.div
        initial={{ y: 12, opacity: 0 }}
        animate={{ y: 0, opacity: 1 }}
        exit={{ y: 12, opacity: 0 }}
        onClick={(e: React.MouseEvent) => e.stopPropagation()}
        className="glass-strong grid w-full max-w-3xl grid-rows-[auto_1fr_auto] overflow-hidden rounded-2xl"
      >
        <div className="flex items-center justify-between border-b border-white/5 p-4">
          <div>
            <p className="text-[10px] font-medium uppercase tracking-wider text-muted-foreground">
              System-prompt preview
            </p>
            <p className="font-display text-base font-semibold">
              What the LLM actually sees
            </p>
            <p className="font-mono text-[10px] text-muted-foreground">
              {preview.user_id}
            </p>
          </div>
          <div className="flex items-center gap-2">
            <Button size="sm" variant="ghost" onClick={() => setShowRaw((s) => !s)}>
              {showRaw ? <EyeOff className="size-3.5" /> : <Eye className="size-3.5" />}
              {showRaw ? "Hide raw" : "Show raw"}
            </Button>
            <Button size="sm" variant="ghost" onClick={onClose}>
              <X className="size-4" />
            </Button>
          </div>
        </div>
        <div className="space-y-4 overflow-y-auto p-4">
          <div>
            <h3 className="mb-1 text-[10px] font-medium uppercase tracking-wider text-muted-foreground">
              Splice appended to the LLM system prompt
            </h3>
            <pre className="max-h-48 overflow-auto whitespace-pre-wrap rounded-md border border-white/10 bg-muted/30 p-3 font-mono text-xs">
              {preview.prefix || "(no system_prompt_prefix rules enabled for this user)"}
            </pre>
          </div>
          {preview.forbidden_tools.length > 0 && (
            <div>
              <h3 className="mb-1 text-[10px] font-medium uppercase tracking-wider text-muted-foreground">
                Tools removed from the allowlist
              </h3>
              <ul className="space-y-1.5">
                {preview.forbidden_tools.map((r) => (
                  <li
                    key={r.id}
                    className="flex items-center justify-between gap-2 rounded-md border border-tier-red/30 bg-tier-red/[0.04] p-2 text-xs"
                  >
                    <div>
                      <code className="font-mono text-[10px]">{r.pattern}</code>
                      <span className="ml-2 text-foreground/80">{r.name}</span>
                    </div>
                    <Badge variant="red">forbidden</Badge>
                  </li>
                ))}
              </ul>
            </div>
          )}
          {showRaw && (
            <div>
              <h3 className="mb-1 text-[10px] font-medium uppercase tracking-wider text-muted-foreground">
                Raw rule snapshot
              </h3>
              <pre className="max-h-64 overflow-auto rounded-md border border-white/10 bg-muted/30 p-3 font-mono text-[10px]">
                {JSON.stringify(preview.snapshot, null, 2)}
              </pre>
            </div>
          )}
        </div>
        <div className="flex justify-end border-t border-white/5 p-4">
          <Button variant="outline" onClick={onClose}>
            Close
          </Button>
        </div>
      </motion.div>
    </motion.div>
  );
}

function Stat({
  label,
  value,
  icon,
}: {
  label: string;
  value: number;
  icon: React.ReactNode;
}) {
  return (
    <Card>
      <CardContent className="flex items-center justify-between p-4">
        <div>
          <p className="text-[10px] font-medium uppercase tracking-wider text-muted-foreground">
            {label}
          </p>
          <p className="mt-1 font-display text-2xl font-semibold tracking-tight">
            {value}
          </p>
        </div>
        <span className="grid size-9 place-items-center rounded-md bg-white/5 ring-1 ring-white/10">
          {icon}
        </span>
      </CardContent>
    </Card>
  );
}
