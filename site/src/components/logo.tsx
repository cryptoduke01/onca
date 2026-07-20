import { cn } from "@/lib/utils";

type LogoProps = {
  className?: string;
  markClassName?: string;
  wordmark?: boolean;
};

/** Three swept strokes — the Onca claw. Bare mark, no tile behind it. */
export function Logo({ className, markClassName, wordmark = true }: LogoProps) {
  return (
    <span className={cn("inline-flex items-center gap-2.5", className)}>
      <svg
        viewBox="0 0 32 32"
        className={cn("h-7 w-7 shrink-0", markClassName)}
        fill="none"
        aria-hidden="true"
      >
        <g
          stroke="currentColor"
          strokeWidth="2.4"
          strokeLinecap="round"
          className="text-signal"
        >
          <path d="M9 6.5C12 12.5 12 19 10 25.5" />
          <path d="M16 5.5C19.2 12.5 19.2 20 16 26.5" />
          <path d="M23 6.5C26 12.5 26 19 22 25.5" />
        </g>
      </svg>
      {wordmark ? (
        <span className="text-[1.2rem] font-semibold tracking-tight text-ink">
          Onca
        </span>
      ) : null}
    </span>
  );
}
