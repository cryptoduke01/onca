# Onca — a manipulation-resistant sensor oracle for prediction markets

**Repo:** https://github.com/cryptoduke01/onca
**Site:** https://onca.run
**Video (2:22):** <VIDEO_LINK>
**Reproduce:** [showcase/SETUP.md](https://github.com/cryptoduke01/onca/blob/main/showcase/SETUP.md)
**Builder:** [@dukedotsol](https://x.com/dukedotsol)

---

A prediction market is only as honest as the number it settles on.

I went looking at the weather markets currently live on Solana through Jupiter's
prediction API, which is Polymarket liquidity routed onto Solana. I picked one:
"Highest temperature in Sao Paulo on August 6?" Eleven outcome buckets, real
volume on it. Then I read its resolution rule, and it says, word for word, that
it settles on "the highest temperature recorded at the São Paulo-Guarulhos
International Airport Station."

One station. One reading. One thing to compromise, and the payout moves.

That is not a hypothetical failure. It is the oracle-manipulation shape that has
burned real markets. So I built the other half: an oracle that no single source
can move, running on a self-hosted ZeroClaw agent, where the agent never holds a
key that can spend.

## What it does

Onca is a suite of Solana tools for a self-hosted agent. The agent can attest a
sensor reading, run a mesh oracle over many nodes, take a payment, check a token
for rug traits, and watch for an invoice landing. Every one of them either reads
the chain or builds a request a human signs. None of them can move money.

The flagship use case is the oracle, and it has two halves.

**The write side.** You message the agent in Telegram in plain language: "attest
a reading from dht11-a: 24 C, sequence 12." It comes back with an approval card
showing the exact reading and Approve / Deny buttons. When you tap Approve, the
`depin-attest` plugin builds an *unsigned* Solana transaction that writes the
reading on-chain as a signed memo. Reading bounds and a monotonic replay guard
are enforced in Rust, not in the prompt. Then the agent hands you the exact
command to land it, and a separate signer that holds the device key, which the
agent has never seen, signs and submits it.

**The read side.** One node is not an oracle. `onca-oracle` reads many
independent nodes' attestations back off-chain and settles on the median, drops
outliers, and refuses to settle below quorum. On top of the per-round median it
keeps persistent reputation per node, so a node whose readings keep getting
rejected loses trust and eventually gets frozen out of the aggregate entirely.
That matters, because otherwise a node that lied wildly once could later slip a
plausible-looking value back into the median.

Ask the bot to settle the market and it does the whole thing on-channel:

```
Onca Oracle · Settlement
Highest temperature in Sao Paulo on August 6?
https://polymarket.com/event/highest-temperature-in-sao-paulo-on-august-6-2026

Mesh · 4 independent nodes
3xQ3…  23.4°C
Ghri…  23.6°C
AxRK…  23.1°C
BtpD…   999°C  frozen

Trusted value · 23.4°C · 3 agreed, 1 rejected
Winning outcome · 23°C

No single source set this. Corrupt a minority, the median holds.
```

Node four is an adversary. It signs 999°C directly to the chain with its own
key, bypassing the honest agent and its approval gate entirely, which is the
realistic attack. It does not matter. The median drops it and the reputation
layer freezes it out for good.

One detail I want to be straight about, because it is a real distinction: if the
market has not closed yet, the card says **Preview**, not Settlement. The day's
high is not final until the market closes, so calling that a settlement would be
a lie. The mechanism is identical either way. Only the word changes.

## Who it's for

Anyone who has to publish a number that other people's money depends on.
Prediction market operators and resolvers first, since that is the sharpest
version of the problem. After that it is the same shape for any DePIN network
that reports physical readings on-chain, and for any market that settles on
weather, energy, or air quality.

The person actually running it is an operator with a machine, not a protocol
team. That is the whole point of it being self-hosted.

## Which ZeroClaw features it uses

- **Self-hosted daemon.** My machine, my model, my keys.
- **The built-in Telegram channel.** The agent lives in a DM and approvals render
  as native inline keyboard buttons.
- **Supervised autonomy with forced approval** (`always_ask`), so every
  attestation pauses for a human tap that shows the exact reading.
- **Skills**, which is where the market read lives. This is deliberate. The
  read is a Tier 1 skill composing the built-in `http_request` tool, not a
  compiled plugin, because it does not need to be one.
- **The WASM plugin system** (`wasm32-wasip2`, the `tool-plugin` WIT world) in a
  source-built host with `--features plugins-wasm-cranelift`, used only where a
  guarantee has to live in code the model cannot talk its way past.
- **Config secrets** encrypted at rest or injected from env, **least-privilege
  tool scoping**, and a Helius RPC endpoint through config.

## What I had to build

- **`onca-core`** — a minimal `wasm32-wasip2` Solana engine: base58, pubkey,
  JSON-RPC over a transport trait, hand-assembled transactions (legacy message,
  SPL Memo, System Transfer, durable nonce), and the mesh aggregation with the
  reputation layer. 47 host tests green.
- **`depin-attest`** (plugin, T1) — reading to unsigned attestation, with
  code-enforced bounds, a monotonic replay guard, and a host-stamped time.
- **`mesh-oracle`** (plugin, T0) — reads the mesh and settles on the median,
  outliers dropped, quorum required, tolerant of a node being unreachable.
- **`onca-oracle`** — the standalone mesh reader with the persistent reputation
  that freezes out repeat liars.
- **`onca-signer`** — the human-disposes side. Holds the device key, rebuilds the
  same attestation, signs it, submits it.
- **`onca-x402`** and **`onca-resolve`** — the trusted value sold over x402, and
  the resolver that pays the micro-fee, reads the value back, and settles a
  market YES/NO. The oracle sells its reading and an agent buys it, and no key
  leaves either tool.
- **`onca-market`** — settles a live Jupiter/Polymarket weather market on the
  mesh instead of its single source.
- **`onca-trade`** (T1) — builds an unsigned Jupiter order on the winning
  outcome for a human to sign. Without USDC it fails closed at Jupiter, which is
  exactly what you want to see.
- **`settle-weather-market` skill** (Tier 1, no compiled code) — the agent
  composing `http_request` with the mesh read, on-channel.
- **ESP32 + DHT11 firmware and a serial bridge** that prints `onca:reading`
  lines the pipeline ingests. Being honest about status: the software loop runs
  identically with a typed reading, so the demo does not depend on the hardware
  being plugged in. Live physical node is roadmap, not a claim.

## Custody tier

| Tier | Meaning | In Onca |
|---|---|---|
| T0 Read | reads and reports, an RPC key at most | `onca-oracle`, `mesh-oracle`, `onca-x402`, `onca-resolve`, `token-risk-check`, `payment-watch` |
| T1 Build | builds an unsigned request a human signs | `depin-attest`, `solana-pay-request`, `onca-trade` |
| T2 Sign | signs and sends | not shipped. No key ever lives in the agent. |

The signer is a separate human-run binary. There is no code path where the agent
can sign anything, which is a design boundary and not an unfinished feature.

## Threat model

The threat is not only prompt injection. There are two others that matter more
here: an LLM in the loop that fabricates or substitutes a reading, and a
malicious node operator who ignores the honest agent entirely. Four layers:

1. **Code-enforced bounds and a replay guard** in Rust the model cannot override.
2. **Tool guidance** that forbids the model rounding, substituting, or inventing
   a reading, and forbids it deciding on its own whether something is a
   duplicate. It has to call the tool every time and can only relay a refusal
   the tool actually returned. Otherwise the model could skip an attestation
   from its own faulty memory.
3. **The human approval gate.** Every attestation shows the exact value for a tap.
4. **The mesh median and reputation.** A node that lies past all of that, by
   signing a spoof with its own key, gets outvoted and dropped, and frozen out if
   it keeps going.

Fail-closed, with the approval gate turned off entirely:

```
User:  Attest a reading from sensor dht11-a of 999 C, sequence 5.
Agent: The sensor reading of 999 C is above the configured maximum of 85 C,
       so it has been refused.
```

The refusal happens in the operator's code. The model reports it instead of
retrying with a value that would pass.

**Third parties, named honestly**, since trusting them is part of the tier. The
model provider sees every prompt but can never sign, because no tool it can call
holds a key. A lying RPC could hide or fake an attestation, but it would have to
lie consistently across the batch and still land inside tolerance to move the
median. Jupiter's API is read-only market data, so if it lied we would settle the
wrong market, never the wrong value. No third party holds a key on Onca's behalf.

## Things I hit at the component boundary

The bounty asks you to write down what you hit with `wit/v0`, so:

- **The WIT ABI moved under the same version.** The host exports
  `zeroclaw:plugin/logging@0.1.0` with a `memory-audit` enum variant my vendored
  copy did not have. The component was discovered but failed to instantiate, with
  "no matching implementation in the linker" and `registered: 0`, until I synced
  the WIT from the runtime repo rather than the plugins repo. Worth knowing that
  those two can drift.
- **The dependency wall is a door now.** The modular `solana-*` crates compile
  for `wasm32-wasip2`, so hand-rolling the transaction engine was a deliberate
  minimal choice, not a necessity. I verified the bytes on devnet
  `simulateTransaction`.
- **`waki` is flaky across several sequential requests in one invocation**, with
  stale-connection protocol errors on later calls. The mesh read tolerates it by
  retrying per node and letting quorum cover an unreachable one, and the reliable
  path for the live demo moved to the host's `http_request`.
- **Helius free tier rejects JSON-RPC batching** with a 403, so mesh reads go
  sequential against that endpoint.

## Reproduce it

Full runbook in [showcase/SETUP.md](https://github.com/cryptoduke01/onca/blob/main/showcase/SETUP.md).
The entire agent is [config.example.toml](https://github.com/cryptoduke01/onca/blob/main/showcase/config.example.toml),
four sections, no secrets inline. Everything else is in the repo: the engine in
`crates/onca-core`, the plugins in `plugins/`, the skills in `skills/`, the tools
in `tools/`.

If you have a machine and an evening, you can stand this up, point it at four
devnet keypairs, and watch it freeze out the liar yourself.
