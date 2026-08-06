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
    // Point at a pro endpoint (Helius/Solami/etc.) with ONCA_RPC; defaults to public devnet.
    let url = std::env::var("ONCA_RPC").unwrap_or_else(|_| RPC.to_string());
    match ureq::post(&url).send_json(body) {
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

/// Now as an ISO-8601 UTC string (`YYYY-MM-DDTHH:MM:SSZ`), so we can compare it
/// lexically against a market's `closeTime` and tell open from closed without a
/// date crate (Howard Hinnant's civil-from-days for the date part).
fn now_iso() -> String {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) as i64;
    let (days, tod) = (secs / 86400, secs % 86400);
    let (h, mi, s) = (tod / 3600, (tod % 3600) / 60, tod % 60);
    let z = days + 719468;
    let era = (if z >= 0 { z } else { z - 146096 }) / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    format!("{y:04}-{m:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
}

/// Find the current open "Highest temperature in São Paulo" market so the demo
/// never goes stale — the daily market rolls over at 12:00 UTC. Picks the
/// soonest-closing market whose close date is today or later.
fn find_open_market() -> Result<String, String> {
    let url = format!("{JUP}/events/search?query=highest%20temperature%20Sao%20Paulo&limit=40");
    let body: Value = match ureq::get(&url).call() {
        Ok(r) => r.into_json().map_err(|e| e.to_string())?,
        Err(e) => return Err(e.to_string()),
    };
    let now = now_iso();
    let mut best: Option<(String, String)> = None; // (closeTime, eventId)
    for e in body["data"].as_array().ok_or("no data")? {
        let md = &e["metadata"];
        let title = md["title"].as_str().unwrap_or("");
        if !title.contains("Sao Paulo") || !title.to_lowercase().contains("temperature") {
            continue;
        }
        let close = md["closeTime"].as_str().unwrap_or("");
        if close < now.as_str() {
            continue; // already closed
        }
        let id = e["eventId"].as_str().unwrap_or("").to_string();
        match &best {
            Some((bc, _)) if close >= bc.as_str() => {} // keep the soonest-closing open market
            _ => best = Some((close.to_string(), id)),
        }
    }
    best.map(|(_, id)| id).ok_or_else(|| "no open São Paulo temperature market found".into())
}

fn main() {
    // No --event: auto-find today's open São Paulo market so the demo never
    // points at a market that already closed.
    let event_id = {
        let e = arg("--event", "");
        if e.is_empty() {
            match find_open_market() {
                Ok(id) => { println!("auto-selected open market: {id}"); id }
                Err(err) => { eprintln!("could not auto-find an open São Paulo market: {err}\n  pass one with --event <POLY-…>"); std::process::exit(1); }
            }
        } else {
            e
        }
    };
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

    // A market that hasn't closed yet cannot be *settled* — the day's high isn't
    // final until it closes. Show the live mesh value as a PREVIEW (what the
    // resolver would commit at close), and only call it a SETTLEMENT once closed.
    let is_open = !close.is_empty() && close.as_str() > now_iso().as_str();
    if is_open {
        println!("\nPREVIEW  (market still open until {close} — provisional, not final)");
        println!("  Onca mesh value now: {value} C  ({} of {} nodes trusted & agree)", agg.inliers.len(), readings.len());
        println!("  leading outcome: \"{}\"  (market {})", winner.label, winner.market_id);
        println!("  final settlement fires after close, when the day's high is known.");
    } else {
        println!("\nSETTLEMENT");
        println!("  Onca mesh value: {value} C  ({} of {} nodes trusted & agree)", agg.inliers.len(), readings.len());
        println!("  winning outcome: \"{}\"  (market {})", winner.label, winner.market_id);
        println!("  no single source set this — corrupting the settlement needs a majority of independent nodes.");
    }
    println!("\n  To trade it: Jupiter POST /orders returns an UNSIGNED tx a human signs (T1) — the agent holds no key.");
    println!("  Not financial advice — the mesh reports a fact, not a bet. Trading on it is a human's call and risk.");
}
