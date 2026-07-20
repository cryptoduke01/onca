import type { Metadata } from "next";
import Link from "next/link";
import { REPO } from "@/lib/utils";

export const metadata: Metadata = {
  title: "Docs",
  description:
    "Documentation for the Onca Solana tools: custody, install, build notes, and each plugin.",
};

const guides = [
  {
    title: "Custody and the threat model",
    href: `${REPO}/blob/main/docs/custody.md`,
    body: "The custody ladder, where each guardrail lives, and the worst case for every secret in the suite.",
  },
  {
    title: "Install and run",
    href: `${REPO}/blob/main/docs/install.md`,
    body: "Build a component, place it next to its manifest, set the config, run the agent.",
  },
  {
    title: "Notes on wasm32-wasip2",
    href: `${REPO}/blob/main/docs/wasm-notes.md`,
    body: "What was hard about compiling Solana tooling into a WIT component, and what solved it.",
  },
  {
    title: "Writing style",
    href: `${REPO}/blob/main/docs/writing-style.md`,
    body: "Simplified Technical English (ASD-STE100) rules and term list for the suite.",
  },
];

const plugins = [
  {
    name: "solana-pay-request",
    href: `${REPO}/tree/main/plugins/solana-pay-request`,
    tier: "T1 · build",
  },
  {
    name: "token-risk-check",
    href: `${REPO}/tree/main/plugins/token-risk-check`,
    tier: "T0 · read",
  },
  {
    name: "payment-watch",
    href: `${REPO}/tree/main/plugins/payment-watch`,
    tier: "T0 · read",
  },
  {
    name: "onca-core",
    href: `${REPO}/tree/main/crates/onca-core`,
    tier: "core",
  },
];

export default function DocsPage() {
  return (
    <main className="mx-auto max-w-3xl px-6 pb-24 pt-32 sm:px-8">
      <Link
        href="/"
        className="text-sm text-ink-dim transition-colors duration-150 hover:text-ink"
      >
        ← Home
      </Link>

      <h1 className="mt-6 text-[clamp(2.2rem,5vw,3.25rem)] font-semibold tracking-tight text-ink">
        Documentation
      </h1>
      <p className="mt-4 max-w-xl text-[1.05rem] leading-relaxed text-ink-dim">
        Canonical docs live in the repository, written in Simplified Technical
        English. Start with custody if you care about safety, or install if you
        want to run the tools today.
      </p>

      <h2 className="mt-16 text-2xl font-semibold tracking-tight text-ink">
        Guides
      </h2>
      <ul className="mt-6 grid gap-3 sm:grid-cols-2">
        {guides.map((g) => (
          <li key={g.title}>
            <a
              href={g.href}
              target="_blank"
              rel="noopener noreferrer"
              className="edge block h-full rounded-2xl p-5 transition-colors duration-150 hover:bg-surface-2"
            >
              <h3 className="text-[1.05rem] font-medium text-ink">{g.title}</h3>
              <p className="mt-2 text-sm leading-relaxed text-ink-dim">{g.body}</p>
              <span className="data mt-4 inline-block text-[0.72rem] text-signal">
                read
              </span>
            </a>
          </li>
        ))}
      </ul>

      <h2 className="mt-16 text-2xl font-semibold tracking-tight text-ink">
        Per plugin
      </h2>
      <p className="mt-2 text-ink-dim">
        Each plugin has its own README with a threat model and a fail-closed
        transcript.
      </p>
      <ul className="mt-6 overflow-hidden rounded-2xl border border-line">
        {plugins.map((p) => (
          <li key={p.name} className="border-b border-line last:border-b-0">
            <a
              href={p.href}
              target="_blank"
              rel="noopener noreferrer"
              className="flex min-h-14 items-center justify-between gap-4 px-5 py-4 transition-colors duration-150 hover:bg-surface"
            >
              <span className="data text-ink">{p.name}</span>
              <span className="data shrink-0 text-[0.78rem] text-ink-faint">
                {p.tier}
              </span>
            </a>
          </li>
        ))}
      </ul>
    </main>
  );
}
