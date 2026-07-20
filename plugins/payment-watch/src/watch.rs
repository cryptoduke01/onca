//! Pure core for the `payment-watch` tool. No wasm, no I/O: it interprets the
//! JSON that `getSignaturesForAddress` and `getTransaction` return and decides
//! whether an expected payment has landed. Host-tested with canned fixtures.
//!
//! Custody tier **T0 (Read)**. It signs nothing and moves nothing. It closes the
//! loop opened by `solana-pay-request`: that tool embeds a unique `reference`
//! pubkey in the Solana Pay URL; this tool watches that reference and confirms
//! the money actually arrived — the right amount, to the right recipient.
//!
//! Design note: the Solana Pay `reference` is a read-only account attached to
//! the transfer *specifically* so a watcher can locate the transaction with
//! `getSignaturesForAddress(reference)`. A unique reference per invoice means a
//! single confirmed signature is the payment.

use serde_json::Value;

use solana_core::pubkey::{known, Pubkey};
use solana_core::rpc::{RpcClient, RpcTransport};
use solana_core::shape::abbrev;

/// Tiny epsilon so floating-point rounding never marks an exact payment as
/// underpaid (e.g. 24.999999 vs 25).
const EPSILON: f64 = 1e-9;

/// What the watcher concluded.
#[derive(Debug, Clone, PartialEq)]
pub enum PaymentStatus {
    /// No confirmed transaction references this invoice yet.
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
                format!("Paid ✓ — {amount} {symbol}{who}. Tx {}.", abbrev(signature))
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
/// errored or is unconfirmed.
fn confirmed_signature(entry: &Value) -> Option<String> {
    if !entry.get("err").map(Value::is_null).unwrap_or(true) {
        return None; // transaction failed
    }
    let status = entry
        .get("confirmationStatus")
        .and_then(Value::as_str)
        .unwrap_or("");
    // Treat missing status as confirmed (older nodes omit it on finalized txs).
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

/// Compute how much `recipient` received in this transaction, in UI units, and
/// a display string for it. `None` mint = native SOL. Returns `(delta_ui,
/// display)`; `delta_ui <= 0` means this tx did not credit the recipient.
fn credited_amount(tx: &Value, recipient: &Pubkey, mint: &Option<Pubkey>) -> (f64, String) {
    let meta = match tx.get("meta") {
        Some(m) if !m.is_null() => m,
        _ => return (0.0, "0".into()),
    };
    let recipient_b58 = recipient.to_base58();

    match mint {
        // ---- native SOL: diff pre/postBalances at the recipient's index ----
        None => {
            let keys = tx
                .pointer("/transaction/message/accountKeys")
                .and_then(Value::as_array);
            let idx = keys.and_then(|arr| {
                arr.iter().position(|k| {
                    let s = match k {
                        Value::String(s) => Some(s.as_str()),
                        Value::Object(_) => k.get("pubkey").and_then(Value::as_str),
                        _ => None,
                    };
                    s == Some(recipient_b58.as_str())
                })
            });
            let Some(i) = idx else { return (0.0, "0".into()) };
            let pre = meta.pointer(&format!("/preBalances/{i}")).and_then(Value::as_u64).unwrap_or(0);
            let post = meta.pointer(&format!("/postBalances/{i}")).and_then(Value::as_u64).unwrap_or(0);
            let delta = post as i128 - pre as i128;
            let ui = delta as f64 / 1_000_000_000.0;
            (ui, format!("{:.9}", ui.max(0.0)).trim_end_matches('0').trim_end_matches('.').to_string())
        }
        // ---- SPL / Token-2022: diff post/preTokenBalances for owner+mint ----
        Some(mint) => {
            let mint_b58 = mint.to_base58();
            let match_bal = |arr: Option<&Vec<Value>>| -> Option<Value> {
                arr?.iter()
                    .find(|b| {
                        b.get("owner").and_then(Value::as_str) == Some(recipient_b58.as_str())
                            && b.get("mint").and_then(Value::as_str) == Some(mint_b58.as_str())
                    })
                    .cloned()
            };
            let post = match_bal(meta.get("postTokenBalances").and_then(Value::as_array));
            let pre = match_bal(meta.get("preTokenBalances").and_then(Value::as_array));
            let ui_of = |b: &Option<Value>| -> f64 {
                b.as_ref()
                    .and_then(|b| b.pointer("/uiTokenAmount/uiAmount").and_then(Value::as_f64))
                    .unwrap_or(0.0)
            };
            let post_ui = ui_of(&post);
            let pre_ui = ui_of(&pre);
            let delta = post_ui - pre_ui;
            let display = post
                .as_ref()
                .and_then(|b| b.pointer("/uiTokenAmount/uiAmountString").and_then(Value::as_str))
                .map(str::to_string)
                .unwrap_or_else(|| format!("{delta}"));
            (delta, display)
        }
    }
}

/// Decide the status from a confirmed signature and its fetched transaction.
pub fn assess(query: &WatchQuery, signature: &str, tx: &Value) -> PaymentStatus {
    let (delta_ui, got_display) = credited_amount(tx, &query.recipient, &query.mint);
    if delta_ui <= 0.0 {
        return PaymentStatus::Pending;
    }
    if delta_ui + EPSILON >= query.expected {
        PaymentStatus::Paid {
            signature: signature.to_string(),
            amount: got_display,
            symbol: query.symbol.clone(),
            from: fee_payer(tx),
        }
    } else {
        PaymentStatus::Underpaid {
            signature: signature.to_string(),
            got: got_display,
            want: trim_float(query.expected),
            symbol: query.symbol.clone(),
        }
    }
}

fn trim_float(f: f64) -> String {
    format!("{f:.9}").trim_end_matches('0').trim_end_matches('.').to_string()
}

/// Orchestrate the reads: find a confirmed signature on the reference, fetch it,
/// and assess. Generic over the transport so tests use a mock and the shim uses
/// `waki`. No I/O in this crate.
pub fn watch<T: RpcTransport>(
    rpc_url: &str,
    transport: &T,
    query: &WatchQuery,
) -> Result<PaymentStatus, String> {
    let client = RpcClient::new(rpc_url, transport);

    let sigs = client
        .call(
            "getSignaturesForAddress",
            serde_json::json!([query.reference.to_base58(), {"limit": 10}]),
        )
        .map_err(|e| e.to_string())?;

    let entries = sigs.as_array().cloned().unwrap_or_default();
    let Some(signature) = entries.iter().find_map(confirmed_signature) else {
        return Ok(PaymentStatus::Pending);
    };

    let tx = client
        .call(
            "getTransaction",
            serde_json::json!([signature, {"encoding": "jsonParsed", "maxSupportedTransactionVersion": 0, "commitment": "confirmed"}]),
        )
        .map_err(|e| e.to_string())?;

    if tx.is_null() {
        return Ok(PaymentStatus::Pending);
    }

    Ok(assess(query, &signature, &tx))
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
            reference: Pubkey::from_base58("H1meanwhile111111111111111111111111111111").unwrap_or(Pubkey::ZERO),
            recipient: Pubkey::from_base58(RECIPIENT).unwrap(),
            mint,
            expected,
            symbol,
        }
    }

    fn spl_tx(owner: &str, mint: &str, pre: f64, post: f64) -> Value {
        json!({
            "transaction": {"message": {"accountKeys": [{"pubkey": PAYER, "signer": true}]}},
            "meta": {
                "preTokenBalances": [{"owner": owner, "mint": mint, "uiTokenAmount": {"uiAmount": pre, "uiAmountString": pre.to_string()}}],
                "postTokenBalances": [{"owner": owner, "mint": mint, "uiTokenAmount": {"uiAmount": post, "uiAmountString": post.to_string()}}]
            }
        })
    }

    #[test]
    fn exact_usdc_payment_is_paid() {
        let q = query(25.0, Some("USDC"));
        let tx = spl_tx(RECIPIENT, known::USDC_MINT, 0.0, 25.0);
        let status = assess(&q, "sig123", &tx);
        assert!(status.is_paid());
        assert!(status.render("Ref11").contains("25 USDC"));
        assert!(status.render("Ref11").contains("from 9WzD"));
    }

    #[test]
    fn short_payment_is_underpaid() {
        let q = query(25.0, Some("USDC"));
        let tx = spl_tx(RECIPIENT, known::USDC_MINT, 0.0, 10.0);
        let status = assess(&q, "sig123", &tx);
        assert!(matches!(status, PaymentStatus::Underpaid { .. }));
        assert!(status.render("Ref11").contains("expected 25"));
    }

    #[test]
    fn payment_to_someone_else_is_pending() {
        let q = query(25.0, Some("USDC"));
        // credited a different owner — recipient got nothing
        let tx = spl_tx(PAYER, known::USDC_MINT, 0.0, 25.0);
        assert_eq!(assess(&q, "sig123", &tx), PaymentStatus::Pending);
    }

    #[test]
    fn native_sol_payment_is_paid() {
        let q = query(1.5, None);
        let tx = json!({
            "transaction": {"message": {"accountKeys": [PAYER, RECIPIENT]}},
            "meta": {"preBalances": [5_000_000_000u64, 0u64], "postBalances": [3_500_000_000u64, 1_500_000_000u64]}
        });
        let status = assess(&q, "sigSOL", &tx);
        assert!(status.is_paid());
    }

    #[test]
    fn confirmed_signature_filters_failures() {
        assert!(confirmed_signature(&json!({"signature": "s", "err": null, "confirmationStatus": "finalized"})).is_some());
        assert!(confirmed_signature(&json!({"signature": "s", "err": {"InstructionError": []}, "confirmationStatus": "finalized"})).is_none());
        assert!(confirmed_signature(&json!({"signature": "s", "err": null, "confirmationStatus": "processed"})).is_none());
    }
}
