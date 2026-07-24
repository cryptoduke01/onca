//! Pure core for `mesh-oracle` (custody **T0, Read**). No wasm; the only I/O is
//! behind the `RpcTransport` seam, so the whole thing is host-tested with a mock.
//!
//! It reads every mesh node's latest on-chain `onca:attest` and settles on
//! `onca-core`'s manipulation-resistant aggregate (median, outliers dropped,
//! quorum required). The mesh membership, tolerance, and quorum are **operator
//! config** — the model cannot add a device or loosen the threshold to move the
//! number. This is the read half of the oracle; `depin-attest` is the write half.

use std::collections::HashMap;

use onca_core::mesh::{aggregate, parse_attest, Aggregate, NodeReading};
use onca_core::rpc::{RpcClient, RpcTransport};
use serde_json::{json, Value};

/// Operator-set mesh policy, read from the plugin's jailed config section.
#[derive(Debug, Clone)]
pub struct OracleConfig {
    /// The mesh: the device pubkeys whose attestations are trusted inputs.
    pub devices: Vec<String>,
    /// Default sensor id if the request does not name one.
    pub sensor: String,
    /// Max absolute distance from the mesh median before a reading is an outlier.
    pub tolerance: f64,
    /// Minimum agreeing nodes required to settle.
    pub quorum: usize,
}

impl OracleConfig {
    pub fn from_section(s: &HashMap<String, String>) -> Self {
        OracleConfig {
            devices: s
                .get("devices")
                .map(|v| v.split(',').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect())
                .unwrap_or_default(),
            sensor: s.get("sensor").cloned().unwrap_or_else(|| "dht11-a".to_string()),
            tolerance: s.get("tolerance").and_then(|v| v.trim().parse().ok()).unwrap_or(5.0),
            quorum: s.get("quorum").and_then(|v| v.trim().parse().ok()).unwrap_or(3),
        }
    }
}

/// The settled oracle read.
#[derive(Debug, Clone)]
pub struct OracleResult {
    pub sensor: String,
    pub agg: Aggregate,
    pub nodes_read: usize,
    pub quorum: usize,
}

impl OracleResult {
    /// Compact line the agent relays (never a JSON blob).
    pub fn summary(&self) -> String {
        if self.agg.has_quorum {
            let dropped = if self.agg.outliers.is_empty() {
                String::new()
            } else {
                format!(", {} outlier(s) rejected", self.agg.outliers.len())
            };
            format!(
                "{} = {} (median of {} of {} nodes{})",
                self.sensor, fmt(self.agg.value), self.agg.inliers.len(), self.nodes_read, dropped
            )
        } else {
            format!(
                "no settlement for {}: only {} of {} nodes agreed, quorum is {}",
                self.sensor, self.agg.inliers.len(), self.nodes_read, self.quorum
            )
        }
    }
}

fn fmt(v: f64) -> String {
    let s = format!("{v:.2}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// One node's latest attested reading for `sensor`, read from its signature
/// history. The memo may carry a `[len] ` prefix, so we scan to `onca:attest`.
fn latest_reading<T: RpcTransport>(
    client: &RpcClient<T>,
    device: &str,
    sensor: &str,
) -> Result<Option<NodeReading>, String> {
    // One call per node. Public devnet RPC rate-limits bursts, so a busy mesh
    // wants a real endpoint (Helius/Triton/QuickNode) in `rpc_url` — operators
    // run their own anyway. An empty/rate-limited answer just drops that node;
    // quorum protects the settlement.
    let res = client
        .call("getSignaturesForAddress", json!([device, {"limit": 25}]))
        .map_err(|e| e.to_string())?;
    let Some(entries) = res.as_array() else { return Ok(None) };
    for entry in entries {
        let memo = entry.get("memo").and_then(Value::as_str).unwrap_or("");
        let Some(start) = memo.find("onca:attest") else { continue };
        if let Some((s, value, seq, timestamp)) = parse_attest(&memo[start..]) {
            if s == sensor {
                return Ok(Some(NodeReading { device: device.to_string(), value, seq, timestamp }));
            }
        }
    }
    Ok(None)
}

/// Read the mesh and settle. `sensor_override` (from the request) may pick a
/// sensor; membership and thresholds always come from operator config.
pub fn read_oracle<T: RpcTransport>(
    rpc_url: &str,
    transport: &T,
    cfg: &OracleConfig,
    sensor_override: Option<&str>,
) -> Result<OracleResult, String> {
    if cfg.devices.is_empty() {
        return Err("no mesh devices configured — set `devices` in this plugin's config".into());
    }
    let sensor = sensor_override
        .filter(|s| !s.is_empty())
        .unwrap_or(&cfg.sensor)
        .to_string();

    let client = RpcClient::new(rpc_url, transport);
    let mut readings = Vec::new();
    for device in &cfg.devices {
        match latest_reading(&client, device, &sensor) {
            Ok(Some(r)) => readings.push(r),
            Ok(None) => {}
            Err(e) => return Err(format!("mesh read failed for {device}: {e}")),
        }
    }
    let agg = aggregate(&readings, cfg.tolerance, cfg.quorum);
    Ok(OracleResult { sensor, agg, nodes_read: readings.len(), quorum: cfg.quorum })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mock node: answers `getSignaturesForAddress` per device with one memo.
    struct MockRpc {
        memos: HashMap<String, String>,
    }
    impl RpcTransport for MockRpc {
        fn post_json(&self, _url: &str, body: &str) -> onca_core::Result<String> {
            let req: Value = serde_json::from_str(body).unwrap();
            let device = req["params"][0].as_str().unwrap_or("");
            let memo = self.memos.get(device).cloned().unwrap_or_default();
            let result = if memo.is_empty() {
                json!([])
            } else {
                json!([{"signature": "sig", "memo": memo, "err": null}])
            };
            Ok(json!({"jsonrpc": "2.0", "id": 1, "result": result}).to_string())
        }
    }

    fn cfg(devices: &[&str], quorum: usize) -> OracleConfig {
        OracleConfig {
            devices: devices.iter().map(|s| s.to_string()).collect(),
            sensor: "dht11-a".into(),
            tolerance: 5.0,
            quorum,
        }
    }

    /// The anti-manipulation guarantee, end to end through the RPC read: three
    /// honest nodes and one adversary at 999 (memo carries a `[len] ` prefix).
    /// The oracle drops the liar and settles on the honest median.
    #[test]
    fn oracle_rejects_a_lying_node() {
        let memos = HashMap::from([
            ("dev1".to_string(), "onca:attest s=dht11-a v=23.4 u=C seq=2 t=1".to_string()),
            ("dev2".to_string(), "onca:attest s=dht11-a v=23.6 u=C seq=2 t=1".to_string()),
            ("dev3".to_string(), "onca:attest s=dht11-a v=23.1 u=C seq=2 t=1".to_string()),
            ("dev4".to_string(), "[51] onca:attest s=dht11-a v=999 u=C seq=2 t=1".to_string()),
        ]);
        let rpc = MockRpc { memos };
        let res = read_oracle("https://rpc", &rpc, &cfg(&["dev1", "dev2", "dev3", "dev4"], 3), None).unwrap();
        assert!(res.agg.has_quorum);
        assert_eq!(res.agg.value, 23.4);
        assert_eq!(res.agg.outliers.len(), 1);
        assert_eq!(res.agg.outliers[0].device, "dev4");
        assert!(res.summary().contains("23.4"));
        assert!(res.summary().contains("rejected"));
    }

    #[test]
    fn no_settlement_below_quorum() {
        let memos = HashMap::from([
            ("dev1".to_string(), "onca:attest s=dht11-a v=23.4 u=C seq=1 t=1".to_string()),
        ]);
        let rpc = MockRpc { memos };
        let res = read_oracle("https://rpc", &rpc, &cfg(&["dev1", "dev2"], 3), None).unwrap();
        assert!(!res.agg.has_quorum);
        assert!(res.summary().contains("no settlement"));
    }

    #[test]
    fn no_devices_is_an_error() {
        let rpc = MockRpc { memos: HashMap::new() };
        assert!(read_oracle("https://rpc", &rpc, &cfg(&[], 3), None).is_err());
    }
}
