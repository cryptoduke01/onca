# Onca

Solana tools for the [ZeroClaw](https://github.com/zeroclaw-labs/zeroclaw) agent
runtime, built so an autonomous agent can handle money without becoming a
liability.

**The name.** *Onça* is Portuguese for the jaguar, the largest cat in the
Americas and a fixture of Brazilian wildlife. It keeps ZeroClaw's animal, the
claw, and points it at Solana. We spell it `Onca`, without the cedilla, so it is
easy to type in a shell and a package name; it is pronounced *ON-sah*.

## Why this exists

ZeroClaw is a self-hosted agent: one Rust binary, your choice of model, running
on your own machine with your own keys. That property is the whole appeal, and
it is also the danger. The moment an agent can move funds, its language model,
which reads untrusted Telegram messages, emails, and web pages all day, becomes
a path to your wallet. A private key behind an LLM is a hot wallet with a prompt
injection surface.

Onca is written around that problem rather than in spite of it. Every tool here
sits at the safe end of what the bounty calls the custody ladder: the agent
*proposes*, and a human (or a multisig, or a scoped session key) *disposes*. A
tool either reads the chain and reports, or it builds an unsigned request that a
person signs on their phone. Nothing in this repository holds a spendable key,
and the guardrails that make that true live in plain Rust that the model cannot
talk its way around.

## What is in the box

| Component | Tier | Holds | What it does |
|---|---|---|---|
| [`onca-core`](crates/onca-core) | — | nothing | Shared Solana substrate: base58, pubkey parsing, JSON-RPC, amount math. Pure Rust, no I/O, imported by every plugin. |
| [`solana-pay-request`](plugins/solana-pay-request) | T1 | nothing | Turns "charge table 4 for 25 USDC" into a Solana Pay URL and QR code. A human signs it. |
| [`token-risk-check`](plugins/token-risk-check) | T0 | RPC key | Reads a mint and returns a red/amber/green safety verdict: authorities, Token-2022 honeypot traps, holder concentration. |
| [`payment-watch`](plugins/payment-watch) | T0 | RPC key | Watches a Solana Pay reference and confirms an invoice was actually paid: the right amount, to the right wallet. |

Read the tiers as: **T0** reads and reports, **T1** builds something a human
signs. There is no T2 (a tool that signs and submits) in this repository, and
that is on purpose. It is the tier where a single successful injection drains a
wallet, and none of these tools need it.

The three plugins tell one story. `solana-pay-request` asks for money,
`payment-watch` confirms it arrived, and `token-risk-check` keeps the agent from
touching a poisoned token in the first place. They share `onca-core`, which is
what makes the fourth piece worth its own attention.

## The core, and why it had to be written

The obvious way to talk to Solana from Rust is `solana-sdk` and
`solana-client`. Neither compiles cleanly to `wasm32-wasip2` inside a WIT
component. They assume sockets, threads, and a real operating system that a
sandboxed plugin does not have. Fighting that in three separate plugins would
have been three times the pain, so the Solana primitives live once, in
[`onca-core`](crates/onca-core).

`onca-core` is a plain library with no wasm dependency and no I/O of its own. It
knows how to parse and render a 32-byte address, build a JSON-RPC request,
interpret the response, and turn base units into human amounts. The one thing it
deliberately does *not* do is make the HTTP call. That sits behind a trait:

```rust
pub trait RpcTransport {
    fn post_json(&self, url: &str, body: &str) -> onca_core::Result<String>;
}
```

Each plugin implements `RpcTransport` with the blocking [`waki`](https://crates.io/crates/waki)
client on wasm, where the host performs TLS. The tests implement it with a
canned mock. Because the seam is a trait, the entire RPC layer (request shapes,
response parsing, the logic that decides whether a payment landed) runs under
`cargo test` on the host with no network at all. That is the difference between
tests that check string formatting and tests that check behaviour.

[`docs/wasm-notes.md`](docs/wasm-notes.md) is the honest write-up of everything
that fought us on `wasm32-wasip2`, which the bounty asks for and which is the
reason a shared core earns its keep.

## How each plugin is built

Every component follows the same split, which the bounty requires and which also
happens to be the right way to write this:

```
src/<logic>.rs   pure Rust, all validation and policy, no wasm, host-tested
src/lib.rs       a thin wasm shim that wires the logic to the tool-plugin world
tests/           host-run tests over the pure core, RPC mocked
manifest.toml    name, version, capabilities, and the fewest permissions that work
README.md        what it does, its config, its custody tier, and its threat model
```

The point of the split is not tidiness. It is that the guardrails (spend caps,
mint allowlists, address validation, amount checks) sit in the pure core and
run on every call. The language model never sees the config that constrains it
and cannot disable it. To get past a guardrail you would have to edit the code
and rebuild the component, at which point you are the operator, not the attacker.

## Building and testing

Everything is host-testable without a wasm toolchain or a network:

```bash
cd crates/onca-core         && cargo test
cd plugins/solana-pay-request && cargo test
cd plugins/token-risk-check   && cargo test
cd plugins/payment-watch      && cargo test
```

Building an actual component:

```bash
rustup target add wasm32-wasip2
cd plugins/solana-pay-request
cargo build --target wasm32-wasip2 --release
```

## Status

`onca-core` and all three plugins are complete: pure core, wasm component that
builds clean, host tests including a fail-closed prompt-injection case in each,
a manifest with minimal permissions, and a README with a threat model.

Still on the list: a short demo of a live ZeroClaw agent on Telegram running the
full loop, and, as a stretch, versioned-transaction and durable-nonce support
in `onca-core` so the suite can grow a signed-transfer builder without giving up
the custody discipline.

## License

MIT. See [LICENSE](LICENSE).
