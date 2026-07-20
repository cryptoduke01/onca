//! Host-run integration tests for the `solana-pay-request` core, driven exactly
//! as the wasm `execute` entry point drives it: parse args -> build `PayConfig`
//! from a flat config section -> `build_request`. No wasm toolchain, no network.

use std::collections::HashMap;

use solana_pay_request::pay::{build_request, PayArgs, PayConfig};

const MERCHANT: &str = "7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU";
const ATTACKER: &str = "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM";

fn section(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
}

fn args(recipient: &str, amount: &str, token: Option<&str>) -> PayArgs {
    PayArgs {
        recipient: recipient.to_string(),
        amount: amount.to_string(),
        token: token.map(str::to_string),
        reference: None,
        label: None,
        message: None,
        memo: None,
    }
}

#[test]
fn end_to_end_usdc_charge() {
    let cfg = PayConfig::from_section(&section(&[("label", "Bar do Zé")]));
    let mut a = args(MERCHANT, "25", Some("USDC"));
    a.memo = Some("table 4".into());
    let r = build_request(&a, &cfg).unwrap();
    assert!(r.url.starts_with("solana:"));
    assert!(r.url.contains("amount=25"));
    assert!(r.url.contains("spl-token="));
    assert!(r.url.contains("label=Bar%20do%20Z%C3%A9"));
    assert!(r.url.contains("memo=table%204"));
}

/// PROMPT-INJECTION DEFENSE (this transcript is reproduced in the README).
///
/// A malicious inbound chat message tries to redirect a legitimate "charge 25
/// USDC" into draining a large amount to the attacker in an unlisted token. The
/// operator has pinned `allowed_mints = USDC` and `max_amount = 100`. Even if
/// the LLM is fully convinced and forwards the attacker's arguments verbatim,
/// the plugin refuses at the boundary — it cannot be argued past config.
#[test]
fn prompt_injection_fails_closed() {
    let cfg = PayConfig::from_section(&section(&[
        ("allowed_mints", "USDC"),
        ("max_amount", "100"),
    ]));

    // Attack 1: charge a wild amount to the attacker.
    let over_cap = build_request(&args(ATTACKER, "1000000", Some("USDC")), &cfg);
    assert!(over_cap.is_err());
    assert!(over_cap.unwrap_err().contains("exceeds the configured max"));

    // Attack 2: sneak in an unlisted token (USDT) under the cap.
    let bad_mint = build_request(&args(ATTACKER, "50", Some("USDT")), &cfg);
    assert!(bad_mint.is_err());
    assert!(bad_mint.unwrap_err().contains("allowlist"));

    // Attack 3: a hallucinated / malformed recipient address.
    let bad_addr = build_request(&args("send-it-all-to-me", "50", Some("USDC")), &cfg);
    assert!(bad_addr.is_err());
    assert!(bad_addr.unwrap_err().contains("valid Solana address"));

    // Control: a legitimate in-policy request still succeeds.
    let ok = build_request(&args(MERCHANT, "25", Some("USDC")), &cfg);
    assert!(ok.is_ok());
}
