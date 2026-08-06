---
name: settle-weather-market
description: >-
  Settle a live Solana weather prediction market on the Onca mesh oracle instead of a single source. Use when asked to settle, resolve, or check a temperature/weather market, or which outcome the mesh picks.
license: MIT
version: 0.1.0
---

# Settle Weather Market

Settle a live Solana weather prediction market on the **Onca mesh** — the median
of many independent on-chain temperature attestations, outliers dropped and
repeat liars frozen out — instead of the single oracle the market ships with.
That single source is the Polymarket-style manipulation this removes.

Use the `http_request` tool for everything here — it uses the host's reliable
HTTP. (Do not use `mesh_oracle`; its in-sandbox HTTP client is flaky.)

## Steps

1. **Read the mesh yourself, in ONE batch call.** POST to
   `https://api.devnet.solana.com` with `Content-Type: application/json` and this
   exact JSON-RPC batch body (one `getSignaturesForAddress` per node):
   ```json
   [{"jsonrpc":"2.0","id":0,"method":"getSignaturesForAddress","params":["3xQ33DfPLL6py9zCZAm4CfowL6TPZbiMNJrBRPudhSNR",{"limit":25}]},
    {"jsonrpc":"2.0","id":1,"method":"getSignaturesForAddress","params":["GhriBBob3iUczrGR81mXaKQ9LJBpF2STU8uEnVZLoX9a",{"limit":25}]},
    {"jsonrpc":"2.0","id":2,"method":"getSignaturesForAddress","params":["AxRKqyyT9DXbKGMf47ULRCEqwRPQgCa1nX1UdVNTGQGh",{"limit":25}]},
    {"jsonrpc":"2.0","id":3,"method":"getSignaturesForAddress","params":["BtpDcpYMfeZa6MtwFX6VeAnHjyq6qqh9V2X86oKQdUDy",{"limit":25}]}]
   ```
   The response is an array of 4 results. For each node, scan its entries
   (newest first) for the latest `memo` containing `onca:attest s=dht11-a`, and
   read the number after `v=` (°C). You now have up to 4 readings, one per node.

2. **Aggregate — this is the anti-manipulation core, do it exactly.** Take the
   median of the readings. **Drop** any reading more than 5°C from that median (a
   lying or broken node, e.g. one reporting 999). Require at least **3** surviving
   readings (quorum) — if fewer, refuse to settle and say the mesh lacks quorum.
   The median of the survivors is the trusted value; note how many agreed and how
   many you rejected (name the rejected one).

3. **Get the market and its buckets in ONE call.** GET
   `https://api.jup.ag/prediction/v1/events/search?query=highest%20temperature%20Sao%20Paulo&includeMarkets=true&limit=20`.
   Pick the event whose `metadata.closeTime` is still in the future (the daily
   São Paulo market rolls over at 12:00 UTC). That event carries its outcome
   buckets inline in its `markets` array. Each entry is a bucket; its label is
   `metadata.groupItemTitle` (e.g. `23°C`, `21°C or below`, `31°C or higher`) and
   its id is `marketId`. Read the integer °C out of each label.

4. **Settle.** Round the mesh value to the nearest whole °C and pick the bucket
   whose integer is closest to it. That bucket is the winning outcome.

5. **Report** in three short lines, nothing more:
   - the market title and event id,
   - the mesh value plus `N agreed, M rejected` (name the frozen liar if there
     is one),
   - the winning bucket label and its `marketId`, and the one-line point: no
     single source set this, so moving it takes a majority of independent nodes.

## Output discipline

Do not paste raw API JSON into the reply — it floods context and costs the
operator money. Read the fields you need and report only the five lines above.

## Trading it (custody T1)

If the user wants to *act* on the outcome, do not sign anything. Jupiter's
`POST https://api.jup.ag/prediction/v1/orders` returns an **unsigned**
transaction (it needs an `x-api-key` and the trader's wallet pubkey). Return that
unsigned transaction for a human to sign with `onca-signer`. You hold no key and
submit nothing.
