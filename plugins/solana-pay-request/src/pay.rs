//! Pure core for the `solana-pay-request` tool. No wasm, no I/O — builds and
//! validates a [Solana Pay] *transfer request* URL from typed arguments and the
//! operator's config guardrails. Host-tested with a plain `cargo test`.
//!
//! Custody tier **T1 (Build)**: the output is a `solana:` URI a human scans and
//! signs in their own wallet. This plugin holds no keys and moves no funds. Its
//! job is to make the *right* request and to refuse a malformed or out-of-policy
//! one — the guardrails below are enforced here, where the LLM cannot argue past
//! them.
//!
//! [Solana Pay]: https://docs.solanapay.com/

use std::collections::HashMap;

use solana_core::pubkey::{known, Pubkey};

/// Operator-set guardrails, resolved from the plugin's own config section. Every
/// field is a ceiling or an allowlist: config can only *restrict* what the tool
/// will emit, never widen it, so a prompt injection cannot talk its way out.
#[derive(Debug, Default, Clone)]
pub struct PayConfig {
    /// Default label (merchant/display name) when the caller omits one.
    pub label: Option<String>,
    /// If non-empty, only these mints (resolved to base58) may be charged.
    pub allowed_mints: Vec<String>,
    /// If set, reject any amount strictly greater than this (in UI units).
    pub max_amount: Option<f64>,
}

impl PayConfig {
    /// Build from the flat `string -> string` map the host injects as
    /// `__config`. Absent keys mean "no restriction / no default".
    pub fn from_section(section: &HashMap<String, String>) -> Self {
        let label = section
            .get("label")
            .filter(|v| !v.is_empty())
            .cloned();
        let allowed_mints = section
            .get("allowed_mints")
            .map(|v| {
                v.split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .filter_map(|s| resolve_mint(s).ok())
                    .collect()
            })
            .unwrap_or_default();
        let max_amount = section
            .get("max_amount")
            .and_then(|v| v.trim().parse::<f64>().ok())
            .filter(|v| *v > 0.0);
        PayConfig {
            label,
            allowed_mints,
            max_amount,
        }
    }
}

/// Typed request arguments (already deserialized from JSON by the shim).
#[derive(Debug, Clone)]
pub struct PayArgs {
    pub recipient: String,
    pub amount: String,
    /// Optional token: a symbol (`USDC`, `USDT`, `SOL`) or a base58 mint.
    /// Absent / `SOL` means a native SOL transfer.
    pub token: Option<String>,
    pub reference: Option<String>,
    pub label: Option<String>,
    pub message: Option<String>,
    pub memo: Option<String>,
}

/// A finished, validated payment request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PayRequest {
    /// The `solana:` transfer-request URI — this exact string is the QR payload.
    pub url: String,
    /// Short human summary the approval gate / chat renders next to the QR.
    pub summary: String,
}

/// Resolve a token symbol or raw base58 mint to a canonical base58 mint.
/// Returns `Ok(None)` for native SOL.
fn resolve_mint(token: &str) -> Result<String, String> {
    match token.trim().to_ascii_uppercase().as_str() {
        "" | "SOL" => Ok(known::WSOL_MINT.to_string()), // sentinel handled by caller
        "USDC" => Ok(known::USDC_MINT.to_string()),
        "USDT" => Ok(known::USDT_MINT.to_string()),
        _ => {
            // Anything else must be a valid 32-byte base58 mint.
            Pubkey::from_base58(token)
                .map(|p| p.to_base58())
                .map_err(|e| format!("token '{token}' is not a known symbol or valid mint: {e}"))
        }
    }
}

/// Minimal, allocation-light percent-encoder for query values. Encodes
/// everything outside the RFC 3986 unreserved set, so labels/memos with spaces,
/// `&`, `?`, or non-ASCII can never break out of their query parameter.
fn encode(s: &str) -> String {
    const UNRESERVED: &[u8] = b"-_.~";
    let mut out = String::with_capacity(s.len());
    for &b in s.as_bytes() {
        if b.is_ascii_alphanumeric() || UNRESERVED.contains(&b) {
            out.push(b as char);
        } else {
            out.push('%');
            out.push_str(&format!("{b:02X}"));
        }
    }
    out
}

/// Validate an amount string: a positive, finite decimal within `max_amount`.
fn validate_amount(amount: &str, max: Option<f64>) -> Result<String, String> {
    let a = amount.trim();
    let parsed: f64 = a
        .parse()
        .map_err(|_| format!("amount '{a}' is not a number"))?;
    if !parsed.is_finite() || parsed <= 0.0 {
        return Err(format!("amount '{a}' must be a positive number"));
    }
    if let Some(cap) = max {
        if parsed > cap {
            return Err(format!(
                "amount {a} exceeds the configured max of {cap} — refused"
            ));
        }
    }
    Ok(a.to_string())
}

/// Build a validated Solana Pay transfer request. All policy is enforced here.
pub fn build_request(args: &PayArgs, cfg: &PayConfig) -> Result<PayRequest, String> {
    // 1. Recipient must be a real 32-byte address, never a hallucinated string.
    let recipient = Pubkey::from_base58(&args.recipient)
        .map_err(|e| format!("recipient is not a valid Solana address: {e}"))?;

    // 2. Amount: positive, finite, within the operator's ceiling.
    let amount = validate_amount(&args.amount, cfg.max_amount)?;

    // 3. Token: resolve symbol/mint; enforce the allowlist if one is configured.
    let token_norm = args.token.as_deref().map(|t| t.trim().to_ascii_uppercase());
    let is_native = match token_norm.as_deref() {
        None => true,
        Some(s) => s.is_empty() || s == "SOL",
    };
    let (spl_token, token_label) = if is_native {
        (None, "SOL".to_string())
    } else {
        let raw = args.token.as_deref().unwrap();
        let mint = resolve_mint(raw)?;
        if !cfg.allowed_mints.is_empty() && !cfg.allowed_mints.contains(&mint) {
            return Err(format!(
                "token {} is not in the operator's allowed_mints allowlist — refused",
                short_symbol(raw, &mint)
            ));
        }
        (Some(mint), short_symbol(raw, "").to_string())
    };

    // 4. Optional reference must itself be a valid pubkey (used for on-chain
    //    reconciliation by payment-watch); a bad one is rejected, not passed on.
    let reference = match &args.reference {
        Some(r) if !r.trim().is_empty() => Some(
            Pubkey::from_base58(r)
                .map_err(|e| format!("reference is not a valid Solana address: {e}"))?
                .to_base58(),
        ),
        _ => None,
    };

    // 5. Assemble the URI.
    let mut url = format!("solana:{recipient}?amount={amount}");
    if let Some(mint) = &spl_token {
        url.push_str(&format!("&spl-token={mint}"));
    }
    if let Some(r) = &reference {
        url.push_str(&format!("&reference={r}"));
    }
    let label = args.label.clone().or_else(|| cfg.label.clone());
    if let Some(l) = &label {
        url.push_str(&format!("&label={}", encode(l)));
    }
    if let Some(m) = &args.message {
        url.push_str(&format!("&message={}", encode(m)));
    }
    if let Some(memo) = &args.memo {
        url.push_str(&format!("&memo={}", encode(memo)));
    }

    let summary = format!(
        "Payment request: {amount} {token_label} to {}{}",
        solana_core::shape::abbrev(&recipient.to_base58()),
        memo_suffix(&args.memo),
    );

    Ok(PayRequest { url, summary })
}

/// Prefer the human symbol the caller used, else fall back to an abbreviated mint.
fn short_symbol(raw: &str, mint: &str) -> String {
    let up = raw.trim().to_ascii_uppercase();
    if matches!(up.as_str(), "USDC" | "USDT" | "SOL") {
        up
    } else if mint.is_empty() {
        solana_core::shape::abbrev(raw)
    } else {
        solana_core::shape::abbrev(mint)
    }
}

fn memo_suffix(memo: &Option<String>) -> String {
    match memo {
        Some(m) if !m.is_empty() => format!(" — memo: {}", solana_core::shape::clamp_text(m, 60)),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    const MERCHANT: &str = "7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU";

    #[test]
    fn builds_usdc_request() {
        let r = build_request(&args(MERCHANT, "25", Some("USDC")), &PayConfig::default()).unwrap();
        assert!(r.url.starts_with(&format!("solana:{MERCHANT}?amount=25")));
        assert!(r.url.contains(&format!("spl-token={}", known::USDC_MINT)));
        assert!(r.summary.contains("25 USDC"));
    }

    #[test]
    fn native_sol_has_no_spl_token() {
        let r = build_request(&args(MERCHANT, "1.5", None), &PayConfig::default()).unwrap();
        assert!(!r.url.contains("spl-token"));
        assert!(r.summary.contains("1.5 SOL"));
    }

    #[test]
    fn rejects_bad_recipient() {
        let err = build_request(&args("not-a-real-address!!", "25", Some("USDC")), &PayConfig::default()).unwrap_err();
        assert!(err.contains("valid Solana address"));
    }

    #[test]
    fn encodes_memo_and_label() {
        let mut a = args(MERCHANT, "25", Some("USDC"));
        a.memo = Some("Invoice #412 & table 4".into());
        a.label = Some("Bar do Zé".into());
        let r = build_request(&a, &PayConfig::default()).unwrap();
        // spaces/&/# never appear raw in the query
        assert!(r.url.contains("memo=Invoice%20%23412%20%26%20table%204"));
        assert!(r.url.contains("label=Bar%20do%20Z%C3%A9"));
    }

    // ---- guardrails: these are the prompt-injection defenses ----

    #[test]
    fn enforces_max_amount() {
        let cfg = PayConfig { max_amount: Some(100.0), ..Default::default() };
        let err = build_request(&args(MERCHANT, "1000000", Some("USDC")), &cfg).unwrap_err();
        assert!(err.contains("exceeds the configured max"));
    }

    #[test]
    fn enforces_mint_allowlist() {
        let cfg = PayConfig {
            allowed_mints: vec![known::USDC_MINT.to_string()],
            ..Default::default()
        };
        // USDC is allowed
        assert!(build_request(&args(MERCHANT, "10", Some("USDC")), &cfg).is_ok());
        // USDT is not
        let err = build_request(&args(MERCHANT, "10", Some("USDT")), &cfg).unwrap_err();
        assert!(err.contains("allowlist"));
    }

    #[test]
    fn rejects_zero_and_negative() {
        for bad in ["0", "-5", "abc"] {
            assert!(build_request(&args(MERCHANT, bad, Some("USDC")), &PayConfig::default()).is_err());
        }
    }
}
