"use client";

import { useEffect, useState } from "react";
import { motion, useSpring, useTransform } from "framer-motion";
import { fenToCny } from "@/lib/utils";

export function CountUp({
  value,
  format = (n) => n.toLocaleString(),
  className,
}: {
  value: number;
  format?: (n: number) => string;
  className?: string;
}) {
  const spring = useSpring(0, { mass: 0.6, stiffness: 110, damping: 18 });
  const display = useTransform(spring, (v) => format(Math.round(v)));
  const [, setReady] = useState(false);
  useEffect(() => {
    setReady(true);
    spring.set(value);
  }, [value, spring]);
  return <motion.span className={className}>{display}</motion.span>;
}

export function CurrencyCountUp({ fen, className }: { fen: number; className?: string }) {
  return <CountUp value={fen} format={fenToCny} className={className} />;
}
