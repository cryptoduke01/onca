//! onca-trade — build an UNSIGNED order on the outcome the mesh settled (custody T1).
//!
//! `onca-market` reads the mesh and picks the winning outcome bucket of a live
//! Jupiter/Polymarket weather market. This tool takes that bucket's `marketId`
//! and asks Jupiter to construct a buy order on it, printing the **unsigned**
//! transaction Jupiter returns. It holds no key, signs nothing, submits nothing.
//! A human signs the unsigned tx — in their wallet, or with `onca-signer`. The
//! agent proposes; a human disposes.
//!
//! Jupiter's `POST /orders` returns the unsigned tx for a funded wallet and fails
//! closed otherwise ("Minimum order is $5", then "Insufficient funds"). That gate
//! is honest custody: no funds, no order, and the agent never held a key anyway.
//!
//!   onca-trade --market POLY-3350728 --owner <pubkey> [--amount 5] [--no] [--mint <USDC>]

use std::env;

use serde_json::{json, Value};

const JUP: &str = "https://api.jup.ag/prediction/v1";
// USDC mainnet mint (Jupiter prediction markets settle in USDC / JupUSD).
const USDC: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

fn arg(flag: &str, default: &str) -> String {
    let a: Vec<String> = env::args().collect();
    a.windows(2).find(|w| w[0] == flag).map(|w| w[1].clone()).unwrap_or_else(|| default.to_string())
}

fn has_flag(flag: &str) -> bool {
    env::args().any(|a| a == flag)
}

fn main() {
    let market = arg("--market", "");
    let owner = arg("--owner", "");
    let amount_usd: f64 = arg("--amount", "5").parse().unwrap_or(5.0);
    let mint = arg("--mint", USDC);
    let is_yes = !has_flag("--no"); // default buys YES; --no buys the NO side
    if market.is_empty() || owner.is_empty() {
        eprintln!("usage: onca-trade --market <outcome marketId> --owner <pubkey> [--amount 5] [--no]");
        eprintln!("  run onca-market first to get the winning outcome's marketId");
        std::process::exit(1);
    }

    // Jupiter deposit amounts are native units: 1_000_000 = $1.00.
    let deposit = format!("{}", (amount_usd * 1_000_000.0).round() as u64);
    let body = json!({
        "ownerPubkey": owner,
        "marketId": market,
        "isYes": is_yes,
        "isBuy": true,
        "depositAmount": deposit,
        "depositMint": mint,
    });

    println!("ORDER (custody T1 — building an UNSIGNED transaction, signing nothing)");
    println!("  market {market}  ·  side {}  ·  ${amount_usd}  ·  owner {}…", if is_yes { "YES" } else { "NO" }, &owner[..owner.len().min(6)]);

    let resp: Value = match ureq::post(&format!("{JUP}/orders")).send_json(body) {
        Ok(r) => r.into_json().unwrap_or_else(|e| json!({"message": e.to_string()})),
        Err(ureq::Error::Status(code, r)) => {
            let mut v = r.into_json().unwrap_or_else(|_| json!({"message": format!("HTTP {code}")}));
            v["_http"] = json!(code);
            v
        }
        Err(e) => { eprintln!("request failed: {e}"); std::process::exit(1); }
    };

    if let Some(tx) = resp["transaction"].as_str() {
        // Funded wallet: Jupiter handed back the unsigned order transaction.
        println!("\n  UNSIGNED TX (base64, {} bytes):", tx.len());
        println!("  {}…{}", &tx[..tx.len().min(48)], &tx[tx.len().saturating_sub(12)..]);
        let ord = &resp["order"];
        if !ord.is_null() {
            println!("  order {}  ·  contracts {}", ord["orderPubkey"].as_str().unwrap_or("?"), ord["contracts"]);
        }
        println!("\n  Sign it yourself: your wallet, or onca-signer. Onca built it and holds no key.");
    } else {
        // No funds / below minimum: the order fails closed, exactly as it should.
        let msg = resp["message"].as_str().unwrap_or("order not created");
        let code = resp["code"].as_str().unwrap_or("");
        println!("\n  NO ORDER BUILT: {msg}{}", if code.is_empty() { String::new() } else { format!("  [{code}]") });
        println!("  Fails closed without USDC — the agent never held a key, so nothing is at risk.");
        println!("  Fund the owner wallet with >= $5 USDC (mainnet) to receive a real unsigned order to sign.");
    }
}
