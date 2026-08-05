# Onca

Onca is a set of Solana tools for the [ZeroClaw](https://github.com/zeroclaw-labs/zeroclaw)
agent runtime. The tools let an agent touch Solana — take a payment, check a
token, attest a sensor reading, run an oracle — and keep the agent safe while it
does so.

[![Onca — the agent proposes, you dispose](docs/assets/site-hero.jpg)](https://onca.run)

Site: [onca.run](https://onca.run) · Source: this monorepo (`site/`)

**The flagship use case** is a manipulation-resistant DePIN oracle for prediction
markets: independent sensor nodes attest their readings on Solana behind a human
approval gate, a mesh settles on the median with outliers dropped and repeat
liars frozen out on reputation, and the trusted value is sold over x402 so a
market resolver can pay for it and settle. The full write-up, with on-chain
proofs, is in [showcase/SHOWCASE.md](showcase/SHOWCASE.md); the runbook is
[showcase/SETUP.md](showcase/SETUP.md).

## The name

Onca comes from *onça*, the Portuguese word for the jaguar. The jaguar is the
largest cat in the Americas and a common sight in Brazil. The name keeps the
claw of ZeroClaw and points it at Solana. We write the name as `Onca`, without
the cedilla, because a plain name is easy to type in a shell and a package. You
say the name as *ON-sah*.

## Why Onca exists

ZeroClaw is a self-hosted agent. It runs as one Rust binary on your own machine,
with your model and your keys. This design is the strength of ZeroClaw. It is
also the risk.

When an agent can move money, the model becomes a path to your wallet. The model
reads messages, mail, and web pages that you do not control. An attacker can
hide an instruction in that text. A private key behind a model is a hot wallet
with an attack surface.

Onca answers this risk directly. Each tool stays at the safe end of the custody
ladder. The agent makes a proposal. A person, a multisig, or a limited session
key approves it. A tool does one of two things: it reads the chain and reports,
or it builds an unsigned request for a person to sign. No tool in Onca holds a
key that can spend funds. The rules that keep this true live in plain Rust that
the model cannot change. [docs/custody.md](docs/custody.md) explains the ladder
and the threat model for the whole suite.

## The tools

| Component | Tier | Holds | What it does |
|---|---|---|---|
| [`depin-attest`](plugins/depin-attest) | T1 | nothing | Turns a sensor reading into an unsigned attestation transaction with a replay guard. A ZeroClaw device becomes a Solana-reporting DePIN node. |
| [`onca-signer`](tools/onca-signer) | — | device key | The human-disposes side: holds the device key the agent never sees, rebuilds the approved attestation, signs it, and submits. Run by a person, not the agent. |
| [`mesh-oracle`](plugins/mesh-oracle) / [`onca-oracle`](tools/onca-oracle) | T0 | RPC key | Reads the mesh of on-chain attestations and settles on the median: outliers dropped, quorum required, and repeat liars frozen out on persistent reputation. |
| [`onca-x402`](tools/onca-x402) | T0 | RPC key | Sells the trusted value over [x402](https://x402.org): `402 Payment Required`, verify the Solana payment landed, then return the mesh reading. |
| [`onca-resolve`](tools/onca-resolve) | — | pays only | The consumer: pays the x402 fee, reads the value back, and settles a prediction market YES/NO. Holds nothing of the oracle's. |
| [`onca-market`](tools/onca-market) | T0 | RPC key | Settles a *live* Solana prediction market: reads a real weather market from Jupiter's keyless prediction API (Polymarket liquidity on Solana) and maps the mesh value to the winning outcome. |
| [`onca-trade`](tools/onca-trade) | T1 | nothing | Builds an *unsigned* Jupiter order on the mesh's winning outcome for a human to sign. Fails closed without USDC. |
| [`settle-weather-market`](skills/settle-weather-market) | Tier 1 | nothing | A ZeroClaw skill (no compiled code): the agent composes the built-in `http_request` with `mesh_oracle` to settle the live market on-channel. |
| [`solana-pay-request`](plugins/solana-pay-request) | T1 | nothing | Turns a request such as "charge table 4 for 25 USDC" into a Solana Pay URL and QR code. A person signs it. |
| [`token-risk-check`](plugins/token-risk-check) | T0 | RPC key | Reads a mint and gives a red, amber, or green verdict: authorities, Token-2022 traps, and holder concentration. |
| [`payment-watch`](plugins/payment-watch) | T0 | RPC key | Watches a Solana Pay reference and confirms that an invoice was paid: the right amount, to the right wallet. |
| [`onca-core`](crates/onca-core) | — | nothing | The shared Solana library every tool sits on: base58, pubkey, JSON-RPC, amount math, hand-rolled transaction assembly (memo, System Transfer, durable nonce), and the mesh aggregation with a reputation layer. Pure Rust, no input or output. |

Read the tiers this way. A T0 tool reads and reports. A T1 tool builds a request
that a person signs. Onca has no T2 tool. A T2 tool signs and sends. That tier
is where one successful attack empties a wallet, and no tool here needs it. The
one component that holds a spending key, `onca-signer`, is a separate binary a
human runs — never the agent — which is the T1 custody boundary made literal.

The tools reach from a physical sensor to a settled market. `depin-attest` lets a
device report a reading behind a human tap; `onca-signer` lands it; `onca-oracle`
aggregates the mesh into one manipulation-resistant value; `onca-x402` sells that
value and `onca-resolve` buys it to settle a market. The payment tools round it
out: `solana-pay-request` asks for money, `payment-watch` confirms it arrived,
and `token-risk-check` stops the agent before it touches a bad token. All build
on `onca-core`, and none of the agent-side tools holds a key that can spend.

## The core

The usual way to touch Solana from Rust is `solana-sdk`. As of mid-2026 the
modular Solana crates (`solana-pubkey`, `solana-instruction`, `solana-message`,
`solana-transaction`, `solana-hash`) — and `solana-sdk` itself — compile cleanly
for `wasm32-wasip2`, so a plugin no longer *has* to hand-roll anything to build a
transaction. `onca-core` still assembles transactions by hand, on purpose: a
minimal component with the fewest dependencies, and an encoding proven against
the real runtime rather than only against a library the ZeroClaw host has not yet
instantiated as a component.

That proof is the point. `cargo run --example memo_tx` emits an unsigned
transaction; devnet `simulateTransaction` deserializes it, recognizes the fee
payer as signer, and runs the SPL Memo program. The bytes are correct at the
wire, not only in unit tests (which also check RFC 4648 base64 and the documented
compact-u16 vectors). The core is a plain library with no input or output: it
parses an address, builds a JSON-RPC request, reads the response, shapes amounts,
and assembles the legacy message, the SPL Memo instruction, and the durable-nonce
advance that keeps an approval-gated transaction valid while a human takes their
time to sign (the bounty's blockhash-expiry trap). Wrapping the modular crates
instead is a reasonable future direction; hand-rolling buys minimal size and a
runtime-verified path today. The one thing the core does not do is make the HTTP
call. A trait does that:

```rust
pub trait RpcTransport {
    fn post_json(&self, url: &str, body: &str) -> onca_core::Result<String>;
}
```

Each plugin gives the trait a real client. On wasm, the plugin uses the
[`waki`](https://crates.io/crates/waki) client, and the host does TLS. The tests
give the trait a mock. Because the trait is the only seam, the full RPC layer
runs under `cargo test` on the host with no network. The tests check behavior,
not only text. [docs/wasm-notes.md](docs/wasm-notes.md) records what was hard
about `wasm32-wasip2`.

## The shape of each plugin

Every plugin uses the same split. The bounty requires it, and it is also the
correct design:

```
src/<logic>.rs   pure Rust. All checks and policy. No wasm. The host tests it.
src/lib.rs       a thin wasm shim. It connects the logic to the tool-plugin world.
tests/           host tests over the pure core. The RPC is a mock.
manifest.toml    name, version, capabilities, and the fewest permissions that work.
README.md        what the tool does, its config, its custody tier, its threat model.
```

The split has a purpose beyond order. The rules that limit the tool, such as a
spend cap, a mint allowlist, and address checks, live in the pure core. They run
on every call. The model never sees the config that limits it, and it cannot
turn the config off. To pass a rule, you would have to change the code and build
the component again. At that point you are the operator, not the attacker.

## Build and test

You can test everything on the host. You do not need a wasm toolchain or a
network:

```bash
cd crates/onca-core           && cargo test   # engine + mesh + reputation
cd plugins/depin-attest       && cargo test
cd plugins/mesh-oracle        && cargo test
cd plugins/solana-pay-request && cargo test
cd plugins/token-risk-check   && cargo test
cd plugins/payment-watch      && cargo test
```

To build one component:

```bash
rustup target add wasm32-wasip2
cd plugins/solana-pay-request
cargo build --target wasm32-wasip2 --release
```

[docs/install.md](docs/install.md) is the full guide for an operator.

## Status

The core and all five plugins are complete: each has a pure Rust core, a wasm
component that builds clean for `wasm32-wasip2`, host tests with a fail-closed
prompt-injection case, a manifest with the fewest permissions, and a README with
a threat model.

The use case runs end to end, proven on devnet. A human-approved attestation
lands on-chain through the Telegram agent and the human-run signer; the mesh
reader aggregates independent nodes into one median, drops an outlier, and
freezes a repeat liar on reputation; and the x402 loop pays for that value and
settles a prediction market. The plugins run inside a source-built host
(`--features plugins-wasm-cranelift`), where the component boundary is exercised
for real, not just under host tests. On-chain proofs and the full walkthrough are
in [showcase/SHOWCASE.md](showcase/SHOWCASE.md).

What remains is presentation, not engineering: the three-minute video and the
final submission. A live ESP32 node and USDC settlement are on the roadmap; the
software loop stands on its own without either.

## License

MIT. See [LICENSE](LICENSE).
