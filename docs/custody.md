# Custody and the threat model

This document explains the custody ladder and the threat model for the whole
Onca suite. Each plugin README repeats the part that applies to it. Read this
document for the shared picture.

## The problem

An agent joins a private key to a language model. The model reads text that you
do not control: chat messages, mail, and web pages. An attacker can hide an
instruction in that text. If the agent can sign and send, one successful hidden
instruction can move your funds. This attack is prompt injection.

You cannot remove prompt injection from a language model. So Onca limits what a
successful injection can reach. The rule is simple. The agent proposes. A person,
a multisig, or a limited session key disposes.

## The custody ladder

The bounty defines four tiers. Onca uses only the two safe tiers.

| Tier | Name | The tool can | Secrets it holds |
|---|---|---|---|
| T0 | Read | read the chain and report | an RPC key at most |
| T1 | Build | return an unsigned request for a person to sign | none |
| T2 | Sign | sign and send | a scoped session key |

Onca ships T0 and T1 tools only. It ships no T2 tool.

The reason is direct. A T2 tool is the tier where one successful injection empties
a wallet. A T2 tool is possible with hard spend caps, a mint allowlist inside the
plugin, a session key that holds little, and an approval gate. None of the Onca
tools need that risk to do their job, so none of them take it.

## Where the rules live

Each guardrail lives in the pure core of its plugin, not in the prompt. The
guardrail runs on every call. The model never sees the config that limits it,
and the model cannot turn the config off. To pass a guardrail, a person would
have to change the source and build the component again.

| Plugin | Guardrail | Enforced in |
|---|---|---|
| solana-pay-request | max amount, mint allowlist, address check, memo encoding | [`pay.rs`](../plugins/solana-pay-request/src/pay.rs) |
| token-risk-check | verdict from chain facts only; refuse a malformed mint | [`risk.rs`](../plugins/token-risk-check/src/risk.rs) |
| payment-watch | amount check in base units; scan every signature | [`watch.rs`](../plugins/payment-watch/src/watch.rs) |

## How each tool fails closed

A tool fails closed when the unsafe path returns "no", not "maybe". Onca proves
this with a test for each tool. Each test sends a hostile input and checks that
the tool refuses.

- `solana-pay-request` refuses an amount over the cap, a token off the allowlist,
  and an invalid recipient. See `prompt_injection_fails_closed`.
- `token-risk-check` returns red for a honeypot, whatever the message claims. See
  `honeypot_is_red_regardless_of_claims`.
- `payment-watch` returns underpaid for dust and pending for a failed
  transaction. It never returns paid from a message alone. See
  `dust_only_is_underpaid_not_paid` and `dust_after_real_payment_still_resolves_paid`.

## What a secret can reach

The table shows the worst case for each secret. No secret in Onca can spend
funds.

| Secret | Held by | Worst case if leaked |
|---|---|---|
| RPC URL with API key | token-risk-check, payment-watch | read access to a node, and RPC quota use |
| (none) | solana-pay-request | nothing to leak |

The RPC URL can carry an API key, so no tool writes the URL to a log or puts it
in an error message. An error reports the HTTP status code, not the URL.
