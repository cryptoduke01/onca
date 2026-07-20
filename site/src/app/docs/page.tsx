import type { Metadata } from "next";
import Link from "next/link";
import type { ReactNode } from "react";
import { REPO } from "@/lib/utils";

export const metadata: Metadata = {
  title: "Docs",
  description:
    "Onca documentation: the custody model and threat model, an install guide, and notes on building Solana tooling for wasm32-wasip2.",
};

const sections = [
  { id: "custody", label: "Custody & threat model" },
  { id: "install", label: "Install & run" },
  { id: "wasm", label: "Building for wasm" },
  { id: "plugins", label: "Per plugin" },
];

const plugins = [
  { name: "solana-pay-request", tier: "T1 · build", href: `${REPO}/tree/main/plugins/solana-pay-request` },
  { name: "token-risk-check", tier: "T0 · read", href: `${REPO}/tree/main/plugins/token-risk-check` },
  { name: "payment-watch", tier: "T0 · read", href: `${REPO}/tree/main/plugins/payment-watch` },
  { name: "depin-attest", tier: "T1 · build", href: `${REPO}/tree/main/plugins/depin-attest` },
  { name: "onca-core", tier: "core", href: `${REPO}/tree/main/crates/onca-core` },
];

function Code({ children }: { children: ReactNode }) {
  return (
    <pre className="data mt-5 overflow-x-auto rounded-xl border border-line bg-void p-4 text-[0.82rem] leading-[1.7] text-ink sm:p-5">
      {children}
    </pre>
  );
}

function H2({ id, children }: { id: string; children: ReactNode }) {
  return (
    <h2
      id={id}
      className="scroll-mt-28 text-[clamp(1.6rem,3.2vw,2.1rem)] font-semibold tracking-tight text-ink"
    >
      {children}
    </h2>
  );
}

function H3({ children }: { children: ReactNode }) {
  return <h3 className="mt-10 text-lg font-medium tracking-tight text-ink">{children}</h3>;
}

function P({ children }: { children: ReactNode }) {
  return <p className="mt-4 text-[1.02rem] leading-relaxed text-ink-dim">{children}</p>;
}

const cell = "py-3 pr-5 align-top text-[0.95rem]";
const head = "data py-2 pr-5 text-[0.74rem] font-normal uppercase tracking-wide text-ink-faint";

export default function DocsPage() {
  return (
    <main className="mx-auto max-w-3xl px-6 pb-28 pt-32 sm:px-8">
      <Link
        href="/"
        className="text-sm text-ink-dim transition-colors duration-150 hover:text-ink"
      >
        ← Home
      </Link>

      <h1 className="mt-6 text-[clamp(2.2rem,5vw,3.25rem)] font-semibold tracking-tight text-ink">
        Documentation
      </h1>
      <p className="mt-4 max-w-xl text-[1.08rem] leading-relaxed text-ink-dim">
        Everything you need to understand and run Onca, on one page. The custody
        model first, then how to install the tools, then the notes on building
        Solana tooling for wasm.
      </p>

      <nav className="mt-8 flex flex-wrap gap-x-6 gap-y-2" aria-label="On this page">
        {sections.map((s) => (
          <a
            key={s.id}
            href={`#${s.id}`}
            className="data text-[0.82rem] text-ink-dim transition-colors duration-150 hover:text-ink"
          >
            {s.label}
          </a>
        ))}
      </nav>

      <div className="mt-14 h-px bg-line" />

      {/* ── Custody ── */}
      <section className="pt-14">
        <H2 id="custody">Custody &amp; the threat model</H2>
        <P>
          An agent joins a private key to a language model, and that model reads
          text you do not control: chat messages, mail, web pages. An attacker can
          hide an instruction in that text. If the agent can sign and send, one
          hidden instruction can move your funds. That attack is prompt injection.
        </P>
        <P>
          You cannot remove prompt injection from a model. So Onca limits what a
          successful injection can reach. The rule is simple: the agent proposes,
          and a person, a multisig, or a limited session key disposes.
        </P>

        <H3>The custody ladder</H3>
        <P>The bounty defines four tiers. Onca ships only the two safe ones.</P>
        <div className="mt-6 overflow-x-auto">
          <table className="w-full border-collapse text-left">
            <thead>
              <tr className="border-b border-line">
                <th className={head}>Tier</th>
                <th className={head}>Name</th>
                <th className={head}>The tool can</th>
                <th className={head}>Secrets held</th>
              </tr>
            </thead>
            <tbody className="text-ink-dim">
              <tr className="border-b border-line">
                <td className={`${cell} data text-ink`}>T0</td>
                <td className={`${cell} text-ink`}>Read</td>
                <td className={cell}>read the chain and report</td>
                <td className={cell}>an RPC key at most</td>
              </tr>
              <tr className="border-b border-line">
                <td className={`${cell} data text-ink`}>T1</td>
                <td className={`${cell} text-ink`}>Build</td>
                <td className={cell}>return an unsigned request for a person to sign</td>
                <td className={cell}>none</td>
              </tr>
              <tr className="border-b border-line opacity-55">
                <td className={`${cell} data text-ink-faint line-through decoration-bad/50`}>T2</td>
                <td className={cell}>Sign &amp; send</td>
                <td className={cell}>sign and submit</td>
                <td className={cell}>a scoped session key</td>
              </tr>
            </tbody>
          </table>
        </div>
        <P>
          A T2 tool is the tier where one successful injection empties a wallet. It
          is possible to build one safely, with hard spend caps, a mint allowlist
          in the plugin, a session key that holds little, and an approval gate. But
          none of the Onca tools need that risk to do their job, so none of them
          take it. This is a deliberate boundary, not an unfinished feature.
        </P>

        <H3>Where the guardrails live</H3>
        <P>
          Each guardrail lives in the pure Rust core of its plugin, not in the
          prompt, and runs on every call. The model never sees the config that
          limits it and cannot turn it off. To pass a guardrail you would have to
          change the source and rebuild the component.
        </P>
        <div className="mt-6 overflow-x-auto">
          <table className="w-full border-collapse text-left">
            <thead>
              <tr className="border-b border-line">
                <th className={head}>Plugin</th>
                <th className={head}>Guardrail</th>
              </tr>
            </thead>
            <tbody className="text-ink-dim">
              <tr className="border-b border-line">
                <td className={`${cell} data text-ink`}>solana-pay-request</td>
                <td className={cell}>max amount, mint allowlist, address check, memo encoding</td>
              </tr>
              <tr className="border-b border-line">
                <td className={`${cell} data text-ink`}>token-risk-check</td>
                <td className={cell}>verdict from chain facts only; refuse a malformed mint</td>
              </tr>
              <tr className="border-b border-line">
                <td className={`${cell} data text-ink`}>payment-watch</td>
                <td className={cell}>amount checked in base units; scan every signature</td>
              </tr>
            </tbody>
          </table>
        </div>

        <H3>How each tool fails closed</H3>
        <P>
          Every tool ships a test that sends a hostile input and checks that the
          tool refuses. The unsafe path returns &ldquo;no,&rdquo; never
          &ldquo;maybe.&rdquo;
        </P>
        <ul className="mt-4 space-y-3 text-[1.02rem] leading-relaxed text-ink-dim">
          <li>
            <span className="data text-ink">solana-pay-request</span> refuses an
            amount over the cap, a token off the allowlist, and an invalid recipient.
          </li>
          <li>
            <span className="data text-ink">token-risk-check</span> returns red for a
            honeypot, whatever the message claims about it.
          </li>
          <li>
            <span className="data text-ink">payment-watch</span> returns underpaid for
            dust and pending for a failed transaction. It never returns paid from a
            message alone.
          </li>
        </ul>

        <H3>What a secret can reach</H3>
        <P>The worst case for every secret in the suite. None of them can spend funds.</P>
        <div className="mt-6 overflow-x-auto">
          <table className="w-full border-collapse text-left">
            <thead>
              <tr className="border-b border-line">
                <th className={head}>Secret</th>
                <th className={head}>Held by</th>
                <th className={head}>Worst case if leaked</th>
              </tr>
            </thead>
            <tbody className="text-ink-dim">
              <tr className="border-b border-line">
                <td className={cell}>RPC URL with API key</td>
                <td className={`${cell} data`}>token-risk-check, payment-watch</td>
                <td className={cell}>read access to a node, plus RPC quota use</td>
              </tr>
              <tr className="border-b border-line">
                <td className={cell}>none</td>
                <td className={`${cell} data`}>solana-pay-request</td>
                <td className={cell}>nothing to leak</td>
              </tr>
            </tbody>
          </table>
        </div>
        <P>
          The RPC URL can carry an API key, so no tool writes it to a log or an
          error message. An error reports the HTTP status code, never the URL.
        </P>
      </section>

      <div className="mt-16 h-px bg-line" />

      {/* ── Install ── */}
      <section className="pt-14">
        <H2 id="install">Install &amp; run</H2>
        <P>For an operator running a ZeroClaw agent who wants to add the Onca tools.</P>

        <H3>Before you start</H3>
        <ul className="mt-4 space-y-2 text-[1.02rem] leading-relaxed text-ink-dim">
          <li>A Rust toolchain, 1.80 or later, and the <span className="data text-ink">wasm32-wasip2</span> target.</li>
          <li>A ZeroClaw build with a wasm plugin backend.</li>
          <li>A Solana JSON-RPC endpoint of your own; the two read tools need it.</li>
        </ul>
        <Code>rustup target add wasm32-wasip2</Code>

        <H3>Build a component</H3>
        <P>Each build writes a <span className="data text-ink">.wasm</span> file under the plugin&rsquo;s target directory.</P>
        <Code>
{`cd plugins/solana-pay-request
cargo build --target wasm32-wasip2 --release
# → target/wasm32-wasip2/release/solana_pay_request.wasm`}
        </Code>
        <P>Do the same for <span className="data text-ink">token-risk-check</span> and <span className="data text-ink">payment-watch</span>.</P>

        <H3>Place it and configure</H3>
        <P>
          Copy the <span className="data text-ink">.wasm</span> next to its
          <span className="data text-ink"> manifest.toml</span> in your ZeroClaw
          plugins directory, then enable plugins and add a section per tool. The
          host hands each plugin only its own section, because the manifest asks
          for <span className="data text-ink">config_read</span>.
        </P>
        <Code>
{`[plugins]
enabled = true

[[plugins.entries.solana-pay-request]]
label = "Bar do Zé"
allowed_mints = "USDC"
max_amount = "100"

[[plugins.entries.token-risk-check]]
rpc_url = "https://your-rpc-endpoint/?api-key=…"

[[plugins.entries.payment-watch]]
rpc_url = "https://your-rpc-endpoint/?api-key=…"`}
        </Code>

        <H3>Run it, use it</H3>
        <P>
          Start ZeroClaw with a wasm plugin backend. Then
          <span className="data text-ink"> solana-pay-request</span> and
          <span className="data text-ink"> token-risk-check</span> answer a chat
          request directly &mdash; &ldquo;charge table 4 for 25 USDC,&rdquo; &ldquo;is
          this mint safe?&rdquo; <span className="data text-ink">payment-watch</span>
          {" "}works best on a cron SOP that polls an open invoice and posts to your
          channel when it clears.
        </P>
        <Code>zeroclaw --features plugins-wasm,plugins-wasm-cranelift</Code>
      </section>

      <div className="mt-16 h-px bg-line" />

      {/* ── wasm ── */}
      <section className="pt-14">
        <H2 id="wasm">Building for wasm32-wasip2</H2>
        <P>
          The hard part was never the Solana logic. It was compiling anything
          Solana-shaped into a WIT component. The notes that cost the most time:
        </P>

        <H3>solana-sdk does not belong here</H3>
        <P>
          <span className="data text-ink">solana-sdk</span> and
          <span className="data text-ink"> solana-client</span> expect a full
          operating system: sockets, threads, getrandom, a socket RPC client. A
          sandboxed component has none of that, and forcing them to build bloats
          the artifact. So <span className="data text-ink">onca-core</span> hand-writes
          the small surface the plugins need &mdash; address parsing, a JSON-RPC
          message, amount math &mdash; on <span className="data text-ink">serde</span>,
          {" "}<span className="data text-ink">serde_json</span>, and
          <span className="data text-ink"> bs58</span>, all pure Rust that builds for
          the target unchanged.
        </P>

        <H3>Keep HTTP out of the core</H3>
        <P>
          <span className="data text-ink">waki</span>, the blocking wasi:http client,
          exists only on wasm; if the core imported it, host tests would fail to
          build. So the core declares an <span className="data text-ink">RpcTransport</span>
          {" "}trait and makes no call itself. The wasm shim supplies
          <span className="data text-ink"> waki</span>; the tests supply a mock. That
          one seam is what puts the whole RPC layer under
          <span className="data text-ink"> cargo test</span> with no network.
        </P>
        <Code>
{`[target.'cfg(target_family = "wasm")'.dependencies]
waki = { version = "0.5.1", features = ["json"] }`}
        </Code>

        <H3>The output is a component, not a module</H3>
        <P>
          Built with <span className="data text-ink">wit-bindgen</span> and
          <span className="data text-ink"> export!</span>, the binary starts
          <span className="data text-ink"> 00 61 73 6d 0d 00 01 00</span> &mdash; the
          <span className="data text-ink"> 0d</span> is the component-model layer, so
          no <span className="data text-ink">cargo component</span> post-step is
          needed. The release profile uses
          <span className="data text-ink"> opt-level = &quot;s&quot;</span>, lto, and
          strip to keep each component small.
        </P>
      </section>

      <div className="mt-16 h-px bg-line" />

      {/* ── Per plugin ── */}
      <section className="pt-14">
        <H2 id="plugins">Per plugin</H2>
        <P>Each plugin has its own README with a threat model and a fail-closed transcript.</P>
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
                <span className="data shrink-0 text-[0.78rem] text-ink-faint">{p.tier}</span>
              </a>
            </li>
          ))}
        </ul>
      </section>

      <p className="mt-16 border-t border-line pt-8 text-[0.98rem] leading-relaxed text-ink-dim">
        This page is the summary. The full source, every test, the per-plugin
        READMEs, and the raw docs live in{" "}
        <a
          href={REPO}
          target="_blank"
          rel="noopener noreferrer"
          className="text-signal transition-colors duration-150 hover:text-signal-deep"
        >
          the repository on GitHub
        </a>
        .
      </p>
    </main>
  );
}
