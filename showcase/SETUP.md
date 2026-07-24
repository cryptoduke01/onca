# Reproduce Onca in an evening

The whole use case: a self-hosted ZeroClaw agent on Telegram that turns a sensor
reading into an unsigned Solana attestation behind a human approval gate, plus a
separate signer that a human runs to land it on-chain. Custody tier **T1** — the
agent and the plugin never hold a key.

Everything below is verified on macOS (Apple Silicon) against ZeroClaw `0.8.3`.

## 0. Prerequisites

- Rust stable + the wasm target: `rustup target add wasm32-wasip2`
- The Solana CLI (for the device key and devnet airdrop)
- A [Groq](https://console.groq.com) API key (free tier is enough)
- A Telegram bot token from [@BotFather](https://t.me/BotFather)

Keep both secrets out of the repo. This guide reads them from `~/.onca/`:

```bash
mkdir -p ~/.onca && chmod 700 ~/.onca
echo 'GROQ_API_KEY=gsk_...'            > ~/.onca/groq.env
echo 'TELEGRAM_BOT_TOKEN=123456:AA...' > ~/.onca/telegram.env
chmod 600 ~/.onca/*.env
```

## 1. Build the host with plugins enabled

```bash
git clone --depth 1 https://github.com/zeroclaw-labs/zeroclaw /tmp/zeroclaw
cd /tmp/zeroclaw
cargo build --release --bin zeroclaw --features plugins-wasm-cranelift
ZC=/tmp/zeroclaw/target/release/zeroclaw
```

> The vendored `wit/v0` in this repo is synced to the host ABI. If you build a
> newer host and the plugin is "discovered but not registered", re-copy
> `/tmp/zeroclaw/wit/v0/*.wit` over `wit/v0/` and rebuild the component
> (see `docs/wasm-notes.md`).

## 2. Build and install the plugin

```bash
cd plugins/depin-attest
cargo build --target wasm32-wasip2 --release
mkdir -p /tmp/depin-attest
cp manifest.toml target/wasm32-wasip2/release/depin_attest.wasm /tmp/depin-attest/
$ZC plugin install /tmp/depin-attest/
$ZC config set plugins.enabled true
$ZC plugin list        # should show depin-attest
```

## 3. Make a device key and fund it (devnet)

```bash
solana-keygen new --no-bip39-passphrase -o ~/.onca/device.json
chmod 600 ~/.onca/device.json
DEVICE=$(solana-keygen pubkey ~/.onca/device.json)
solana airdrop 1 "$DEVICE" --url devnet   # retry if the public faucet rate-limits
```

## 4. Drop in the config

Copy [`config.example.toml`](config.example.toml) to `~/.zeroclaw/config.toml`
and replace `<DEVICE_PUBKEY>` with `$DEVICE`.

## 5. Run the agent

```bash
source ~/.onca/groq.env ~/.onca/telegram.env
export ZEROCLAW_providers__models__groq__default__api_key="$GROQ_API_KEY"
export ZEROCLAW_channels__telegram__main__bot_token="$TELEGRAM_BOT_TOKEN"
$ZC daemon --host 127.0.0.1 --port 42617
```

In Telegram, open your bot and send `/bind <code>` (the code prints on daemon
startup). Then message it: **"Attest a reading from dht11-a: 23.4 C, sequence 1"**.
The agent shows an approval card with the exact reading and inline
Approve / Deny buttons. Tap **Approve** and it returns the unsigned attestation.

## 6. Sign and land it (the human disposes)

```bash
cargo build --release --manifest-path tools/onca-signer/Cargo.toml
tools/onca-signer/target/release/onca-signer \
  --sensor dht11-a --value 23.4 --unit C --seq 1
# prints: SUBMITTED <sig>  +  https://explorer.solana.com/tx/<sig>?cluster=devnet
```

That is the full loop: chat → human approval → unsigned attestation → the human's
signer lands it on Solana. The key never touches the agent.

## 7. The mesh oracle (read side)

One node is not an oracle. Stand up several nodes (each is a keypair +
`onca-signer --keypair <path>` attesting its own reading), then aggregate them
into one manipulation-resistant value with `onca-oracle` (custody T0, read-only):

```bash
cargo build --release --manifest-path tools/onca-oracle/Cargo.toml
tools/onca-oracle/target/release/onca-oracle \
  --devices "<pubkey1>,<pubkey2>,<pubkey3>,<pubkey4>" \
  --sensor dht11-a --tolerance 5 --quorum 3
```

It reads each device's latest on-chain attestation, drops outliers (a lying or
broken node), and prints the median the market settles on. A single corrupted
node cannot move it.

## Hardware (optional)

An ESP32 + DHT11 running [`firmware/onca-dht11`](../plugins/depin-attest/firmware/onca-dht11)
prints `onca:reading` lines over USB; the bridge
([`examples/bridge.rs`](../plugins/depin-attest/examples/bridge.rs)) feeds them in.
The software loop above runs identically with a typed reading, so the demo does
not depend on hardware being present.
