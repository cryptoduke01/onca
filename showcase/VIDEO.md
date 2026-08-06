# Onca — 3-minute demo (premium cut)

Terminal + phone, no slides. Everything below is real and pre-verified. The goal
is *clean and smooth*: no fumbling, no dead air, every command already on the
clipboard. Read the narration naturally, don't recite it.

## Before you hit record (5-minute setup)

- **Bot is live.** Daemon running on the 0.8.4 host, on Claude. Test it once in a
  fresh chat, then leave that chat clean for the take.
- **Fresh Telegram chat.** Delete the old Onca thread or start clean so no past
  messages are on screen. History is already wiped server-side.
- **Terminal, made for camera:** one window, dark theme, font bumped to ~16–18pt,
  prompt short (`cd` into the repo first so the path is short). Clear it (`Cmd+K`)
  right before each terminal scene.
- **Phone in frame.** Mirror the phone into the same recording: QuickTime →
  New Movie Recording → select your iPhone as the camera (cable). Crisp, and it
  lives in one screen capture so there's nothing to composite.
- **Recorder:** QuickTime or `Cmd+Shift+5`, whole screen, 1080p+. If you can,
  60fps — the card and the animations read smoother.
- **Clipboard loaded.** Copy the three commands below into a notes app so each
  paste is one keystroke on camera. Never type a long command live.
- **Pick a fresh sequence number.** Last on-chain was seq 9/10 — use **11**.

## The three commands (pre-copy these)

```
cd /Users/duke/Desktop/Projects/workspace/zeroclaw-solana
```
```
tools/onca-signer/target/release/onca-signer --sensor dht11-a --value 24 --unit C --seq 11
```
```
tools/onca-market/target/release/onca-market
```

---

## Scene 1 — The problem (0:00–0:15)

On screen: the live Polymarket market in a browser (or the `onca-market` header).

> "Prediction markets pay out on real-world data. This São Paulo weather market
> settles on one weather station. One source. Control it and you control the
> payout — that's the manipulation that's burned Polymarket. Here's the fix."

Keep it to two breaths. Don't over-explain.

## Scene 2 — The agent, on your phone (0:15–1:00)

On screen: the phone, fresh chat with the Onca bot.

Type: **`Attest a reading from dht11-a: 24 C, sequence 11`**

The approval card appears (depin_attest, reading 24, seq 11). Pause on it.

> "This is a self-hosted ZeroClaw agent in my Telegram. It holds no key. It builds
> an unsigned attestation and stops at a human tap."

Tap **Approve**. It replies: *Attestation built — unsigned and ready*, with the
exact signer command in a copy block.

> "Approved. It hands me the command to land it — the key never touches the agent.
> I dispose."

## Scene 3 — On-chain, in one paste (1:00–1:25)

On screen: terminal (cleared). Paste command #2, Enter.

> "The signer holds the device key. It signs the exact reading I approved and
> lands it on devnet."

It prints `SUBMITTED` + an Explorer link. Click it. Show the `onca:attest` memo on
Solana Explorer for a beat.

> "On-chain. That's one node reporting."

## Scene 4 — The mesh catches a liar (1:25–2:20) — the payoff

On screen: back to the phone.

Type: **`settle the São Paulo weather market`**

Let the card land. Don't talk over the reveal.

> "Now settle a live market on the mesh. Four independent nodes. One is lying —
> 999 degrees, signed straight to the chain, past the honest agent."

Point at the card as you read it:

> "The mesh takes the median, drops the liar, freezes it on reputation, and
> settles on the honest three — 23 degrees. There's the live Polymarket link. No
> single source set this number. To move it you'd have to corrupt a majority of
> independent nodes, not one box on one desk."

## Scene 5 — The product (2:20–2:45)

On screen: [onca.run](https://onca.run) (or localhost). Slow scroll from the hero
("The agent proposes. You dispose.") down to the settlement section.

> "It's a real suite — the tools read the chain or build a request a human signs.
> Never a key that can spend."

## Scene 6 — Close (2:45–3:00)

On screen: the custody line, or the repo.

> "Custody tier T0 and T1. Fails closed, prompt-injection tested, no key in the
> agent. Set it up in an evening. That's Onca — prediction markets that can't be
> rigged, because no single source sets the number."

---

## Smoothness checklist (this is what makes it feel premium)

- **Never type a long command** — paste. Type only the short Telegram messages.
- **Let reveals breathe** — the approval card, the settlement card, the Explorer
  page each get ~2 seconds of silence. Rushing reads as nervous.
- **Cut the wait** — the bot takes ~5–10s to reply. In the edit, trim that to ~1s
  so it feels instant. That single cut is the difference between "demo" and
  "product".
- **One clean take per scene** beats one perfect long take. Record scenes
  separately, stitch in the edit.
- **No music with lyrics.** If you add a bed, keep it low and instrumental. The
  bounty wants the real thing, not a hype reel.
- **End on the winning card or the site**, not on a terminal prompt.
