import Link from "next/link";
import { REPO } from "@/lib/utils";

const links = [
  { href: REPO, label: "GitHub", external: true },
  { href: "/docs/", label: "Docs", external: false },
  {
    href: `${REPO}/blob/main/docs/custody.md`,
    label: "Custody",
    external: true,
  },
  { href: `${REPO}/blob/main/LICENSE`, label: "MIT", external: true },
];

/**
 * Sparse footer + oversized wordmark. No tagline pile-up.
 */
export function SiteFooter() {
  return (
    <footer className="relative mt-24 overflow-hidden border-t border-line bg-void">
      <div className="mx-auto max-w-5xl px-6 pb-4 pt-16 sm:px-8">
        <div className="flex flex-col gap-10 sm:flex-row sm:items-start sm:justify-between">
          <p className="max-w-sm text-lg font-medium leading-snug tracking-tight text-ink sm:text-xl">
            Solana hands for a self-hosted agent. The agent proposes. You
            dispose.
          </p>

          <nav
            className="flex flex-wrap gap-x-6 gap-y-3 sm:justify-end"
            aria-label="Footer"
          >
            {links.map((link) =>
              link.external ? (
                <a
                  key={link.label}
                  href={link.href}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-[0.95rem] text-ink-dim transition-colors duration-150 hover:text-ink"
                >
                  {link.label}
                </a>
              ) : (
                <Link
                  key={link.label}
                  href={link.href}
                  className="text-[0.95rem] text-ink-dim transition-colors duration-150 hover:text-ink"
                >
                  {link.label}
                </Link>
              )
            )}
          </nav>
        </div>
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
