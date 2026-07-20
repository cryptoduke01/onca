"use client";

import { AnimatePresence } from "motion/react";
import { useState, type ReactNode } from "react";
import { easeOut, motion, useReducedMotion } from "@/components/motion";
import { cn } from "@/lib/utils";

type Tab = "pay" | "risk" | "watch";

const panes: Record<
  Tab,
  {
    title: string;
    subtitle: string;
    lines: { role: string; text: ReactNode; dim?: boolean }[];
  }
> = {
  pay: {
    title: "Charge table",
    subtitle: "solana-pay-request · T1",
    lines: [
      { role: "you", text: "charge table 4 for 25 USDC" },
      {
        role: "onca",
        text: (
          <>
            Payment request ·{" "}
            <strong className="font-semibold text-card-ink">25 USDC</strong> → 7xKX…gAsU
          </>
        ),
      },
      {
        role: "",
        text: "solana:7xKX…?amount=25&spl-token=EPjF…&memo=table%204",
        dim: true,
      },
      {
        role: "",
        text: "QR rendered in chat. Customer signs in their wallet.",
        dim: true,
      },
    ],
  },
  risk: {
    title: "Token risk",
    subtitle: "token-risk-check · T0",
    lines: [
      { role: "you", text: "is 9x…rug safe to accept?" },
      {
        role: "onca",
        text: (
          <>
            Token risk · <strong className="font-semibold text-card-bad">RED</strong> · 3 flags
          </>
        ),
      },
      {
        role: "",
        text: "permanent delegate set: can move tokens anytime",
        dim: true,
      },
      {
        role: "",
        text: "transfer fee 20% · top holder 90% of supply",
        dim: true,
      },
    ],
  },
  watch: {
    title: "Invoice watch",
    subtitle: "payment-watch · T0",
    lines: [
      { role: "cron", text: "watch invoice #412" },
      {
        role: "onca",
        text: (
          <>
            Paid · <strong className="font-semibold text-card-ok">25 USDC</strong> from 9WzD…AWWM
          </>
        ),
      },
      {
        role: "",
        text: "tx 5Q54…ge4j · exact amount, verified on-chain",
        dim: true,
      },
      {
        role: "",
        text: "every signature on the reference checked",
        dim: true,
      },
    ],
  },
};

const tabs: { id: Tab; label: string }[] = [
  { id: "pay", label: "pay" },
  { id: "risk", label: "risk" },
  { id: "watch", label: "watch" },
];

/**
 * Signature product artifact: larger window chrome with traffic lights,
 * real tool tabs, expanded body. User-requested mac-style border.
 */
export function AgentCard() {
  const [tab, setTab] = useState<Tab>("pay");
  const [copied, setCopied] = useState(false);
  const pane = panes[tab];
  const cmd = "cargo build --target wasm32-wasip2 --release";
  const reduce = useReducedMotion();

  async function copy() {
    try {
      await navigator.clipboard.writeText(cmd);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1400);
    } catch {
      /* clipboard may be blocked */
    }
  }

  return (
    <div
      className={cn(
        "w-full min-w-0 max-w-2xl overflow-hidden rounded-[1.15rem]",
        "border border-card-line bg-card",
        "shadow-[0_1px_0_rgba(255,255,255,0.55)_inset,0_28px_64px_-18px_rgba(0,0,0,0.55)]"
      )}
    >
      {/* Title bar with traffic lights */}
      <div className="flex flex-wrap items-center gap-3 border-b border-card-line px-4 py-3 sm:flex-nowrap sm:px-5">
        <div className="flex shrink-0 items-center gap-1.5" aria-hidden="true">
          <span className="h-3 w-3 rounded-full bg-[#ff5f57]" />
          <span className="h-3 w-3 rounded-full bg-[#febc2e]" />
          <span className="h-3 w-3 rounded-full bg-[#28c840]" />
        </div>
        <div className="min-w-0 flex-1 text-left">
          <p className="truncate text-[0.98rem] font-medium text-card-ink">
            {pane.title}
          </p>
          <p className="data truncate text-[0.72rem] text-card-muted">
            {pane.subtitle}
          </p>
        </div>
        <div
          className="flex w-full shrink-0 gap-0.5 rounded-lg bg-card-tab p-0.5 sm:w-auto"
          role="tablist"
          aria-label="Onca tools"
        >
          {tabs.map((t) => (
            <button
              key={t.id}
              type="button"
              role="tab"
              aria-selected={tab === t.id}
              onClick={() => setTab(t.id)}
              className={cn(
                "data relative min-h-9 rounded-md px-3 text-[0.78rem] transition-colors duration-200",
                tab === t.id
                  ? "text-card-ink"
                  : "text-card-muted hover:text-card-ink"
              )}
            >
              {tab === t.id && !reduce ? (
                <motion.span
                  layoutId="agent-tab"
                  className="absolute inset-0 rounded-md bg-card shadow-sm"
                  transition={{ type: "spring", stiffness: 420, damping: 34 }}
                />
              ) : tab === t.id ? (
                <span className="absolute inset-0 rounded-md bg-card shadow-sm" />
              ) : null}
              <span className="relative z-10">{t.label}</span>
            </button>
          ))}
        </div>
      </div>

      <div
        className="data min-h-[14rem] overflow-hidden px-5 py-5 text-left text-[0.9rem] leading-relaxed text-card-ink sm:min-h-[15.5rem] sm:px-6 sm:py-6 sm:text-[0.95rem]"
        role="tabpanel"
      >
        <AnimatePresence mode="wait" initial={false}>
          <motion.div
            key={tab}
            className="space-y-3"
            initial={reduce ? false : { y: 8 }}
            animate={{ y: 0 }}
            exit={reduce ? undefined : { y: -6 }}
            transition={{ duration: 0.22, ease: easeOut }}
          >
            {pane.lines.map((line, i) => (
              <p
                key={i}
                className={cn(
                  "[overflow-wrap:anywhere]",
                  line.dim && "pl-[3.6rem] text-card-faint",
                  !line.dim && "text-card-ink/90"
                )}
              >
                {line.role ? (
                  <span
                    className={cn(
                      "inline-block w-[3.6rem] shrink-0",
                      line.role === "onca" ? "text-card-signal" : "text-card-faint"
                    )}
                  >
                    {line.role}
                  </span>
                ) : null}
                {line.text}
              </p>
            ))}
          </motion.div>
        </AnimatePresence>
      </div>

      <div className="flex items-center gap-3 border-t border-card-line bg-card-foot px-4 py-3 pl-5 sm:px-5">
        <code className="data min-w-0 flex-1 truncate text-[0.8rem] text-card-muted sm:text-[0.84rem]">
          {cmd}
        </code>
        <button
          type="button"
          onClick={copy}
          className={cn(
            "data min-h-10 shrink-0 rounded-lg px-4 text-[0.78rem] transition-colors duration-200",
            "bg-card-ink text-white hover:opacity-90",
            copied && "bg-card-signal text-white hover:opacity-100"
          )}
          aria-label="Copy build command"
        >
          {copied ? "copied" : "copy"}
        </button>
      </div>
    </div>
  );
}
