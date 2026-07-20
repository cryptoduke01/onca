"use client";

import Link from "next/link";
import { GitHubIcon } from "@/components/github-icon";
import { FadeUp } from "@/components/motion";
import { REPO } from "@/lib/utils";

const X_URL = "https://x.com/dukedotsol";

const links = [
  { href: "/docs/", label: "Docs", external: false },
  {
    href: `${REPO}/blob/main/docs/custody.md`,
    label: "Custody",
    external: true,
  },
  { href: `${REPO}/blob/main/LICENSE`, label: "MIT", external: true },
];

/**
 * Sparse footer + oversized wordmark.
 * Credit: "Crafted by Duke" with @dukedotsol as the quiet handle behind the name.
 */
export function SiteFooter() {
  return (
    <footer className="relative mt-24 overflow-hidden border-t border-line bg-void">
      <div className="mx-auto max-w-5xl px-6 pb-4 pt-16 sm:px-8">
        <FadeUp className="flex flex-col gap-10 sm:flex-row sm:items-start sm:justify-between">
          <div className="max-w-sm">
            <p className="text-lg font-medium leading-snug tracking-tight text-ink sm:text-xl">
              Solana hands for a self-hosted agent. The agent proposes. You
              dispose.
            </p>
            <p className="mt-4 text-[0.92rem] text-ink-dim">
              Crafted by{" "}
              <a
                href={X_URL}
                target="_blank"
                rel="noopener noreferrer"
                className="group inline-flex items-baseline gap-1.5 transition-colors duration-200"
              >
                <span className="font-medium text-ink group-hover:text-signal">
                  Duke
                </span>
                <span className="data text-[0.78rem] text-ink-faint group-hover:text-ink-dim">
                  @dukedotsol
                </span>
              </a>
            </p>
          </div>

          <nav
            className="flex flex-wrap items-center gap-x-6 gap-y-3 sm:justify-end"
            aria-label="Footer"
          >
            <a
              href={REPO}
              target="_blank"
              rel="noopener noreferrer"
              className="inline-flex min-h-10 min-w-10 items-center justify-center text-ink-dim transition-colors duration-200 hover:text-ink"
              aria-label="Onca on GitHub"
            >
              <GitHubIcon className="h-[1.15rem] w-[1.15rem]" />
            </a>
            <a
              href={X_URL}
              target="_blank"
              rel="noopener noreferrer"
              className="inline-flex min-h-10 min-w-10 items-center justify-center text-ink-dim transition-colors duration-200 hover:text-ink"
              aria-label="@dukedotsol on X"
            >
              <svg
                viewBox="0 0 16 16"
                width="16"
                height="16"
                fill="currentColor"
                aria-hidden="true"
              >
                <path d="M12.6 1.5h2.2l-4.8 5.5 5.6 7.5h-4.4l-3.4-4.5-3.9 4.5H1.7l5.1-5.9L1.4 1.5h4.5l3.1 4.1 3.6-4.1Zm-.8 11.7h1.2L4.3 2.7H3L11.8 13.2Z" />
              </svg>
            </a>
            {links.map((link) =>
              link.external ? (
                <a
                  key={link.label}
                  href={link.href}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-[0.95rem] text-ink-dim transition-colors duration-200 hover:text-ink"
                >
                  {link.label}
                </a>
              ) : (
                <Link
                  key={link.label}
                  href={link.href}
                  className="text-[0.95rem] text-ink-dim transition-colors duration-200 hover:text-ink"
                >
                  {link.label}
                </Link>
              )
            )}
          </nav>
        </FadeUp>
      </div>

      <div
        className="pointer-events-none select-none px-4 pb-2 pt-10 text-center"
        aria-hidden="true"
      >
        <p
          className="font-semibold leading-none tracking-[-0.04em]"
          style={{
            fontSize: "clamp(5.5rem, 28vw, 18rem)",
            color: "var(--footer-mark)",
          }}
        >
          ONCA
        </p>
      </div>
    </footer>
  );
}
