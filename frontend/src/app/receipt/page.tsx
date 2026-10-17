"use client";

import { useEffect, useRef, useState } from "react";
import { motion } from "framer-motion";
import {
  Camera,
  Image as ImageIcon,
  Loader2,
  Receipt as ReceiptIcon,
  Sparkles,
  Upload,
} from "lucide-react";
import { AppShell } from "@/components/layout/app-shell";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { useToast } from "@/components/ui/toast";
import { receipts, type ReceiptDraft } from "@/lib/api";
import { FadeIn, Stagger, StaggerItem, AuroraBackdrop } from "@/components/motion/aurora";
import { agent, ApiError } from "@/lib/api";
import { ConfirmationCard } from "@/components/confirmation-card";
import { cnyToFen, fenToCny, cn } from "@/lib/utils";
import { useT, useLanguage } from "@/i18n/LanguageProvider";

export default function ReceiptPage() {
  return (
    <AppShell>
      <Body />
    </AppShell>
  );
}

const SAMPLE_FILENAMES = [
  "sun-cafe-receipt.jpg",
  "acme-books-invoice.png",
  "telco-monthly-bill.jpg",
  "citypower-electric.png",
  "winter-tuition-notice.jpg",
];

function Body() {
  const t = useT();
  const { lang } = useLanguage();
  const [filename, setFilename] = useState<string>(SAMPLE_FILENAMES[0]);
  const [merchantHint, setMerchantHint] = useState<string>("");
  const [amountCny, setAmountCny] = useState<string>("");
  const [preview, setPreview] = useState<string | null>(null);
  const [draft, setDraft] = useState<ReceiptDraft | null>(null);
  const [busy, setBusy] = useState(false);
  const [running, setRunning] = useState(false);
  const { toast } = useToast();
  const fileInput = useRef<HTMLInputElement | null>(null);

  // When the user picks a real file we read it as a data URL for the
  // preview, but we only send the *length* of the bytes to the
  // simulation backend (it does not actually decode image bytes).
  const onFile = (file: File | null) => {
    if (!file) return;
    setFilename(file.name);
    const reader = new FileReader();
    reader.onload = () => setPreview(typeof reader.result === "string" ? reader.result : null);
    reader.readAsDataURL(file);
  };

  const scan = async () => {
    setBusy(true);
    setDraft(null);
    try {
      const d = await receipts.scan({
        filename,
        merchant_hint: merchantHint || undefined,
        amount_cny: amountCny ? Number(amountCny) : undefined,
        image_b64: preview ? preview.split(",")[1] : undefined,
      });
      setDraft(d);
      toast({
        variant: "success",
        title: lang === "zh" ? "小票已扫描" : "Receipt scanned",
        description: `${d.payee_hint} · ${fenToCny(d.amount_fen)}`,
      });
    } catch (e) {
      const msg = e instanceof Error ? e.message : "Scan failed";
      toast({ variant: "error", title: lang === "zh" ? "小票识别失败" : "Receipt scan failed", description: msg });
    } finally {
      setBusy(false);
    }
  };

  // Send the draft to the agent as a transfer-create message. The
  // backend will run policy + fraud guard and return a normal
  // confirmation card the user can tap to commit.
  const sendToAgent = async () => {
    if (!draft) return;
    setRunning(true);
    try {
      const text = `Pay ${draft.payee_hint} ${fenToCny(draft.amount_fen)} — receipt scan`;
      await agent.message(text, "en");
      toast({
        variant: "success",
        title: "Sent to Assistant",
        description: "Review the plan in the Assistant tab.",
      });
    } catch (e) {
      const msg = e instanceof ApiError ? e.message : "Failed to send to assistant";
      toast({ variant: "error", title: "Assistant call failed", description: msg });
    } finally {
      setRunning(false);
    }
  };

  return (
    <div className="space-y-6">
      <AuroraBackdrop />
      <FadeIn>
        <div>
          <p className="text-[11px] font-medium uppercase tracking-widest text-muted-foreground">
            Receipts · vision → payment draft
          </p>
          <h1 className="mt-1 font-display text-3xl font-semibold tracking-tight">
            Snap a receipt, draft a payment.
          </h1>
          <p className="mt-1 max-w-2xl text-sm text-muted-foreground">
            Upload a photo of a receipt or invoice. The vision model extracts
            payee, amount, and due date; the result is a Yellow-tier draft the
            user confirms before any money moves.
          </p>
        </div>
      </FadeIn>

      <div className="grid gap-4 lg:grid-cols-[1fr_360px]">
        <FadeIn delay={0.05}>
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Camera className="size-4 text-primary" />
                Upload
              </CardTitle>
              <CardDescription>JPEG / PNG / HEIC. Stored only in memory for the demo.</CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div
                onClick={() => fileInput.current?.click()}
                onDragOver={(e) => e.preventDefault()}
                onDrop={(e) => {
                  e.preventDefault();
                  onFile(e.dataTransfer.files[0] ?? null);
                }}
                className="grid h-44 cursor-pointer place-items-center rounded-lg border border-dashed border-white/10 bg-white/[0.02] text-center text-xs text-muted-foreground transition-colors hover:border-white/20 hover:bg-white/[0.04]"
              >
                {preview ? (
                  // eslint-disable-next-line @next/next/no-img-element
                  <img
                    src={preview}
                    alt="Receipt preview"
                    className="max-h-40 max-w-full rounded object-contain"
                  />
                ) : (
                  <div className="flex flex-col items-center gap-2">
                    <Upload className="size-5" />
                    <p>Drop a receipt here, or click to choose a file</p>
                    <p className="text-[10px] text-muted-foreground/70">
                      Try a demo filename from the list on the right
                    </p>
                  </div>
                )}
                <input
                  ref={fileInput}
                  type="file"
                  accept="image/*"
                  className="hidden"
                  onChange={(e) => onFile(e.target.files?.[0] ?? null)}
                />
              </div>

              <div className="grid gap-3 sm:grid-cols-2">
                <div>
                  <Label htmlFor="merchant">Merchant hint (optional)</Label>
                  <Input
                    id="merchant"
                    value={merchantHint}
                    onChange={(e) => setMerchantHint(e.target.value)}
                    placeholder="e.g. Sun Cafe"
                  />
                </div>
                <div>
                  <Label htmlFor="amount">Amount override (CNY, optional)</Label>
                  <Input
                    id="amount"
                    type="number"
                    min={0}
                    step="0.01"
                    value={amountCny}
                    onChange={(e) => setAmountCny(e.target.value)}
                    placeholder="auto-detect"
                  />
                </div>
              </div>

              <div className="flex items-center justify-end gap-2">
                <Button onClick={scan} disabled={busy}>
                  {busy ? <Loader2 className="size-4 animate-spin" /> : <Sparkles className="size-4" />}
                  Scan receipt
                </Button>
              </div>
            </CardContent>
          </Card>
        </FadeIn>

        <FadeIn delay={0.1}>
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <ImageIcon className="size-4 text-primary" />
                Demo files
              </CardTitle>
              <CardDescription>Pick a filename to simulate a known receipt.</CardDescription>
            </CardHeader>
            <CardContent>
              <Stagger>
                <ul className="space-y-1.5">
                  {SAMPLE_FILENAMES.map((f) => (
                    <StaggerItem key={f}>
                      <motion.li layout>
                        <button
                          onClick={() => {
                            setFilename(f);
                            setPreview(null);
                          }}
                          className={cn(
                            "w-full rounded-md border px-2.5 py-1.5 text-left text-xs transition-colors",
                            filename === f
                              ? "border-primary/40 bg-primary/[0.08] text-foreground"
                              : "border-white/5 bg-white/[0.03] hover:bg-accent"
                          )}
                        >
                          {f}
                        </button>
                      </motion.li>
                    </StaggerItem>
                  ))}
                </ul>
              </Stagger>
            </CardContent>
          </Card>
        </FadeIn>
      </div>

      <FadeIn delay={0.15}>
        <DraftCard draft={draft} running={running} onSendToAgent={sendToAgent} />
      </FadeIn>
    </div>
  );
}

function DraftCard({
  draft,
  running,
  onSendToAgent,
}: {
  draft: ReceiptDraft | null;
  running: boolean;
  onSendToAgent: () => void;
}) {
  if (!draft) {
    return (
      <Card>
        <CardContent className="grid h-32 place-items-center text-sm text-muted-foreground">
          The extracted draft will appear here.
        </CardContent>
      </Card>
    );
  }
  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <ReceiptIcon className="size-4 text-primary" />
          Extracted draft
          <Badge variant={draft.source === "llm" ? "green" : "muted"}>
            {draft.source === "llm" ? "vision model" : "stub"}
          </Badge>
          <Badge variant="yellow">Yellow tier</Badge>
        </CardTitle>
        <CardDescription>
          Confidence: {(draft.confidence * 100).toFixed(0)}% · currency {draft.currency}
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="grid gap-3 sm:grid-cols-3">
          <KV label="Payee" value={draft.payee_hint} />
          <KV label="Amount" value={fenToCny(draft.amount_fen)} />
          <KV label="Due" value={new Date(draft.due_at).toLocaleString()} />
        </div>
        {draft.notes.length > 0 && (
          <ul className="list-disc pl-5 text-xs text-muted-foreground">
            {draft.notes.map((n, i) => (
              <li key={i}>{n}</li>
            ))}
          </ul>
        )}
        <div className="flex items-center justify-end gap-2">
          <Button onClick={onSendToAgent} disabled={running}>
            {running ? <Loader2 className="size-4 animate-spin" /> : <Sparkles className="size-4" />}
            Send to Assistant to confirm
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}

function KV({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-md border border-white/5 bg-white/[0.03] p-3">
      <p className="text-[10px] font-medium uppercase tracking-wider text-muted-foreground">
        {label}
      </p>
      <p className="mt-1 text-sm font-medium">{value}</p>
    </div>
  );
}
