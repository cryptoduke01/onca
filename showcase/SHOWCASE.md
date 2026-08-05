# Onca — a manipulation-resistant sensor oracle on Solana

**Independent DePIN nodes attest their sensor readings on Solana under a human
tap-to-approve; a mesh aggregates them into one value a prediction market can
settle on, and no minority of lying nodes can move it. Self-hosted on a ZeroClaw
agent you own. The agent never holds a key.**

Builder: [@dukedotsol](https://x.com/dukedotsol) · Code:
[github.com/cryptoduke01/onca](https://github.com/cryptoduke01/onca) ·
Reproduce: [SETUP.md](SETUP.md)

## The problem

A prediction market that settles on real-world data — *"did Lagos exceed 30°C
yesterday?"* — is only as trustworthy as its data source. One sensor is one
throat to choke: whoever owns it can lie, the market pays out wrong, and that is
exactly the oracle-manipulation failure that has burned real markets. A usable
oracle needs data that is attributable to a real device, tamper-evident, and
resistant to any single source lying. Onca is that oracle, running on hardware
you own.

## What it does — two halves

**Write side (custody T1).** You message the agent in plain language: *"attest a
reading from dht11-a: 23.4 C, sequence 4."* It replies with an approval card
showing the exact reading and Approve / Deny buttons. On Approve, the
`depin-attest` plugin builds an **unsigned** Solana transaction that writes the
reading on-chain as a signed memo — with operator-set reading bounds and a
monotonic replay guard enforced in code. A separate signer, run by a human
holding the device key the agent never sees, signs it and submits. The agent
proposes; a human disposes.

**Read side (custody T0).** One node is not an oracle. `onca-oracle` reads many
independent nodes' attestations back off-chain and settles on the **median**,
dropping outliers, and refusing to settle below quorum. On top of the per-round
median it keeps a **persistent reputation** for each node: a node whose readings
keep getting rejected loses trust and is eventually frozen out of the aggregate
entirely, so a node that lied wildly cannot later slip a plausible-looking value
back into the median. That single aggregated value is what a market consumes, and
corrupting a minority of nodes changes nothing.

**Machine-commerce side (custody T0).** The trusted value is also sold over
[x402](https://x402.org), the pay-per-call HTTP protocol: `onca-x402` answers a
request with `402 Payment Required`, the caller pays a micro-fee on Solana, and
on a verified payment the server returns the mesh value. `onca-resolve` is the
other end — a prediction-market resolver that pays, reads the trusted number
back, and settles a market YES/NO. The oracle sells its reading and an agent buys
it, with no key ever leaving either tool.

## It runs — proofs on devnet

Human-approved attestations, signed by the device key, finalized on Solana. Each
started as a Telegram approval tap and was landed by the human-run signer:

- [`onca:attest s=dht11-a v=23.7 u=C seq=7`](https://explorer.solana.com/tx/TntBSvQGSuzVscVU633q6Fjhnfbqk7jBRzfTz48r2ZwfeswLGqQjDbV8zjeNsaZrmxPLXGzmyVVXShQj9DXg2GE?cluster=devnet)
- [`onca:attest s=dht11-a v=24.1 u=C seq=2`](https://explorer.solana.com/tx/5ATMiYLVGunuuZUa1F2svs1cKaoezm7NYkEsQeCzzozpymAr4Gvq9A1jHUAT3jTE6r4hTGfV4cXFeyPi9ZpEQX9z?cluster=devnet)
- [`onca:attest s=dht11-a v=22.8 u=C seq=3`](https://explorer.solana.com/tx/4zcKbaX8WrPr4vEVWsmFYZ84wyEeiuE1HvpwskWbUM2X6FsTe3Yd1cShuPutPKt2ujgwy1HR6JR1nkeUyNJA3QEN?cluster=devnet)

Device: `BMpwFSKbLJvPpK4yo5EoqBiQUDxt9NdgFRToXJpiphrC`

**The oracle rejecting a lying node, live on devnet.** A 4-node mesh; node 4 is
an adversary signing 999°C *directly* — bypassing the honest agent and its
approval gate, which is the real-world threat. `onca-oracle` read all four
on-chain, dropped the outlier, and froze the repeat liar on trust:

```
node 3xQ3…  reading 23.4      node Ghri…  reading 23.6
node AxRK…  reading 23.1      node BtpD…  reading 999

node 3xQ3…  trust 1.00        node Ghri…  trust 1.00
node AxRK…  trust 1.00        node BtpD…  trust 0.00  FROZEN OUT

rejected this round: BtpD…=999
ORACLE VALUE: 23.4 C  (3 of 4 nodes trusted & agree)
```

The same value served through the agent on Telegram: *"the trusted temperature
is 23.4°C, the median of 3 trusted nodes; 1 outlier rejected."* Corrupting one
node does nothing — that is the property a market needs, and the reason this is
an oracle and not just a sensor.

**The paid oracle settling a market, live on devnet.** `onca-resolve` paid the
x402 fee on Solana, read the trusted value back, and settled:

```
paid 1000000 lamports → x402 tx 29D5t8t4…BM63r2zS
MARKET      Lagos temp > 25C
ORACLE      23.4 C  (3 nodes agreed, 1 rejected)
CONDITION   temp gt 25
SETTLEMENT  NO
```

**Settling a REAL live market on the mesh, not a single source.** This is Kaue's
exact ask, closed against a market that exists today. `onca-market` reads a live
**weather** market from Jupiter's keyless prediction API — Polymarket liquidity
routed onto Solana, in São Paulo — and settles which outcome the mesh picks,
instead of the single oracle the market ships with (the Polymarket
single-source manipulation Kaue named):

```
MARKET   POLY-798942  "Highest temperature in Sao Paulo on August 6?"  (live, 11 buckets)
MESH     node BtpD signs 999 → FROZEN OUT;  3 nodes agree
SETTLE   Onca mesh 23.4 C → winning outcome "23°C"  — no single source set it
```

The market names its own single source. Its resolution rule, verbatim from
Jupiter: *"the highest temperature recorded at the São Paulo-Guarulhos
International Airport Station."* One station, one reading, one point to
manipulate. Onca settles the same market on a mesh of independent nodes instead.

**And the agent does this itself, on the channel — not a script.** A ZeroClaw
Tier-1 skill ([`skills/settle-weather-market`](../skills/settle-weather-market/SKILL.md))
composes the built-in `http_request` (read the live market) with the
`mesh_oracle` tool (the trusted value). Asked in Telegram to settle the São Paulo
market, the agent returns:

```
Highest temperature in Sao Paulo on August 6?  (POLY-798942)
23.4°C (3 agreed, 1 rejected)
winning bucket 23°C — marketId POLY-3350728
No single source set this; it takes a majority of independent nodes to move it.
```

To *trade* the outcome (custody T1), `onca-trade` asks Jupiter to build the
order: `POST /orders` returns an **unsigned** transaction a human signs (in a
wallet or with `onca-signer`). Without USDC it fails closed at Jupiter
("Insufficient funds") — the agent built the request, held no key, and submitted
nothing.

## Custody ladder

| Tier | Meaning | In Onca |
|---|---|---|
| **T0 Read** | reads and reports; a key at most | `onca-oracle` and `mesh-oracle` (the aggregate), `onca-x402` (sells it), `onca-resolve` (buys + settles), `token-risk-check`, `payment-watch` |
| **T1 Build** | builds an unsigned request a human signs | `depin-attest`, `solana-pay-request` |
| **T2 Sign** | signs and sends | **not shipped** — no key ever lives in the agent |

T0 and T1 are the sweet spot the bounty names. Onca never reaches T2 inside the
agent; the signer is a separate human-run binary.

## Threat model — defense in depth

The threat is not only prompt injection. It is an LLM in the loop that might
**fabricate or substitute a reading**, and a **malicious node operator** who
bypasses the honest agent entirely. Four layers answer it:

1. **Code-enforced bounds + replay guard** in pure Rust the model cannot override.
2. **Oracle guidance**: the tool forbids the model rounding, substituting, or
   inventing a reading. It also forbids the model deciding on its own whether a
   reading is a duplicate — it must call the tool every time and only relay a
   refusal the tool actually returns, so the model can neither fabricate a value
   nor silently skip an attestation from its own (possibly wrong) memory.
3. **The human approval gate**: every attestation shows the exact value for a tap.
4. **The mesh median + reputation**: a node that lies past all of that — signing a
   spoof with its own key — is outvoted and dropped, and if it keeps lying it is
   frozen out of the aggregate for good.

### Fail-closed transcript (required)

Even at *full* autonomy (approval gate off), a spoofed out-of-bounds reading dies
in the operator's code and the model reports the refusal instead of retrying with
a passing value:

```
User:  Attest a reading from sensor dht11-a of 999 C, sequence 5.
Agent: The sensor reading of 999 C is above the configured maximum of 85 C,
       so it has been refused.
```

## ZeroClaw features used

- **Self-hosted daemon** — my machine, my model, my keys.
- **Built-in Telegram channel** — the agent lives in a DM; approvals render as
  native inline keyboard buttons.
- **Supervised autonomy + forced approval** (`always_ask`) — every attestation
  pauses for a human tap that shows the exact reading.
- **WASM plugin system** (`wasm32-wasip2`, the `tool-plugin` WIT world) in a
  source-built host (`--features plugins-wasm-cranelift`).
- **Config secrets** encrypted at rest / env-injected, **least-privilege tool
  scoping**, the `groq` provider slot, and a **Helius** RPC endpoint via config.

## What I built

- **`onca-core`** — a minimal `wasm32-wasip2` Solana engine: base58, pubkey,
  JSON-RPC over a transport trait, hand-assembled transactions (legacy message,
  SPL Memo, System Transfer, durable nonce), and the mesh aggregation with a
  reputation layer. Devnet-verified; 26 host tests.
- **`depin-attest`** (Tier 3 plugin, **T1**) — reading → unsigned attestation,
  with code-enforced bounds, a monotonic replay guard, and a host-stamped time.
- **`mesh-oracle`** (Tier 3 plugin, **T0**) — reads the mesh and settles on the
  median, outliers dropped, quorum required. Fault-tolerant per-node reads.
- **`onca-oracle`** — the standalone, reliable mesh reader, with persistent
  per-node reputation that freezes out repeat liars (the aggregate a market
  consumes).
- **`onca-signer`** — the human-disposes side: holds the device key, rebuilds the
  same attestation with `onca-core`, signs (ed25519), submits.
- **`onca-x402`** — sells the trusted value over x402: `402 Payment Required`,
  verify the Solana payment landed, then return the mesh reading. Replay-guarded.
- **`onca-resolve`** — the consumer: pays the x402 fee (a real transfer built and
  signed with `onca-core`), reads the value back, and settles a market YES/NO.
- **`onca-market`** — settles a **live** Solana prediction market: reads a real
  weather market from Jupiter's keyless prediction API (Polymarket liquidity on
  Solana) and maps the mesh value to the winning outcome bucket. Tier 1 read.
- **`onca-trade`** (**T1**) — builds an *unsigned* Jupiter order on the mesh's
  winning outcome (`POST /orders`), for a human to sign. Fails closed without
  USDC; holds no key.
- **`settle-weather-market` skill** (**Tier 1**, no compiled code) — the agent
  itself composes the built-in `http_request` with the `mesh_oracle` tool to
  settle the live market on-channel. Correct layering: the read is a skill, not a
  plugin. Proven end to end through `zeroclaw agent`.
- **ESP32 firmware + serial bridge** — a DHT11 node printing `onca:reading` lines
  the pipeline ingests. The software loop runs identically with a typed reading,
  so the demo does not depend on hardware.

## What I hit at the component boundary (documented)

The bounty says `wit/v0` is experimental and to write down what you hit. I did:

- **The WIT ABI moved under the same version.** The host exports
  `zeroclaw:plugin/logging@0.1.0` with a `memory-audit` enum variant my vendored
  copy lacked (no `.frozen` marker). The component was *discovered but failed to
  instantiate* — "no matching implementation in the linker", `registered: 0` —
  until I synced the WIT. Then it loaded and executed.
- **The dependency wall is a door.** The modular `solana-*` crates compile for
  `wasm32-wasip2` now, so hand-rolling the transaction engine is a deliberate
  minimal + runtime-verified choice, not a necessity (I verified the bytes on
  devnet `simulateTransaction`).
- **`waki` is flaky across several sequential requests in one invocation**
  (stale-connection HTTP protocol errors on later calls). The mesh read tolerates
  it — retry per node, drop an unreachable node, quorum protects — and the
  standalone `onca-oracle` (blocking `ureq`) is the reliable path.
- **Helius free tier rejects JSON-RPC batching** (403), so the mesh reads are
  sequential against a real endpoint (each single call is a reliable 200).

## Reproduce in an evening

Full runbook in [SETUP.md](SETUP.md); the entire agent is
[config.example.toml](config.example.toml) — four sections, no secrets inline.
Engine: [`crates/onca-core`](../crates/onca-core); plugins:
[`plugins/depin-attest`](../plugins/depin-attest),
[`plugins/mesh-oracle`](../plugins/mesh-oracle); tools:
[`tools/onca-signer`](../tools/onca-signer),
[`tools/onca-oracle`](../tools/onca-oracle),
[`tools/onca-x402`](../tools/onca-x402),
[`tools/onca-resolve`](../tools/onca-resolve).

## Roadmap

- **Live ESP32** as a real physical node in the mesh (firmware + bridge written).
- **Geographic, staked mesh** — independent operators run nodes, stake, and earn;
  the reputation layer that already freezes liars grows into on-chain staking
  where a proven liar is slashed, not just frozen.
- **USDC settlement** — the x402 demo prices in native SOL so it runs on the
  funded devnet keys; production swaps the asset for SPL USDC (the payment-verify
  shape, a balance delta credited to the treasury, is identical).
- **Squads multisig dispose** — the agent proposes, a multisig approves from a
  phone, replacing the single-key signer.
