"use client";

import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
} from "react";
import { dictionaries, isLang, type DictKey, type Lang } from "./dictionaries";

const STORAGE_KEY = "ftp.lang";

type LanguageContextValue = {
  lang: Lang;
  setLang: (next: Lang, opts?: { persist?: boolean }) => void;
  /** Localize a key. Missing keys fall back to the English string, then to
   *  the key itself, so a typo is obvious in the UI. */
  t: (key: DictKey, vars?: Record<string, string | number>) => string;
};

const LanguageContext = createContext<LanguageContextValue | null>(null);

function readInitialLang(): Lang {
  if (typeof window === "undefined") return "en";
  const saved = window.localStorage.getItem(STORAGE_KEY);
  if (isLang(saved)) return saved;
  // Honour the browser's preferred language on first visit.
  const nav = (window.navigator?.language || "en").toLowerCase();
  if (nav.startsWith("zh")) return "zh";
  return "en";
}

function interpolate(template: string, vars?: Record<string, string | number>): string {
  if (!vars) return template;
  return template.replace(/\{(\w+)\}/g, (_m, k) => {
    const v = vars[k];
    return v == null ? `{${k}}` : String(v);
  });
}

export function LanguageProvider({ children }: { children: React.ReactNode }) {
  const [lang, setLangState] = useState<Lang>("en");

  // Read saved language on mount. The first render is "en" so the
  // server-rendered HTML matches; we then swap on hydration. This keeps
  // the markup stable across the SSR/CSR boundary.
  useEffect(() => {
    setLangState(readInitialLang());
  }, []);

  // Reflect the active language on the <html lang> attribute and the
  // the lang cookie so the backend can read it.
  useEffect(() => {
    if (typeof document !== "undefined") {
      document.documentElement.lang = lang === "zh" ? "zh-CN" : "en";
    }
  }, [lang]);

  const setLang = useCallback((next: Lang, opts?: { persist?: boolean }) => {
    setLangState(next);
    if (typeof window !== "undefined") {
      window.localStorage.setItem(STORAGE_KEY, next);
      // 1-year cookie. Backend may read it on the next request.
      const oneYear = 60 * 60 * 24 * 365;
      document.cookie = `${STORAGE_KEY}=${next}; Max-Age=${oneYear}; Path=/; SameSite=Lax`;
    }
    if (opts?.persist !== false) {
      // Fire and forget — let the backend know about the new preference.
      // We import lazily to avoid a circular import at module-init time.
      import("@/lib/api").then(({ users }) => {
        users
          .setPreferredLanguage(next)
          .catch(() => {
            /* not signed in yet — fine */
          });
      });
    }
  }, []);

  const t = useCallback(
    (key: DictKey, vars?: Record<string, string | number>) => {
      const fromActive = dictionaries[lang]?.[key];
      const fallback = dictionaries.en[key];
      const template = fromActive ?? fallback ?? (key as string);
      return interpolate(template, vars);
    },
    [lang],
  );

  const value = useMemo<LanguageContextValue>(
    () => ({ lang, setLang, t }),
    [lang, setLang, t],
  );

  return <LanguageContext.Provider value={value}>{children}</LanguageContext.Provider>;
}

export function useLanguage(): LanguageContextValue {
  const ctx = useContext(LanguageContext);
  if (!ctx) {
    // Provider missing — fail open with English strings so we never crash
    // a page just because the tree is mounted in isolation (tests, etc.).
    return {
      lang: "en",
      setLang: () => {},
      t: (key) => (dictionaries.en[key] ?? (key as string)),
    };
  }
  return ctx;
}

export function useT() {
  return useLanguage().t;
}
