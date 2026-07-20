"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { AnimatePresence } from "motion/react";
import { useEffect, useId, useState } from "react";
import { GitHubIcon } from "@/components/github-icon";
import { Logo } from "@/components/logo";
import {
  easeOut,
  motion,
  navVariants,
  sheetVariants,
  useReducedMotion,
} from "@/components/motion";
import { ThemeToggle } from "@/components/theme-toggle";
import { REPO, cn } from "@/lib/utils";

const links = [
  { href: "/#tools", label: "Tools" },
  { href: "/#custody", label: "Custody" },
  { href: "/#proof", label: "Proof" },
  { href: "/docs/", label: "Docs" },
];

export function SiteNav() {
  const pathname = usePathname();
  const onDocs = pathname?.startsWith("/docs");
  const [open, setOpen] = useState(false);
  const panelId = useId();
  const reduce = useReducedMotion();

  useEffect(() => {
    setOpen(false);
  }, [pathname]);

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };
    document.addEventListener("keydown", onKey);
    document.body.style.overflow = "hidden";
    return () => {
      document.removeEventListener("keydown", onKey);
      document.body.style.overflow = "";
    };
  }, [open]);

  return (
    <header className="pointer-events-none fixed inset-x-0 top-0 z-50 flex justify-center px-3 pt-4 sm:px-4 sm:pt-5">
      <motion.div
        className={cn(
          "pointer-events-auto flex w-full max-w-4xl items-center justify-between gap-3",
          "rounded-full border border-nav-border bg-nav px-3 py-2 pl-4",
          "shadow-[inset_0_1px_0_color-mix(in_srgb,var(--color-ink)_6%,transparent)] backdrop-blur-xl backdrop-saturate-150"
        )}
        variants={reduce ? undefined : navVariants}
        initial={reduce ? false : "hidden"}
        animate="show"
      >
        <Link
          href="/"
          className="shrink-0 rounded-full transition-opacity duration-200 hover:opacity-90"
          aria-label="Onca home"
        >
          <Logo />
        </Link>

        <nav className="hidden items-center gap-1 md:flex" aria-label="Primary">
          {links.map((link) => {
            const active = link.href === "/docs/" && onDocs;
            return (
              <Link
                key={link.href}
                href={link.href}
                className={cn(
                  "relative rounded-full px-3 py-1.5 text-[0.92rem] transition-colors duration-200",
                  active ? "text-ink" : "text-ink-dim hover:text-ink"
                )}
              >
                {link.label}
              </Link>
            );
          })}
        </nav>

        <div className="flex items-center gap-0.5">
          <ThemeToggle />
          {/* Always icon — never the word "GitHub" in the bar */}
          <motion.a
            href={REPO}
            target="_blank"
            rel="noopener noreferrer"
            className="github-pill inline-flex min-h-10 min-w-10 items-center justify-center rounded-full text-ink"
            aria-label="Onca on GitHub"
            whileHover={reduce ? undefined : { scale: 1.04 }}
            whileTap={reduce ? undefined : { scale: 0.96 }}
            transition={{ duration: 0.18, ease: easeOut }}
          >
            <GitHubIcon className="h-[1.15rem] w-[1.15rem]" />
          </motion.a>
          <button
            type="button"
            className="hover-wash inline-flex min-h-10 min-w-10 items-center justify-center rounded-full text-ink-dim transition-colors duration-200 hover:text-ink md:hidden"
            aria-expanded={open}
            aria-controls={panelId}
            aria-label={open ? "Close menu" : "Open menu"}
            onClick={() => setOpen((v) => !v)}
          >
            {open ? (
              <svg width="18" height="18" viewBox="0 0 18 18" fill="none" aria-hidden="true">
                <path
                  d="M4.5 4.5 13.5 13.5M13.5 4.5 4.5 13.5"
                  stroke="currentColor"
                  strokeWidth="1.6"
                  strokeLinecap="round"
                />
              </svg>
            ) : (
              <svg width="18" height="18" viewBox="0 0 18 18" fill="none" aria-hidden="true">
                <path
                  d="M3.5 5.5h11M3.5 9h11M3.5 12.5h11"
                  stroke="currentColor"
                  strokeWidth="1.6"
                  strokeLinecap="round"
                />
              </svg>
            )}
          </button>
        </div>
      </motion.div>

      <AnimatePresence>
        {open ? (
          <>
            <motion.button
              type="button"
              key="backdrop"
              className="pointer-events-auto fixed inset-0 z-40 bg-black/40 md:hidden"
              aria-label="Close menu"
              onClick={() => setOpen(false)}
              initial={reduce ? false : { opacity: 0 }}
              animate={{ opacity: 1 }}
              exit={{ opacity: 0 }}
              transition={{ duration: 0.2 }}
            />
            <motion.div
              id={panelId}
              key="sheet"
              className="pointer-events-auto fixed inset-x-3 top-[4.5rem] z-50 overflow-hidden rounded-2xl border border-nav-border bg-surface shadow-lg md:hidden"
              variants={reduce ? undefined : sheetVariants}
              initial={reduce ? false : "closed"}
              animate="open"
              exit="closed"
            >
              <nav className="flex flex-col p-2" aria-label="Mobile">
                {links.map((link, i) => (
                  <motion.div
                    key={link.href}
                    initial={reduce ? false : { y: 8 }}
                    animate={{ y: 0 }}
                    transition={{ delay: 0.04 * i, duration: 0.28, ease: easeOut }}
                  >
                    <Link
                      href={link.href}
                      onClick={() => setOpen(false)}
                      className="block rounded-xl px-4 py-3 text-[1.02rem] text-ink transition-colors duration-200 hover:bg-surface-2"
                    >
                      {link.label}
                    </Link>
                  </motion.div>
                ))}
                <motion.div
                  initial={reduce ? false : { y: 8 }}
                  animate={{ y: 0 }}
                  transition={{ delay: 0.16, duration: 0.28, ease: easeOut }}
                >
                  <a
                    href={REPO}
                    target="_blank"
                    rel="noopener noreferrer"
                    onClick={() => setOpen(false)}
                    className="inline-flex min-h-12 w-full items-center gap-3 rounded-xl px-4 py-3 text-[1.02rem] text-ink transition-colors duration-200 hover:bg-surface-2"
                    aria-label="Onca on GitHub"
                  >
                    <GitHubIcon className="h-5 w-5 text-ink" />
                    <span className="sr-only">GitHub</span>
                  </a>
                </motion.div>
              </nav>
            </motion.div>
          </>
        ) : null}
      </AnimatePresence>
    </header>
  );
}
