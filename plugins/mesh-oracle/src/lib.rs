//! ZeroClaw WIT tool plugin: `mesh_oracle`.
//!
//! Custody tier **T0 (Read)**. It reads the mesh of device attestations on-chain
//! and returns the manipulation-resistant aggregate (median, outliers dropped,
//! quorum required) — the value a prediction market settles on. It holds no key
//! and moves nothing. The mesh membership and thresholds are operator config the
//! request cannot change.
//!
//! All policy lives in [`oracle`] (pure, host-tested). This file is the thin
//! `#[cfg(target_family = "wasm")]` shim: it implements `RpcTransport` with the
//! blocking `waki` client and wires the logic to the `tool-plugin` world.

pub mod oracle;

#[cfg(target_family = "wasm")]
mod component {
    wit_bindgen::generate!({
        path: "../../wit/v0",
        world: "tool-plugin",
        features: ["plugins-wit-v0"],
    });

    use std::collections::HashMap;

    use crate::oracle::{read_oracle, OracleConfig};
    use exports::zeroclaw::plugin::plugin_info::Guest as PluginInfo;
    use exports::zeroclaw::plugin::tool::{Guest as Tool, ToolResult};
    use onca_core::rpc::RpcTransport;
    use zeroclaw::plugin::logging::{
        log_record, LogLevel, PluginAction, PluginEvent, PluginOutcome,
    };

    struct MeshOracle;

    const PLUGIN_NAME: &str = "mesh-oracle";
    const PLUGIN_VERSION: &str = env!("CARGO_PKG_VERSION");
    const TOOL_NAME: &str = "mesh_oracle";

    #[derive(serde::Deserialize)]
    struct ExecuteArgs {
        #[serde(default)]
        sensor: String,
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
            String::from_utf8(bytes).map_err(|e| onca_core::CoreError::Transport(e.to_string()))
        }
    }

    impl PluginInfo for MeshOracle {
        fn plugin_name() -> String {
            PLUGIN_NAME.to_string()
        }
        fn plugin_version() -> String {
            PLUGIN_VERSION.to_string()
        }
    }

    impl Tool for MeshOracle {
        fn name() -> String {
            TOOL_NAME.to_string()
        }

        fn description() -> String {
            "Read the current trusted value from the sensor mesh. It reads every configured node's \
             latest on-chain attestation, drops outliers (a lying or broken node), and returns the \
             median the mesh agrees on — the value a prediction market settles on. Read-only: it \
             holds no key and moves nothing. Optionally pass `sensor` to pick a sensor; the mesh \
             membership, tolerance, and quorum are operator config the request cannot change. If \
             fewer than quorum nodes agree, it reports no settlement rather than guessing."
                .to_string()
        }

        fn parameters_schema() -> String {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "sensor": { "type": "string", "description": "Optional sensor id, e.g. \"dht11-a\". Defaults to the configured sensor." }
                }
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

            let cfg = OracleConfig::from_section(&parsed.config);
            let sensor = if parsed.sensor.is_empty() { None } else { Some(parsed.sensor.as_str()) };

            emit(PluginAction::Invoke, PluginOutcome::Success, "reading mesh");
            match read_oracle(&rpc_url, &WakiTransport, &cfg, sensor) {
                Ok(res) => {
                    emit(PluginAction::Complete, PluginOutcome::Success, "settled mesh read");
                    let trusted = res
                        .agg
                        .inliers
                        .iter()
                        .map(|r| format!("{}…={}", &r.device[..r.device.len().min(4)], r.value))
                        .collect::<Vec<_>>()
                        .join(", ");
                    let output = format!("{}\ntrusted nodes: {}", res.summary(), trusted);
                    Ok(ToolResult { success: true, output, error: None })
                }
                Err(e) => {
                    emit(PluginAction::Reject, PluginOutcome::Failure, "mesh read refused");
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
                function_name: "mesh_oracle::tool::execute".to_string(),
                action,
                outcome: Some(outcome),
                duration_ms: None,
                attrs: None,
                message: message.to_string(),
            },
        );
    }

    export!(MeshOracle);
}
