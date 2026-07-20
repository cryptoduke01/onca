"use client";

import { AgentCard } from "@/components/agent-card";
import { HeroField } from "@/components/hero-field";
import { FadeUp, easeOut, motion, useReducedMotion } from "@/components/motion";
import {
  CustodySection,
  InstallSection,
  ProofSection,
  ToolsSection,
} from "@/components/sections";
import { REPO } from "@/lib/utils";

/**
 * Asymmetric hero: type left-biased on large screens, product card
 * offset right and larger. Motion is Y-only settles — content stays visible.
 */
export default function HomePage() {
  const reduce = useReducedMotion();

  return (
    <main>
      <section className="relative flex min-h-dvh flex-col justify-center overflow-hidden px-6 pb-20 pt-28 sm:px-8 lg:pt-32">
        <HeroField />

        <div className="relative z-10 mx-auto grid w-full max-w-6xl items-center gap-12 lg:grid-cols-[1fr_1.05fr] lg:gap-10 xl:gap-14">
          <div className="min-w-0 max-w-xl lg:max-w-none">
            <motion.h1
              className="text-[clamp(2.5rem,5.5vw,4.5rem)] font-semibold leading-[1.04] tracking-[-0.03em] text-ink"
              initial={reduce ? false : { y: 18 }}
              animate={{ y: 0 }}
              transition={{ duration: 0.6, ease: easeOut }}
            >
              The agent proposes.
              <br />
              <span className="text-signal">You dispose.</span>
            </motion.h1>

            <motion.p
              className="mt-6 max-w-[38ch] text-[1.08rem] leading-relaxed text-ink-dim"
              initial={reduce ? false : { y: 14 }}
              animate={{ y: 0 }}
              transition={{ duration: 0.55, delay: 0.06, ease: easeOut }}
            >
              Solana tools for ZeroClaw agents. They read the chain, or build a
              request a person signs. Never a key that can spend.
            </motion.p>

            <motion.div
              initial={reduce ? false : { y: 12 }}
              animate={{ y: 0 }}
              transition={{ duration: 0.5, delay: 0.12, ease: easeOut }}
            >
              <a
                href={REPO}
                target="_blank"
                rel="noopener noreferrer"
                className="mt-9 inline-flex min-h-11 items-center gap-2 rounded-xl bg-btn px-5 text-[0.95rem] font-medium text-btn-ink transition-colors duration-200 hover:bg-btn-hover"
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
            </motion.div>
          </div>

          <motion.div
            className="flex w-full min-w-0 justify-start lg:justify-end lg:pl-2"
            initial={reduce ? false : { y: 20 }}
            animate={{ y: 0 }}
            transition={{ duration: 0.65, delay: 0.1, ease: easeOut }}
          >
            <AgentCard />
          </motion.div>
        </div>
      </section>

      <FadeUp as="div" delay={0.02}>
        <CustodySection />
      </FadeUp>
      <FadeUp as="div" delay={0.04}>
        <ToolsSection />
      </FadeUp>
      <FadeUp as="div" delay={0.04}>
        <ProofSection />
      </FadeUp>
      <FadeUp as="div" delay={0.04}>
        <InstallSection />
      </FadeUp>
    </main>
  );
}
