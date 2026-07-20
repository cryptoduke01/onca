//! Pure core for the `token-risk-check` tool. No wasm, no I/O: it takes the JSON
//! that `getAccountInfo` (jsonParsed) and `getTokenLargestAccounts` return and
//! turns it into a small red/amber/green verdict with reasons. Host-tested with
//! canned RPC fixtures — see `tests/risk.rs`.
//!
//! Custody tier **T0 (Read)**. This tool never signs, sends, or holds a key. Its
//! entire job is to make *other* actions safer by telling a human (or an agent's
//! guardrail) whether a mint is a honeypot before anyone touches it.

use serde_json::Value;

use onca_core::pubkey::known;
use onca_core::rpc::{commitment, RpcClient, RpcTransport};
use onca_core::shape::{abbrev, render_amount};

/// Severity of a single finding, and of the report overall (the max of its
/// flags). Ordered so `max` gives the worst.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Green,
    Amber,
    Red,
}

impl Severity {
    fn label(self) -> &'static str {
        match self {
            Severity::Green => "GREEN",
            Severity::Amber => "AMBER",
            Severity::Red => "RED",
        }
    }
}

/// One risk finding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Flag {
    pub severity: Severity,
    pub text: String,
}

/// Which token program owns the mint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenProgram {
    Token,
    Token2022,
}

impl TokenProgram {
    fn label(self) -> &'static str {
        match self {
            TokenProgram::Token => "SPL Token",
            TokenProgram::Token2022 => "Token-2022",
        }
    }
}

/// Structured facts pulled out of the mint account, before scoring.
#[derive(Debug, Clone)]
pub struct MintFacts {
    pub program: TokenProgram,
    pub decimals: u8,
    pub supply: u128,
    pub mint_authority: Option<String>,
    pub freeze_authority: Option<String>,
    /// Raw Token-2022 extension objects (empty for classic SPL Token).
    pub extensions: Vec<Value>,
}

/// The finished verdict.
#[derive(Debug, Clone)]
pub struct RiskReport {
    pub mint: String,
    pub program: TokenProgram,
    pub rating: Severity,
    pub flags: Vec<Flag>,
    pub decimals: u8,
    pub supply: u128,
    pub holders_sampled: usize,
}

impl RiskReport {
    /// Overall rating = worst flag, or GREEN if none.
    fn from_flags(mint: String, facts: &MintFacts, flags: Vec<Flag>, holders_sampled: usize) -> Self {
        let rating = flags.iter().map(|f| f.severity).max().unwrap_or(Severity::Green);
        RiskReport {
            mint,
            program: facts.program,
            rating,
            flags,
            decimals: facts.decimals,
            supply: facts.supply,
            holders_sampled,
        }
    }

    /// Shaped, ~150-token human summary. This is what the agent sees — never the
    /// raw multi-kilobyte RPC payloads.
    pub fn render(&self) -> String {
        let mut out = format!(
            "Token risk: {} — {} flag{}\nMint {} ({})\n",
            self.rating.label(),
            self.flags.len(),
            if self.flags.len() == 1 { "" } else { "s" },
            abbrev(&self.mint),
            self.program.label(),
        );
        if self.flags.is_empty() {
            out.push_str("• No mint/freeze authority, no risky extensions, no extreme holder concentration detected.\n");
        } else {
            for f in &self.flags {
                out.push_str(&format!("• [{}] {}\n", f.severity.label(), f.text));
            }
        }
        out.push_str(&format!(
            "Supply {} · decimals {} · {} holders sampled\n\
             Note: this is a heuristic, not an audit. GREEN ≠ safe.",
            render_amount(self.supply, self.decimals),
            self.decimals,
            self.holders_sampled,
        ));
        out
    }
}

/// Parse the `result` of a `getAccountInfo` (jsonParsed) call on a mint.
pub fn parse_mint(result: &Value) -> Result<MintFacts, String> {
    let value = result
        .get("value")
        .filter(|v| !v.is_null())
        .ok_or("mint account not found (null) — is this a valid mint address?")?;

    let owner = value.get("owner").and_then(Value::as_str).unwrap_or("");
    let program = if owner == known::TOKEN_2022_PROGRAM {
        TokenProgram::Token2022
    } else {
        TokenProgram::Token
    };

    let info = value
        .pointer("/data/parsed/info")
        .ok_or("account is not a parsed SPL mint (no data.parsed.info)")?;

    let parsed_type = value
        .pointer("/data/parsed/type")
        .and_then(Value::as_str)
        .unwrap_or("");
    if parsed_type != "mint" {
        return Err(format!("account is a '{parsed_type}', not a mint"));
    }

    let decimals = info.get("decimals").and_then(Value::as_u64).unwrap_or(0) as u8;
    let supply = info
        .get("supply")
        .and_then(Value::as_str)
        .and_then(|s| s.parse::<u128>().ok())
        .unwrap_or(0);
    let mint_authority = info
        .get("mintAuthority")
        .and_then(Value::as_str)
        .map(str::to_string);
    let freeze_authority = info
        .get("freezeAuthority")
        .and_then(Value::as_str)
        .map(str::to_string);
    let extensions = info
        .get("extensions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    Ok(MintFacts {
        program,
        decimals,
        supply,
        mint_authority,
        freeze_authority,
        extensions,
    })
}

/// Score the mint facts + largest-holder data into a report. Pure — this is the
/// heart of the tool and the most-tested function.
pub fn assess(mint: &str, facts: &MintFacts, largest_result: &Value) -> RiskReport {
    let mut flags: Vec<Flag> = Vec::new();

    // --- authorities ---
    if let Some(auth) = &facts.mint_authority {
        flags.push(Flag {
            severity: Severity::Amber,
            text: format!(
                "Mint authority is set ({}) — supply can be inflated at will.",
                abbrev(auth)
            ),
        });
    }
    if let Some(auth) = &facts.freeze_authority {
        flags.push(Flag {
            severity: Severity::Amber,
            text: format!(
                "Freeze authority is set ({}) — your token account can be frozen.",
                abbrev(auth)
            ),
        });
    }

    // --- Token-2022 extensions: the honeypot vectors ---
    for ext in &facts.extensions {
        let name = ext.get("extension").and_then(Value::as_str).unwrap_or("");
        match name {
            "permanentDelegate" => {
                let d = ext
                    .pointer("/state/delegate")
                    .and_then(Value::as_str)
                    .map(abbrev)
                    .unwrap_or_else(|| "unknown".into());
                flags.push(Flag {
                    severity: Severity::Red,
                    text: format!(
                        "Permanent delegate is set ({d}) — this address can move or burn your tokens at any time."
                    ),
                });
            }
            "transferHook" => {
                let p = ext
                    .pointer("/state/programId")
                    .and_then(Value::as_str)
                    .map(abbrev)
                    .unwrap_or_else(|| "unknown".into());
                flags.push(Flag {
                    severity: Severity::Red,
                    text: format!(
                        "Transfer hook is set ({p}) — arbitrary program code runs on every transfer; it can block or tax sells."
                    ),
                });
            }
            "nonTransferable" => flags.push(Flag {
                severity: Severity::Red,
                text: "Non-transferable extension — tokens cannot be moved once received.".into(),
            }),
            "defaultAccountState" => {
                let state = ext
                    .pointer("/state/accountState")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                if state.eq_ignore_ascii_case("frozen") {
                    flags.push(Flag {
                        severity: Severity::Red,
                        text: "Default account state is 'frozen' — new holders start frozen until the authority thaws them.".into(),
                    });
                }
            }
            "transferFeeConfig" => {
                let bps = ext
                    .pointer("/state/newerTransferFee/transferFeeBasisPoints")
                    .and_then(Value::as_u64)
                    .unwrap_or(0);
                flags.push(Flag {
                    severity: if bps >= 1000 { Severity::Red } else { Severity::Amber },
                    text: format!(
                        "Transfer fee of {:.2}% ({bps} bps) is charged on every transfer.",
                        bps as f64 / 100.0
                    ),
                });
            }
            _ => {}
        }
    }

    // --- holder concentration ---
    let holders = largest_result
        .get("value")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let holders_sampled = holders.len();
    if facts.supply > 0 && !holders.is_empty() {
        let amount_of = |h: &Value| -> u128 {
            h.get("amount")
                .and_then(Value::as_str)
                .and_then(|s| s.parse::<u128>().ok())
                .unwrap_or(0)
        };
        let top1 = amount_of(&holders[0]);
        let top1_pct = pct(top1, facts.supply);
        let top5: u128 = holders.iter().take(5).map(amount_of).sum();
        let top5_pct = pct(top5, facts.supply);

        if top1_pct >= 50.0 {
            flags.push(Flag {
                severity: Severity::Red,
                text: format!("Top holder controls {top1_pct:.0}% of supply — extreme concentration."),
            });
        } else if top1_pct >= 25.0 {
            flags.push(Flag {
                severity: Severity::Amber,
                text: format!("Top holder controls {top1_pct:.0}% of supply."),
            });
        } else if top5_pct >= 80.0 {
            flags.push(Flag {
                severity: Severity::Amber,
                text: format!("Top 5 holders control {top5_pct:.0}% of supply."),
            });
        }
    }

    RiskReport::from_flags(mint.to_string(), facts, flags, holders_sampled)
}

fn pct(part: u128, whole: u128) -> f64 {
    if whole == 0 {
        return 0.0;
    }
    (part as f64 / whole as f64) * 100.0
}

/// Orchestrate the two RPC reads and produce a report. Generic over the
/// transport, so tests drive it with a mock and the wasm shim drives it with
/// `waki`. No I/O lives in this crate — the transport does it.
pub fn analyze<T: RpcTransport>(rpc_url: &str, transport: &T, mint: &str) -> Result<RiskReport, String> {
    // Validate the mint address before spending an RPC call on it.
    let mint = onca_core::Pubkey::from_base58(mint)
        .map_err(|e| format!("invalid mint address: {e}"))?
        .to_base58();

    let client = RpcClient::new(rpc_url, transport);

    let account = client
        .call(
            "getAccountInfo",
            serde_json::json!([mint, {"encoding": "jsonParsed", "commitment": "confirmed"}]),
        )
        .map_err(|e| e.to_string())?;
    let facts = parse_mint(&account)?;

    let largest = client
        .call("getTokenLargestAccounts", serde_json::json!([mint, commitment("confirmed")]))
        .map_err(|e| e.to_string())?;

    Ok(assess(&mint, &facts, &largest))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn clean_mint_result() -> Value {
        json!({"context": {"slot": 1}, "value": {
            "owner": known::TOKEN_PROGRAM,
            "data": {"parsed": {"type": "mint", "info": {
                "decimals": 6, "supply": "1000000000000",
                "mintAuthority": null, "freezeAuthority": null, "isInitialized": true
            }}}
        }})
    }

    fn largest(amounts: &[&str]) -> Value {
        let arr: Vec<Value> = amounts
            .iter()
            .map(|a| json!({"address": "H1", "amount": a, "decimals": 6}))
            .collect();
        json!({"context": {"slot": 1}, "value": arr})
    }

    #[test]
    fn clean_token_is_green() {
        let facts = parse_mint(&clean_mint_result()).unwrap();
        // even spread across 5 holders
        let r = assess("Mint1111", &facts, &largest(&["10000000000"; 5]));
        assert_eq!(r.rating, Severity::Green);
        assert!(r.flags.is_empty());
    }

    #[test]
    fn mint_and_freeze_authority_flag_amber() {
        let mut v = clean_mint_result();
        v["value"]["data"]["parsed"]["info"]["mintAuthority"] = json!("Auth1111");
        v["value"]["data"]["parsed"]["info"]["freezeAuthority"] = json!("Auth2222");
        let facts = parse_mint(&v).unwrap();
        let r = assess("Mint1111", &facts, &largest(&["10000000000"; 5]));
        assert_eq!(r.rating, Severity::Amber);
        assert_eq!(r.flags.len(), 2);
    }

    #[test]
    fn permanent_delegate_is_red() {
        let mut v = clean_mint_result();
        v["value"]["owner"] = json!(known::TOKEN_2022_PROGRAM);
        v["value"]["data"]["parsed"]["info"]["extensions"] = json!([
            {"extension": "permanentDelegate", "state": {"delegate": "Rug111111111111111111111111111111111111111"}}
        ]);
        let facts = parse_mint(&v).unwrap();
        assert_eq!(facts.program, TokenProgram::Token2022);
        let r = assess("Mint1111", &facts, &largest(&["10000000000"; 5]));
        assert_eq!(r.rating, Severity::Red);
        assert!(r.render().contains("Permanent delegate"));
    }

    #[test]
    fn transfer_hook_is_red() {
        let mut v = clean_mint_result();
        v["value"]["owner"] = json!(known::TOKEN_2022_PROGRAM);
        v["value"]["data"]["parsed"]["info"]["extensions"] = json!([
            {"extension": "transferHook", "state": {"programId": "Hook11111111111111111111111111111111111111"}}
        ]);
        let facts = parse_mint(&v).unwrap();
        let r = assess("Mint1111", &facts, &largest(&["10000000000"; 5]));
        assert_eq!(r.rating, Severity::Red);
    }

    #[test]
    fn high_concentration_is_red() {
        let facts = parse_mint(&clean_mint_result()).unwrap();
        // one holder with 60% of the 1_000_000 (×10^6) supply
        let r = assess("Mint1111", &facts, &largest(&["600000000000", "50000000000"]));
        assert_eq!(r.rating, Severity::Red);
        assert!(r.flags.iter().any(|f| f.text.contains("Top holder")));
    }

    #[test]
    fn missing_mint_is_error() {
        let v = json!({"context": {"slot": 1}, "value": null});
        assert!(parse_mint(&v).is_err());
    }

    #[test]
    fn transfer_fee_reports_bps() {
        let mut v = clean_mint_result();
        v["value"]["owner"] = json!(known::TOKEN_2022_PROGRAM);
        v["value"]["data"]["parsed"]["info"]["extensions"] = json!([
            {"extension": "transferFeeConfig", "state": {"newerTransferFee": {"transferFeeBasisPoints": 500}}}
        ]);
        let facts = parse_mint(&v).unwrap();
        let r = assess("Mint1111", &facts, &largest(&["10000000000"; 5]));
        assert_eq!(r.rating, Severity::Amber);
        assert!(r.render().contains("5.00%"));
    }
}
