//! Host-run integration tests for `depin-attest`, driving `build_attestation`
//! through a mock `RpcTransport` keyed by method (so the durable-nonce path is
//! exercised too). No wasm toolchain, no network.

use std::collections::HashMap;

use serde_json::{json, Value};
use onca_core::pubkey::Pubkey;
use onca_core::rpc::RpcTransport;
use depin_attest::attest::{build_attestation, AttestArgs, AttestConfig};

const DEVICE: &str = "7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU";
const NONCE: &str = "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1";
const AUTH: &str = "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM";
const HASH: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"; // valid 32-byte base58

struct MockRpc {
    results: HashMap<String, Value>,
}
impl RpcTransport for MockRpc {
    fn post_json(&self, _url: &str, body: &str) -> onca_core::Result<String> {
        let req: Value = serde_json::from_str(body).unwrap();
        let method = req["method"].as_str().unwrap_or("");
        let result = self.results.get(method).cloned().unwrap_or(json!(null));
        Ok(json!({"jsonrpc": "2.0", "id": 1, "result": result}).to_string())
    }
}

fn cfg() -> AttestConfig {
    AttestConfig {
        device: Some(Pubkey::from_base58(DEVICE).unwrap()),
        min_reading: Some(-40.0),
        max_reading: Some(85.0),
        min_seq: 100,
        ..Default::default()
    }
}

fn args(reading: f64, seq: u64) -> AttestArgs {
    AttestArgs { sensor: "bme280-a".into(), reading, unit: "C".into(), seq, timestamp: 1_753_000_000 }
}

fn latest_blockhash_rpc() -> MockRpc {
    MockRpc {
        results: HashMap::from([(
            "getLatestBlockhash".to_string(),
            json!({"context": {"slot": 1}, "value": {"blockhash": HASH, "lastValidBlockHeight": 200}}),
        )]),
    }
}

#[test]
fn end_to_end_latest_blockhash() {
    let a = build_attestation("https://rpc/secret", &latest_blockhash_rpc(), &cfg(), &args(22.7, 101)).unwrap();
    assert!(a.memo.starts_with("onca:attest"));
    assert!(a.memo.contains("seq=101"));
    assert!(!a.base64.is_empty());
}

#[test]
fn durable_nonce_path_uses_stored_blockhash() {
    let rpc = MockRpc {
        results: HashMap::from([(
            "getAccountInfo".to_string(),
            json!({"context": {"slot": 1}, "value": {"data": {"parsed": {"info": {"blockhash": HASH, "authority": AUTH}}}}}),
        )]),
    };
    let c = AttestConfig {
        nonce_account: Some(Pubkey::from_base58(NONCE).unwrap()),
        nonce_authority: Some(Pubkey::from_base58(AUTH).unwrap()),
        ..cfg()
    };
    // Durable-nonced tx is larger (extra advance-nonce instruction) but still builds.
    let a = build_attestation("https://rpc", &rpc, &c, &args(22.7, 101)).unwrap();
    assert!(!a.base64.is_empty());
    assert!(a.memo.contains("seq=101"));
}

/// FAIL-CLOSED / prompt-injection (this transcript is in the README).
///
/// A hostile inbound message tries to make the device attest a fabricated,
/// out-of-range reading and to replay an old sequence to overwrite history. The
/// bounds and the monotonic replay guard live in the pure core, so the tool
/// refuses at the boundary. And because it only *builds* an unsigned tx and
/// holds no key, even a passed attestation moves nothing until a human signs.
#[test]
fn prompt_injection_fails_closed() {
    let rpc = latest_blockhash_rpc();
    let cfg = cfg(); // bounds -40..85 C, last seq 100

    // Attack 1: attest a fabricated 999°C reading.
    let hot = build_attestation("https://rpc", &rpc, &cfg, &args(999.0, 101));
    assert!(hot.unwrap_err().contains("above the configured max"));

    // Attack 2: replay sequence 100 to overwrite the last real attestation.
    let replay = build_attestation("https://rpc", &rpc, &cfg, &args(22.0, 100));
    assert!(replay.unwrap_err().contains("replay refused"));

    // Attack 3: a plausible reading but a stale sequence.
    let stale = build_attestation("https://rpc", &rpc, &cfg, &args(22.0, 50));
    assert!(stale.unwrap_err().contains("replay refused"));

    // Control: an in-bounds reading with a fresh sequence is built normally.
    assert!(build_attestation("https://rpc", &rpc, &cfg, &args(22.0, 101)).is_ok());
}
