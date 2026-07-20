//! Pure core for the `depin-attest` tool. No wasm, no I/O: it turns a sensor
//! reading into an unsigned Solana transaction that records a signed attestation
//! on-chain, and it enforces the guardrails a human cannot see or argue past.
//! Host-tested with a mock RPC.
//!
//! Custody tier **T1 (Build)**. The tool builds an unsigned transaction; the
//! host or a person signs it. It holds no key and moves no funds. Its power is
//! bounded to producing a Memo that says "sensor X read Y at time T, sequence N".
//!
//! The differentiator is the **replay guard**. Two layers:
//!   1. A monotonic sequence nonce in every attestation. The core refuses a
//!      sequence at or below the last one, so a captured reading cannot be
//!      replayed as fresh.
//!   2. An optional durable nonce account, so the *signed* transaction itself can
//!      only ever land once (the on-chain nonce advances when it does).

use std::collections::HashMap;

use onca_core::pubkey::Pubkey;
use onca_core::rpc::{commitment, RpcClient, RpcTransport};
use onca_core::tx::{
    advance_nonce_instruction, compile_message, memo_instruction, unsigned_transaction_base64,
};

/// Operator config, resolved from the plugin's jailed config section. Every
/// field is set by the operator, never by the model.
#[derive(Debug, Clone, Default)]
pub struct AttestConfig {
    /// The device identity (fee payer and signer of the attestation).
    pub device: Option<Pubkey>,
    /// Reject a reading below this, if set (a spoofed or broken sensor).
    pub min_reading: Option<f64>,
    /// Reject a reading above this, if set.
    pub max_reading: Option<f64>,
    /// The last attested sequence. A new attestation must exceed it.
    pub min_seq: u64,
    /// Durable nonce account, if the operator wants approval-gated, one-time txs.
    pub nonce_account: Option<Pubkey>,
    /// Authority that signs the nonce advance (usually the device).
    pub nonce_authority: Option<Pubkey>,
}

impl AttestConfig {
    pub fn from_section(s: &HashMap<String, String>) -> Self {
        let pk = |k: &str| s.get(k).and_then(|v| Pubkey::from_base58(v).ok());
        let f = |k: &str| s.get(k).and_then(|v| v.trim().parse::<f64>().ok());
        AttestConfig {
            device: pk("device"),
            min_reading: f("min_reading"),
            max_reading: f("max_reading"),
            min_seq: s.get("min_seq").and_then(|v| v.trim().parse().ok()).unwrap_or(0),
            nonce_account: pk("nonce_account"),
            nonce_authority: pk("nonce_authority"),
        }
    }
}

/// One reading to attest, passed in by the host's hardware tool or SOP.
#[derive(Debug, Clone)]
pub struct AttestArgs {
    pub sensor: String,
    pub reading: f64,
    pub unit: String,
    pub seq: u64,
    pub timestamp: u64,
}

/// The finished attestation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attestation {
    /// The unsigned transaction, base64, ready for the host or a human to sign.
    pub base64: String,
    /// The exact memo string committed on-chain.
    pub memo: String,
    /// A short human summary for the approval gate.
    pub summary: String,
}

/// Decode a 32-byte base58 value (a blockhash) into raw bytes.
fn decode_hash(s: &str) -> Result<[u8; 32], String> {
    Pubkey::from_base58(s)
        .map(|p| p.to_bytes())
        .map_err(|e| format!("invalid blockhash: {e}"))
}

/// Fetch the blockhash to bind the transaction to: the durable nonce value when
/// a nonce account is configured, otherwise the latest network blockhash.
fn fetch_blockhash<T: RpcTransport>(
    client: &RpcClient<T>,
    cfg: &AttestConfig,
) -> Result<[u8; 32], String> {
    if let Some(nonce) = &cfg.nonce_account {
        let acc = client
            .call(
                "getAccountInfo",
                serde_json::json!([nonce.to_base58(), {"encoding": "jsonParsed", "commitment": "confirmed"}]),
            )
            .map_err(|e| e.to_string())?;
        let bh = acc
            .pointer("/value/data/parsed/info/blockhash")
            .and_then(|v| v.as_str())
            .ok_or("nonce account has no stored blockhash (is it a real nonce account?)")?;
        decode_hash(bh)
    } else {
        let res = client
            .call("getLatestBlockhash", serde_json::json!([commitment("confirmed")]))
            .map_err(|e| e.to_string())?;
        let bh = res
            .pointer("/value/blockhash")
            .and_then(|v| v.as_str())
            .ok_or("getLatestBlockhash returned no blockhash")?;
        decode_hash(bh)
    }
}

/// Format the on-chain memo. Kept compact on purpose: a small, greppable line,
/// never a JSON blob that bloats the reader's context.
fn format_memo(a: &AttestArgs) -> String {
    format!(
        "onca:attest s={} v={} u={} seq={} t={}",
        a.sensor, trim(a.reading), a.unit, a.seq, a.timestamp
    )
}

fn trim(f: f64) -> String {
    let s = format!("{f:.4}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// Build an unsigned attestation transaction, or refuse. All policy is here.
pub fn build_attestation<T: RpcTransport>(
    rpc_url: &str,
    transport: &T,
    cfg: &AttestConfig,
    args: &AttestArgs,
) -> Result<Attestation, String> {
    // 1. Device identity must be configured by the operator.
    let device = cfg.device.ok_or("no device configured — set `device` in this plugin's config")?;

    // 2. Reading must be a real number, within the operator's sane bounds.
    if !args.reading.is_finite() {
        return Err("reading is not a finite number".into());
    }
    if let Some(min) = cfg.min_reading {
        if args.reading < min {
            return Err(format!("reading {} is below the configured min of {min} — refused", trim(args.reading)));
        }
    }
    if let Some(max) = cfg.max_reading {
        if args.reading > max {
            return Err(format!("reading {} is above the configured max of {max} — refused", trim(args.reading)));
        }
    }

    // 3. Replay guard: the sequence must move forward.
    if args.seq <= cfg.min_seq {
        return Err(format!(
            "sequence {} is not greater than the last attested {} — replay refused",
            args.seq, cfg.min_seq
        ));
    }

    // 4. Bind to a blockhash (durable nonce if configured).
    let client = RpcClient::new(rpc_url, transport);
    let blockhash = fetch_blockhash(&client, cfg)?;

    // 5. Assemble instructions. A durable-nonce advance must come first.
    let memo = format_memo(args);
    let mut instructions = Vec::new();
    if let (Some(nonce), Some(auth)) = (&cfg.nonce_account, &cfg.nonce_authority) {
        instructions.push(advance_nonce_instruction(nonce, auth));
    }
    instructions.push(memo_instruction(&memo, &[device]));

    let msg = compile_message(&device, blockhash, &instructions);
    let base64 = unsigned_transaction_base64(&msg);

    let summary = format!(
        "Attestation #{}: {}{} from {} — unsigned tx ready to sign",
        args.seq,
        trim(args.reading),
        args.unit,
        args.sensor,
    );

    Ok(Attestation { base64, memo, summary })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const DEVICE: &str = "7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU";
    // A valid 32-byte base58 value used as a fake blockhash.
    const BLOCKHASH: &str = "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM";

    struct MockRpc {
        blockhash: String,
    }
    impl RpcTransport for MockRpc {
        fn post_json(&self, _url: &str, _body: &str) -> onca_core::Result<String> {
            Ok(json!({"jsonrpc": "2.0", "id": 1, "result": {"value": {"blockhash": self.blockhash, "lastValidBlockHeight": 1}}}).to_string())
        }
    }

    fn cfg() -> AttestConfig {
        AttestConfig {
            device: Some(Pubkey::from_base58(DEVICE).unwrap()),
            min_reading: Some(-40.0),
            max_reading: Some(85.0),
            min_seq: 41,
            ..Default::default()
        }
    }

    fn args(reading: f64, seq: u64) -> AttestArgs {
        AttestArgs { sensor: "bme280-a".into(), reading, unit: "C".into(), seq, timestamp: 1_753_000_000 }
    }

    #[test]
    fn builds_a_valid_attestation() {
        let rpc = MockRpc { blockhash: BLOCKHASH.into() };
        let a = build_attestation("https://rpc", &rpc, &cfg(), &args(23.4, 42)).unwrap();
        assert!(a.memo.contains("s=bme280-a"));
        assert!(a.memo.contains("v=23.4"));
        assert!(a.memo.contains("seq=42"));
        assert!(!a.base64.is_empty());
        assert!(a.summary.contains("#42"));
    }

    #[test]
    fn replay_is_refused() {
        let rpc = MockRpc { blockhash: BLOCKHASH.into() };
        // seq 41 == min_seq (last attested) — a replay.
        let err = build_attestation("https://rpc", &rpc, &cfg(), &args(23.4, 41)).unwrap_err();
        assert!(err.contains("replay refused"));
    }

    #[test]
    fn out_of_range_reading_refused() {
        let rpc = MockRpc { blockhash: BLOCKHASH.into() };
        let hot = build_attestation("https://rpc", &rpc, &cfg(), &args(999.0, 42)).unwrap_err();
        assert!(hot.contains("above the configured max"));
        let cold = build_attestation("https://rpc", &rpc, &cfg(), &args(-100.0, 42)).unwrap_err();
        assert!(cold.contains("below the configured min"));
    }

    #[test]
    fn no_device_refused() {
        let rpc = MockRpc { blockhash: BLOCKHASH.into() };
        let c = AttestConfig { device: None, ..cfg() };
        assert!(build_attestation("https://rpc", &rpc, &c, &args(23.4, 42)).unwrap_err().contains("no device"));
    }
}
