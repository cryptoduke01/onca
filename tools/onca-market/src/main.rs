//! onca-market — settle a *real* prediction market on the mesh, not a single source.
//!
//! Kaue (Superteam dev lead) asked for exactly this: a weather prediction market
//! settled on provably attested temperature with no trusted single source, and a
//! mesh so an insider cannot manipulate the reading — the failure that has burned
//! Polymarket. This tool closes that loop against a market that exists **today**:
//!
//!   1. It reads a live weather market from Jupiter's prediction API (keyless GET,
//!      Polymarket liquidity routed onto Solana). That market settles on ONE
//!      oracle — the single-source risk.
//!   2. It reads the Onca mesh: many independent DePIN nodes' on-chain temperature
//!      attestations, aggregated to the median with outliers dropped and repeat
//!      liars frozen out on reputation (`onca-core::mesh`).
//!   3. It maps the mesh value to the market's winning outcome bucket — the
//!      settlement no minority of nodes could move.
//!
//! Custody T0: it reads and reports, holds no key, moves nothing. To actually
//! trade the outcome, Jupiter's `POST /orders` returns an *unsigned* transaction
//! a human signs — the same T1 ladder `onca-signer` already runs.
//!
//!   onca-market --event POLY-798942            # Sao Paulo daily temperature
//!               [--devices <pk,pk,...>] [--sensor dht11-a] [--tolerance 5] [--quorum 3]

use std::collections::HashMap;
use std::env;
use std::fs;

use onca_core::mesh::{aggregate_trusted, parse_attest, update_reputation, NodeReading, Reputation};
use serde_json::{json, Value};

const RPC: &str = "https://api.devnet.solana.com";
const JUP: &str = "https://api.jup.ag/prediction/v1";
// The 4-node devnet mesh (node4 = BtpD is the adversary that signs 999).
const DEFAULT_DEVICES: &str = "3xQ33DfPLL6py9zCZAm4CfowL6TPZbiMNJrBRPudhSNR,GhriBBob3iUczrGR81mXaKQ9LJBpF2STU8uEnVZLoX9a,AxRKqyyT9DXbKGMf47ULRCEqwRPQgCa1nX1UdVNTGQGh,BtpDcpYMfeZa6MtwFX6VeAnHjyq6qqh9V2X86oKQdUDy";

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

/// The latest `onca:attest` reading a device published for `sensor`, if any.
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

/// One outcome bucket of a temperature market, e.g. `23°C` or `31°C or higher`.
struct Bucket {
    label: String,
    /// The integer °C the bucket names (21 for "21°C or below", 31 for "31°C or higher").
    celsius: i64,
    market_id: String,
}

/// Pull an integer out of a bucket label like "23°C" / "21°C or below".
fn parse_celsius(label: &str) -> Option<i64> {
    let digits: String = label.chars().skip_while(|c| !c.is_ascii_digit()).take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().ok()
}

/// GET the Jupiter event and its outcome buckets (keyless read).
fn fetch_market(event_id: &str) -> Result<(String, String, Vec<Bucket>), String> {
    let url = format!("{JUP}/events/{event_id}?includeMarkets=true");
    let body: Value = match ureq::get(&url).call() {
        Ok(r) => r.into_json().map_err(|e| e.to_string())?,
        Err(ureq::Error::Status(code, _)) => return Err(format!("Jupiter HTTP {code}")),
        Err(e) => return Err(e.to_string()),
    };
    let ev = if body.get("data").is_some() { &body["data"] } else { &body };
    let title = ev["metadata"]["title"].as_str().unwrap_or("(untitled market)").to_string();
    let close = ev["metadata"]["closeTime"].as_str().unwrap_or("").to_string();
    let mut buckets = Vec::new();
    if let Some(markets) = ev["markets"].as_array() {
        for m in markets {
            let md = &m["metadata"];
            let label = md["groupItemTitle"].as_str().or_else(|| md["title"].as_str()).or_else(|| m["title"].as_str()).unwrap_or("").to_string();
            if let Some(celsius) = parse_celsius(&label) {
                buckets.push(Bucket { label, celsius, market_id: m["marketId"].as_str().unwrap_or("").to_string() });
            }
        }
    }
    if buckets.is_empty() {
        return Err("no temperature buckets in this market".into());
    }
    Ok((title, close, buckets))
}

fn short(device: &str) -> &str {
    &device[..device.len().min(4)]
}

fn main() {
    let event_id = arg("--event", "POLY-798942");
    let devices = arg("--devices", DEFAULT_DEVICES);
    let sensor = arg("--sensor", "dht11-a");
    let tolerance: f64 = arg("--tolerance", "5.0").parse().unwrap_or(5.0);
    let quorum: usize = arg("--quorum", "3").parse().unwrap_or(3);
    let min_score: f64 = arg("--min-score", "0.4").parse().unwrap_or(0.4);

    // 1) The market that exists today (settles on a single oracle).
    println!("MARKET (live on Solana via Jupiter, Polymarket liquidity)");
    let (title, close, buckets) = match fetch_market(&event_id) {
        Ok(v) => v,
        Err(e) => { eprintln!("  could not read market {event_id}: {e}"); std::process::exit(1); }
    };
    println!("  {event_id}  \"{title}\"");
    if !close.is_empty() {
        println!("  closes {close}  ·  {} outcome buckets  ·  settles on ONE oracle", buckets.len());
    }

    // 2) The Onca mesh (no single trusted source).
    println!("\nONCA MESH (independent on-chain attestations, sensor={sensor})");
    let rep_path = format!("{}/.onca/reputation.json", env::var("HOME").unwrap_or_default());
    let mut reps: HashMap<String, Reputation> = fs::read_to_string(&rep_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();

    let mut readings = Vec::new();
    for device in devices.split(',').filter(|s| !s.is_empty()) {
        if let Some(r) = latest_reading(device, &sensor) {
            let score = reps.get(device).map(Reputation::score).unwrap_or(0.5);
            let tag = if score < min_score { "  FROZEN OUT" } else { "" };
            println!("  node {}…  reading {}  trust {:.2}{tag}", short(device), r.value, score);
            readings.push(r);
        }
    }
    let agg = aggregate_trusted(&readings, &reps, tolerance, quorum, min_score);
    update_reputation(&mut reps, &agg);
    let _ = fs::write(&rep_path, serde_json::to_string_pretty(&reps).unwrap_or_default());

    if !agg.outliers.is_empty() {
        let liars: Vec<String> = agg.outliers.iter().map(|r| format!("{}…={}", short(&r.device), r.value)).collect();
        println!("  rejected: {}  (a lone source could not move this)", liars.join(", "));
    }
    if !agg.has_quorum {
        println!("\n  NO SETTLEMENT: only {} trusted node(s) agreed, quorum is {}", agg.inliers.len(), quorum);
        std::process::exit(2);
    }
    let value = agg.value;

    // 3) Map the mesh value to the market's winning bucket.
    let rounded = value.round() as i64;
    let winner = buckets.iter().min_by_key(|b| (b.celsius - rounded).abs()).unwrap();
    println!("\nSETTLEMENT");
    println!("  Onca mesh value: {value} C  ({} of {} nodes trusted & agree)", agg.inliers.len(), readings.len());
    println!("  winning outcome: \"{}\"  (market {})", winner.label, winner.market_id);
    println!("  no single source set this — corrupting the settlement needs a majority of independent nodes.");
    println!("\n  To trade it: Jupiter POST /orders returns an UNSIGNED tx a human signs (T1) — the agent holds no key.");
}
