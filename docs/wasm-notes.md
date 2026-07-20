# What fought us on `wasm32-wasip2`

The bounty says this write-up is worth points, and it is genuinely the hardest
part. Notes, kept current as we build.

## 1. `solana-sdk` / `solana-client` are not your friends here

They pull in `getrandom`, `rayon`, socket-based RPC, and a pile of transitive
crates that assume a real OS. Inside a WIT component targeting `wasm32-wasip2`
they do not compile cleanly, and even where they can be coerced, they bloat the
component far past what a host wants to load.

**What we did instead:** hand-rolled the small surface we actually need into
[`crates/solana-core`](../crates/solana-core), with only `serde`/`serde_json`,
`bs58`, and `borsh` — all pure Rust that builds for the target without special
handling. A Solana address is 32 bytes and base58; a JSON-RPC call is a POST
with a known envelope. We do not need the full SDK to do either.

## 2. Keep HTTP out of the core

`waki` (the blocking `wasi:http` client the published channel plugins use) is a
wasm-only dependency. If the core crate depended on it directly, `cargo test` on
the host would try to compile it and fail.

**What we did:** the core defines a `RpcTransport` trait and never performs I/O.
The plugin's `#[cfg(target_family = "wasm")]` shim implements it with `waki`;
tests implement it with a mock. `waki` is declared as a target-gated dependency
so the host build never sees it:

```toml
[target.'cfg(target_family = "wasm")'.dependencies]
waki = { version = "0.5.1", features = ["json"] }
```

This is the single most important structural decision: it is what makes the
whole RPC layer host-testable and keeps the "pure core, thin shim" split honest.

## 3. `wit-bindgen::generate!` path is relative to the crate

The macro reads the WIT at build time from `path: "../../wit/v0"`. That means the
plugin only builds where that path resolves — so this repo **vendors** `wit/v0`
at its root (copied verbatim from upstream). When upstream bumps the ABI
(`wit/v0` is explicitly experimental, no `.frozen` marker), we re-vendor and
rebuild. Pin your assumptions; expect a rebuild.

## 4. The component is a component, not a module

`cargo build --target wasm32-wasip2` on a `crate-type = ["cdylib"]` lib with
`wit-bindgen` + `export!` produces a proper WIT **component** (binary starts
`00 61 73 6d 0d 00 01 00` — the `0d` is the component-model layer), not a core
module. No `cargo component` or `wasm-tools component new` post-step was needed
with `wit-bindgen 0.46` and a `wasip2` target. Good — one less moving part.

## 5. `opt-level = "s"` + `lto` + `strip` matters

The host loads and (optionally) precompiles every component. Release profile
with `opt-level = "s"`, `lto = true`, `strip = true`, `codegen-units = 1` keeps
`solana-pay-request` at ~210 KB. Worth it.

## Still ahead

- Versioned (v0) transaction assembly by hand: compact-u16 arrays, message
  header, address-table handling — borsh + manual encoding, no SDK.
- Durable nonce accounts to beat blockhash expiry in an approval-gated flow
  (trap #1). The plan: fetch the nonce account, use its stored blockhash, make
  the first instruction `AdvanceNonceAccount`. Notes to follow once built.
