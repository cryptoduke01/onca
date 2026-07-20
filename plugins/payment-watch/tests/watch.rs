//! Host-run integration tests for `payment-watch`, driving the real `watch()`
//! orchestration through a mock `RpcTransport` keyed by method. No wasm, no net.

use std::collections::HashMap;

use serde_json::{json, Value};
use solana_core::pubkey::{known, Pubkey};
use solana_core::rpc::RpcTransport;
use payment_watch::watch::{resolve_token, watch, PaymentStatus, WatchQuery};

struct MockRpc {
    results: HashMap<String, Value>,
}
impl MockRpc {
    fn new(pairs: Vec<(&str, Value)>) -> Self {
        MockRpc { results: pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect() }
    }
}
impl RpcTransport for MockRpc {
    fn post_json(&self, _url: &str, body: &str) -> solana_core::Result<String> {
        let req: Value = serde_json::from_str(body).unwrap();
        let method = req["method"].as_str().unwrap_or("");
        let result = self.results.get(method).cloned().unwrap_or(json!(null));
        Ok(json!({"jsonrpc": "2.0", "id": 1, "result": result}).to_string())
    }
}

const REFERENCE: &str = "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1";
const RECIPIENT: &str = "7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU";
const PAYER: &str = "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM";

fn query(expected: f64, token: Option<&str>) -> WatchQuery {
    let (mint, symbol) = resolve_token(token).unwrap();
    WatchQuery {
        reference: Pubkey::from_base58(REFERENCE).unwrap(),
        recipient: Pubkey::from_base58(RECIPIENT).unwrap(),
        mint,
        expected,
        symbol,
    }
}

fn sigs(entries: Value) -> Value {
    entries
}

fn spl_tx(post: f64) -> Value {
    json!({
        "transaction": {"message": {"accountKeys": [{"pubkey": PAYER}]}},
        "meta": {
            "preTokenBalances": [],
            "postTokenBalances": [{"owner": RECIPIENT, "mint": known::USDC_MINT,
                "uiTokenAmount": {"uiAmount": post, "uiAmountString": post.to_string()}}]
        }
    })
}

#[test]
fn no_signatures_is_pending() {
    let rpc = MockRpc::new(vec![("getSignaturesForAddress", json!([]))]);
    let status = watch("https://rpc", &rpc, &query(25.0, Some("USDC"))).unwrap();
    assert_eq!(status, PaymentStatus::Pending);
}

#[test]
fn confirmed_full_payment_end_to_end() {
    let rpc = MockRpc::new(vec![
        ("getSignaturesForAddress", sigs(json!([
            {"signature": "goodSig", "err": null, "confirmationStatus": "finalized"}
        ]))),
        ("getTransaction", spl_tx(25.0)),
    ]);
    let status = watch("https://rpc", &rpc, &query(25.0, Some("USDC"))).unwrap();
    assert!(status.is_paid());
    assert!(status.render(REFERENCE).contains("Paid"));
}

/// FAIL-CLOSED (reproduced in the README). An attacker who learns the invoice's
/// public reference can attach a dust payment to it, hoping the watcher marks
/// the whole invoice paid. The amount is verified on-chain: a 0.01 USDC transfer
/// against a 25 USDC invoice is UNDERPAID, never Paid.
#[test]
fn dust_spoof_against_reference_is_not_paid() {
    let rpc = MockRpc::new(vec![
        ("getSignaturesForAddress", json!([
            {"signature": "dustSig", "err": null, "confirmationStatus": "finalized"}
        ])),
        ("getTransaction", spl_tx(0.01)),
    ]);
    let status = watch("https://rpc", &rpc, &query(25.0, Some("USDC"))).unwrap();
    assert!(!status.is_paid());
    assert!(matches!(status, PaymentStatus::Underpaid { .. }));
}

#[test]
fn failed_transaction_on_reference_is_pending() {
    let rpc = MockRpc::new(vec![
        ("getSignaturesForAddress", json!([
            {"signature": "revertedSig", "err": {"InstructionError": [0, "Custom"]}, "confirmationStatus": "finalized"}
        ])),
    ]);
    let status = watch("https://rpc", &rpc, &query(25.0, Some("USDC"))).unwrap();
    assert_eq!(status, PaymentStatus::Pending);
}
