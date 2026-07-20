# Building Solana tooling for `wasm32-wasip2`

The awkward part of this project was not the Solana logic; it was getting
anything Solana-shaped to compile into a WIT component in the first place. These
are the things that cost time, written down so the next person spends it on
something else.

## `solana-sdk` and `solana-client` do not belong here

They assume a real operating system — sockets, threads, `getrandom`, a socket
RPC client, and a long tail of transitive crates that expect all of it. Inside a
`wasm32-wasip2` component none of that holds, and even where a dependency can be
coerced into building, it bloats the component well past what a host wants to
load.

So we did not use them. `onca-core` hand-rolls the small surface the plugins
actually need — address parsing, JSON-RPC request and response handling, amount
math — on top of `serde`, `serde_json`, and `bs58`, all of which are pure Rust
and build for the target without any special handling. A Solana address is
thirty-two bytes and base58; a JSON-RPC call is a POST with a known envelope.
Neither needs the full SDK.

## Keep HTTP out of the core, or the host tests won't compile

`waki`, the blocking `wasi:http` client the published channel plugins use, only
exists on wasm. If `onca-core` depended on it directly, `cargo test` on the host
would try to build it and fail. The fix is to keep I/O out of the core entirely:
it defines an `RpcTransport` trait and never makes a call itself. The plugin's
wasm shim implements that trait with `waki`; the tests implement it with a mock.
`waki` is pulled in only for the wasm target:

```toml
[target.'cfg(target_family = "wasm")'.dependencies]
waki = { version = "0.5.1", features = ["json"] }
```

This one decision is what makes the RPC layer testable on the host and keeps the
"pure core, thin shim" split honest rather than aspirational.

## `wit-bindgen::generate!` reads WIT at build time

The macro loads the interface from `path: "../../wit/v0"`, so a plugin only
builds where that path resolves. This repo vendors `wit/v0` at its root, copied
verbatim from upstream. The interface is still marked experimental — there is no
`.frozen` marker — so when upstream moves the ABI, the plan is to re-vendor and
rebuild rather than to have pinned to a version that has quietly drifted.

## The output really is a component

Running `cargo build --target wasm32-wasip2 --release` against a
`crate-type = ["cdylib"]` library that uses `wit-bindgen` and `export!` produces
a proper WIT component, not a bare core module — the binary starts
`00 61 73 6d 0d 00 01 00`, where the `0d` byte is the component-model layer. No
`cargo component` or `wasm-tools component new` step was needed with
`wit-bindgen 0.46` on a `wasip2` target, which is one less thing to get wrong.

## The release profile is worth setting

The host loads and may precompile every component, so size is not free.
`opt-level = "s"`, `lto = true`, `strip = true`, and `codegen-units = 1` keep
`solana-pay-request` near 210 KB and the two RPC plugins under 370 KB.

## Still ahead

If the suite grows a signed-transfer builder, two things follow. First,
versioned (v0) transaction assembly by hand — compact-u16 arrays, the message
header, address-table lookups — encoded manually, since the SDK is not available.
Second, durable nonce accounts, to survive the gap between an agent building a
transaction and a human getting around to approving it: fetch the nonce account,
use its stored blockhash, and make the first instruction an
`AdvanceNonceAccount`. Notes will follow once that exists.
