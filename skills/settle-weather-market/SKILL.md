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
   The response is an array of 4 results — match each back to its node by its
   `id` field (0 = `3xQ3…`, 1 = `Ghri…`, 2 = `AxRK…`, 3 = `BtpD…`), because the
   array order may differ from the request. For each node, scan its entries
   (newest first) for the latest `memo` containing `onca:attest s=dht11-a`, and
   read the number right after `v=`. Copy that number **verbatim** — never round,
   average, or reuse another node's value. You now have up to 4 readings.

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

   Build the **Polymarket link** now, from the market title: lowercase it, delete
   the `?`, replace each space with a hyphen, and append `-2026`. Example:
   `Highest temperature in Sao Paulo on August 6?` becomes
   `https://polymarket.com/event/highest-temperature-in-sao-paulo-on-august-6-2026`.
   (This is the event's `metadata.slug`; construct it so you always have the link.)

4. **Settle.** Round the mesh value to the nearest whole °C and pick the bucket
   whose integer is closest to it. That bucket is the winning outcome.

5. **Reply with the Onca settlement card**, in exactly this shape (Telegram
   Markdown), filled with the real values you computed. Nothing before or after
   it, no preamble:

   ```
   🐆 *Onca Oracle · Settlement*

   *<market title>*
   🔗 https://polymarket.com/event/<slug>

   *Mesh · 4 independent nodes*
   `✅ 3xQ3…  23.4°C`
   `✅ Ghri…  23.6°C`
   `✅ AxRK…  23.1°C`
   `❌ BtpD…   999°C  frozen`

   *Trusted value* · `23.4°C`  ·  3 agreed, 1 rejected
   🏆 *Winning outcome* · `23°C`  (`<winning marketId>`)

   _No single source set this. Corrupt a minority, the median holds._
   ```

   Use the real node short-ids (first 4 chars), each node's actual reading, a
   ✅ for a counted node and a ❌ for a dropped/frozen one, and the true agreed
   and rejected counts. The second line — `🔗 ` followed by the full
   `https://polymarket.com/event/<slug>` URL you built in step 3 — is
   **mandatory and must never be omitted**; it is the whole point of showing a
   live market. Send **only** this card: no sentence before it, no explanation of
   your steps after it. If the mesh lacked quorum, skip the card and say plainly
   that the mesh could not reach quorum.

## Output discipline

Never paste raw API JSON into the reply — it floods context and costs money.
Read only the fields you need and reply with just the card.

## Trading it (custody T1)

If the user wants to *act* on the outcome, do not sign anything. Jupiter's
`POST https://api.jup.ag/prediction/v1/orders` returns an **unsigned**
transaction (it needs an `x-api-key` and the trader's wallet pubkey). Return that
unsigned transaction for a human to sign with `onca-signer`. You hold no key and
submit nothing.
