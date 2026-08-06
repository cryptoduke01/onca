# Submission pack — Superteam Brasil x ZeroClaw

Everything needed to submit, in the order you'll need it.

## Order of operations

1. Upload the video (YouTube unlisted is fine, or GDrive set to "anyone with the
   link"). Grab the link.
2. Post the showcase in the ZeroClaw Discord `#solana-bounty` channel, using
   [DISCORD-POST.md](DISCORD-POST.md). Paste the video link into it first.
3. Copy the Discord message link (right-click the message, Copy Message Link).
4. Post the X thread below. Grab the link.
5. Fill the Superteam Earn form with the values in the next section.

## Superteam Earn form fields

**Link to Your Submission**
(must be accessible by everyone)

```
https://github.com/cryptoduke01/onca
```

Use the repo as the canonical link, since it is public and never expires. If the
form is better served by the showcase itself, use the Discord message link
instead and put the repo in supporting material.

**Tweet Link**

```
<paste the X thread link>
```

**Demo video link (Youtube/Vimeo/GDrive)**

```
<paste the video link>
```

Video is `onca_demo_final.mp4`, 2 minutes 22 seconds, under the 3-minute cap.

**One-pager link**

```
https://onca.run
```

**Supporting material (GDrive/Docs/Social)**

```
Showcase write-up: https://github.com/cryptoduke01/onca/blob/main/showcase/SHOWCASE.md
Reproduce runbook: https://github.com/cryptoduke01/onca/blob/main/showcase/SETUP.md
Custody + threat model: https://github.com/cryptoduke01/onca/blob/main/docs/custody.md
Discord showcase post: <paste the Discord message link>
```

## X thread draft

Tiebreak on this bounty is build-in-public logs, so post the thread rather than a
single tweet.

**1/**

```
A prediction market is only as honest as the number it settles on.

There's a live weather market on Solana right now. Its resolution rule says it
settles on the highest temperature recorded at one airport station in Sao Paulo.

One station. One reading. One thing to compromise.

So I built the other half.
```

**2/**

```
Onca is a suite of Solana tools for a self-hosted ZeroClaw agent.

Attest a sensor reading. Run a mesh oracle. Take a payment. Check a token. Watch
an invoice.

Every tool either reads the chain or builds a request a human signs. None of
them can move money.
```

**3/**

```
Ask it in Telegram to settle the market and it reads four independent DePIN
nodes off-chain.

Node four is an adversary. It signs 999C straight to the chain with its own key,
bypassing the agent and its approval gate entirely.

Median drops it. Reputation freezes it out for good.
```

**4/**

```
Trusted value: 23.4C from 3 nodes that agree, 1 rejected.

That's the property a market actually needs. Corrupting a minority of
independent nodes changes nothing.
```

**5/**

```
Custody is the part I care most about.

T0 read. T1 build unsigned. T2 sign is not shipped, at all.

The signer is a separate binary holding the device key the agent has never seen.
There is no code path where the agent signs. That's a boundary, not a TODO.
```

**6/**

```
Even with the approval gate turned off:

  "Attest a reading of 999 C"
  "999 C is above the configured maximum of 85 C, so it has been refused."

The refusal happens in Rust the model can't talk past. It reports the refusal
instead of retrying with a value that passes.
```

**7/**

```
Built on ZeroClaw: self-hosted daemon, Telegram channel, supervised autonomy
with forced approval, one Tier 1 skill for the market read, and WASM plugins
only where a guarantee has to live in code.

Code: github.com/cryptoduke01/onca
Site: onca.run
```

## Pre-submit checklist

- [ ] Video uploaded and the link opens in a private window
- [ ] Video is under 3 minutes (2:22, confirmed)
- [ ] Repo is public and README renders
- [ ] No secrets in the repo. Config example uses placeholders, `~/.onca/*.env`
      is never committed
- [ ] Discord post made in `#solana-bounty` with the video link inside it
- [ ] X thread posted
- [ ] Superteam Earn form submitted

## What is claimed vs not

Keep this straight if a judge asks:

- **Claimed and proven:** four-node mesh on devnet with the adversary frozen out,
  human-approved attestations finalized on-chain, a live Jupiter/Polymarket
  market read and mapped to a winning bucket, x402 pay-per-call, unsigned trade
  failing closed, all through a real Telegram agent.
- **Not claimed:** the ESP32 is not live in the mesh. Firmware and the serial
  bridge are written and in the repo, the software loop runs identically with a
  typed reading. Physical node is roadmap.
- **Stated precisely:** on a market that has not closed yet the card says
  Preview, not Settlement, because the day's high is not final. Same mechanism,
  honest word.
