//! Pure core for the `payment-watch` tool. No wasm, no I/O: it interprets the
//! JSON that `getSignaturesForAddress` and `getTransaction` return and decides
//! whether an expected payment has landed. Host-tested with canned fixtures.
//!
//! Custody tier **T0 (Read)**. It signs nothing and moves nothing. It closes the
//! loop opened by `solana-pay-request`: that tool embeds a unique `reference`
//! pubkey in the Solana Pay URL; this tool watches that reference and confirms
//! the money actually arrived — the right amount, to the right recipient.
//!
//! The Solana Pay `reference` is a read-only account attached to the transfer
//! *specifically* so a watcher can locate the transaction with
//! `getSignaturesForAddress(reference)`. Because that reference is public,
//! anyone can attach a second transaction to it — so we inspect every confirmed
//! signature, not just the newest, and compare the credited amount in exact
//! integer base units.

use serde_json::Value;

use onca_core::pubkey::{known, Pubkey};
use onca_core::rpc::{RpcClient, RpcTransport};
use onca_core::shape::{abbrev, render_amount};

/// How many recent signatures on the reference we are willing to inspect.
const MAX_SIGNATURES: usize = 10;

/// What the watcher concluded.
#[derive(Debug, Clone, PartialEq)]
pub enum PaymentStatus {
    /// No confirmed transaction has credited the recipient yet.
    Pending,
    /// A confirmed transfer landed but for less than expected.
    Underpaid {
        signature: String,
        got: String,
        want: String,
        symbol: String,
    },
    /// A confirmed transfer of at least the expected amount landed.
    Paid {
        signature: String,
        amount: String,
        symbol: String,
        from: Option<String>,
    },
}

impl PaymentStatus {
    pub fn is_paid(&self) -> bool {
        matches!(self, PaymentStatus::Paid { .. })
    }

    /// Human line the SOP turns into a chat notification.
    pub fn render(&self, reference: &str) -> String {
        match self {
            PaymentStatus::Pending => {
                format!("Payment pending — no confirmed transfer for reference {} yet.", abbrev(reference))
            }
            PaymentStatus::Underpaid { signature, got, want, symbol } => format!(
                "Underpaid — received {got} {symbol} but expected {want} {symbol}. Tx {}.",
                abbrev(signature)
            ),
            PaymentStatus::Paid { signature, amount, symbol, from } => {
                let who = from
                    .as_ref()
                    .map(|f| format!(" from {}", abbrev(f)))
                    .unwrap_or_default();
                format!("Paid — {amount} {symbol}{who}. Tx {}.", abbrev(signature))
            }
        }
    }
}

/// The parsed request: what we are watching for.
#[derive(Debug, Clone)]
pub struct WatchQuery {
    pub reference: Pubkey,
    pub recipient: Pubkey,
    /// `None` means native SOL.
    pub mint: Option<Pubkey>,
    pub expected: f64,
    pub symbol: String,
}

/// Resolve a token symbol / mint into `(Option<mint>, display symbol)`.
pub fn resolve_token(token: Option<&str>) -> Result<(Option<Pubkey>, String), String> {
    match token.map(|t| t.trim().to_ascii_uppercase()).as_deref() {
        None | Some("") | Some("SOL") => Ok((None, "SOL".into())),
        Some("USDC") => Ok((Some(Pubkey::from_base58(known::USDC_MINT).unwrap()), "USDC".into())),
        Some("USDT") => Ok((Some(Pubkey::from_base58(known::USDT_MINT).unwrap()), "USDT".into())),
        Some(_) => {
            let raw = token.unwrap();
            let mint = Pubkey::from_base58(raw)
                .map_err(|e| format!("token '{raw}' is not a known symbol or valid mint: {e}"))?;
            Ok((Some(mint), abbrev(&mint.to_base58())))
        }
    }
}

/// A confirmed signature from `getSignaturesForAddress`, or `None` if the entry
/// errored or is not yet at least `confirmed`.
fn confirmed_signature(entry: &Value) -> Option<String> {
    if !entry.get("err").map(Value::is_null).unwrap_or(true) {
        return None; // transaction failed
    }
    let status = entry
        .get("confirmationStatus")
        .and_then(Value::as_str)
        .unwrap_or("");
    // Missing status: older nodes omit it on finalized txs — treat as confirmed.
    if status.is_empty() || status == "confirmed" || status == "finalized" {
        entry.get("signature").and_then(Value::as_str).map(str::to_string)
    } else {
        None
    }
}

/// Pull the fee payer (first account key) out of a (jsonParsed) transaction.
fn fee_payer(tx: &Value) -> Option<String> {
    let key = tx.pointer("/transaction/message/accountKeys/0")?;
    match key {
        Value::String(s) => Some(s.clone()),
        Value::Object(_) => key.get("pubkey").and_then(Value::as_str).map(str::to_string),
        _ => None,
    }
}

/// How much the recipient was credited in one transaction, as exact base units,
/// plus the token's decimals and a display string. `None` mint = native SOL.
/// `base == 0` means this transaction did not credit the recipient.
struct Credit {
    base: u128,
    decimals: u8,
    display: String,
}

fn account_key_str(key: &Value) -> Option<&str> {
    match key {
        Value::String(s) => Some(s.as_str()),
        Value::Object(_) => key.get("pubkey").and_then(Value::as_str),
        _ => None,
    }
}

fn credited(tx: &Value, recipient: &Pubkey, mint: &Option<Pubkey>) -> Credit {
    let none = Credit { base: 0, decimals: 0, display: "0".into() };
    let meta = match tx.get("meta") {
        Some(m) if !m.is_null() => m,
        _ => return none,
    };
    let recipient_b58 = recipient.to_base58();

    match mint {
        // ---- native SOL: diff pre/postBalances at the recipient's index ----
        None => {
            let keys = tx
                .pointer("/transaction/message/accountKeys")
                .and_then(Value::as_array);
            let Some(i) = keys.and_then(|arr| {
                arr.iter().position(|k| account_key_str(k) == Some(recipient_b58.as_str()))
            }) else {
                return none;
            };
            let pre = meta.pointer(&format!("/preBalances/{i}")).and_then(Value::as_u64).unwrap_or(0);
            let post = meta.pointer(&format!("/postBalances/{i}")).and_then(Value::as_u64).unwrap_or(0);
            let base = (post.saturating_sub(pre)) as u128;
            Credit { base, decimals: 9, display: render_amount(base, 9) }
        }
        // ---- SPL / Token-2022: diff post/preTokenBalances for owner+mint ----
        Some(mint) => {
            let mint_b58 = mint.to_base58();
            let find = |field: &str| -> Option<Value> {
                meta.get(field)
                    .and_then(Value::as_array)?
                    .iter()
                    .find(|b| {
                        b.get("owner").and_then(Value::as_str) == Some(recipient_b58.as_str())
                            && b.get("mint").and_then(Value::as_str) == Some(mint_b58.as_str())
                    })
                    .cloned()
            };
            let base_of = |b: &Option<Value>| -> u128 {
                b.as_ref()
                    .and_then(|b| b.pointer("/uiTokenAmount/amount").and_then(Value::as_str))
                    .and_then(|s| s.parse::<u128>().ok())
                    .unwrap_or(0)
            };
            let post = find("postTokenBalances");
            let pre = find("preTokenBalances");
            let base = base_of(&post).saturating_sub(base_of(&pre));
            let decimals = post
                .as_ref()
                .and_then(|b| b.pointer("/uiTokenAmount/decimals").and_then(Value::as_u64))
                .unwrap_or(0) as u8;
            let display = post
                .as_ref()
                .and_then(|b| b.pointer("/uiTokenAmount/uiAmountString").and_then(Value::as_str))
                .map(str::to_string)
                .unwrap_or_else(|| render_amount(base, decimals));
            Credit { base, decimals, display }
        }
    }
}

/// Convert an expected UI amount to base units for an exact integer comparison.
fn expected_base(expected: f64, decimals: u8) -> u128 {
    let scale = 10f64.powi(decimals as i32);
    (expected * scale).round().max(0.0) as u128
}

fn trim_float(f: f64) -> String {
    format!("{f:.9}").trim_end_matches('0').trim_end_matches('.').to_string()
}

/// Assess a single confirmed transaction against the query.
pub fn assess(query: &WatchQuery, signature: &str, tx: &Value) -> PaymentStatus {
    let credit = credited(tx, &query.recipient, &query.mint);
    if credit.base == 0 {
        return PaymentStatus::Pending;
    }
    let want = expected_base(query.expected, credit.decimals);
    if credit.base >= want {
        PaymentStatus::Paid {
            signature: signature.to_string(),
            amount: credit.display,
            symbol: query.symbol.clone(),
            from: fee_payer(tx),
        }
    } else {
        PaymentStatus::Underpaid {
            signature: signature.to_string(),
            got: credit.display,
            want: trim_float(query.expected),
            symbol: query.symbol.clone(),
        }
    }
}

/// Orchestrate the reads. Inspect every confirmed signature on the reference —
/// not just the newest — so an attacker cannot hide a completed payment by
/// attaching a later dust transaction to the (public) reference. Returns `Paid`
/// as soon as any transaction satisfies the amount; otherwise the largest
/// underpayment seen; otherwise `Pending`. Generic over the transport.
pub fn watch<T: RpcTransport>(
    rpc_url: &str,
    transport: &T,
    query: &WatchQuery,
) -> Result<PaymentStatus, String> {
    let client = RpcClient::new(rpc_url, transport);

    let sigs = client
        .call(
            "getSignaturesForAddress",
            serde_json::json!([query.reference.to_base58(), {"limit": MAX_SIGNATURES}]),
        )
        .map_err(|e| e.to_string())?;

    let confirmed: Vec<String> = sigs
        .as_array()
        .map(|a| a.iter().filter_map(confirmed_signature).collect())
        .unwrap_or_default();
    if confirmed.is_empty() {
        return Ok(PaymentStatus::Pending);
    }

    let mut best_underpaid: Option<(f64, PaymentStatus)> = None;
    for signature in confirmed {
        let tx = client
            .call(
                "getTransaction",
                serde_json::json!([signature, {"encoding": "jsonParsed", "maxSupportedTransactionVersion": 0, "commitment": "confirmed"}]),
            )
            .map_err(|e| e.to_string())?;
        if tx.is_null() {
            continue;
        }
        match assess(query, &signature, &tx) {
            paid @ PaymentStatus::Paid { .. } => return Ok(paid),
            under @ PaymentStatus::Underpaid { .. } => {
                let got = match &under {
                    PaymentStatus::Underpaid { got, .. } => got.parse::<f64>().unwrap_or(0.0),
                    _ => 0.0,
                };
                if best_underpaid.as_ref().map(|(g, _)| got > *g).unwrap_or(true) {
                    best_underpaid = Some((got, under));
                }
            }
            PaymentStatus::Pending => {}
        }
    }

    Ok(best_underpaid.map(|(_, s)| s).unwrap_or(PaymentStatus::Pending))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const RECIPIENT: &str = "7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU";
    const PAYER: &str = "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM";

    fn query(expected: f64, token: Option<&str>) -> WatchQuery {
        let (mint, symbol) = resolve_token(token).unwrap();
        WatchQuery {
            reference: Pubkey::from_base58("5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1").unwrap(),
            recipient: Pubkey::from_base58(RECIPIENT).unwrap(),
            mint,
            expected,
            symbol,
        }
    }

    /// USDC (6 decimals) transaction crediting `ui` tokens to the recipient.
    fn spl_tx(owner: &str, mint: &str, pre_ui: f64, post_ui: f64) -> Value {
        let to_base = |ui: f64| ((ui * 1_000_000.0).round() as u128).to_string();
        json!({
            "transaction": {"message": {"accountKeys": [{"pubkey": PAYER, "signer": true}]}},
            "meta": {
                "preTokenBalances": [{"owner": owner, "mint": mint,
                    "uiTokenAmount": {"amount": to_base(pre_ui), "decimals": 6, "uiAmount": pre_ui, "uiAmountString": pre_ui.to_string()}}],
                "postTokenBalances": [{"owner": owner, "mint": mint,
                    "uiTokenAmount": {"amount": to_base(post_ui), "decimals": 6, "uiAmount": post_ui, "uiAmountString": post_ui.to_string()}}]
            }
        })
    }

    #[test]
    fn exact_usdc_payment_is_paid() {
        let q = query(25.0, Some("USDC"));
        let status = assess(&q, "sig123", &spl_tx(RECIPIENT, known::USDC_MINT, 0.0, 25.0));
        assert!(status.is_paid());
        assert!(status.render("Ref11").contains("25 USDC"));
        assert!(status.render("Ref11").contains("from 9WzD"));
    }

    #[test]
    fn short_payment_is_underpaid() {
        let q = query(25.0, Some("USDC"));
        let status = assess(&q, "sig123", &spl_tx(RECIPIENT, known::USDC_MINT, 0.0, 10.0));
        assert!(matches!(status, PaymentStatus::Underpaid { .. }));
        assert!(status.render("Ref11").contains("expected 25"));
    }

    #[test]
    fn payment_to_someone_else_is_pending() {
        let q = query(25.0, Some("USDC"));
        let status = assess(&q, "sig123", &spl_tx(PAYER, known::USDC_MINT, 0.0, 25.0));
        assert_eq!(status, PaymentStatus::Pending);
    }

    #[test]
    fn native_sol_payment_is_paid() {
        let q = query(1.5, None);
        let tx = json!({
            "transaction": {"message": {"accountKeys": [PAYER, RECIPIENT]}},
            "meta": {"preBalances": [5_000_000_000u64, 0u64], "postBalances": [3_500_000_000u64, 1_500_000_000u64]}
        });
        assert!(assess(&q, "sigSOL", &tx).is_paid());
    }

    #[test]
    fn confirmed_signature_filters_failures() {
        assert!(confirmed_signature(&json!({"signature": "s", "err": null, "confirmationStatus": "finalized"})).is_some());
        assert!(confirmed_signature(&json!({"signature": "s", "err": {"InstructionError": []}, "confirmationStatus": "finalized"})).is_none());
        assert!(confirmed_signature(&json!({"signature": "s", "err": null, "confirmationStatus": "processed"})).is_none());
    }

    #[test]
    fn expected_base_is_exact() {
        assert_eq!(expected_base(25.0, 6), 25_000_000);
        assert_eq!(expected_base(1.5, 9), 1_500_000_000);
        assert_eq!(expected_base(0.000001, 6), 1);
    }
}
