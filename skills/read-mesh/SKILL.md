---
name: read-mesh
description: >-
  Read the Onca DePIN mesh oracle live and report each node's temperature attestation, dropping liars by median and reputation. Use when asked to read the mesh, show the oracle, check the nodes, what the sensors report, or the current trusted temperature — WITHOUT settling a market.
license: MIT
version: 0.1.0
---

# Read the Mesh

Report the live Onca mesh oracle: every node's latest on-chain temperature
attestation, with outliers dropped and repeat liars frozen out. This is the pure
read-only oracle — **no market, no settlement, no key.** Use it when the user
just wants to see what the mesh is reporting right now.

Use the `http_request` tool (host HTTP, reliable). Do **not** use `mesh_oracle`;
its in-sandbox HTTP client is flaky.

## Steps

1. **Read the mesh in ONE batch call.** POST to `https://api.devnet.solana.com`
   with `Content-Type: application/json` and this exact JSON-RPC batch body (one
   `getSignaturesForAddress` per node):
   ```json
   [{"jsonrpc":"2.0","id":0,"method":"getSignaturesForAddress","params":["3xQ33DfPLL6py9zCZAm4CfowL6TPZbiMNJrBRPudhSNR",{"limit":25}]},
    {"jsonrpc":"2.0","id":1,"method":"getSignaturesForAddress","params":["GhriBBob3iUczrGR81mXaKQ9LJBpF2STU8uEnVZLoX9a",{"limit":25}]},
    {"jsonrpc":"2.0","id":2,"method":"getSignaturesForAddress","params":["AxRKqyyT9DXbKGMf47ULRCEqwRPQgCa1nX1UdVNTGQGh",{"limit":25}]},
    {"jsonrpc":"2.0","id":3,"method":"getSignaturesForAddress","params":["BtpDcpYMfeZa6MtwFX6VeAnHjyq6qqh9V2X86oKQdUDy",{"limit":25}]}]
   ```
   The response is an array of 4 results — match each back to its node by its
   `id` field (0 = `3xQ3…`, 1 = `Ghri…`, 2 = `AxRK…`, 3 = `BtpD…`); array order
   may differ from the request. For each node, scan its entries (newest first) for
   the latest `memo` containing `onca:attest s=dht11-a`, and read the number right
   after `v=`. Copy that number **verbatim** — never round, average, or reuse
   another node's value.

2. **Aggregate — the anti-manipulation core.** Take the median of the readings.
   **Drop** any reading more than 5°C from that median (a lying or broken node,
   e.g. one reporting 999). Require at least **3** surviving readings (quorum) —
   if fewer, say the mesh lacks quorum and stop. The median of the survivors is
   the trusted oracle value; note how many agreed and how many you rejected.

3. **Reply with the Onca mesh card**, in exactly this shape (Telegram Markdown),
   filled with the real values. Nothing before or after it, no preamble:

   ```
   🐆 *Onca Oracle · Mesh*

   sensor `dht11-a` · 4 independent nodes
   `✅ 3xQ3…  23.4°C`
   `✅ Ghri…  23.6°C`
   `✅ AxRK…  23.1°C`
   `❌ BtpD…   999°C  frozen`

   *Oracle value* · `23.4°C`  ·  3 of 4 trusted & agree

   _No single source set this. Corrupt a minority, the median holds._
   ```

   Use the real node short-ids (first 4 chars), each node's actual reading, a ✅
   for a counted node and a ❌ for a dropped/frozen one, and the true trusted
   count. Send **only** this card — no sentence before it, no step-by-step after.
   If the mesh lacked quorum, skip the card and say plainly the mesh could not
   reach quorum.

## Output discipline

Never paste raw API JSON into the reply — read only the fields you need and reply
with just the card. This is a read: you hold no key and submit nothing.
