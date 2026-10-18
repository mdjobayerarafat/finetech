"use client";

import { useEffect, useMemo, useState } from "react";
import { motion } from "framer-motion";
import { AppShell } from "@/components/layout/app-shell";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { payees, friendlyError, type ApiError, type Payee } from "@/lib/api";
import { FadeIn, Stagger, StaggerItem } from "@/components/motion/aurora";
import { useToast } from "@/components/ui/toast";
import { useT } from "@/i18n/LanguageProvider";
import { usePageTitle } from "@/lib/use-page-title";
import {
  ArrowDownUp,
  CheckCircle2,
  Loader2,
  Phone,
  Plus,
  UserPlus,
  X,
} from "lucide-react";

export default function PayeesPage() {
  return (
    <AppShell>
      <Body />
    </AppShell>
  );
}

function Body() {
  const t = useT();
  usePageTitle("meta.payees.title", "Payees");
  const { toast } = useToast();
  const [list, setList] = useState<Payee[]>([]);
  const [loading, setLoading] = useState(true);
  const [addOpen, setAddOpen] = useState(false);
  const [verifyOf, setVerifyOf] = useState<Payee | null>(null);
  const [mergeOf, setMergeOf] = useState<{ keep: Payee; merge: Payee } | null>(null);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [busy, setBusy] = useState(false);

  const refresh = () => {
    setLoading(true);
    payees
      .list()
      .then(setList)
      .catch((e: ApiError) => {
        const fe = friendlyError(e);
        toast({ variant: "error", title: fe.title, description: fe.body });
      })
      .finally(() => setLoading(false));
  };
  useEffect(() => { refresh(); }, []);

  const toggle = (id: string) => {
    setSelected((s) => {
      const next = new Set(s);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const duplicateCandidates = useMemo(() => {
    // Heuristic: payees sharing a phone tail are duplicate candidates.
    const groups: Record<string, Payee[]> = {};
    list.forEach((p) => {
      if (!p.phone_tail) return;
      if (!groups[p.phone_tail]) groups[p.phone_tail] = [];
      groups[p.phone_tail].push(p);
    });
    return Object.values(groups).filter((g) => g.length >= 2);
  }, [list]);

  const runMerge = async (keep: Payee, merge: Payee) => {
    setBusy(true);
    try {
      const r = await payees.mergeDuplicates(keep.id, merge.id);
      toast({
        variant: "success",
        title: t("agent.payees.merge.done"),
        description: t("agent.payees.merge.done_fmt", {
          keep: keep.display_name,
          merge: merge.display_name,
          n: r.transfers_rewritten,
        }),
      });
      setMergeOf(null);
      setSelected(new Set());
      refresh();
    } catch (e) {
      const fe = friendlyError(e as ApiError);
      toast({ variant: "error", title: fe.title, description: fe.body });
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="space-y-6">
      <FadeIn>
        <div className="flex flex-wrap items-end justify-between gap-3">
          <div>
            <p className="text-[11px] font-medium uppercase tracking-widest text-muted-foreground">
              {t("payees.kicker")}
            </p>
            <h1 className="mt-1 font-display text-3xl font-semibold tracking-tight">
              {t("payees.title")}
            </h1>
            <p className="mt-1 max-w-2xl text-sm text-muted-foreground">
              {t("payees.subtitle")}
            </p>
          </div>
          <Button onClick={() => setAddOpen(true)}>
            <Plus className="size-4" />
            {t("agent.payees.add")}
          </Button>
        </div>
      </FadeIn>

      {duplicateCandidates.length > 0 && (
        <FadeIn>
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <ArrowDownUp className="size-4 text-primary" />
                {t("payees.duplicates.title")}
              </CardTitle>
              <CardDescription>
                {t("payees.duplicates.subtitle")}
              </CardDescription>
            </CardHeader>
            <CardContent>
              <ul className="divide-y divide-white/5">
                {duplicateCandidates.map((group) => (
                  <li key={group.map((p) => p.id).join("|")} className="flex flex-wrap items-center gap-2 py-2 text-sm">
                    <span className="text-muted-foreground">{t("payees.duplicates.tail")}:</span>
                    <span className="font-mono">{group[0].phone_tail}</span>
                    <span className="text-muted-foreground">·</span>
                    {group.map((p) => (
                      <span key={p.id} className="font-medium">{p.display_name}</span>
                    )).reduce((acc, el, i) => acc.concat(i === 0 ? [el] : [<span key={`s${i}`} className="text-muted-foreground">,</span>, el]), [] as React.ReactNode[])}
                    <div className="ml-auto flex gap-1">
                      {group.map((p) => (
                        <Button
                          key={p.id}
                          size="sm"
                          variant="outline"
                          onClick={() => setMergeOf({ keep: p, merge: group.find((q) => q.id !== p.id) ?? p })}
                        >
                          {t("payees.duplicates.keep", { name: p.display_name })}
                        </Button>
                      ))}
                    </div>
                  </li>
                ))}
              </ul>
            </CardContent>
          </Card>
        </FadeIn>
      )}

      <FadeIn>
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <UserPlus className="size-4 text-primary" />
              {t("payees.list.title")}
            </CardTitle>
            <CardDescription>
              {loading ? t("payees.list.loading") : `${list.length} ${t("payees.list.count_suffix")}`}
            </CardDescription>
          </CardHeader>
          <CardContent>
            {loading ? (
              <div className="flex items-center gap-2 text-sm text-muted-foreground">
                <Loader2 className="size-4 animate-spin" /> {t("payees.list.loading")}
              </div>
            ) : list.length === 0 ? (
              <p className="text-sm text-muted-foreground">{t("payees.list.empty")}</p>
            ) : (
              <Stagger>
                <ul className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
                  {list.map((p) => (
                    <StaggerItem key={p.id}>
                      <motion.div
                        whileHover={{ y: -2 }}
                        className="rounded-xl border border-white/5 bg-white/[0.03] p-4"
                      >
                        <div className="flex items-start justify-between gap-2">
                          <div>
                            <div className="flex items-center gap-2">
                              <span className="font-medium">{p.display_name}</span>
                              {p.verified ? (
                                <Badge variant="muted" className="gap-1">
                                  <CheckCircle2 className="size-3" /> {t("agent.payees.verified")}
                                </Badge>
                              ) : (
                                <Badge variant="yellow">{t("agent.payees.unverified")}</Badge>
                              )}
                            </div>
                            {p.phone_tail && (
                              <p className="mt-1 flex items-center gap-1 text-xs text-muted-foreground">
                                <Phone className="size-3" />
                                <span className="font-mono">****{p.phone_tail}</span>
                              </p>
                            )}
                            <p className="mt-1 text-[10px] uppercase tracking-wider text-muted-foreground">
                              {p.mock_destination}
                            </p>
                          </div>
                          <input
                            type="checkbox"
                            className="size-4 accent-primary"
                            checked={selected.has(p.id)}
                            onChange={() => toggle(p.id)}
                            aria-label={t("payees.select", { name: p.display_name })}
                          />
                        </div>
                        <div className="mt-3 flex flex-wrap gap-1">
                          {!p.verified && (
                            <Button size="sm" variant="outline" onClick={() => setVerifyOf(p)}>
                              {t("agent.payees.verify")}
                            </Button>
                          )}
                          {selected.size >= 1 && selected.has(p.id) && (
                            <Button
                              size="sm"
                              variant="outline"
                              onClick={() => {
                                const others = list.filter((q) => selected.has(q.id) && q.id !== p.id);
                                if (others.length > 0) setMergeOf({ keep: p, merge: others[0] });
                              }}
                            >
                              {t("agent.payees.merge", { name: p.display_name })}
                            </Button>
                          )}
                        </div>
                      </motion.div>
                    </StaggerItem>
                  ))}
                </ul>
              </Stagger>
            )}
          </CardContent>
        </Card>
      </FadeIn>

      {/* Add-payee dialog */}
      <AddPayeeDialog
        open={addOpen}
        onOpenChange={setAddOpen}
        onCreated={() => { setAddOpen(false); refresh(); }}
      />

      {/* Verify dialog (small mock transfer) */}
      <VerifyPayeeDialog
        payee={verifyOf}
        onClose={() => setVerifyOf(null)}
        onVerified={() => { setVerifyOf(null); refresh(); }}
      />

      {/* Confirm merge */}
      <Dialog open={!!mergeOf} onOpenChange={(o) => { if (!o) setMergeOf(null); }}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <ArrowDownUp className="size-4 text-primary" />
              {t("payees.merge.title")}
            </DialogTitle>
            <DialogDescription>
              {mergeOf && (
                <>
                  {t("payees.merge.body", {
                    keep: mergeOf.keep.display_name,
                    merge: mergeOf.merge.display_name,
                  })}
                </>
              )}
            </DialogDescription>
          </DialogHeader>
          <div className="flex justify-end gap-2">
            <Button variant="ghost" onClick={() => setMergeOf(null)} disabled={busy}>
              {t("form.cancel")}
            </Button>
            <Button onClick={() => mergeOf && runMerge(mergeOf.keep, mergeOf.merge)} disabled={busy || !mergeOf}>
              {busy ? <Loader2 className="size-4 animate-spin" /> : <ArrowDownUp className="size-4" />}
              {t("payees.merge.confirm")}
            </Button>
          </div>
        </DialogContent>
      </Dialog>
    </div>
  );
}

function AddPayeeDialog({ open, onOpenChange, onCreated }: { open: boolean; onOpenChange: (o: boolean) => void; onCreated: () => void }) {
  const t = useT();
  const { toast } = useToast();
  const [name, setName] = useState("");
  const [phone, setPhone] = useState("");
  const [busy, setBusy] = useState(false);

  const submit = async () => {
    setBusy(true);
    try {
      await payees.create({ display_name: name, phone_tail: phone || undefined });
      toast({ variant: "success", title: t("agent.payees.created") });
      setName(""); setPhone("");
      onCreated();
    } catch (e) {
      const fe = friendlyError(e as ApiError);
      toast({ variant: "error", title: fe.title, description: fe.body });
    } finally {
      setBusy(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Plus className="size-4 text-primary" />
            {t("agent.payees.add")}
          </DialogTitle>
          <DialogDescription>
            {t("agent.payees.add.note")}
          </DialogDescription>
        </DialogHeader>
        <div className="space-y-3">
          <div className="space-y-1">
            <Label htmlFor="payee-name">{t("agent.payees.add.name")}</Label>
            <Input id="payee-name" value={name} onChange={(e) => setName(e.target.value)} placeholder="Alex Friend" />
          </div>
          <div className="space-y-1">
            <Label htmlFor="payee-phone">{t("agent.payees.add.phone")}</Label>
            <Input id="payee-phone" value={phone} onChange={(e) => setPhone(e.target.value)} placeholder="1234" inputMode="numeric" maxLength={4} />
          </div>
          <div className="flex justify-end gap-2">
            <Button variant="ghost" onClick={() => onOpenChange(false)} disabled={busy}>
              <X className="size-4" /> {t("form.cancel")}
            </Button>
            <Button onClick={submit} disabled={busy || !name.trim()}>
              {busy ? <Loader2 className="size-4 animate-spin" /> : <Plus className="size-4" />}
              {t("agent.payees.add.save")}
            </Button>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}

function VerifyPayeeDialog({ payee, onClose, onVerified }: { payee: Payee | null; onClose: () => void; onVerified: () => void }) {
  const t = useT();
  const { toast } = useToast();
  const [challenge, setChallenge] = useState("100");
  const [code, setCode] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => { setCode(null); setChallenge("100"); }, [payee?.id]);

  const start = async () => {
    if (!payee) return;
    setBusy(true);
    try {
      const out = await payees.startVerification(payee.id, parseInt(challenge, 10) || 100);
      setCode(out.demo_code);
      toast({
        variant: "success",
        title: t("payees.verify.sent"),
        description: t("payees.verify.code_fmt", { code: out.demo_code }),
      });
      // The mock endpoint auto-confirms in the demo, so refresh.
      onVerified();
    } catch (e) {
      const fe = friendlyError(e as ApiError);
      toast({ variant: "error", title: fe.title, description: fe.body });
    } finally {
      setBusy(false);
    }
  };

  return (
    <Dialog open={!!payee} onOpenChange={(o) => { if (!o) onClose(); }}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <CheckCircle2 className="size-4 text-primary" />
            {t("payees.verify.title")}
          </DialogTitle>
          <DialogDescription>
            {payee && t("payees.verify.body", { name: payee.display_name })}
          </DialogDescription>
        </DialogHeader>
        <div className="space-y-3">
          <div className="space-y-1">
            <Label htmlFor="challenge">{t("payees.verify.amount")}</Label>
            <Input id="challenge" value={challenge} onChange={(e) => setChallenge(e.target.value)} inputMode="numeric" />
            <p className="text-[10px] uppercase tracking-wider text-muted-foreground">
              {t("payees.verify.amount_hint")}
            </p>
          </div>
          <Textarea readOnly value={code ?? t("payees.verify.code_pending")} rows={2} />
          <div className="flex justify-end gap-2">
            <Button variant="ghost" onClick={onClose} disabled={busy}>
              {t("form.cancel")}
            </Button>
            <Button onClick={start} disabled={busy || !payee}>
              {busy ? <Loader2 className="size-4 animate-spin" /> : <CheckCircle2 className="size-4" />}
              {t("payees.verify.run")}
            </Button>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
