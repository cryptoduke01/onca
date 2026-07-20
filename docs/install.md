# Install and run

This guide is for an operator who runs a ZeroClaw agent and wants to add the
Onca tools. It follows Simplified Technical English (ASD-STE100).

## Before you start

You need these items:

- A Rust toolchain, version 1.80 or later.
- The `wasm32-wasip2` target.
- A ZeroClaw build with a wasm plugin backend.
- A Solana JSON-RPC endpoint. The two read tools need it. Use your own endpoint.

Add the target once:

```bash
rustup target add wasm32-wasip2
```

## Build the components

Build each plugin you want. Each build writes a `.wasm` file under the plugin
`target` directory.

```bash
cd plugins/solana-pay-request
cargo build --target wasm32-wasip2 --release
# result: target/wasm32-wasip2/release/solana_pay_request.wasm
```

Do the same for `token-risk-check` and `payment-watch`.

## Place a component

Copy the `.wasm` file next to its `manifest.toml` in your ZeroClaw plugins
directory. The manifest field `wasm_path` names the file:

```bash
cp target/wasm32-wasip2/release/solana_pay_request.wasm \
   <your-zeroclaw-plugins-dir>/solana-pay-request/solana_pay_request.wasm
```

Some hosts run a runtime backend with no compiler. On such a host, precompile
the component with a matching `wasmtime` first, and point `wasm_path` at the
`.cwasm` file:

```bash
wasmtime compile --target <triple> solana_pay_request.wasm -o solana_pay_request.cwasm
```

## Turn on plugins and set config

Enable plugins in your ZeroClaw config, then add a section for each plugin. Each
key is the plugin config that this suite documents.

```toml
[plugins]
enabled = true

[[plugins.entries.solana-pay-request]]
label = "Bar do Zé"
allowed_mints = "USDC"
max_amount = "100"

[[plugins.entries.token-risk-check]]
rpc_url = "https://mainnet.helius-rpc.com/?api-key=YOUR_KEY"

[[plugins.entries.payment-watch]]
rpc_url = "https://mainnet.helius-rpc.com/?api-key=YOUR_KEY"
```

The host gives each plugin only its own section. It does this because the
manifest asks for `config_read`. A plugin cannot read the global config or
another plugin section.

## Run the agent

Run ZeroClaw with a wasm plugin backend. For example:

```bash
zeroclaw --features plugins-wasm,plugins-wasm-cranelift
```

For the exact feature flags and the plugin backend options, see the ZeroClaw
documentation.

## Use the tools

`solana-pay-request` and `token-risk-check` respond to a chat request. You ask
the agent, and the agent calls the tool.

- "charge table 4 for 25 USDC"
- "is EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v safe?"

`payment-watch` works best on a schedule. You wire it to a ZeroClaw SOP with a
cron trigger. The SOP calls the tool with the invoice reference, the recipient,
the amount, and the token. When the tool returns a paid result, the SOP sends a
message to your channel. For the SOP file format and the cron syntax, see the
ZeroClaw SOP documentation.
