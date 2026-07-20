import { AgentCard } from "@/components/agent-card";
import { HeroField } from "@/components/hero-field";
import {
  CustodySection,
  InstallSection,
  ProofSection,
  ToolsSection,
} from "@/components/sections";
import { REPO } from "@/lib/utils";

/**
 * Asymmetric hero: type left-biased on large screens, product card
 * offset right and larger. Not a dead-center stack.
 */
export default function HomePage() {
  return (
    <main>
      <section className="relative flex min-h-dvh flex-col justify-center overflow-hidden px-6 pb-20 pt-28 sm:px-8 lg:pt-32">
        <HeroField />

        <div className="relative z-10 mx-auto grid w-full max-w-6xl items-center gap-12 lg:grid-cols-[1fr_1.05fr] lg:gap-10 xl:gap-14">
          <div className="max-w-xl lg:max-w-none">
            <h1 className="text-[clamp(2.5rem,5.5vw,4.5rem)] font-semibold leading-[1.04] tracking-[-0.03em] text-ink">
              The agent proposes.
              <br />
              <span className="text-signal">You dispose.</span>
            </h1>

            <p className="mt-6 max-w-[38ch] text-[1.08rem] leading-relaxed text-ink-dim">
              Solana tools for ZeroClaw agents. They read the chain, or build a
              request a person signs. Never a key that can spend.
            </p>

            <a
              href={REPO}
              target="_blank"
              rel="noopener noreferrer"
              className="mt-9 inline-flex min-h-11 items-center gap-2 rounded-xl bg-btn px-5 text-[0.95rem] font-medium text-btn-ink transition-colors duration-150 hover:bg-btn-hover"
            >
              Read the code
              <svg
                viewBox="0 0 16 16"
                width="14"
                height="14"
                fill="none"
                aria-hidden="true"
                className="opacity-80"
              >
                <path
                  d="M4.5 11.5 11.5 4.5M6 4.5h5.5V10"
                  stroke="currentColor"
                  strokeWidth="1.6"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                />
              </svg>
            </a>
          </div>

          <div className="flex w-full justify-start lg:justify-end lg:pl-2">
            <AgentCard />
          </div>
        </div>
      </section>

      <CustodySection />
      <ToolsSection />
      <ProofSection />
      <InstallSection />
    </main>
  );
}
