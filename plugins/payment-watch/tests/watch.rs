//! Host-run integration tests for `payment-watch`, driving the real `watch()`
//! orchestration through a mock `RpcTransport`. The mock answers
//! `getSignaturesForAddress` from a fixed list and `getTransaction` per
//! signature, so multi-transaction scenarios (dust attached to a real payment)
//! can be exercised. No wasm, no network.

use std::collections::HashMap;

use serde_json::{json, Value};
use onca_core::pubkey::{known, Pubkey};
use onca_core::rpc::RpcTransport;
use payment_watch::watch::{resolve_token, watch, PaymentStatus, WatchQuery};

const REFERENCE: &str = "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1";
const RECIPIENT: &str = "7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU";
const PAYER: &str = "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM";

/// Mock node: a signature list (newest-first) and a signature -> transaction map.
struct MockRpc {
    signatures: Value,
    transactions: HashMap<String, Value>,
}
impl RpcTransport for MockRpc {
    fn post_json(&self, _url: &str, body: &str) -> onca_core::Result<String> {
        let req: Value = serde_json::from_str(body).unwrap();
        let result = match req["method"].as_str().unwrap_or("") {
            "getSignaturesForAddress" => self.signatures.clone(),
            "getTransaction" => {
                let sig = req["params"][0].as_str().unwrap_or("");
                self.transactions.get(sig).cloned().unwrap_or(json!(null))
            }
            _ => json!(null),
        };
        Ok(json!({"jsonrpc": "2.0", "id": 1, "result": result}).to_string())
    }
}

fn sig_entry(sig: &str) -> Value {
    json!({"signature": sig, "err": null, "confirmationStatus": "finalized"})
}

/// USDC (6 decimals) transfer of `ui` tokens to the recipient.
fn usdc_tx(ui: f64) -> Value {
    let base = ((ui * 1_000_000.0).round() as u128).to_string();
    json!({
        "transaction": {"message": {"accountKeys": [{"pubkey": PAYER}]}},
        "meta": {
            "preTokenBalances": [],
            "postTokenBalances": [{"owner": RECIPIENT, "mint": known::USDC_MINT,
                "uiTokenAmount": {"amount": base, "decimals": 6, "uiAmount": ui, "uiAmountString": ui.to_string()}}]
        }
    })
}

fn query(expected: f64) -> WatchQuery {
    let (mint, symbol) = resolve_token(Some("USDC")).unwrap();
    WatchQuery {
        reference: Pubkey::from_base58(REFERENCE).unwrap(),
        recipient: Pubkey::from_base58(RECIPIENT).unwrap(),
        mint,
        expected,
        symbol,
    }
}

#[test]
fn no_signatures_is_pending() {
    let rpc = MockRpc { signatures: json!([]), transactions: HashMap::new() };
    assert_eq!(watch("https://rpc", &rpc, &query(25.0)).unwrap(), PaymentStatus::Pending);
}

#[test]
fn confirmed_full_payment_end_to_end() {
    let rpc = MockRpc {
        signatures: json!([sig_entry("goodSig")]),
        transactions: HashMap::from([("goodSig".to_string(), usdc_tx(25.0))]),
    };
    let status = watch("https://rpc", &rpc, &query(25.0)).unwrap();
    assert!(status.is_paid());
    assert!(status.render(REFERENCE).contains("Paid"));
}

/// REGRESSION (audit bug #1). A real 25 USDC payment lands, then the attacker
/// attaches a 0.01 USDC dust transfer to the public reference. `getSignatures`
/// returns the dust newest-first. The watcher must still resolve to PAID by
/// scanning every confirmed signature, not just the latest.
#[test]
fn dust_after_real_payment_still_resolves_paid() {
    let rpc = MockRpc {
        // newest-first: dust is on top, the real payment is older
        signatures: json!([sig_entry("dustSig"), sig_entry("realSig")]),
        transactions: HashMap::from([
            ("dustSig".to_string(), usdc_tx(0.01)),
            ("realSig".to_string(), usdc_tx(25.0)),
        ]),
    };
    let status = watch("https://rpc", &rpc, &query(25.0)).unwrap();
    assert!(status.is_paid(), "real payment must win over later dust");
    match status {
        PaymentStatus::Paid { signature, .. } => assert_eq!(signature, "realSig"),
        other => panic!("expected Paid, got {other:?}"),
    }
}

/// FAIL-CLOSED. Only a dust transfer exists against the reference — never enough
/// to clear the invoice. Reported as underpaid, never paid.
#[test]
fn dust_only_is_underpaid_not_paid() {
    let rpc = MockRpc {
        signatures: json!([sig_entry("dustSig")]),
        transactions: HashMap::from([("dustSig".to_string(), usdc_tx(0.01))]),
    };
    let status = watch("https://rpc", &rpc, &query(25.0)).unwrap();
    assert!(!status.is_paid());
    assert!(matches!(status, PaymentStatus::Underpaid { .. }));
}

#[test]
fn failed_transaction_on_reference_is_pending() {
    let rpc = MockRpc {
        signatures: json!([{"signature": "revertedSig", "err": {"InstructionError": [0, "Custom"]}, "confirmationStatus": "finalized"}]),
        transactions: HashMap::new(),
    };
    assert_eq!(watch("https://rpc", &rpc, &query(25.0)).unwrap(), PaymentStatus::Pending);
}
