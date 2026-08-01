//! onca-oracle — the read side of the oracle (custody T0, read only).
//!
//! `depin-attest` + `onca-signer` put each node's readings on-chain. This tool
//! reads them back across a *mesh* of independent devices and settles on the
//! manipulation-resistant aggregate from `onca-core::mesh`: the median of the
//! nodes that agree, with outliers (a lying or broken sensor) dropped. That
//! single value is what a weather prediction market consumes to settle — and no
//! minority of corrupted nodes can move it.
//!
//! Usage:
//!
//!     onca-oracle --devices <pubkey,pubkey,...> --sensor dht11-a [--tolerance 5] [--quorum 3]

use std::env;

use std::collections::HashMap;
use std::fs;

use onca_core::mesh::{aggregate_trusted, parse_attest, update_reputation, NodeReading, Reputation};
use serde_json::{json, Value};

const RPC: &str = "https://api.devnet.solana.com";

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

fn arg(flag: &str, default: &str) -> String {
    let a: Vec<String> = env::args().collect();
    a.windows(2)
        .find(|w| w[0] == flag)
        .map(|w| w[1].clone())
        .unwrap_or_else(|| default.to_string())
}

/// The latest `onca:attest` reading a device published for `sensor`, if any.
/// getSignaturesForAddress returns entries newest-first, each with the tx's memo
/// (sometimes prefixed with a `[len] ` marker we skip past).
fn latest_reading(device: &str, sensor: &str) -> Option<NodeReading> {
    let sigs = rpc("getSignaturesForAddress", json!([device, {"limit": 25}]));
    for entry in sigs["result"].as_array()? {
        let memo = entry["memo"].as_str().unwrap_or("");
        let Some(start) = memo.find("onca:attest") else { continue };
        if let Some((s, value, seq, timestamp)) = parse_attest(&memo[start..]) {
            if s == sensor {
                return Some(NodeReading { device: device.to_string(), value, seq, timestamp });
            }
        }
    }
    None
}

fn short(device: &str) -> &str {
    &device[..device.len().min(4)]
}

fn main() {
    let devices = arg("--devices", "");
    let sensor = arg("--sensor", "dht11-a");
    let tolerance: f64 = arg("--tolerance", "5.0").parse().unwrap_or(5.0);
    let quorum: usize = arg("--quorum", "3").parse().unwrap_or(3);
    let min_score: f64 = arg("--min-score", "0.4").parse().unwrap_or(0.4);
    if devices.is_empty() {
        eprintln!("usage: onca-oracle --devices <pubkey,pubkey,...> --sensor dht11-a [--tolerance 5] [--quorum 3] [--min-score 0.4]");
        std::process::exit(1);
    }

    // Reputation persists across reads: a node that keeps getting rejected loses
    // trust and is eventually frozen out — even a plausible reading from it.
    let rep_path = format!("{}/.onca/reputation.json", env::var("HOME").unwrap_or_default());
    let mut reps: HashMap<String, Reputation> = fs::read_to_string(&rep_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();

    println!("mesh oracle · sensor={sensor} · tolerance=±{tolerance} · quorum={quorum} · min-trust={min_score}\n");
    let mut readings = Vec::new();
    for device in devices.split(',').filter(|s| !s.is_empty()) {
        match latest_reading(device, &sensor) {
            Some(r) => {
                println!("  node {}…  reading {} (seq {})", short(device), r.value, r.seq);
                readings.push(r);
            }
            None => println!("  node {}…  no {sensor} attestation found", short(device)),
        }
    }

    let agg = aggregate_trusted(&readings, &reps, tolerance, quorum, min_score);
    update_reputation(&mut reps, &agg);
    let _ = fs::write(&rep_path, serde_json::to_string_pretty(&reps).unwrap_or_default());

    println!();
    // Each node's standing after this round: score, and whether it is frozen out.
    for r in &readings {
        let score = reps.get(&r.device).map(Reputation::score).unwrap_or(0.5);
        let tag = if score < min_score { "  FROZEN OUT" } else { "" };
        println!("  node {}…  trust {:.2}{tag}", short(&r.device), score);
    }
    println!();
    if !agg.outliers.is_empty() {
        let liars: Vec<String> = agg.outliers.iter().map(|r| format!("{}…={}", short(&r.device), r.value)).collect();
        println!("  rejected this round: {}", liars.join(", "));
    }
    if agg.has_quorum {
        println!("  ORACLE VALUE: {} {}  ({} of {} nodes trusted & agree)", agg.value, sensor_unit(&sensor), agg.inliers.len(), readings.len());
    } else {
        println!("  NO SETTLEMENT: only {} trusted node(s) agreed, quorum is {}", agg.inliers.len(), quorum);
        std::process::exit(2);
    }
}

fn sensor_unit(_sensor: &str) -> &'static str {
    "C"
}
