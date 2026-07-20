//! ZeroClaw WIT tool plugin: `token_risk_check`.
//!
//! Custody tier **T0 (Read)**. Given a mint address, it reads the chain and
//! returns a red/amber/green safety verdict: mint/freeze authority, Token-2022
//! honeypot extensions (permanent delegate, transfer hook, transfer fee,
//! non-transferable, default-frozen), and holder concentration. It never signs,
//! sends, or holds anything but an RPC key. It makes every *other* action safer.
//!
//! All analysis lives in [`risk`] (pure, host-tested with canned RPC fixtures).
//! This file is the thin `#[cfg(target_family = "wasm")]` shim: it implements
//! the `RpcTransport` with the blocking `waki` client and wires it to the
//! `tool-plugin` world.
//!
//! Build:  rustup target add wasm32-wasip2
//!         cargo build --target wasm32-wasip2 --release

pub mod risk;

#[cfg(target_family = "wasm")]
mod component {
    wit_bindgen::generate!({
        path: "../../wit/v0",
        world: "tool-plugin",
        features: ["plugins-wit-v0"],
    });

    use std::collections::HashMap;

    use crate::risk::analyze;
    use exports::zeroclaw::plugin::plugin_info::Guest as PluginInfo;
    use exports::zeroclaw::plugin::tool::{Guest as Tool, ToolResult};
    use onca_core::rpc::RpcTransport;
    use zeroclaw::plugin::logging::{
        log_record, LogLevel, PluginAction, PluginEvent, PluginOutcome,
    };

    struct TokenRiskCheck;

    const PLUGIN_NAME: &str = "token-risk-check";
    const PLUGIN_VERSION: &str = env!("CARGO_PKG_VERSION");
    const TOOL_NAME: &str = "token_risk_check";

    #[derive(serde::Deserialize)]
    struct ExecuteArgs {
        mint: String,
        #[serde(rename = "__config", default)]
        config: HashMap<String, String>,
    }

    /// Blocking `wasi:http` transport. TLS is performed host-side; we only build
    /// the request and read the response. The URL (which may embed an API key)
    /// is never logged.
    struct WakiTransport;
    impl RpcTransport for WakiTransport {
        fn post_json(&self, url: &str, body: &str) -> onca_core::Result<String> {
            let resp = waki::Client::new()
                .post(url)
                .header("Content-Type", "application/json")
                .body(body.as_bytes().to_vec())
                .send()
                .map_err(|e| onca_core::CoreError::Transport(e.to_string()))?;
            // Surface a non-2xx (rate limit, auth, node error) as a clean
            // transport error instead of letting an HTML error page fall through
            // as "not JSON". The status code carries no secret; the URL does, so
            // it is never included.
            let status = resp.status_code();
            if !(200..300).contains(&status) {
                return Err(onca_core::CoreError::Transport(format!("RPC returned HTTP {status}")));
            }
            let bytes = resp
                .body()
                .map_err(|e| onca_core::CoreError::Transport(e.to_string()))?;
            String::from_utf8(bytes)
                .map_err(|e| onca_core::CoreError::Transport(e.to_string()))
        }
    }

    impl PluginInfo for TokenRiskCheck {
        fn plugin_name() -> String {
            PLUGIN_NAME.to_string()
        }
        fn plugin_version() -> String {
            PLUGIN_VERSION.to_string()
        }
    }

    impl Tool for TokenRiskCheck {
        fn name() -> String {
            TOOL_NAME.to_string()
        }

        fn description() -> String {
            "Check whether a Solana token (SPL or Token-2022) is safe to hold or trade. Given a \
             mint address, returns a red/amber/green risk verdict with reasons: mint and freeze \
             authority, Token-2022 honeypot extensions (permanent delegate, transfer hook, \
             transfer fee, non-transferable, default-frozen), and holder concentration. \
             Read-only — it never moves funds. Use it before acting on any unfamiliar token."
                .to_string()
        }

        fn parameters_schema() -> String {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "mint": { "type": "string", "description": "The token mint address to check (base58)." }
                },
                "required": ["mint"]
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

            // The RPC URL is operator config, never hardcoded. It may hold a key.
            let rpc_url = match parsed.config.get("rpc_url").map(String::as_str) {
                Some(u) if !u.is_empty() => u.to_string(),
                _ => {
                    emit(PluginAction::Fail, PluginOutcome::Failure, "no rpc_url configured");
                    return Ok(fail(
                        "no rpc_url configured — set `rpc_url` in this plugin's config section".into(),
                    ));
                }
            };

            emit(PluginAction::Query, PluginOutcome::Success, "checking mint");
            match analyze(&rpc_url, &WakiTransport, &parsed.mint) {
                Ok(report) => {
                    emit(PluginAction::Complete, PluginOutcome::Success, "assessed mint");
                    Ok(ToolResult { success: true, output: report.render(), error: None })
                }
                Err(e) => {
                    emit(PluginAction::Fail, PluginOutcome::Failure, "assessment failed");
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
                function_name: "token_risk_check::tool::execute".to_string(),
                action,
                outcome: Some(outcome),
                duration_ms: None,
                attrs: None,
                message: message.to_string(),
            },
        );
    }

    export!(TokenRiskCheck);
}
