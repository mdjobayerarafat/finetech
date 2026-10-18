"use client";

// Tiny helper for client components to set a per-page browser-tab title
// (and a brief description) without needing a server-rendered metadata
// export. Pulls the title from the i18n dictionary when given a
// `meta.*.title` key, otherwise uses the literal string.

import { useEffect } from "react";
import { useT } from "@/i18n/LanguageProvider";

export function usePageTitle(
  key: string | undefined,
  fallback: string,
  description?: string,
) {
  const t = useT();
  useEffect(() => {
    if (typeof document === "undefined") return;
    const brand = "FinTech Pioneers";
    const resolved = key ? t(key as never) || fallback : fallback;
    document.title = `${resolved} · ${brand}`;
    if (description) {
      const meta = document.querySelector(
        'meta[name="description"]',
      ) as HTMLMetaElement | null;
      if (meta) meta.content = description;
    }
  }, [key, fallback, description, t]);
}
