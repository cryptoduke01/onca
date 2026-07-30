//! onca-resolve — the consumer side of the oracle (x402 client + market resolver).
//!
//! A prediction market's resolver needs a trusted number to settle. It pays
//! Onca's x402 oracle a micro-fee (a real Solana transfer built and signed with
//! onca-core), reads back the manipulation-resistant value, and resolves the
//! market. That closes the machine-commerce loop: Onca sells the reading
//! (onca-x402), this agent buys it, and a prediction settles on the answer.
//! No key ever leaves this tool.
//!
//!   onca-resolve --treasury <pubkey> --price 1000000 \
//!     --question "Lagos > 25C tomorrow" --op gt --threshold 25 [--keypair <path>]

use std::{env, fs, thread, time::Duration};

use ed25519_dalek::{Signer, SigningKey};
use onca_core::pubkey::Pubkey;
use onca_core::tx::{base64, compile_message, encode_len, transfer_instruction};
use serde_json::{json, Value};

const RPC: &str = "https://api.devnet.solana.com";

fn arg(flag: &str, default: &str) -> String {
    let a: Vec<String> = env::args().collect();
    a.windows(2).find(|w| w[0] == flag).map(|w| w[1].clone()).unwrap_or_else(|| default.to_string())
}

fn rpc(method: &str, params: Value) -> Value {
    let body = json!({"jsonrpc": "2.0", "id": 1, "method": method, "params": params});
    match ureq::post(RPC).send_json(body) {
        Ok(r) => r.into_json().unwrap_or_else(|e| json!({"error": e.to_string()})),
        Err(ureq::Error::Status(code, r)) => r.into_json().unwrap_or_else(|_| json!({"error": format!("HTTP {code}")})),
        Err(e) => json!({"error": e.to_string()}),
    }
}

fn main() {
    let oracle = arg("--oracle", "http://127.0.0.1:8402/oracle");
    let treasury = arg("--treasury", "");
    let price: u64 = arg("--price", "1000000").parse().unwrap_or(1_000_000);
    let question = arg("--question", "temperature over threshold");
    let op = arg("--op", "gt"); // gt | lt
    let threshold: f64 = arg("--threshold", "25").parse().unwrap_or(25.0);
    let keyfile = arg("--keypair", &format!("{}/.onca/mesh/node1.json", env::var("HOME").unwrap()));
    if treasury.is_empty() {
        eprintln!("usage: onca-resolve --treasury <pubkey> --question \"...\" --op gt|lt --threshold 25");
        std::process::exit(1);
    }

    // Resolver's own wallet (it pays; it holds nothing of the oracle's).
    let raw = fs::read_to_string(&keyfile).unwrap_or_else(|_| { eprintln!("no keypair at {keyfile}"); std::process::exit(1); });
    let bytes: Vec<u8> = serde_json::from_str(&raw).expect("keypair must be a Solana JSON array");
    let seed: [u8; 32] = bytes[..32].try_into().expect("bad keypair");
    let sk = SigningKey::from_bytes(&seed);
    let resolver = Pubkey::from_base58(&bs58::encode(sk.verifying_key().to_bytes()).into_string()).unwrap();
    eprintln!("resolver: {}", resolver.to_base58());
    eprintln!("market:   \"{question}\"  → settle YES if temp {op} {threshold}");

    // 1) Pay the oracle: a real SOL transfer resolver -> treasury, built + signed here.
    let treas = Pubkey::from_base58(&treasury).expect("bad treasury pubkey");
    let bh_str = rpc("getLatestBlockhash", json!([{"commitment": "confirmed"}]))["result"]["value"]["blockhash"]
        .as_str().expect("no blockhash").to_string();
    let blockhash = Pubkey::from_base58(&bh_str).unwrap().to_bytes();
    let msg = compile_message(&resolver, blockhash, &[transfer_instruction(&resolver, &treas, price)]);
    let signature = sk.sign(&msg.bytes).to_bytes();
    let mut tx = Vec::with_capacity(1 + 64 + msg.bytes.len());
    tx.extend(encode_len(1));
    tx.extend_from_slice(&signature);
    tx.extend_from_slice(&msg.bytes);
    let send = rpc("sendTransaction", json!([base64(&tx), {"encoding": "base64", "preflightCommitment": "confirmed"}]));
    let paysig = match send["result"].as_str() {
        Some(s) => s.to_string(),
        None => { eprintln!("payment failed: {send}"); std::process::exit(1); }
    };
    eprintln!("paid {price} lamports → x402 tx {paysig}");
    thread::sleep(Duration::from_secs(9)); // confirmation

    // 2) Buy the reading: call the paywalled oracle with the payment.
    let body: Value = match ureq::get(&oracle).set("X-PAYMENT", &paysig).call() {
        Ok(r) => r.into_json().unwrap_or_else(|_| json!({})),
        Err(ureq::Error::Status(_, r)) => r.into_json().unwrap_or_else(|_| json!({})),
        Err(e) => { eprintln!("oracle call failed: {e}"); std::process::exit(1); }
    };
    let Some(value) = body["value"].as_f64() else {
        eprintln!("oracle did not return a value: {body}");
        std::process::exit(1);
    };

    // 3) Resolve the market on the trusted value.
    let outcome = if op == "lt" { value < threshold } else { value > threshold };
    println!("MARKET      {question}");
    println!("ORACLE      {value} C  ({} nodes agreed, {} rejected)", body["nodes_agreed"], body["nodes_rejected"]);
    println!("CONDITION   temp {op} {threshold}");
    println!("SETTLEMENT  {}", if outcome { "YES" } else { "NO" });
}
