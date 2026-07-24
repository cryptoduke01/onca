//! Mesh aggregation — the read side of the oracle.
//!
//! One sensor is not an oracle: whoever owns it can lie, and a prediction market
//! that settles on it is a market you can rob (the Polymarket-style single-source
//! manipulation). Onca's answer is a mesh: many independent DePIN nodes each
//! attest their own readings on-chain (each human-approved, bounds-checked, and
//! replay-guarded by `depin-attest`), and the market settles on the **median of
//! the mesh, with outliers dropped**. Moving the settlement then requires
//! corrupting a *majority* of independent nodes, not one box on one desk.
//!
//! This module is pure and host-tested: it parses attestation memos and computes
//! the manipulation-resistant aggregate. No I/O — `onca-oracle` supplies the
//! on-chain readings.

/// One node's latest attested reading, as read back from its on-chain memo.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeReading {
    pub device: String,
    pub value: f64,
    pub seq: u64,
    pub timestamp: u64,
}

/// Parse an `onca:attest s=<sensor> v=<value> u=<unit> seq=<n> t=<ts>` memo into
/// `(sensor, value, seq, timestamp)`. Returns `None` for anything that is not one
/// of our attestations, so a reader can scan a device's history and ignore noise.
pub fn parse_attest(memo: &str) -> Option<(String, f64, u64, u64)> {
    if !memo.starts_with("onca:attest") {
        return None;
    }
    let (mut sensor, mut value, mut seq, mut ts) = (None, None, None, None);
    for tok in memo.split_whitespace().skip(1) {
        if let Some((k, v)) = tok.split_once('=') {
            match k {
                "s" => sensor = Some(v.to_string()),
                "v" => value = v.parse::<f64>().ok(),
                "seq" => seq = v.parse::<u64>().ok(),
                "t" => ts = v.parse::<u64>().ok(),
                _ => {}
            }
        }
    }
    Some((sensor?, value?, seq?, ts?))
}

/// The value a market settles on, plus which nodes were trusted and which were
/// rejected — so the settlement is auditable, not a black box.
#[derive(Debug, Clone, PartialEq)]
pub struct Aggregate {
    /// The oracle value: the median of the inlier readings. `NaN` if no quorum.
    pub value: f64,
    /// Nodes whose readings agreed with the mesh and were counted.
    pub inliers: Vec<NodeReading>,
    /// Nodes rejected as outliers (a lying or broken sensor).
    pub outliers: Vec<NodeReading>,
    /// True when at least `quorum` nodes agreed. A market must refuse to settle
    /// on fewer, never guess.
    pub has_quorum: bool,
}

fn median(values: &[f64]) -> f64 {
    if values.is_empty() {
        return f64::NAN;
    }
    let mut v = values.to_vec();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = v.len();
    if n % 2 == 1 {
        v[n / 2]
    } else {
        (v[n / 2 - 1] + v[n / 2]) / 2.0
    }
}

/// Aggregate the mesh into one manipulation-resistant value.
///
/// A reading is an outlier if it sits more than `tolerance` from the mesh median
/// (a lying node reporting 999 against a mesh at 23). The oracle value is the
/// median of the inliers, so a *minority* of corrupted nodes changes nothing —
/// they land in `outliers` and are dropped. `quorum` is the minimum number of
/// agreeing nodes required to settle at all.
pub fn aggregate(readings: &[NodeReading], tolerance: f64, quorum: usize) -> Aggregate {
    let seed = median(&readings.iter().map(|r| r.value).collect::<Vec<_>>());
    let (mut inliers, mut outliers) = (Vec::new(), Vec::new());
    for r in readings {
        if seed.is_finite() && (r.value - seed).abs() > tolerance {
            outliers.push(r.clone());
        } else {
            inliers.push(r.clone());
        }
    }
    let value = median(&inliers.iter().map(|r| r.value).collect::<Vec<_>>());
    let has_quorum = inliers.len() >= quorum;
    Aggregate { value, inliers, outliers, has_quorum }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(device: &str, value: f64) -> NodeReading {
        NodeReading { device: device.into(), value, seq: 1, timestamp: 0 }
    }

    #[test]
    fn parses_an_attestation_memo() {
        let (s, v, seq, t) = parse_attest("onca:attest s=dht11-a v=23.4 u=C seq=2 t=1784906764").unwrap();
        assert_eq!(s, "dht11-a");
        assert_eq!(v, 23.4);
        assert_eq!(seq, 2);
        assert_eq!(t, 1784906764);
        assert!(parse_attest("hello world").is_none());
    }

    #[test]
    fn honest_mesh_settles_on_the_median() {
        let mesh = [r("a", 23.4), r("b", 23.6), r("c", 23.1)];
        let agg = aggregate(&mesh, 5.0, 3);
        assert_eq!(agg.value, 23.4);
        assert!(agg.has_quorum);
        assert!(agg.outliers.is_empty());
    }

    /// The anti-manipulation guarantee: one node lying wildly (or broken) cannot
    /// move the settlement — it is dropped as an outlier and the median holds.
    #[test]
    fn one_lying_node_cannot_move_the_settlement() {
        let mesh = [r("a", 23.4), r("b", 23.6), r("c", 23.1), r("attacker", 999.0)];
        let agg = aggregate(&mesh, 5.0, 3);
        assert_eq!(agg.value, 23.4); // still the honest median
        assert_eq!(agg.outliers.len(), 1);
        assert_eq!(agg.outliers[0].device, "attacker");
        assert!(agg.has_quorum);
    }

    /// A cold-side lie is caught the same way as a hot-side one.
    #[test]
    fn a_low_liar_is_also_rejected() {
        let mesh = [r("a", 23.4), r("b", 23.6), r("c", 23.1), r("attacker", -50.0)];
        let agg = aggregate(&mesh, 5.0, 3);
        assert_eq!(agg.value, 23.4);
        assert_eq!(agg.outliers[0].device, "attacker");
    }

    #[test]
    fn refuses_to_settle_below_quorum() {
        let mesh = [r("a", 23.4), r("b", 23.6)];
        let agg = aggregate(&mesh, 5.0, 3);
        assert!(!agg.has_quorum); // a market must not settle on two nodes
    }
}
