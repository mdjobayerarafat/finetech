"use client";

import { Languages, Check } from "lucide-react";
import { motion, AnimatePresence } from "framer-motion";
import { useEffect, useRef, useState } from "react";
import { useLanguage } from "@/i18n/LanguageProvider";
import { cn } from "@/lib/utils";
import type { Lang } from "@/i18n/dictionaries";

const LABELS: Record<Lang, string> = {
  en: "English",
  zh: "中文",
};

const DESCRIPTIONS: Record<Lang, { eyebrow: string; native: string; sample: string }> = {
  en: {
    eyebrow: "English",
    native: "English",
    sample: "Translate every input and AI reply to English.",
  },
  zh: {
    eyebrow: "中文",
    native: "中文 (简体)",
    sample: "将所有输入和 AI 回复切换为中文。",
  },
};

export function LanguageButton({ className }: { className?: string }) {
  const { lang, setLang, t } = useLanguage();
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const onDoc = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };
    document.addEventListener("mousedown", onDoc);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDoc);
      document.removeEventListener("keydown", onKey);
    };
  }, [open]);

  return (
    <div ref={ref} className={cn("relative", className)}>
      <button
        type="button"
        onClick={() => setOpen((o) => !o)}
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-label={t("lang.label")}
        className={cn(
          "group inline-flex items-center gap-2 rounded-md border border-white/10 bg-card/50 px-2.5 py-1.5",
          "text-xs font-medium text-foreground/90 backdrop-blur transition-colors",
          "hover:border-primary/40 hover:bg-card/80",
        )}
      >
        <Languages className="size-3.5 text-primary" />
        <span className="font-display tracking-tight">{LABELS[lang]}</span>
        <span aria-hidden className="text-muted-foreground/60">▾</span>
      </button>

      <AnimatePresence>
        {open && (
          <motion.div
            initial={{ opacity: 0, y: -4, scale: 0.98 }}
            animate={{ opacity: 1, y: 0, scale: 1 }}
            exit={{ opacity: 0, y: -4, scale: 0.98 }}
            transition={{ duration: 0.12 }}
            role="listbox"
            aria-label={t("lang.label")}
            className={cn(
              "absolute right-0 z-50 mt-2 w-72 overflow-hidden rounded-xl border border-white/10",
              "bg-popover/95 shadow-[0_18px_40px_-12px_rgba(0,0,0,0.5)] backdrop-blur-xl",
            )}
          >
            <div className="border-b border-white/5 px-3 py-2">
              <p className="text-[10px] font-medium uppercase tracking-[0.18em] text-muted-foreground">
                {t("lang.label")}
              </p>
              <p className="mt-0.5 text-[11px] text-muted-foreground">
                {lang === "zh"
                  ? "切换后会同时改变界面文字与 AI 助手的回复语言。"
                  : "Switches UI labels and the assistant's reply language."}
              </p>
            </div>
            <ul className="p-1.5">
              {(["en", "zh"] as Lang[]).map((l) => {
                const active = l === lang;
                const meta = DESCRIPTIONS[l];
                return (
                  <li key={l}>
                    <button
                      type="button"
                      role="option"
                      aria-selected={active}
                      onClick={() => {
                        setLang(l);
                        setOpen(false);
                      }}
                      className={cn(
                        "flex w-full items-start gap-3 rounded-lg px-2.5 py-2 text-left text-sm transition-colors",
                        active
                          ? "bg-primary/15 ring-1 ring-primary/30"
                          : "hover:bg-white/[0.05]",
                      )}
                    >
                      <span
                        aria-hidden
                        className={cn(
                          "mt-0.5 grid size-6 place-items-center rounded-md text-[10px] font-bold uppercase tracking-wider",
                          active
                            ? "bg-primary text-primary-foreground"
                            : "bg-white/10 text-muted-foreground",
                        )}
                      >
                        {l === "zh" ? "中" : "EN"}
                      </span>
                      <span className="min-w-0 flex-1">
                        <span className="block font-medium text-foreground">
                          {meta.eyebrow}
                          <span className="ml-1.5 text-[11px] font-normal text-muted-foreground">
                            · {meta.native}
                          </span>
                        </span>
                        <span className="mt-0.5 block text-[11px] text-muted-foreground">
                          {meta.sample}
                        </span>
                      </span>
                      {active && (
                        <Check className="mt-0.5 size-4 shrink-0 text-primary" />
                      )}
                    </button>
                  </li>
                );
              })}
            </ul>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
