//! The serial bridge: turn a line of ESP32 sensor output into a signed-ready
//! Solana attestation, running the real `depin-attest` core.
//!
//! An ESP32 running a DHT11 prints one line per reading over USB serial:
//!
//!     onca:reading s=dht11-a v=23.4 u=C seq=42 t=1753000000
//!
//! This bridge reads those lines, parses each into an `AttestArgs`, and runs the
//! same `build_attestation` the wasm plugin runs: reading bounds, the monotonic
//! replay guard, and the hand-rolled unsigned transaction. Accepted readings
//! print an unsigned tx ready for a human (or a Squads multisig) to sign;
//! refused readings print exactly why.
//!
//! Tonight, with no USB cable yet, it reads a mock feed so the whole pipeline is
//! provable with zero hardware:
//!
//!     cargo run --example bridge -- examples/mock-readings.txt
//!
//! When the cable arrives, the source is the only thing that changes — point it
//! at the live device (after `stty -f /dev/cu.usbserial-0001 115200`):
//!
//!     cargo run --example bridge -- /dev/cu.usbserial-0001
//!
//! or pipe anything into it:  `cat feed | cargo run --example bridge`.
//!
//! The RPC transport here is an offline stub that returns a fixed blockhash, so
//! the dry run is deterministic and needs no network. In production the plugin
//! supplies the `waki` transport and a real `getLatestBlockhash`; the core in
//! between is byte-for-byte the same.

use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::time::{SystemTime, UNIX_EPOCH};

use depin_attest::attest::{build_attestation, AttestArgs, AttestConfig};
use onca_core::pubkey::Pubkey;
use onca_core::rpc::RpcTransport;

/// The device that signs the attestation (fee payer). Operator-set, never model-set.
const DEVICE: &str = "7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU";
/// Any RPC URL — the offline stub ignores it. A real run uses the operator's.
const RPC_URL: &str = "https://api.devnet.solana.com";
/// A valid 32-byte base58 value standing in for a fetched blockhash.
const STUB_BLOCKHASH: &str = "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM";

/// Offline transport: answers `getLatestBlockhash` with a fixed value so the
/// dry run is deterministic and needs no network. Swapped for `waki` in the
/// real plugin; the core between it and the output is unchanged.
struct OfflineRpc;
impl RpcTransport for OfflineRpc {
    fn post_json(&self, _url: &str, _body: &str) -> onca_core::Result<String> {
        Ok(format!(
            r#"{{"jsonrpc":"2.0","id":1,"result":{{"value":{{"blockhash":"{STUB_BLOCKHASH}","lastValidBlockHeight":1}}}}}}"#
        ))
    }
}

/// One parsed sensor line.
struct Reading {
    sensor: String,
    value: f64,
    unit: String,
    seq: u64,
    timestamp: u64,
}

/// Parse `onca:reading s=.. v=.. u=.. seq=.. t=..`. Returns `None` for any line
/// that is not a reading (boot logs, wifi noise), so the bridge can be pointed
/// at a raw serial stream and simply ignore everything that is not ours.
fn parse_reading(line: &str) -> Option<Reading> {
    let line = line.trim();
    if !line.starts_with("onca:reading") {
        return None;
    }
    let mut kv: HashMap<&str, &str> = HashMap::new();
    for tok in line.split_whitespace().skip(1) {
        if let Some((k, v)) = tok.split_once('=') {
            kv.insert(k, v);
        }
    }
    Some(Reading {
        sensor: kv.get("s")?.to_string(),
        value: kv.get("v")?.parse().ok()?,
        unit: kv.get("u").copied().unwrap_or("").to_string(),
        seq: kv.get("seq")?.parse().ok()?,
        // The ESP32 has no wall clock; if it does not stamp `t`, the host does.
        timestamp: kv.get("t").and_then(|t| t.parse().ok()).unwrap_or_else(now_secs),
    })
}

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn main() -> io::Result<()> {
    // Operator config. Bounds and the device identity are set here, out of the
    // model's and the sensor's reach. `min_seq` is the last attested sequence;
    // it advances only when a reading is accepted.
    let base = AttestConfig {
        device: Some(Pubkey::from_base58(DEVICE).expect("valid device pubkey")),
        min_reading: Some(-40.0),
        max_reading: Some(85.0),
        min_seq: 0,
        ..Default::default()
    };

    // Read from a file/device argument if given, else stdin.
    let reader: Box<dyn BufRead> = match env::args().nth(1) {
        Some(path) => Box::new(BufReader::new(File::open(path)?)),
        None => Box::new(BufReader::new(io::stdin())),
    };

    println!("onca depin bridge  |  serial reading -> Solana attestation  (offline dry run)");
    println!("device {DEVICE}");
    println!("bounds -40..85  |  replay guard: sequence must increase\n");

    let mut last_seq = 0u64;
    let (mut accepted, mut refused) = (0u32, 0u32);

    for line in reader.lines() {
        let line = line?;
        let Some(r) = parse_reading(&line) else {
            continue; // not a reading — ignore serial noise
        };

        println!("[reading] s={} v={} u={} seq={}", r.sensor, r.value, r.unit, r.seq);

        let mut cfg = base.clone();
        cfg.min_seq = last_seq;
        let args = AttestArgs {
            sensor: r.sensor,
            reading: r.value,
            unit: r.unit,
            seq: r.seq,
            timestamp: r.timestamp,
        };

        match build_attestation(RPC_URL, &OfflineRpc, &cfg, &args) {
            Ok(att) => {
                last_seq = r.seq;
                accepted += 1;
                println!("  accepted  {}", att.summary);
                println!("  memo      {}", att.memo);
                println!("  tx        {}", att.base64);
            }
            Err(e) => {
                refused += 1;
                println!("  REFUSED   {e}");
            }
        }
        println!();
    }

    println!("done  |  {accepted} accepted, {refused} refused");
    Ok(())
}
