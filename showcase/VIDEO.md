# Onca — 3-minute showcase script (shot-by-shot)

Terminal + phone, no slides. Total target: under 3:00. Read the narration
naturally, do not read it word for word. Everything below actually runs.

**Honesty guardrail (say this, do not fudge it):** our devnet mesh nodes and the
DHT11 are the *mechanism*. In production the nodes sit where the market is (São
Paulo); here our nodes drive the same end-to-end settlement. Do not imply a Jos
sensor is measuring São Paulo weather. Show the mechanism, claim only that.

Before recording, grab today's open São Paulo market (the daily one rolls over at
12:00 UTC):

```bash
curl -s "https://api.jup.ag/prediction/v1/events/search?query=highest%20temperature%20Sao%20Paulo&limit=20" | python3 -c "import sys,json;[print(e['eventId'], e['metadata']['title']) for e in json.load(sys.stdin)['data'] if 'Sao Paulo' in e['metadata']['title']]"
```

Use that `POLY-…` id as `--event` in scene 4.

---

## Scene 1 — the problem (0:00–0:20)

On screen: the live market in a browser or the `onca-market` header in the terminal.

> "Prediction markets settle on data. A market that settles on one oracle is a
> market you can rob. That is the Polymarket manipulation. Here is a live weather
> market on Solana, and here is how you settle it with no single source to bribe."

## Scene 2 — the agent, on a real channel (0:20–1:10)

On screen: phone, Telegram DM with the Onca bot. (Daemon already running.)

Type in Telegram:

> Attest a reading from dht11-a: 23.7 C, sequence 8

The approval card appears (Tool: depin_attest, reading 23.7, seq 8). Tap **Approve**.

> "This is a self-hosted ZeroClaw agent in my Telegram. It does not hold a key. It
> builds an unsigned attestation and stops at a human tap. I approve, and it hands
> back the unsigned transaction. The agent proposes, I dispose."

## Scene 3 — land it on-chain (1:10–1:35)

On screen: terminal.

```bash
tools/onca-signer/target/release/onca-signer --sensor dht11-a --value 23.7 --unit C --seq 8
```

> "The signer holds the device key the agent never sees. It signs the exact
> reading I approved and lands it on devnet."

Click the explorer link. Show the `onca:attest` memo on-chain.

## Scene 4 — the mesh settles a REAL market (1:35–2:30) ← the payoff

On screen: terminal. Use today's open event id.

```bash
tools/onca-market/target/release/onca-market --event POLY-798942
```

> "Now the read side. Four independent nodes attested on-chain. One is an
> attacker signing 999 degrees, straight to the chain, past the honest agent.
> The mesh takes the median, drops the outlier, and freezes the liar on
> reputation, so even a plausible lie from it later is ignored. That trusted
> value maps to the winning outcome on the live São Paulo market. No single
> source set it. To move it you would have to corrupt a majority of independent
> nodes, not one box on one desk."

Point at `FROZEN OUT` and the winning bucket.

## Scene 5 — machine commerce, optional (2:30–2:50)

Two terminals. Left: the paid oracle. Right: a resolver paying for the read.

```bash
# left
tools/onca-x402/target/release/onca-x402 --treasury BMpwFSKbLJvPpK4yo5EoqBiQUDxt9NdgFRToXJpiphrC --price 1000000 --devices "3xQ33DfPLL6py9zCZAm4CfowL6TPZbiMNJrBRPudhSNR,GhriBBob3iUczrGR81mXaKQ9LJBpF2STU8uEnVZLoX9a,AxRKqyyT9DXbKGMf47ULRCEqwRPQgCa1nX1UdVNTGQGh,BtpDcpYMfeZa6MtwFX6VeAnHjyq6qqh9V2X86oKQdUDy" --sensor dht11-a
# right
tools/onca-resolve/target/release/onca-resolve --treasury BMpwFSKbLJvPpK4yo5EoqBiQUDxt9NdgFRToXJpiphrC --price 1000000 --question "Sao Paulo temp bucket" --op gt --threshold 25
```

> "The oracle also sells its reading over x402. An agent pays a micro-fee on
> Solana, and only then gets the value. Sell your capability, pay for data, all
> under a cap."

## Scene 6 — close (2:50–3:00)

On screen: the custody table or the repo.

> "No tool in the agent holds a spending key. Read and build only, T0 and T1.
> Prompt-injection tested, fails closed. Config and runbook are in the repo. You
> can set this up in an evening. That is Onca."

---

## Pre-flight checklist

- [ ] Daemon running: `zeroclaw daemon --host 127.0.0.1 --port 42617` (with Groq + Telegram env exported)
- [ ] Telegram chat already bound (bot replies)
- [ ] Device funded (signer scene): `solana balance <device> --url devnet`
- [ ] Fresh open São Paulo event id for scene 4
- [ ] Port 8402 free for scene 5 (`lsof -iTCP:8402`)
- [ ] Screen recorder + phone mirror ready
