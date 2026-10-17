"use client";

import { useEffect, useState } from "react";
import { util } from "@/lib/api";
import { useLanguage } from "@/i18n/LanguageProvider";
import type { Lang } from "@/i18n/dictionaries";

/**
 * useTranslated(text)
 *
 * Re-translates an LLM-narrated string (fraud narrative, forecast
 * narrative, etc.) into the user's currently selected language whenever
 * `lang` changes. The backend route is best-effort and never fails the
 * caller, so this hook is also infallible — if the model is offline the
 * user just sees the original text.
 *
 * Heuristic short-circuits:
 *   - empty input → empty output
 *   - English mode + input is already plain ASCII English → pass through
 *   - Chinese mode + input is already CJK-heavy → pass through
 *     (handled implicitly by the backend, mirrored client-side for the
 *      common case of an empty/null response)
 */
export function useTranslated(text: string | null | undefined): {
  translated: string;
  isTranslating: boolean;
} {
  const { lang } = useLanguage();
  const [translated, setTranslated] = useState<string>(text ?? "");
  const [isTranslating, setIsTranslating] = useState(false);

  useEffect(() => {
    const v = text ?? "";
    setTranslated(v);
    if (!v) {
      setIsTranslating(false);
      return;
    }
    // Don't burn a round-trip when the input is plausibly already in
    // the target language.
    if (alreadyInTarget(v, lang)) {
      setIsTranslating(false);
      return;
    }
    let cancelled = false;
    setIsTranslating(true);
    util
      .translate([v], lang as Lang)
      .then((out) => {
        if (cancelled) return;
        const first = out[0] ?? v;
        // If the server returned the input unchanged, treat it as a
        // no-op so we don't show the "translating" spinner for nothing.
        if (first && first !== v) {
          setTranslated(first);
        }
      })
      .catch(() => {
        // Best-effort — see util.translate.
      })
      .finally(() => {
        if (!cancelled) setIsTranslating(false);
      });
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [text, lang]);

  return { translated, isTranslating };
}

function alreadyInTarget(s: string, target: string): boolean {
  if (target === "zh") {
    // CJK present → already in target.
    return s
      .trim()
      .split("")
      .some((c) => c >= "\u4e00" && c <= "\u9fff");
  }
  // English target: every non-whitespace char is ASCII → already English
  // (skip the LLM round-trip on numbers, names, URLs, etc.).
  return s
    .trim()
    .split("")
    .every((c) => c.charCodeAt(0) < 128);
}
