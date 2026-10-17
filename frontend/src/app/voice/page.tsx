"use client";

import { useEffect, useRef, useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import {
  Languages,
  Loader2,
  Mic,
  MicOff,
  Send,
  Sparkles,
  Volume2,
  VolumeX,
} from "lucide-react";
import { AppShell } from "@/components/layout/app-shell";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { useToast } from "@/components/ui/toast";
import { voice, agent, ApiError, type SttOut, type TtsOut } from "@/lib/api";
import { FadeIn, Stagger, StaggerItem, AuroraBackdrop } from "@/components/motion/aurora";
import { TierBadge } from "@/components/motion/tier-badge";
import { ConfirmationCard } from "@/components/confirmation-card";
import { cn } from "@/lib/utils";
import { useT, useLanguage } from "@/i18n/LanguageProvider";

declare global {
  interface Window {
    SpeechRecognition?: any;
    webkitSpeechRecognition?: any;
  }
}

const SUGGESTIONS_ZH = ["转 200 给 Li Wei，午饭钱。", "本月花了多少?", "取消 NetStream 订阅。"];
const SUGGESTIONS_EN = [
  "Pay Li Wei 200 yuan for lunch.",
  "Show me my spending this month.",
  "Cancel my NetStream subscription.",
];

export default function VoicePage() {
  return (
    <AppShell>
      <Body />
    </AppShell>
  );
}

type Conversation = {
  id: string;
  role: "user" | "assistant";
  text: string;
  ts: number;
  plan?: any;
  language?: string;
};

function Body() {
  const t = useT();
  const { lang: uiLang } = useLanguage();
  const [language, setLanguage] = useState<"zh" | "en" | "mixed">(uiLang === "zh" ? "zh" : "mixed");
  const [transcript, setTranscript] = useState<string>("");
  const [autoRun, setAutoRun] = useState<boolean>(true);
  const [busy, setBusy] = useState<boolean>(false);
  const [recording, setRecording] = useState<boolean>(false);
  const [ttsBusy, setTtsBusy] = useState<boolean>(false);
  const [ttsPlaying, setTtsPlaying] = useState<boolean>(false);
  const [conversation, setConversation] = useState<Conversation[]>([]);
  const { toast } = useToast();
  const mediaRecorderRef = useRef<MediaRecorder | null>(null);
  const chunksRef = useRef<Blob[]>([]);
  const ttsAudioRef = useRef<HTMLAudioElement | null>(null);

  useEffect(() => {
    return () => {
      try {
        ttsAudioRef.current?.pause();
      } catch {}
    };
  }, []);

  const suggestions = language === "zh" ? SUGGESTIONS_ZH : SUGGESTIONS_EN;

  const startRecording = async () => {
    if (recording) return;
    if (typeof navigator === "undefined" || !navigator.mediaDevices) {
      toast({
        variant: "error",
        title: "Microphone unavailable",
        description: "Try the demo phrases below or paste a transcript instead.",
      });
      return;
    }
    try {
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      const recorder = new MediaRecorder(stream);
      chunksRef.current = [];
      recorder.ondataavailable = (e) => {
        if (e.data && e.data.size > 0) chunksRef.current.push(e.data);
      };
      recorder.onstop = async () => {
        const blob = new Blob(chunksRef.current, { type: "audio/webm" });
        const buf = await blob.arrayBuffer();
        const bytes = new Uint8Array(buf);
        let bin = "";
        const chunk = 0x8000;
        for (let i = 0; i < bytes.length; i += chunk) {
          bin += String.fromCharCode.apply(
            null,
            Array.from(bytes.subarray(i, i + chunk))
          );
        }
        const b64 = btoa(bin);
        await runStt(b64, "audio/webm");
        // Stop mic tracks.
        stream.getTracks().forEach((t) => t.stop());
      };
      recorder.start();
      mediaRecorderRef.current = recorder;
      setRecording(true);
    } catch (e) {
      const msg = e instanceof Error ? e.message : "Microphone permission denied";
      toast({ variant: "error", title: "Could not record", description: msg });
    }
  };

  const stopRecording = () => {
    if (!recording || !mediaRecorderRef.current) return;
    mediaRecorderRef.current.stop();
    setRecording(false);
  };

  const runStt = async (audio_b64: string, mime: string) => {
    setBusy(true);
    try {
      const r = await voice.stt(audio_b64, mime);
      setTranscript(r.text);
      if (r.language) setLanguage(r.language as "zh" | "en" | "mixed");
      if (autoRun) {
        await sendToAgent(r.text);
      }
    } catch (e) {
      const msg = e instanceof Error ? e.message : "STT failed";
      toast({ variant: "error", title: "Transcription failed", description: msg });
    } finally {
      setBusy(false);
    }
  };

  const sendToAgent = async (text: string) => {
    if (!text.trim()) return;
    const userMsg: Conversation = {
      id: crypto.randomUUID(),
      role: "user",
      text,
      ts: Date.now(),
      language,
    };
    setConversation((c) => [...c, userMsg]);
    setBusy(true);
    try {
      const r = await agent.message(text, language);
      const aMsg: Conversation = {
        id: crypto.randomUUID(),
        role: "assistant",
        text: r.clarification_required
          ? r.questions.join(" / ") || "I need a bit more information."
          : "Here's the plan.",
        ts: Date.now(),
        plan: r,
        language,
      };
      setConversation((c) => [...c, aMsg]);
      // If the assistant produces a TTS-able line, speak it.
      if (aMsg.text && aMsg.text.length < 240) {
        speak(aMsg.text);
      }
    } catch (e) {
      const msg = e instanceof ApiError ? e.message : "Assistant failed";
      const aMsg: Conversation = {
        id: crypto.randomUUID(),
        role: "assistant",
        text: `⚠️ ${msg}`,
        ts: Date.now(),
        language,
      };
      setConversation((c) => [...c, aMsg]);
    } finally {
      setBusy(false);
    }
  };

  const speak = async (text: string) => {
    setTtsBusy(true);
    try {
      const r: TtsOut = await voice.tts(text, language);
      // Prefer browser TTS (richer bilingual quality) when available.
      if (typeof window !== "undefined" && "speechSynthesis" in window) {
        try {
          const u = new SpeechSynthesisUtterance(text);
          u.lang = language === "zh" ? "zh-CN" : "en-US";
          window.speechSynthesis.cancel();
          window.speechSynthesis.speak(u);
          toast({ variant: "success", title: "Speaking" });
          return;
        } catch {}
      }
      // Fallback: play the tiny stub WAV.
      const bin = atob(r.audio_b64);
      const bytes = new Uint8Array(bin.length);
      for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
      const blob = new Blob([bytes], { type: r.mime });
      const url = URL.createObjectURL(blob);
      const a = new Audio(url);
      ttsAudioRef.current = a;
      a.onplay = () => setTtsPlaying(true);
      a.onended = () => {
        setTtsPlaying(false);
        URL.revokeObjectURL(url);
      };
      a.onerror = () => {
        setTtsPlaying(false);
        URL.revokeObjectURL(url);
      };
      await a.play();
    } catch (e) {
      const msg = e instanceof Error ? e.message : "TTS failed";
      toast({ variant: "error", title: "TTS failed", description: msg });
    } finally {
      setTtsBusy(false);
    }
  };

  const stopTts = () => {
    if (typeof window !== "undefined" && "speechSynthesis" in window) {
      window.speechSynthesis.cancel();
    }
    try {
      ttsAudioRef.current?.pause();
    } catch {}
    setTtsPlaying(false);
  };

  return (
    <div className="space-y-6">
      <AuroraBackdrop />
      <FadeIn>
        <div>
          <p className="text-[11px] font-medium uppercase tracking-widest text-muted-foreground">
            {t("voice.eyebrow")}
          </p>
          <h1 className="mt-1 font-display text-3xl font-semibold tracking-tight">
            {t("voice.title")}
          </h1>
          <p className="mt-1 max-w-2xl text-sm text-muted-foreground">
            Speech-to-text in, text-to-speech out, with the same planner behind
            it. Try “转 200 给 Li Wei” — the assistant will draft a transfer
            and speak the confirmation back.
          </p>
        </div>
      </FadeIn>

      <div className="grid gap-4 lg:grid-cols-[360px_1fr]">
        <FadeIn delay={0.05}>
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Mic className="size-4 text-primary" />
                Capture
              </CardTitle>
              <CardDescription>
                Records a short clip and transcribes via the voice STT endpoint.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div>
                <Label>Language</Label>
                <div className="mt-2 flex flex-wrap gap-1.5">
                  {(["mixed", "zh", "en"] as const).map((l) => (
                    <button
                      key={l}
                      onClick={() => setLanguage(l)}
                      className={cn(
                        "rounded-full border px-3 py-1 text-xs transition-colors",
                        language === l
                          ? "border-primary/50 bg-primary/[0.1] text-foreground"
                          : "border-white/10 bg-white/[0.03] text-muted-foreground hover:bg-accent"
                      )}
                    >
                      {l === "mixed" ? "Auto" : l === "zh" ? "中文" : "English"}
                    </button>
                  ))}
                </div>
              </div>

              <div className="grid place-items-center gap-3">
                <motion.button
                  whileTap={{ scale: 0.95 }}
                  onClick={recording ? stopRecording : startRecording}
                  disabled={busy}
                  className={cn(
                    "grid size-20 place-items-center rounded-full transition-colors",
                    recording
                      ? "bg-tier-red text-white shadow-[0_0_30px_hsl(0_80%_60%/0.5)]"
                      : "bg-gradient-to-br from-primary to-indigo-500 text-white shadow-glow"
                  )}
                >
                  {recording ? <MicOff className="size-7" /> : <Mic className="size-7" />}
                </motion.button>
                <p className="text-[11px] text-muted-foreground">
                  {recording
                    ? "Listening… tap to stop"
                    : busy
                    ? "Transcribing…"
                    : "Tap to start a 5-second clip"}
                </p>
              </div>

              <div>
                <Label>Or paste a transcript</Label>
                <textarea
                  rows={3}
                  value={transcript}
                  onChange={(e) => setTranscript(e.target.value)}
                  className="mt-1 w-full rounded-md border border-white/10 bg-white/5 px-3 py-2 text-sm"
                  placeholder="转 200 给 Li Wei，午饭钱"
                />
                <div className="mt-2 flex items-center justify-between">
                  <label className="inline-flex items-center gap-2 text-[11px] text-muted-foreground">
                    <input
                      type="checkbox"
                      checked={autoRun}
                      onChange={(e) => setAutoRun(e.target.checked)}
                      className="size-3.5"
                    />
                    Auto-run the planner
                  </label>
                  <div className="flex items-center gap-2">
                    <Button
                      size="sm"
                      variant="outline"
                      onClick={() => sendToAgent(transcript)}
                      disabled={busy || !transcript.trim()}
                    >
                      <Send className="size-3.5" />
                      Send
                    </Button>
                    <Button
                      size="sm"
                      variant="outline"
                      onClick={() => (ttsPlaying ? stopTts() : speak(transcript))}
                      disabled={ttsBusy || !transcript.trim()}
                    >
                      {ttsPlaying ? <VolumeX className="size-3.5" /> : <Volume2 className="size-3.5" />}
                      {ttsPlaying ? "Stop" : "Speak"}
                    </Button>
                  </div>
                </div>
              </div>

              <div>
                <Label>Demo phrases</Label>
                <Stagger>
                  <ul className="mt-2 space-y-1.5">
                    {suggestions.map((s) => (
                      <StaggerItem key={s}>
                        <motion.li layout>
                          <button
                            onClick={() => {
                              setTranscript(s);
                              if (autoRun) sendToAgent(s);
                            }}
                            className="w-full rounded-md border border-white/5 bg-white/[0.03] px-2.5 py-1.5 text-left text-xs transition-colors hover:bg-accent"
                          >
                            {s}
                          </button>
                        </motion.li>
                      </StaggerItem>
                    ))}
                  </ul>
                </Stagger>
              </div>
            </CardContent>
          </Card>
        </FadeIn>

        <FadeIn delay={0.1}>
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Languages className="size-4 text-primary" />
                Conversation
              </CardTitle>
              <CardDescription>
                The transcript is appended verbatim; the assistant response is
                read aloud.
              </CardDescription>
            </CardHeader>
            <CardContent>
              {conversation.length === 0 ? (
                <p className="text-sm text-muted-foreground">
                  Tap a demo phrase or record yourself to start.
                </p>
              ) : (
                <ul className="space-y-3">
                  <AnimatePresence initial={false}>
                    {conversation.map((m) => (
                      <motion.li
                        key={m.id}
                        initial={{ opacity: 0, y: 6 }}
                        animate={{ opacity: 1, y: 0 }}
                        exit={{ opacity: 0, y: -6 }}
                        className={cn(
                          "rounded-xl border px-3 py-2 text-sm",
                          m.role === "user"
                            ? "border-primary/30 bg-primary/[0.06]"
                            : "border-white/10 bg-white/[0.03]"
                        )}
                      >
                        <div className="flex items-center gap-2 text-[10px] uppercase tracking-wider text-muted-foreground">
                          <span>{m.role}</span>
                          {m.language && (
                            <Badge variant="muted" className="font-mono text-[9px]">
                              {m.language}
                            </Badge>
                          )}
                        </div>
                        <p className="mt-1 leading-relaxed">{m.text}</p>
                        {m.plan?.plan?.nodes && (
                          <ul className="mt-2 space-y-1">
                            {m.plan.plan.nodes.map((n: any) => (
                              <li
                                key={n.id}
                                className="flex items-center justify-between rounded-md border border-white/5 bg-background/40 px-2 py-1.5 text-[11px]"
                              >
                                <div className="flex items-center gap-2">
                                  <TierBadge tier={n.risk_tier} size="sm" />
                                  <code className="font-mono">{n.tool}</code>
                                </div>
                                {n.approval_required && (
                                  <Badge variant="muted">approval</Badge>
                                )}
                              </li>
                            ))}
                          </ul>
                        )}
                        {m.plan?.pending_action && (
                          <div className="mt-2">
                            <ConfirmationCard
                              pendingActionId={m.plan.pending_action.id}
                              tool={m.plan.pending_action.tool}
                              parameters={
                                m.plan.plan.nodes.find(
                                  (n: any) => n.pending_action_id === m.plan.pending_action.id
                                )?.parameters ?? {}
                              }
                              riskTier={m.plan.pending_action.risk_tier}
                              workflowId={m.plan.plan.workflow_id}
                            />
                          </div>
                        )}
                      </motion.li>
                    ))}
                  </AnimatePresence>
                </ul>
              )}
            </CardContent>
          </Card>
        </FadeIn>
      </div>
    </div>
  );
}

// We re-use the input/label components. If they aren't already on the
// page we declare stubs (Next.js will tree-shake if unused).
function Label({ children }: { children: React.ReactNode }) {
  return <span className="text-[10px] font-medium uppercase tracking-wider text-muted-foreground">{children}</span>;
}
