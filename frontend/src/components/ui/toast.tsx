"use client";

import * as React from "react";
import { AnimatePresence, motion } from "framer-motion";
import { CheckCircle2, AlertTriangle, Info, XCircle, X } from "lucide-react";
import { cn } from "@/lib/utils";

type ToastVariant = "default" | "success" | "warning" | "error";
type ToastItem = { id: number; title: string; description?: string; variant: ToastVariant };

type Ctx = { toast: (t: Omit<ToastItem, "id">) => void };
const ToastContext = React.createContext<Ctx | null>(null);

export function ToastProvider({ children }: { children: React.ReactNode }) {
  const [items, setItems] = React.useState<ToastItem[]>([]);
  const counter = React.useRef(0);
  const toast = React.useCallback((t: Omit<ToastItem, "id">) => {
    const id = ++counter.current;
    setItems((prev) => [...prev, { id, ...t }]);
    setTimeout(() => setItems((prev) => prev.filter((x) => x.id !== id)), 5000);
  }, []);
  return (
    <ToastContext.Provider value={{ toast }}>
      {children}
      <div className="pointer-events-none fixed bottom-4 right-4 z-[100] flex w-[360px] max-w-[calc(100vw-2rem)] flex-col gap-2">
        <AnimatePresence>
          {items.map((t) => (
            <motion.div
              key={t.id}
              layout
              initial={{ opacity: 0, x: 24, scale: 0.96 }}
              animate={{ opacity: 1, x: 0, scale: 1 }}
              exit={{ opacity: 0, x: 24, scale: 0.96 }}
              transition={{ type: "spring", damping: 22, stiffness: 240 }}
              className={cn(
                "pointer-events-auto flex items-start gap-3 rounded-lg border bg-card p-3 shadow-lg",
                t.variant === "success" && "border-tier-green/30",
                t.variant === "warning" && "border-tier-yellow/30",
                t.variant === "error" && "border-tier-red/30"
              )}
            >
              <div className="mt-0.5">
                {t.variant === "success" && <CheckCircle2 className="size-4 text-tier-green" />}
                {t.variant === "warning" && <AlertTriangle className="size-4 text-tier-yellow" />}
                {t.variant === "error" && <XCircle className="size-4 text-tier-red" />}
                {t.variant === "default" && <Info className="size-4 text-muted-foreground" />}
              </div>
              <div className="min-w-0 flex-1">
                <p className="text-sm font-medium leading-none">{t.title}</p>
                {t.description && (
                  <p className="mt-1 text-xs text-muted-foreground">{t.description}</p>
                )}
              </div>
              <button
                aria-label="Dismiss"
                className="rounded-sm opacity-60 hover:opacity-100"
                onClick={() => setItems((prev) => prev.filter((x) => x.id !== t.id))}
              >
                <X className="size-3.5" />
              </button>
            </motion.div>
          ))}
        </AnimatePresence>
      </div>
    </ToastContext.Provider>
  );
}

export function useToast() {
  const ctx = React.useContext(ToastContext);
  if (!ctx) throw new Error("useToast must be used inside ToastProvider");
  return ctx;
}
