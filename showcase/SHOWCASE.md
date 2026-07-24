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
dropping outliers, and refusing to settle below quorum. That single aggregated
value is what a market consumes — and corrupting a minority of nodes changes
nothing.

## It runs — proofs on devnet

Human-approved attestations, signed by the device key, finalized on Solana:

- [`onca:attest s=dht11-a v=24.1 u=C seq=2`](https://explorer.solana.com/tx/5ATMiYLVGunuuZUa1F2svs1cKaoezm7NYkEsQeCzzozpymAr4Gvq9A1jHUAT3jTE6r4hTGfV4cXFeyPi9ZpEQX9z?cluster=devnet)
- [`onca:attest s=dht11-a v=22.8 u=C seq=3`](https://explorer.solana.com/tx/4zcKbaX8WrPr4vEVWsmFYZ84wyEeiuE1HvpwskWbUM2X6FsTe3Yd1cShuPutPKt2ujgwy1HR6JR1nkeUyNJA3QEN?cluster=devnet)

Device: `BMpwFSKbLJvPpK4yo5EoqBiQUDxt9NdgFRToXJpiphrC`

**The oracle rejecting a lying node, live on devnet.** A 4-node mesh; node 4 is
an adversary signing 999°C *directly* — bypassing the honest agent and its
approval gate, which is the real-world threat. `onca-oracle` read all four
on-chain and settled:

```
node 3xQ3…  23.4      node Ghri…  23.6
node AxRK…  23.1      node BtpD…  999   ← rejected as outlier
ORACLE VALUE: 23.4 C  (3 of 4 nodes agree)
```

The same value served through the agent on Telegram: *"the trusted temperature
is 23.4°C, the median of 3 trusted nodes; 1 outlier rejected."* Corrupting one
node does nothing — that is the property a market needs, and the reason this is
an oracle and not just a sensor.

## Custody ladder

| Tier | Meaning | In Onca |
|---|---|---|
| **T0 Read** | reads and reports; a key at most | `mesh-oracle` (the aggregate), `token-risk-check`, `payment-watch` |
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
   inventing a reading, and requires it to surface a refusal and stop.
3. **The human approval gate**: every attestation shows the exact value for a tap.
4. **The mesh median**: a node that lies past all of that — signing a spoof with
   its own key — is outvoted and dropped.

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
  SPL Memo, durable nonce), and the mesh aggregation. Devnet-verified; 22 host
  tests.
- **`depin-attest`** (Tier 3 plugin, **T1**) — reading → unsigned attestation,
  with code-enforced bounds, a monotonic replay guard, and a host-stamped time.
- **`mesh-oracle`** (Tier 3 plugin, **T0**) — reads the mesh and settles on the
  median, outliers dropped, quorum required. Fault-tolerant per-node reads.
- **`onca-signer`** — the human-disposes side: holds the device key, rebuilds the
  same attestation with `onca-core`, signs (ed25519), submits.
- **`onca-oracle`** — the standalone, reliable mesh reader (the aggregate a
  market consumes).
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
[`tools/onca-oracle`](../tools/onca-oracle).

## Roadmap

- **Live ESP32** as a real physical node in the mesh (firmware + bridge written).
- **Geographic, staked mesh** — independent operators run nodes, stake, and earn;
  reputation weights the aggregate and a proven liar is slashed.
- **Squads multisig dispose** — the agent proposes, a multisig approves from a
  phone, replacing the single-key signer.
