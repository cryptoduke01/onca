"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { useEffect, useId, useState } from "react";
import { Logo } from "@/components/logo";
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
      <div
        className={cn(
          "pointer-events-auto flex w-full max-w-4xl items-center justify-between gap-3",
          "rounded-full border border-nav-border bg-nav px-3 py-2 pl-4",
          "shadow-[inset_0_1px_0_color-mix(in_srgb,var(--color-ink)_6%,transparent)] backdrop-blur-xl backdrop-saturate-150"
        )}
      >
        <Link
          href="/"
          className="shrink-0 rounded-full transition-opacity duration-150 hover:opacity-90"
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
                  "rounded-full px-3 py-1.5 text-[0.92rem] transition-colors duration-150",
                  active ? "text-ink" : "text-ink-dim hover:text-ink"
                )}
              >
                {link.label}
              </Link>
            );
          })}
        </nav>

        <div className="flex items-center gap-1">
          <ThemeToggle />
          <a
            href={REPO}
            target="_blank"
            rel="noopener noreferrer"
            className="github-pill hidden min-h-10 items-center rounded-full px-3.5 text-[0.86rem] text-ink transition-colors duration-150 sm:inline-flex"
          >
            GitHub
          </a>
          <button
            type="button"
            className="hover-wash inline-flex min-h-10 min-w-10 items-center justify-center rounded-full text-ink-dim transition-colors duration-150 hover:text-ink md:hidden"
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
      </div>

      {/* Mobile sheet */}
      <div
        id={panelId}
        className={cn(
          "pointer-events-auto fixed inset-x-3 top-[4.5rem] z-50 overflow-hidden rounded-2xl border border-nav-border bg-surface shadow-lg md:hidden",
          "transition-[opacity,transform] duration-200 ease-out",
          open
            ? "translate-y-0 opacity-100"
            : "pointer-events-none -translate-y-2 opacity-0"
        )}
        hidden={!open}
      >
        <nav className="flex flex-col p-2" aria-label="Mobile">
          {links.map((link) => (
            <Link
              key={link.href}
              href={link.href}
              onClick={() => setOpen(false)}
              className="rounded-xl px-4 py-3 text-[1.02rem] text-ink transition-colors duration-150 hover:bg-surface-2"
            >
              {link.label}
            </Link>
          ))}
          <a
            href={REPO}
            target="_blank"
            rel="noopener noreferrer"
            onClick={() => setOpen(false)}
            className="rounded-xl px-4 py-3 text-[1.02rem] text-ink transition-colors duration-150 hover:bg-surface-2"
          >
            GitHub
          </a>
        </nav>
      </div>

      {open ? (
        <button
          type="button"
          className="pointer-events-auto fixed inset-0 z-40 bg-black/40 md:hidden"
          aria-label="Close menu"
          onClick={() => setOpen(false)}
        />
      ) : null}
    </header>
  );
}
