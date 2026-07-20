//! ZeroClaw WIT tool plugin: `solana_pay_request`.
//!
//! Custody tier **T1 (Build)**. Given a recipient, amount, and token, it returns
//! a [Solana Pay] `solana:` transfer-request URI — the exact string a chat
//! surface renders as a QR code for the payer to scan and sign in their own
//! wallet. This plugin holds no keys, touches no network, and moves no funds.
//!
//! All validation and policy live in [`pay`] (pure, host-tested). This file is
//! the thin `#[cfg(target_family = "wasm")]` shim that wires that logic to the
//! `tool-plugin` world and reports structured log events via `log-record`.
//!
//! Build:  rustup target add wasm32-wasip2
//!         cargo build --target wasm32-wasip2 --release
//!
//! [Solana Pay]: https://docs.solanapay.com/

pub mod pay;

#[cfg(target_family = "wasm")]
mod component {
    wit_bindgen::generate!({
        path: "../../wit/v0",
        world: "tool-plugin",
        features: ["plugins-wit-v0"],
    });

    use std::collections::HashMap;

    use crate::pay::{build_request, PayArgs, PayConfig};
    use exports::zeroclaw::plugin::plugin_info::Guest as PluginInfo;
    use exports::zeroclaw::plugin::tool::{Guest as Tool, ToolResult};
    use zeroclaw::plugin::logging::{
        log_record, LogLevel, PluginAction, PluginEvent, PluginOutcome,
    };

    struct SolanaPayRequest;

    const PLUGIN_NAME: &str = "solana-pay-request";
    const PLUGIN_VERSION: &str = env!("CARGO_PKG_VERSION");
    const TOOL_NAME: &str = "solana_pay_request";

    #[derive(serde::Deserialize)]
    struct ExecuteArgs {
        recipient: String,
        amount: String,
        #[serde(default)]
        token: Option<String>,
        #[serde(default)]
        reference: Option<String>,
        #[serde(default)]
        label: Option<String>,
        #[serde(default)]
        message: Option<String>,
        #[serde(default)]
        memo: Option<String>,
        #[serde(rename = "__config", default)]
        config: HashMap<String, String>,
    }

    impl PluginInfo for SolanaPayRequest {
        fn plugin_name() -> String {
            PLUGIN_NAME.to_string()
        }
        fn plugin_version() -> String {
            PLUGIN_VERSION.to_string()
        }
    }

    impl Tool for SolanaPayRequest {
        fn name() -> String {
            TOOL_NAME.to_string()
        }

        fn description() -> String {
            "Create a Solana Pay payment request. Given a recipient address, an amount, and a \
             token (USDC, USDT, SOL, or a mint address), returns a `solana:` transfer URL that \
             renders as a QR code for the payer to scan and sign in their own wallet. Does NOT \
             move funds and holds no keys — it only builds the request. Operator-configured mint \
             allowlist and maximum amount are enforced and cannot be overridden by the request."
                .to_string()
        }

        fn parameters_schema() -> String {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "recipient": { "type": "string", "description": "Merchant/recipient Solana address (base58)." },
                    "amount": { "type": "string", "description": "Amount in whole token units, e.g. \"25\" or \"1.5\"." },
                    "token": { "type": "string", "description": "Token to charge: USDC, USDT, SOL, or a mint address. Defaults to SOL." },
                    "reference": { "type": "string", "description": "Optional base58 reference pubkey for on-chain reconciliation." },
                    "label": { "type": "string", "description": "Optional merchant/display label shown in the payer's wallet." },
                    "message": { "type": "string", "description": "Optional message shown in the payer's wallet." },
                    "memo": { "type": "string", "description": "Optional memo recorded on-chain, e.g. an invoice number." }
                },
                "required": ["recipient", "amount"]
            })
            .to_string()
        }

        fn execute(args: String) -> Result<ToolResult, String> {
            let parsed: ExecuteArgs = match serde_json::from_str(&args) {
                Ok(a) => a,
                Err(e) => {
                    emit(PluginAction::Fail, PluginOutcome::Failure, "invalid arguments");
                    return Ok(fail(format!("invalid arguments: {e}")));
                }
            };

            let cfg = PayConfig::from_section(&parsed.config);
            let pay_args = PayArgs {
                recipient: parsed.recipient,
                amount: parsed.amount,
                token: parsed.token,
                reference: parsed.reference,
                label: parsed.label,
                message: parsed.message,
                memo: parsed.memo,
            };

            match build_request(&pay_args, &cfg) {
                Ok(req) => {
                    emit(PluginAction::Complete, PluginOutcome::Success, "built payment request");
                    // Output: human summary + the exact QR payload URI on its own line.
                    let output = format!(
                        "{}\n{}\n(Render the URI above as a QR code for the payer to scan.)",
                        req.summary, req.url
                    );
                    Ok(ToolResult { success: true, output, error: None })
                }
                Err(e) => {
                    // Fail closed: refused requests (bad address, over cap, disallowed
                    // mint) are a normal, safe outcome — surfaced, never executed.
                    emit(PluginAction::Reject, PluginOutcome::Failure, "refused payment request");
                    Ok(fail(e))
                }
            }
        }
    }

    fn fail(msg: String) -> ToolResult {
        ToolResult { success: false, output: String::new(), error: Some(msg) }
    }

    fn emit(action: PluginAction, outcome: PluginOutcome, message: &str) {
        log_record(
            LogLevel::Info,
            &PluginEvent {
                function_name: "solana_pay_request::tool::execute".to_string(),
                action,
                outcome: Some(outcome),
                duration_ms: None,
                attrs: None,
                message: message.to_string(),
            },
        );
    }

    export!(SolanaPayRequest);
}
