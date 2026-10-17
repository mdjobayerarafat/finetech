"use client";

import { motion } from "framer-motion";
import { ReactNode } from "react";

/* ------------------------- Fade / stagger primitives ------------------------- */

export function FadeIn({
  children,
  delay = 0,
  y = 8,
  className,
}: {
  children: ReactNode;
  delay?: number;
  y?: number;
  className?: string;
}) {
  return (
    <motion.div
      initial={{ opacity: 0, y }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.5, ease: [0.16, 1, 0.3, 1], delay }}
      className={className}
    >
      {children}
    </motion.div>
  );
}

export function Stagger({
  children,
  delayChildren = 0.05,
  staggerChildren = 0.06,
}: {
  children: ReactNode;
  delayChildren?: number;
  staggerChildren?: number;
}) {
  return (
    <motion.div
      initial="hidden"
      animate="show"
      variants={{
        hidden: { opacity: 1 },
        show: { opacity: 1, transition: { delayChildren, staggerChildren } },
      }}
    >
      {children}
    </motion.div>
  );
}

export function StaggerItem({
  children,
  className,
}: {
  children: ReactNode;
  className?: string;
}) {
  return (
    <motion.div
      variants={{
        hidden: { opacity: 0, y: 10 },
        show: { opacity: 1, y: 0, transition: { duration: 0.4, ease: [0.16, 1, 0.3, 1] } },
      }}
      className={className}
    >
      {children}
    </motion.div>
  );
}

/* ------------------------- Animated background ------------------------- */

/**
 * Decorative, non-interactive background.
 * Three soft aurora blobs + a slow gradient sheet + faint dot grid.
 */
export function AuroraBackdrop() {
  return (
    <div aria-hidden className="pointer-events-none fixed inset-0 -z-10 overflow-hidden">
      {/* base gradient sheet */}
      <div className="absolute inset-0 bg-[linear-gradient(180deg,hsl(212_70%_5%)_0%,hsl(212_60%_9%)_45%,hsl(212_65%_6%)_100%)]" />

      {/* aurora blobs (Novara deep-navy + teal palette) */}
      <div
        className="absolute -top-40 left-1/2 h-[720px] w-[1200px] -translate-x-1/2 rounded-full opacity-80 blur-3xl animate-aurora"
        style={{
          background:
            "radial-gradient(ellipse at center, hsl(186 95% 55% / 0.35), transparent 60%)",
        }}
      />
      <div
        className="absolute top-1/4 -right-40 h-[560px] w-[820px] rounded-full opacity-70 blur-3xl animate-aurora"
        style={{
          background:
            "radial-gradient(ellipse at center, hsl(220 80% 60% / 0.30), transparent 60%)",
          animationDelay: "-5s",
        }}
      />
      <div
        className="absolute bottom-0 -left-40 h-[600px] w-[900px] rounded-full opacity-60 blur-3xl animate-aurora"
        style={{
          background:
            "radial-gradient(ellipse at center, hsl(260 60% 55% / 0.22), transparent 60%)",
          animationDelay: "-9s",
        }}
      />

      {/* horizon glow at the bottom of the viewport */}
      <div className="absolute inset-x-0 bottom-0 h-[40vh] bg-horizon" />

      {/* dot grid overlay */}
      <div className="absolute inset-0 bg-dots opacity-60" />

      {/* top vignette */}
      <div className="absolute inset-x-0 top-0 h-40 bg-[linear-gradient(180deg,hsl(212_70%_4%/.6),transparent)]" />
    </div>
  );
}

/**
 * Mountain silhouette + glowing horizon (decorative).
 * Drop inside any hero card with `relative overflow-hidden rounded-[inherit]`.
 */
export function MountainHero() {
  return (
    <div aria-hidden className="pointer-events-none absolute inset-0 overflow-hidden rounded-[inherit]">
      <div className="absolute inset-0 bg-[linear-gradient(180deg,hsl(212_70%_8%/.0)_0%,hsl(212_70%_4%/.7)_100%)]" />
      <svg
        viewBox="0 0 800 220"
        preserveAspectRatio="none"
        className="absolute inset-x-0 bottom-0 h-2/3 w-full"
      >
        <defs>
          <linearGradient id="mtn-fill" x1="0" x2="0" y1="0" y2="1">
            <stop offset="0%" stopColor="hsl(212 60% 30%)" stopOpacity="0.9" />
            <stop offset="100%" stopColor="hsl(212 70% 6%)" stopOpacity="1" />
          </linearGradient>
          <linearGradient id="mtn-stroke" x1="0" x2="1" y1="0" y2="0">
            <stop offset="0%" stopColor="hsl(186 95% 65% / 0.0)" />
            <stop offset="50%" stopColor="hsl(186 95% 65% / 0.5)" />
            <stop offset="100%" stopColor="hsl(186 95% 65% / 0.0)" />
          </linearGradient>
        </defs>

        {/* back range */}
        <path
          d="M0 160 L70 110 L120 140 L190 80 L260 130 L330 95 L400 150 L470 100 L540 140 L610 90 L690 130 L800 110 L800 220 L0 220 Z"
          fill="url(#mtn-fill)"
          opacity="0.55"
        />
        {/* front range with glowing ridge */}
        <path
          d="M0 200 L80 150 L150 180 L230 130 L310 175 L380 145 L460 190 L540 150 L620 185 L700 155 L800 195 L800 220 L0 220 Z"
          fill="hsl(212 70% 8%)"
        />
        <path
          d="M0 200 L80 150 L150 180 L230 130 L310 175 L380 145 L460 190 L540 150 L620 185 L700 155 L800 195"
          fill="none"
          stroke="url(#mtn-stroke)"
          strokeWidth="1.2"
        />
      </svg>

      {/* glow halo at the horizon */}
      <div
        className="absolute -bottom-10 left-1/2 h-40 w-[120%] -translate-x-1/2 rounded-full opacity-90 blur-3xl animate-pulse-glow"
        style={{
          background:
            "radial-gradient(ellipse at center, hsl(186 95% 60% / 0.45), transparent 60%)",
        }}
      />
    </div>
  );
}

/**
 * Animated line chart that draws itself in, with a glowing head dot.
 */
export function PortfolioLine({
  width = 520,
  height = 180,
  months = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"],
  values = [38, 42, 39, 47, 52, 49, 58, 64, 61, 70, 76, 82],
}: {
  width?: number;
  height?: number;
  months?: string[];
  values?: number[];
}) {
  const min = Math.min(...values);
  const max = Math.max(...values);
  const padX = 20;
  const padY = 24;
  const stepX = (width - padX * 2) / (values.length - 1);
  const pts = values.map((v, i) => {
    const x = padX + i * stepX;
    const y = padY + (1 - (v - min) / (max - min)) * (height - padY * 2);
    return [x, y] as const;
  });
  const path = pts
    .map(([x, y], i) => (i === 0 ? `M${x},${y}` : `L${x},${y}`))
    .join(" ");
  const area = `${path} L${pts[pts.length - 1][0]},${height} L${pts[0][0]},${height} Z`;

  return (
    <div className="relative w-full">
      <svg viewBox={`0 0 ${width} ${height}`} preserveAspectRatio="none" className="h-44 w-full">
        <defs>
          <linearGradient id="pl-stroke" x1="0" x2="1" y1="0" y2="0">
            <stop offset="0%" stopColor="hsl(186 95% 65%)" />
            <stop offset="100%" stopColor="hsl(195 90% 60%)" />
          </linearGradient>
          <linearGradient id="pl-fill" x1="0" x2="0" y1="0" y2="1">
            <stop offset="0%" stopColor="hsl(186 95% 60% / 0.45)" />
            <stop offset="100%" stopColor="hsl(186 95% 60% / 0)" />
          </linearGradient>
          <filter id="pl-glow" x="-20%" y="-20%" width="140%" height="140%">
            <feGaussianBlur stdDeviation="3" result="b" />
            <feMerge>
              <feMergeNode in="b" />
              <feMergeNode in="SourceGraphic" />
            </feMerge>
          </filter>
        </defs>

        {/* grid lines */}
        {[0.25, 0.5, 0.75].map((p) => (
          <line
            key={p}
            x1={padX}
            x2={width - padX}
            y1={padY + p * (height - padY * 2)}
            y2={padY + p * (height - padY * 2)}
            stroke="hsl(200 30% 35% / 0.25)"
            strokeDasharray="2 4"
          />
        ))}

        {/* area fill */}
        <path d={area} fill="url(#pl-fill)" />

        {/* line stroke (draws in) */}
        <motion.path
          d={path}
          fill="none"
          stroke="url(#pl-stroke)"
          strokeWidth="2.5"
          strokeLinecap="round"
          strokeLinejoin="round"
          filter="url(#pl-glow)"
          initial={{ pathLength: 0 }}
          animate={{ pathLength: 1 }}
          transition={{ duration: 1.8, ease: [0.65, 0, 0.35, 1] }}
        />

        {/* moving head dot */}
        <motion.circle
          r="10"
          fill="hsl(186 95% 60% / 0.35)"
          initial={false}
          animate={{ cx: pts[pts.length - 1][0], cy: pts[pts.length - 1][1] }}
          transition={{ duration: 1.8, ease: [0.65, 0, 0.35, 1] }}
        />
        <motion.circle
          r="4"
          fill="hsl(186 95% 70%)"
          stroke="hsl(212 70% 8%)"
          strokeWidth="2"
          initial={false}
          animate={{ cx: pts[pts.length - 1][0], cy: pts[pts.length - 1][1] }}
          transition={{ duration: 1.8, ease: [0.65, 0, 0.35, 1] }}
        />
      </svg>

      <div className="mt-2 flex justify-between text-[10px] uppercase tracking-wider text-muted-foreground/80">
        {months.map((m) => (
          <span key={m}>{m}</span>
        ))}
      </div>
    </div>
  );
}