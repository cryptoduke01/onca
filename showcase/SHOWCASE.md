# Onca — human-approved sensor attestations on Solana

**A manipulation-resistant oracle for prediction markets. Independent DePIN nodes
each attest their sensor readings on Solana under a human tap-to-approve; a mesh
aggregates them into one value a market can settle on, and no minority of lying
nodes can move it. The agent never holds a key.**

Builder: [@dukedotsol](https://x.com/dukedotsol) · Code:
[github.com/cryptoduke01/onca](https://github.com/cryptoduke01/onca) ·
Reproduce: [SETUP.md](SETUP.md)

## What it does

You message the bot in plain language — *"attest a reading from dht11-a: 23.4 C,
sequence 4"*. The agent shows you an approval card with the exact reading and
Approve / Deny buttons. On Approve, the `depin-attest` plugin builds an
**unsigned** Solana transaction that writes the reading on-chain as a signed
memo. A separate tool, `onca-signer`, holds the device key the agent never sees;
a human runs it to sign and submit. The result is a tamper-evident record on
Solana that a specific device reported a specific value at a specific time, with
a monotonic sequence so an old reading can never be replayed as fresh.

That is one node. The oracle is the **mesh**: `onca-oracle` reads many independent
nodes' attestations back off-chain and settles on the median, dropping outliers,
so a lying or broken minority is ignored. That single aggregated value is what a
prediction market consumes.

## Who it's for

A prediction market that settles on real-world data — *what was the temperature
in Lagos yesterday?* — needs an oracle no single party can quietly manipulate
(the failure mode that has burned markets like Polymarket). Onca is the sensor
side of that oracle: each node attests its own readings on Solana under a human
gate, and a mesh of independent nodes (roadmap) makes single-source manipulation
useless. The same primitive covers cold-chain monitoring, environmental
compliance, and any DePIN device that must put trustworthy readings on-chain.

## It runs — proof on devnet

Two human-approved attestations, signed by the device key and finalized on
Solana devnet:

- [`onca:attest s=dht11-a v=24.1 u=C seq=2`](https://explorer.solana.com/tx/5ATMiYLVGunuuZUa1F2svs1cKaoezm7NYkEsQeCzzozpymAr4Gvq9A1jHUAT3jTE6r4hTGfV4cXFeyPi9ZpEQX9z?cluster=devnet)
- [`onca:attest s=dht11-a v=22.8 u=C seq=3`](https://explorer.solana.com/tx/4zcKbaX8WrPr4vEVWsmFYZ84wyEeiuE1HvpwskWbUM2X6FsTe3Yd1cShuPutPKt2ujgwy1HR6JR1nkeUyNJA3QEN?cluster=devnet)

Device: `BMpwFSKbLJvPpK4yo5EoqBiQUDxt9NdgFRToXJpiphrC`

**The oracle rejects a lying node — live on devnet.** A 4-node mesh, each device
attesting independently; node 4 is an adversary reporting 999°C. `onca-oracle`
reads all four on-chain and settles on the honest median, dropping the liar:

```
node 3xQ3…  23.4      node Ghri…  23.6
node AxRK…  23.1      node BtpD…  999   ← rejected as outlier
ORACLE VALUE: 23.4 C  (3 of 4 nodes agree)
```

Corrupting one node changes nothing — that is the property a prediction market
needs to settle on real-world data, and the reason this is an oracle and not just
a sensor.

## ZeroClaw features used

- **Self-hosted daemon** on my own machine, my model, my keys.
- **Built-in Telegram channel** — the agent lives in a DM; approvals render as
  native inline keyboard buttons.
- **Supervised autonomy + forced approval** (`always_ask`) — every attestation
  pauses for a human tap that shows the exact reading.
- **WASM plugin system** (`wasm32-wasip2`, the `tool-plugin` WIT world) running in
  a source-built host (`--features plugins-wasm-cranelift`).
- **Config secrets injected via env**, least-privilege tool scoping
  (`allowed_tools = ["depin_attest"]`), and the `groq` provider slot.

## What I built

- **`onca-core`** — a minimal `wasm32-wasip2` Solana engine: base58, pubkey,
  JSON-RPC over a transport trait, and hand-assembled transactions (legacy
  message, SPL Memo, durable nonce), verified on devnet.
- **`depin-attest`** (Tier 3 plugin, custody **T1**) — reading → unsigned
  attestation, with code-enforced reading bounds and a monotonic replay guard.
- **`onca-signer`** — the "human disposes" side: holds the device key, rebuilds
  the same attestation with `onca-core`, signs (ed25519), submits.
- **`onca-core::mesh` + `onca-oracle`** (custody T0) — the oracle read side:
  aggregate many nodes' on-chain attestations into one value (median, outliers
  dropped, quorum required), so a lying minority cannot move the settlement.
  Five host tests, including "one lying node cannot move the settlement".
- **ESP32 firmware + serial bridge** — a DHT11 node that prints `onca:reading`
  lines the pipeline ingests (the software loop runs identically with a typed
  reading, so the demo doesn't depend on hardware).

## Custody tier and threat model

**Tier T1 (Build).** The agent and the plugin build unsigned transactions and
hold no key. A human runs the signer. The ladder never reaches T2 (sign + send)
inside the agent.

The threat is not just prompt injection — it's an LLM in the loop that might
**fabricate or substitute a reading**. Onca answers it in three layers:

1. **Code-enforced bounds + replay guard** in pure Rust the model cannot override
   (`min_reading` / `max_reading`, `seq` must increase).
2. **Oracle guidance**: the tool forbids the model from rounding, substituting, or
   inventing a reading, and requires it to surface a refusal and stop.
3. **The human approval gate**: every attestation shows the exact value for a tap.

### Fail-closed transcript (required)

A message tries to attest a spoofed, out-of-bounds reading. Even at **full**
autonomy (approval gate off), it dies in the operator's bounds and the model
reports the refusal instead of retrying with a passing value:

```
User:  Attest a reading from sensor dht11-a of 999 C, sequence 5.
Agent: The sensor reading of 999 C is above the configured maximum of 85 C,
       so it has been refused.
```

Under `supervised` (the demo default), that spoof also never reaches an approval
card — and any value that did would show verbatim for a human to Deny.

## Reproduce it in an evening

Full runbook in [SETUP.md](SETUP.md); the entire agent is
[config.example.toml](config.example.toml) (four sections, no secrets inline).
Plugin source: [`plugins/depin-attest`](../plugins/depin-attest); engine:
[`crates/onca-core`](../crates/onca-core); signer:
[`tools/onca-signer`](../tools/onca-signer).

## Roadmap

- **Live ESP32** as a real physical node in the mesh (firmware + bridge already
  written), swapped in for a simulated node.
- **Geographic, staked mesh** — independent operators run nodes, stake, and earn;
  reputation weights the aggregate and a proven liar is slashed.
- **Squads multisig dispose** — the agent proposes, a multisig approves from a
  phone, replacing the single-key signer.
