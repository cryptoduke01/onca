"use client";

import {
  motion,
  useReducedMotion,
  type Variants,
} from "motion/react";
import type { ReactNode } from "react";

/** Shared easing — soft, not bouncy. */
export const easeOut = [0.16, 1, 0.3, 1] as const;

/**
 * Entrance that never hides content: only a small Y settle.
 * Always a motion.div (no polymorphic tag) so props stay type-safe.
 * If reduced motion is on, children render as a plain div.
 */
export function FadeUp({
  children,
  className,
  delay = 0,
}: {
  children: ReactNode;
  className?: string;
  delay?: number;
}) {
  const reduce = useReducedMotion();

  if (reduce) {
    return <div className={className}>{children}</div>;
  }

  return (
    <motion.div
      className={className}
      initial={{ y: 14 }}
      whileInView={{ y: 0 }}
      viewport={{ once: true, margin: "-8% 0px" }}
      transition={{ duration: 0.55, delay, ease: easeOut }}
    >
      {children}
    </motion.div>
  );
}

export const navVariants: Variants = {
  hidden: { y: -12 },
  show: {
    y: 0,
    transition: { duration: 0.45, ease: easeOut },
  },
};

export const sheetVariants: Variants = {
  closed: {
    opacity: 0,
    y: -8,
    transition: { duration: 0.18, ease: "easeIn" },
  },
  open: {
    opacity: 1,
    y: 0,
    transition: { duration: 0.28, ease: easeOut },
  },
};

export { motion, useReducedMotion };
