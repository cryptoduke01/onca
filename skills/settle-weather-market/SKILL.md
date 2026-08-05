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

You have two tools for this:

- `mesh_oracle` — returns the trusted temperature: the mesh median, how many
  nodes agreed, and how many were rejected. This is the value with no single
  source behind it.
- `http_request` — plain HTTPS. Use it to read the live market from Jupiter's
  prediction API (keyless for reads; Polymarket liquidity routed onto Solana).

## Steps

1. **Read the trusted value.** Call `mesh_oracle`. Keep `value` (°C),
   `nodes_agreed`, and `nodes_rejected`.

2. **Find the market** (skip if the user gave an event id). GET
   `https://api.jup.ag/prediction/v1/events/search?query=highest%20temperature%20Sao%20Paulo&limit=20`
   and pick the event whose `metadata.closeTime` is still in the future (the
   daily São Paulo market rolls over at 12:00 UTC). Take its `eventId`.

3. **Read the outcome buckets.** GET
   `https://api.jup.ag/prediction/v1/events/{eventId}?includeMarkets=true`.
   Each entry in `markets` is a bucket; its label is
   `metadata.groupItemTitle` (e.g. `23°C`, `21°C or below`, `31°C or higher`)
   and its id is `marketId`. Read the integer °C out of each label.

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
