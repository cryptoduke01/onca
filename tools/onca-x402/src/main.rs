//! onca-x402 — the mesh oracle, priced (custody T0, read only).
//!
//! Machine commerce, the bounty's flagship pattern: an agent or a prediction
//! market's resolver pays a micro-fee on Solana to read Onca's trusted,
//! manipulation-resistant value. The x402 handshake:
//!
//!   1. GET /oracle with no payment      -> 402 + payment requirements (a price)
//!   2. caller pays on Solana, retries with `X-PAYMENT: <tx-signature>`
//!   3. server verifies the payment landed on-chain (right amount, to us, once)
//!   4. -> 200 with the mesh's settled value
//!
//! Verification reuses the same getSignaturesForAddress / getTransaction reads
//! the plugins use, plus onca-core's mesh aggregation. It holds no key and moves
//! nothing; the caller pays, we read.
//!
//! The demo prices in native SOL so it is testable with the funded devnet keys
//! we already have; production swaps the asset for SPL USDC (the verify shape,
//! a balance delta credited to the treasury, is identical).
//!
//!   onca-x402 --treasury <pubkey> --price 1000000 --devices <pk,pk,...> --sensor dht11-a

use std::collections::HashSet;
use std::env;
use std::sync::Mutex;

use onca_core::mesh::{aggregate, parse_attest, NodeReading};
use serde_json::{json, Value};
use tiny_http::{Header, Response, Server};

fn arg(flag: &str, default: &str) -> String {
    let a: Vec<String> = env::args().collect();
    a.windows(2).find(|w| w[0] == flag).map(|w| w[1].clone()).unwrap_or_else(|| default.to_string())
}

fn rpc(rpc_url: &str, method: &str, params: Value) -> Value {
    let body = json!({"jsonrpc": "2.0", "id": 1, "method": method, "params": params});
    match ureq::post(rpc_url).send_json(body) {
        Ok(r) => r.into_json().unwrap_or_else(|e| json!({"error": e.to_string()})),
        Err(ureq::Error::Status(code, r)) => r.into_json().unwrap_or_else(|_| json!({"error": format!("HTTP {code}")})),
        Err(e) => json!({"error": e.to_string()}),
    }
}

/// Verify `sig` is a confirmed transaction that credited `treasury` at least
/// `min_lamports` (the price). This is the same balance-delta check
/// `payment-watch` does, on native SOL for the demo.
fn payment_ok(rpc_url: &str, sig: &str, treasury: &str, min_lamports: u64) -> bool {
    // getTransaction can lag behind sendTransaction on a public RPC, so poll a
    // few times before concluding the payment is not there.
    for attempt in 0..10 {
        let tx = rpc(rpc_url, "getTransaction", json!([sig, {"encoding": "json", "maxSupportedTransactionVersion": 0, "commitment": "confirmed"}]));
        let result = &tx["result"];
        if result.is_null() {
            if attempt < 9 {
                std::thread::sleep(std::time::Duration::from_millis(1500));
            }
            continue; // not visible yet
        }
        if !result["meta"]["err"].is_null() {
            return false; // failed tx
        }
        let Some(keys) = result["transaction"]["message"]["accountKeys"].as_array() else {
            return false;
        };
        let Some(idx) = keys.iter().position(|k| k.as_str() == Some(treasury)) else {
            return false; // treasury not credited by this tx
        };
        let pre = result["meta"]["preBalances"][idx].as_u64().unwrap_or(0);
        let post = result["meta"]["postBalances"][idx].as_u64().unwrap_or(0);
        return post.saturating_sub(pre) >= min_lamports;
    }
    false
}

/// Read each mesh node's latest attestation and settle (median, outliers
/// dropped). Fault-tolerant per node; quorum guards the result.
fn read_mesh(rpc_url: &str, devices: &[String], sensor: &str, tolerance: f64, quorum: usize) -> onca_core::mesh::Aggregate {
    let mut readings = Vec::new();
    for device in devices {
        let res = rpc(rpc_url, "getSignaturesForAddress", json!([device, {"limit": 25}]));
        if let Some(entries) = res["result"].as_array() {
            for e in entries {
                let memo = e["memo"].as_str().unwrap_or("");
                if let Some(start) = memo.find("onca:attest") {
                    if let Some((s, value, seq, timestamp)) = parse_attest(&memo[start..]) {
                        if s == sensor {
                            readings.push(NodeReading { device: device.clone(), value, seq, timestamp });
                            break;
                        }
                    }
                }
            }
        }
    }
    aggregate(&readings, tolerance, quorum)
}

fn reply(status: u16, body: Value) -> Response<std::io::Cursor<Vec<u8>>> {
    let json = body.to_string().into_bytes();
    let ct = Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap();
    Response::from_data(json).with_status_code(status).with_header(ct)
}

fn main() {
    let rpc_url = arg("--rpc", &std::env::var("ONCA_RPC").unwrap_or_else(|_| "https://api.devnet.solana.com".to_string()));
    let treasury = arg("--treasury", "");
    let price: u64 = arg("--price", "1000000").parse().unwrap_or(1_000_000); // 0.001 SOL demo
    let devices: Vec<String> = arg("--devices", "").split(',').filter(|s| !s.is_empty()).map(str::to_string).collect();
    let sensor = arg("--sensor", "dht11-a");
    let tolerance: f64 = arg("--tolerance", "5").parse().unwrap_or(5.0);
    let quorum: usize = arg("--quorum", "3").parse().unwrap_or(3);
    let addr = arg("--listen", "127.0.0.1:8402");
    if treasury.is_empty() || devices.is_empty() {
        eprintln!("usage: onca-x402 --treasury <pubkey> --devices <pk,pk,...> [--price 1000000] [--sensor dht11-a]");
        std::process::exit(1);
    }

    let used: Mutex<HashSet<String>> = Mutex::new(HashSet::new());
    let server = Server::http(&addr).expect("bind");
    println!("onca-x402: GET http://{addr}/oracle");
    println!("  price {price} lamports (SOL) -> pay {treasury}, then retry with header  X-PAYMENT: <tx-signature>");

    for req in server.incoming_requests() {
        if req.url() != "/oracle" {
            let _ = req.respond(reply(404, json!({"error": "GET /oracle"})));
            continue;
        }
        let payment = req
            .headers()
            .iter()
            .find(|h| h.field.equiv("X-PAYMENT"))
            .map(|h| h.value.as_str().to_string());

        let resp = match payment {
            None => reply(402, json!({
                "x402Version": 1,
                "accepts": [{
                    "scheme": "exact",
                    "network": "solana:devnet",
                    "maxAmountRequired": price.to_string(),
                    "asset": "SOL",
                    "payTo": treasury,
                    "resource": "/oracle",
                    "description": "Onca trusted mesh temperature (median of independent nodes, outliers dropped)"
                }]
            })),
            Some(sig) => {
                let mut seen = used.lock().unwrap();
                if seen.contains(&sig) {
                    reply(402, json!({"error": "payment already used", "code": "replayed"}))
                } else if payment_ok(&rpc_url, &sig, &treasury, price) {
                    seen.insert(sig.clone());
                    let agg = read_mesh(&rpc_url, &devices, &sensor, tolerance, quorum);
                    if agg.has_quorum {
                        reply(200, json!({
                            "sensor": sensor,
                            "value": agg.value,
                            "unit": "C",
                            "nodes_agreed": agg.inliers.len(),
                            "nodes_rejected": agg.outliers.len(),
                            "payment": sig,
                        }))
                    } else {
                        reply(503, json!({"error": "no quorum", "nodes_agreed": agg.inliers.len(), "quorum": quorum}))
                    }
                } else {
                    reply(402, json!({"error": "payment not found, unconfirmed, or below price", "code": "unpaid"}))
                }
            }
        };
        let _ = req.respond(resp);
    }
}
