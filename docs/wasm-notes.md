# Notes on wasm32-wasip2

The hard part of this project was not the Solana logic. The hard part was to
compile anything Solana-shaped into a WIT component. These notes record what
cost time, so the next person spends the time on something else.

## solana-sdk and solana-client do not belong here

`solana-sdk` and `solana-client` expect a full operating system. They need
sockets, threads, `getrandom`, a socket RPC client, and a long list of other
crates that expect the same. A `wasm32-wasip2` component does not provide these.
Even where you can force a dependency to build, it makes the component much
larger than a host wants to load.

So the suite does not use them. `onca-core` writes the small surface that the
plugins need. It parses an address, builds and reads a JSON-RPC message, and does
amount math. It uses only `serde`, `serde_json`, and `bs58`. All three are pure
Rust and build for the target with no special work. A Solana address is 32 bytes
and base58. A JSON-RPC call is a POST with a known shape. Neither one needs the
full SDK.

## Keep HTTP out of the core

`waki` is the blocking `wasi:http` client that the published channel plugins
use. It exists only on wasm. If `onca-core` used `waki` directly, `cargo test`
on the host would try to build it and fail.

The fix is to keep input and output out of the core. The core declares an
`RpcTransport` trait and makes no call itself. The wasm shim gives the trait the
`waki` client. The tests give the trait a mock. The `Cargo.toml` adds `waki` for
the wasm target only:

```toml
[target.'cfg(target_family = "wasm")'.dependencies]
waki = { version = "0.5.1", features = ["json"] }
```

This one decision makes the RPC layer testable on the host. It also keeps the
"pure core, thin shim" split real, not just a claim.

## wit-bindgen reads the WIT at build time

The `wit-bindgen::generate!` macro loads the interface from
`path: "../../wit/v0"`. A plugin builds only where that path resolves. So this
repository keeps a copy of `wit/v0` at its root, taken from upstream without a
change. The interface is still marked experimental and has no `.frozen` marker.
When upstream changes the ABI, the plan is to copy the new interface and build
again.

## The output is a component, not a module

Run `cargo build --target wasm32-wasip2 --release` on a library with
`crate-type = ["cdylib"]` that uses `wit-bindgen` and `export!`. The result is a
WIT component, not a bare core module. The binary starts with the bytes
`00 61 73 6d 0d 00 01 00`. The `0d` byte is the component-model layer. This
needs no `cargo component` step and no `wasm-tools component new` step with
`wit-bindgen 0.46` on a `wasip2` target. That is one less thing to get wrong.

## Set the release profile

The host loads and can precompile every component, so size is not free. The
release profile uses `opt-level = "s"`, `lto = true`, `strip = true`, and
`codegen-units = 1`. These settings keep `solana-pay-request` near 210 KB and
the two RPC plugins under 370 KB.

## Still ahead

A signed-transfer builder would need two more things. The first is versioned
(v0) transaction assembly by hand: the compact-u16 arrays, the message header,
and the address-table lookups, all encoded by hand, because the SDK is not
available. The second is durable nonce accounts. A durable nonce keeps a
transaction valid in the gap between the moment the agent builds it and the
moment a person approves it. The plan is to read the nonce account, use its
stored blockhash, and make the first instruction an `AdvanceNonceAccount`. Notes
will follow when this exists.
