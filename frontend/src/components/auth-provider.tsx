"use client";

import { useEffect } from "react";
import { useAuth } from "@/lib/store";
import { auth } from "@/lib/api";

export function AuthProvider({ children }: { children: React.ReactNode }) {
  const { setMe, setLoading } = useAuth();
  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const me = await auth.me();
        if (!cancelled) setMe(me);
      } catch {
        if (!cancelled) setLoading(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [setMe, setLoading]);
  return <>{children}</>;
}
