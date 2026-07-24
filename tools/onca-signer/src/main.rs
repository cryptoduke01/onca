//! onca-signer — the "human disposes" side of the custody ladder.
//!
//! `depin-attest` (inside the ZeroClaw agent) builds an UNSIGNED attestation and
//! stops. This tool is what a human runs afterward: it holds the device key the
//! agent never sees, rebuilds the exact same attestation with `onca-core`'s
//! transaction engine, signs it, and submits it to Solana. The agent proposes;
//! this disposes. Keeping it a separate binary is the T1 custody boundary made
//! literal — no key ever lives in the agent or the plugin.
//!
//! Usage (after approving the attestation in Telegram):
//!
//!     onca-signer --sensor dht11-a --value 24.1 --unit C --seq 2 --timestamp 1625148650
//!
//! Device key: `~/.onca/device.json` (a standard Solana keypair; create with
//! `solana-keygen new -o ~/.onca/device.json` and fund it with
//! `solana airdrop 1 <pubkey> --url devnet`).

use std::{env, fs};

use ed25519_dalek::{Signer, SigningKey};
use onca_core::pubkey::Pubkey;
use onca_core::tx::{base64, compile_message, encode_len, memo_instruction};
use serde_json::{json, Value};

const RPC: &str = "https://api.devnet.solana.com";

/// One JSON-RPC call to the cluster. Returns the parsed body (or an `error`
/// object) so the caller can inspect `result` / `error` uniformly.
fn rpc(method: &str, params: Value) -> Value {
    let body = json!({"jsonrpc": "2.0", "id": 1, "method": method, "params": params});
    match ureq::post(RPC).send_json(body) {
        Ok(r) => r.into_json().unwrap_or_else(|e| json!({"error": e.to_string()})),
        Err(ureq::Error::Status(code, r)) => r
            .into_json()
            .unwrap_or_else(|_| json!({"error": format!("HTTP {code}")})),
        Err(e) => json!({"error": e.to_string()}),
    }
}

/// `--flag value` lookup with a default.
fn arg(flag: &str, default: &str) -> String {
    let a: Vec<String> = env::args().collect();
    a.windows(2)
        .find(|w| w[0] == flag)
        .map(|w| w[1].clone())
        .unwrap_or_else(|| default.to_string())
}

fn main() {
    // ── device key (the thing the agent must never hold) ──
    let default_key = format!("{}/.onca/device.json", env::var("HOME").unwrap());
    let keyfile = arg("--keypair", &default_key);
    let raw = fs::read_to_string(&keyfile).unwrap_or_else(|_| {
        eprintln!("no device key at {keyfile}\n  create: solana-keygen new -o {keyfile}");
        std::process::exit(1);
    });
    let bytes: Vec<u8> = serde_json::from_str(&raw).expect("device.json must be a Solana keypair (JSON array of 64 bytes)");
    let seed: [u8; 32] = bytes[..32].try_into().expect("keypair too short");
    let sk = SigningKey::from_bytes(&seed);
    let device = Pubkey::from_base58(&bs58::encode(sk.verifying_key().to_bytes()).into_string()).unwrap();
    eprintln!("device:  {}", device.to_base58());

    // ── the attestation to sign (must match what the agent proposed) ──
    let sensor = arg("--sensor", "dht11-a");
    let value = arg("--value", "24.1");
    let unit = arg("--unit", "C");
    let seq = arg("--seq", "2");
    let ts = arg("--timestamp", "1625148650");
    let memo = format!("onca:attest s={sensor} v={value} u={unit} seq={seq} t={ts}");
    eprintln!("memo:    {memo}");

    // ── ensure the device can pay the fee ──
    let bal = rpc("getBalance", json!([device.to_base58()]))["result"]["value"]
        .as_u64()
        .unwrap_or(0);
    eprintln!("balance: {bal} lamports");
    let funded = bal >= 5000;
    if !funded {
        eprintln!("  unfunded — will run a sigVerify simulate instead of submitting.");
        eprintln!("  to land for real: solana airdrop 1 {} --url devnet  (or faucet.solana.com)", device.to_base58());
    }

    // ── build the SAME transaction depin-attest builds, with a live blockhash ──
    let bh_str = rpc("getLatestBlockhash", json!([{"commitment": "confirmed"}]))["result"]["value"]
        ["blockhash"]
        .as_str()
        .expect("getLatestBlockhash returned no blockhash")
        .to_string();
    let blockhash = Pubkey::from_base58(&bh_str).unwrap().to_bytes();
    let msg = compile_message(&device, blockhash, &[memo_instruction(&memo, &[device])]);

    // ── sign the message bytes and assemble the wire transaction ──
    let signature = sk.sign(&msg.bytes).to_bytes();
    let mut tx = Vec::with_capacity(1 + 64 + msg.bytes.len());
    tx.extend(encode_len(1)); // one signature
    tx.extend_from_slice(&signature);
    tx.extend_from_slice(&msg.bytes);
    let tx_b64 = base64(&tx);

    // ── submit ──
    if funded {
        let resp = rpc(
            "sendTransaction",
            json!([tx_b64, {"encoding": "base64", "preflightCommitment": "confirmed"}]),
        );
        match resp["result"].as_str() {
            Some(sig) => {
                println!("SUBMITTED {sig}");
                println!("https://explorer.solana.com/tx/{sig}?cluster=devnet");
            }
            None => {
                eprintln!("send failed: {resp}");
                std::process::exit(1);
            }
        }
    } else {
        // Unfunded: prove the signature verifies against the runtime. A null err
        // (or one about the fee payer's funds/account, not the signature) means
        // the ed25519 signing and wire assembly are correct — only funding is missing.
        let resp = rpc(
            "simulateTransaction",
            json!([tx_b64, {"encoding": "base64", "sigVerify": true, "commitment": "confirmed"}]),
        );
        let err = &resp["result"]["value"]["err"];
        println!("SIMULATE (sigVerify:true) err = {err}");
        if resp.get("error").is_some() {
            println!("rpc error: {}", resp["error"]);
        }
        println!("A null err, or one about the fee payer's funds/account (not 'signature'), means the signature VERIFIED. Fund the device to land it for real.");
    }
}
