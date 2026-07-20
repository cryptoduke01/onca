//! ZeroClaw WIT tool plugin: `payment_watch`.
//!
//! Custody tier **T0 (Read)**. Given an invoice's `reference` pubkey, the
//! recipient, and the expected amount + token, it checks the chain and reports
//! whether the payment has landed: **paid / underpaid / pending**. Wire it into
//! a cron SOP to poll an open invoice and turn a `paid` result into a Telegram
//! message — closing the loop opened by `solana-pay-request`.
//!
//! All detection logic lives in [`watch`] (pure, host-tested against canned RPC
//! fixtures). This file is the thin `#[cfg(target_family = "wasm")]` shim.
//!
//! Build:  rustup target add wasm32-wasip2
//!         cargo build --target wasm32-wasip2 --release

pub mod watch;

#[cfg(target_family = "wasm")]
mod component {
    wit_bindgen::generate!({
        path: "../../wit/v0",
        world: "tool-plugin",
        features: ["plugins-wit-v0"],
    });

    use std::collections::HashMap;

    use crate::watch::{resolve_token, watch, WatchQuery};
    use exports::zeroclaw::plugin::plugin_info::Guest as PluginInfo;
    use exports::zeroclaw::plugin::tool::{Guest as Tool, ToolResult};
    use solana_core::pubkey::Pubkey;
    use solana_core::rpc::RpcTransport;
    use zeroclaw::plugin::logging::{
        log_record, LogLevel, PluginAction, PluginEvent, PluginOutcome,
    };

    struct PaymentWatch;

    const PLUGIN_NAME: &str = "payment-watch";
    const PLUGIN_VERSION: &str = env!("CARGO_PKG_VERSION");
    const TOOL_NAME: &str = "payment_watch";

    #[derive(serde::Deserialize)]
    struct ExecuteArgs {
        reference: String,
        recipient: String,
        amount: String,
        #[serde(default)]
        token: Option<String>,
        #[serde(rename = "__config", default)]
        config: HashMap<String, String>,
    }

    struct WakiTransport;
    impl RpcTransport for WakiTransport {
        fn post_json(&self, url: &str, body: &str) -> solana_core::Result<String> {
            let resp = waki::Client::new()
                .post(url)
                .header("Content-Type", "application/json")
                .body(body.as_bytes().to_vec())
                .send()
                .map_err(|e| solana_core::CoreError::Transport(e.to_string()))?;
            let bytes = resp
                .body()
                .map_err(|e| solana_core::CoreError::Transport(e.to_string()))?;
            String::from_utf8(bytes)
                .map_err(|e| solana_core::CoreError::Transport(e.to_string()))
        }
    }

    impl PluginInfo for PaymentWatch {
        fn plugin_name() -> String {
            PLUGIN_NAME.to_string()
        }
        fn plugin_version() -> String {
            PLUGIN_VERSION.to_string()
        }
    }

    impl Tool for PaymentWatch {
        fn name() -> String {
            TOOL_NAME.to_string()
        }

        fn description() -> String {
            "Check whether an expected Solana payment has arrived. Given the invoice's reference \
             pubkey, the recipient address, the expected amount, and the token (USDC, USDT, SOL, \
             or a mint), it inspects the chain and reports 'paid', 'underpaid', or 'pending' with \
             the transaction signature and payer. Read-only — it verifies the on-chain amount and \
             cannot be told a payment landed when it did not. Poll it from a cron SOP."
                .to_string()
        }

        fn parameters_schema() -> String {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "reference": { "type": "string", "description": "The unique Solana Pay reference pubkey embedded in the payment request." },
                    "recipient": { "type": "string", "description": "The merchant/recipient address that should be credited." },
                    "amount": { "type": "string", "description": "Expected amount in whole token units, e.g. \"25\"." },
                    "token": { "type": "string", "description": "Token: USDC, USDT, SOL, or a mint address. Defaults to SOL." }
                },
                "required": ["reference", "recipient", "amount"]
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

            let rpc_url = match parsed.config.get("rpc_url").map(String::as_str) {
                Some(u) if !u.is_empty() => u.to_string(),
                _ => {
                    emit(PluginAction::Fail, PluginOutcome::Failure, "no rpc_url configured");
                    return Ok(fail("no rpc_url configured — set `rpc_url` in this plugin's config section".into()));
                }
            };

            let reference = match Pubkey::from_base58(&parsed.reference) {
                Ok(p) => p,
                Err(e) => return Ok(fail(format!("invalid reference: {e}"))),
            };
            let recipient = match Pubkey::from_base58(&parsed.recipient) {
                Ok(p) => p,
                Err(e) => return Ok(fail(format!("invalid recipient: {e}"))),
            };
            let expected: f64 = match parsed.amount.trim().parse() {
                Ok(a) if a > 0.0 => a,
                _ => return Ok(fail(format!("amount '{}' must be a positive number", parsed.amount))),
            };
            let (mint, symbol) = match resolve_token(parsed.token.as_deref()) {
                Ok(t) => t,
                Err(e) => return Ok(fail(e)),
            };

            let query = WatchQuery { reference, recipient, mint, expected, symbol };

            emit(PluginAction::Query, PluginOutcome::Success, "checking for payment");
            match watch(&rpc_url, &WakiTransport, &query) {
                Ok(status) => {
                    let action = if status.is_paid() { PluginAction::Complete } else { PluginAction::Note };
                    emit(action, PluginOutcome::Success, "checked payment");
                    Ok(ToolResult { success: true, output: status.render(&parsed.reference), error: None })
                }
                Err(e) => {
                    emit(PluginAction::Fail, PluginOutcome::Failure, "watch failed");
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
                function_name: "payment_watch::tool::execute".to_string(),
                action,
                outcome: Some(outcome),
                duration_ms: None,
                attrs: None,
                message: message.to_string(),
            },
        );
    }

    export!(PaymentWatch);
}
