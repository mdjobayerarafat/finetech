"use client";

import { motion } from "framer-motion";
import { cn } from "@/lib/utils";

export function TierBadge({
  tier,
  className,
  size = "md",
}: {
  tier: "Green" | "Yellow" | "Red" | string | null | undefined;
  className?: string;
  size?: "sm" | "md" | "lg";
}) {
  const color =
    tier === "Green"
      ? "bg-tier-green/15 text-tier-green ring-tier-green/30"
      : tier === "Yellow"
      ? "bg-tier-yellow/15 text-tier-yellow ring-tier-yellow/30"
      : tier === "Red"
      ? "bg-tier-red/15 text-tier-red ring-tier-red/30"
      : "bg-muted text-muted-foreground ring-border";
  const sizes = {
    sm: "h-5 px-1.5 text-[10px]",
    md: "h-6 px-2 text-[11px]",
    lg: "h-7 px-2.5 text-xs",
  };
  return (
    <motion.span
      initial={{ scale: 0.92, opacity: 0 }}
      animate={{ scale: 1, opacity: 1 }}
      transition={{ type: "spring", damping: 20, stiffness: 280 }}
      className={cn(
        "inline-flex items-center gap-1 rounded-full font-semibold uppercase tracking-wider ring-1",
        color,
        sizes[size],
        className
      )}
    >
      <span
        className={cn(
          "size-1.5 rounded-full",
          tier === "Green" && "bg-tier-green",
          tier === "Yellow" && "bg-tier-yellow",
          tier === "Red" && "bg-tier-red",
          (!tier || (tier !== "Green" && tier !== "Yellow" && tier !== "Red")) && "bg-muted-foreground"
        )}
      />
      {tier ?? "—"}
    </motion.span>
  );
}
