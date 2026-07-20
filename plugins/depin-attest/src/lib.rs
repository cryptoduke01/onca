//! ZeroClaw WIT tool plugin: `depin_attest`.
//!
//! Custody tier **T1 (Build)**. Given a sensor reading from the host's hardware
//! tools, it builds an unsigned Solana transaction that records a signed
//! attestation on-chain (an SPL Memo), with a monotonic-sequence replay guard
//! and operator-set reading bounds. The host or a person signs it. This turns a
//! ZeroClaw device — a Raspberry Pi, or a laptop with an ESP32 sensor over
//! MQTT/serial — into a Solana-reporting DePIN node. It holds no key and moves
//! no funds.
//!
//! All policy lives in [`attest`] (pure, host-tested). This file is the thin
//! `#[cfg(target_family = "wasm")]` shim: it implements `RpcTransport` with the
//! blocking `waki` client and wires the logic to the `tool-plugin` world.
//!
//! Build:  rustup target add wasm32-wasip2
//!         cargo build --target wasm32-wasip2 --release

pub mod attest;

#[cfg(target_family = "wasm")]
mod component {
    wit_bindgen::generate!({
        path: "../../wit/v0",
        world: "tool-plugin",
        features: ["plugins-wit-v0"],
    });

    use std::collections::HashMap;

    use crate::attest::{build_attestation, AttestArgs, AttestConfig};
    use exports::zeroclaw::plugin::plugin_info::Guest as PluginInfo;
    use exports::zeroclaw::plugin::tool::{Guest as Tool, ToolResult};
    use onca_core::rpc::RpcTransport;
    use zeroclaw::plugin::logging::{
        log_record, LogLevel, PluginAction, PluginEvent, PluginOutcome,
    };

    struct DepinAttest;

    const PLUGIN_NAME: &str = "depin-attest";
    const PLUGIN_VERSION: &str = env!("CARGO_PKG_VERSION");
    const TOOL_NAME: &str = "depin_attest";

    #[derive(serde::Deserialize)]
    struct ExecuteArgs {
        sensor: String,
        reading: f64,
        #[serde(default)]
        unit: String,
        seq: u64,
        #[serde(default)]
        timestamp: u64,
        #[serde(rename = "__config", default)]
        config: HashMap<String, String>,
    }

    struct WakiTransport;
    impl RpcTransport for WakiTransport {
        fn post_json(&self, url: &str, body: &str) -> onca_core::Result<String> {
            let resp = waki::Client::new()
                .post(url)
                .header("Content-Type", "application/json")
                .body(body.as_bytes().to_vec())
                .send()
                .map_err(|e| onca_core::CoreError::Transport(e.to_string()))?;
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

    impl PluginInfo for DepinAttest {
        fn plugin_name() -> String {
            PLUGIN_NAME.to_string()
        }
        fn plugin_version() -> String {
            PLUGIN_VERSION.to_string()
        }
    }

    impl Tool for DepinAttest {
        fn name() -> String {
            TOOL_NAME.to_string()
        }

        fn description() -> String {
            "Record a hardware sensor reading on Solana as a signed attestation. Given a sensor \
             id, a numeric reading, a unit, and a monotonic sequence number, it returns an \
             unsigned transaction (base64) that writes the reading on-chain as a memo, for the \
             host or a person to sign. It never moves funds and holds no key. A replay guard \
             (the sequence must increase) and operator-set reading bounds are enforced and cannot \
             be overridden by the request."
                .to_string()
        }

        fn parameters_schema() -> String {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "sensor": { "type": "string", "description": "Sensor id, e.g. \"bme280-a\"." },
                    "reading": { "type": "number", "description": "The numeric reading." },
                    "unit": { "type": "string", "description": "Unit, e.g. \"C\", \"%\", \"ppm\"." },
                    "seq": { "type": "integer", "description": "Monotonic sequence number; must exceed the last attested." },
                    "timestamp": { "type": "integer", "description": "Unix seconds of the reading." }
                },
                "required": ["sensor", "reading", "seq"]
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

            let cfg = AttestConfig::from_section(&parsed.config);
            let attest_args = AttestArgs {
                sensor: parsed.sensor,
                reading: parsed.reading,
                unit: parsed.unit,
                seq: parsed.seq,
                timestamp: parsed.timestamp,
            };

            emit(PluginAction::Invoke, PluginOutcome::Success, "building attestation");
            match build_attestation(&rpc_url, &WakiTransport, &cfg, &attest_args) {
                Ok(att) => {
                    emit(PluginAction::Complete, PluginOutcome::Success, "built attestation");
                    let output = format!("{}\ntx(base64): {}\nmemo: {}", att.summary, att.base64, att.memo);
                    Ok(ToolResult { success: true, output, error: None })
                }
                Err(e) => {
                    emit(PluginAction::Reject, PluginOutcome::Failure, "refused attestation");
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
                function_name: "depin_attest::tool::execute".to_string(),
                action,
                outcome: Some(outcome),
                duration_ms: None,
                attrs: None,
                message: message.to_string(),
            },
        );
    }

    export!(DepinAttest);
}
