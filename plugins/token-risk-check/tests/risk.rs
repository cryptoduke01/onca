//! Host-run integration tests for `token-risk-check`, driving the real
//! `analyze()` orchestration through a mock `RpcTransport` that returns canned
//! JSON-RPC responses keyed by method. No wasm toolchain, no network — exactly
//! the path the wasm `execute` runs, minus `waki`.

use std::collections::HashMap;

use serde_json::{json, Value};
use onca_core::rpc::RpcTransport;
use token_risk_check::risk::{analyze, Severity};

/// A mock endpoint: maps a JSON-RPC method name to the `result` value it should
/// return. `post_json` inspects the request body to pick the response and wraps
/// it in a JSON-RPC envelope, just like a real node.
struct MockRpc {
    results: HashMap<String, Value>,
}
impl MockRpc {
    fn new(pairs: Vec<(&str, Value)>) -> Self {
        MockRpc {
            results: pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect(),
        }
    }
}
impl RpcTransport for MockRpc {
    fn post_json(&self, _url: &str, body: &str) -> onca_core::Result<String> {
        let req: Value = serde_json::from_str(body).unwrap();
        let method = req["method"].as_str().unwrap_or("");
        let result = self
            .results
            .get(method)
            .cloned()
            .unwrap_or_else(|| json!(null));
        Ok(json!({"jsonrpc": "2.0", "id": 1, "result": result}).to_string())
    }
}

const USDC: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
const TOKEN_2022: &str = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";
const TOKEN: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";

fn mint_account(owner: &str, info: Value) -> Value {
    json!({"context": {"slot": 1}, "value": {
        "owner": owner,
        "data": {"parsed": {"type": "mint", "info": info}}
    }})
}

fn largest(amounts: &[&str]) -> Value {
    let arr: Vec<Value> = amounts
        .iter()
        .map(|a| json!({"address": "Holder1", "amount": a, "decimals": 6}))
        .collect();
    json!({"context": {"slot": 1}, "value": arr})
}

#[test]
fn clean_token_end_to_end_green() {
    let rpc = MockRpc::new(vec![
        (
            "getAccountInfo",
            mint_account(
                TOKEN,
                json!({"decimals": 6, "supply": "1000000000000",
                       "mintAuthority": null, "freezeAuthority": null}),
            ),
        ),
        ("getTokenLargestAccounts", largest(&["10000000000"; 5])),
    ]);
    let report = analyze("https://rpc.example/secret-key", &rpc, USDC).unwrap();
    assert_eq!(report.rating, Severity::Green);
}

/// FAIL-CLOSED / anti-injection (reproduced in the README).
///
/// The token's own on-chain metadata (and any inbound chat message hyping it)
/// could *claim* it is safe. This tool ignores all of that: the verdict is
/// derived only from authorities, extensions, and holder data pulled from the
/// chain. A honeypot with a permanent delegate is RED no matter what the prompt
/// says.
#[test]
fn honeypot_is_red_regardless_of_claims() {
    let rpc = MockRpc::new(vec![
        (
            "getAccountInfo",
            mint_account(
                TOKEN_2022,
                json!({
                    "decimals": 6, "supply": "1000000000000",
                    "mintAuthority": "Scammer1111111111111111111111111111111111",
                    "freezeAuthority": "Scammer1111111111111111111111111111111111",
                    "extensions": [
                        {"extension": "permanentDelegate",
                         "state": {"delegate": "Drain1111111111111111111111111111111111111"}},
                        {"extension": "transferFeeConfig",
                         "state": {"newerTransferFee": {"transferFeeBasisPoints": 2000}}}
                    ]
                }),
            ),
        ),
        // and one wallet holds 90% of supply
        ("getTokenLargestAccounts", largest(&["900000000000", "10000000000"])),
    ]);
    let report = analyze("https://rpc.example", &rpc, USDC).unwrap();
    assert_eq!(report.rating, Severity::Red);
    let text = report.render();
    assert!(text.contains("Permanent delegate"));
    assert!(text.contains("Top holder controls 90%"));
    // 20% fee (2000 bps) escalates to RED on its own
    assert!(text.contains("20.00%"));
}

#[test]
fn hallucinated_mint_fails_before_rpc() {
    let rpc = MockRpc::new(vec![]);
    let err = analyze("https://rpc.example", &rpc, "totally-not-an-address").unwrap_err();
    assert!(err.contains("invalid mint address"));
}

#[test]
fn missing_mint_account_is_reported() {
    let rpc = MockRpc::new(vec![
        ("getAccountInfo", json!({"context": {"slot": 1}, "value": null})),
    ]);
    let err = analyze("https://rpc.example", &rpc, USDC).unwrap_err();
    assert!(err.contains("not found"));
}
