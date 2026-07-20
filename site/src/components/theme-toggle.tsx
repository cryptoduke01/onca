"use client";

import { useTheme } from "@/components/theme-provider";
import { easeOut, motion, useReducedMotion } from "@/components/motion";
import { cn } from "@/lib/utils";

/** Compact theme switch — not the sun-moon pill. */
export function ThemeToggle({ className }: { className?: string }) {
  const { theme, toggle } = useTheme();
  const isLight = theme === "light";
  const reduce = useReducedMotion();

  return (
    <motion.button
      type="button"
      onClick={toggle}
      className={cn(
        "hover-wash inline-flex min-h-10 min-w-10 items-center justify-center rounded-full",
        "text-ink-dim hover:text-ink",
        className
      )}
      aria-label={isLight ? "Switch to dark mode" : "Switch to light mode"}
      title={isLight ? "Dark" : "Light"}
      whileHover={reduce ? undefined : { scale: 1.05 }}
      whileTap={reduce ? undefined : { scale: 0.95 }}
      transition={{ duration: 0.18, ease: easeOut }}
    >
      <motion.span
        key={theme}
        initial={reduce ? false : { rotate: -20, y: 2 }}
        animate={{ rotate: 0, y: 0 }}
        transition={{ duration: 0.28, ease: easeOut }}
        className="inline-flex"
      >
        {isLight ? (
          <svg width="16" height="16" viewBox="0 0 16 16" fill="none" aria-hidden="true">
            <path
              d="M13.5 8.6A5.5 5.5 0 1 1 7.4 2.5 4.2 4.2 0 0 0 13.5 8.6Z"
              stroke="currentColor"
              strokeWidth="1.4"
              strokeLinejoin="round"
            />
          </svg>
        ) : (
          <svg width="16" height="16" viewBox="0 0 16 16" fill="none" aria-hidden="true">
            <circle cx="8" cy="8" r="3.1" stroke="currentColor" strokeWidth="1.4" />
            <path
              d="M8 1.5v1.2M8 13.3v1.2M1.5 8h1.2M13.3 8h1.2M3.4 3.4l.85.85M11.75 11.75l.85.85M3.4 12.6l.85-.85M11.75 4.25l.85-.85"
              stroke="currentColor"
              strokeWidth="1.4"
              strokeLinecap="round"
            />
          </svg>
        )}
      </motion.span>
    </motion.button>
  );
}
