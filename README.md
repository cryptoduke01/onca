# zeroclaw-solana — Solana-native tool plugins for ZeroClaw

Solana capability for the [ZeroClaw] self-hosted agent runtime, built as
sandboxed `wasm32-wasip2` WIT components against [`wit/v0`]. Deny-by-default:
each plugin gets only what its `manifest.toml` declares.

Built for the **Build Solana-native plugins for ZeroClaw** bounty (Superteam
Brasil). Layout mirrors the canonical [`redact-text`] reference so it drops
straight into a [`zeroclaw-labs/zeroclaw-plugins`] PR.

## The custody thesis

An agent with a private key and an LLM in the loop is a hot wallet with a prompt
injection surface. So everything here sits at the **safe end of the custody
ladder** — the agent *proposes*, a human *disposes*:

| Component | Tier | Secrets held | What it does |
|---|---|---|---|
| [`solana-core`](crates/solana-core) | — | none | Shared substrate (Track E). Base58, JSON-RPC shaping, pubkey, amount math. Pure Rust, host-tested, imported by every plugin below. |
| [`solana-pay-request`](plugins/solana-pay-request) | **T1** | **none** | Chat → Solana Pay URL + QR. Human signs. Enforces mint allowlist + max amount. ✅ shipping |
| [`token-risk-check`](plugins/token-risk-check) | **T0** | RPC key | Mint/freeze authority, holder concentration, Token-2022 hooks → red/amber/green. ✅ shipping |
| [`payment-watch`](plugins/payment-watch) | **T0** | RPC key | Watch a Solana Pay reference; confirm the on-chain amount landed (paid/underpaid/pending). ✅ shipping |

No plugin here takes a raw private key. No plugin here can move funds it was not
architected to move.

## Why a shared core (Track E)

`solana-sdk` / `solana-client` do not compile cleanly for `wasm32-wasip2` inside
a WIT component. Rather than fight that in five places, the Solana primitives
live once in [`solana-core`](crates/solana-core) — a plain `rlib` with **no wasm
dependency and no I/O**. HTTP is abstracted behind a single trait:

```rust
pub trait RpcTransport {
    fn post_json(&self, url: &str, body: &str) -> solana_core::Result<String>;
}
```

Each plugin implements that trait with the blocking [`waki`] client on wasm
(TLS is done host-side by `wasi:http`); tests implement it with a canned mock.
The result: **the entire RPC layer is exercised by `cargo test` with zero
network**, and the plugins stay tiny. See [`docs/wasm-notes.md`](docs/wasm-notes.md)
for the full write-up of what fought us on `wasm32-wasip2`.

## Architecture: pure core, thin shim

Every component follows the reference split, which is also a hard requirement:

```
src/<logic>.rs   # pure Rust, no wasm deps — all validation & policy, host-tested
src/lib.rs       # thin #[cfg(target_family = "wasm")] shim → the tool-plugin world
tests/           # host-run integration tests over the pure core
manifest.toml    # name, version, wasm_path, capabilities, minimal permissions
README.md        # what it does, config, custody tier, threat model, injection transcript
```

The guardrails (caps, allowlists, address validation) live in the pure core and
run on every call, so a prompt injection cannot argue its way past them — it
would have to change the code.

## Build & test

```bash
# host tests for everything (no wasm toolchain, no network):
(cd crates/solana-core            && cargo test)
(cd plugins/solana-pay-request    && cargo test)

# build a component:
rustup target add wasm32-wasip2
(cd plugins/solana-pay-request && cargo build --target wasm32-wasip2 --release)
```

## Status

- [x] `solana-core` — base58, pubkey, JSON-RPC over a mockable transport, amount + output shaping. 10 tests green.
- [x] `solana-pay-request` (T1) — full: pure core, wasm component, host tests, prompt-injection transcript, manifest, README.
- [x] `token-risk-check` (T0) — full: pure analysis core (mockable RPC), wasm component, 11 tests, fail-closed honeypot transcript, manifest, README.
- [x] `payment-watch` (T0, SOP-triggered) — full: pure detection core (mockable RPC), wasm component, 9 tests, fail-closed dust-spoof transcript, manifest, README. Closes the payments loop.
- [ ] Demo video (Telegram agent) + submission write-up.

## License

MIT. See [LICENSE](LICENSE).

[ZeroClaw]: https://github.com/zeroclaw-labs/zeroclaw
[`wit/v0`]: https://github.com/zeroclaw-labs/zeroclaw-plugins/tree/main/wit
[`redact-text`]: https://github.com/zeroclaw-labs/zeroclaw-plugins/tree/main/plugins/redact-text
[`zeroclaw-labs/zeroclaw-plugins`]: https://github.com/zeroclaw-labs/zeroclaw-plugins
[`waki`]: https://crates.io/crates/waki
