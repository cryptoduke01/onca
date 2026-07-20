"use client";

import {
  motion,
  useReducedMotion,
  type HTMLMotionProps,
  type Variants,
} from "motion/react";
import type { ReactNode } from "react";

/** Shared easing — soft, not bouncy. */
export const easeOut = [0.16, 1, 0.3, 1] as const;

/**
 * Entrance that never hides content: only a small Y settle.
 * If reduced motion is on, children render as a plain div.
 */
export function FadeUp({
  children,
  className,
  delay = 0,
  as: Tag = "div",
  ...rest
}: {
  children: ReactNode;
  className?: string;
  delay?: number;
  as?: "div" | "section" | "li" | "header" | "footer";
} & Omit<HTMLMotionProps<"div">, "children" | "className">) {
  const reduce = useReducedMotion();
  const Comp = motion[Tag];

  if (reduce) {
    const Static = Tag;
    return <Static className={className}>{children}</Static>;
  }

  return (
    <Comp
      className={className}
      initial={{ y: 14 }}
      whileInView={{ y: 0 }}
      viewport={{ once: true, margin: "-8% 0px" }}
      transition={{ duration: 0.55, delay, ease: easeOut }}
      {...rest}
    >
      {children}
    </Comp>
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
