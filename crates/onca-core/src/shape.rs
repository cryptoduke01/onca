//! Output shaping. A raw `getProgramAccounts` or DAS response is tens of
//! kilobytes; dropping that into an agent's context nukes the window and costs
//! the operator money on every call. These helpers turn RPC JSON into the small
//! number of tokens the model actually needs.

/// Render a base-unit integer amount as a decimal string with `decimals` places,
/// trimming trailing zeros. `render_amount(25_000_000, 6)` -> `"25"`,
/// `render_amount(1_500_000, 6)` -> `"1.5"`.
pub fn render_amount(base_units: u128, decimals: u8) -> String {
    if decimals == 0 {
        return base_units.to_string();
    }
    // 10^39 overflows u128. A mint reporting decimals that large is degenerate;
    // fall back to the raw integer rather than panicking inside `execute`.
    let divisor = match 10u128.checked_pow(decimals as u32) {
        Some(d) => d,
        None => return base_units.to_string(),
    };
    let whole = base_units / divisor;
    let frac = base_units % divisor;
    if frac == 0 {
        return whole.to_string();
    }
    let mut frac_str = format!("{frac:0width$}", width = decimals as usize);
    while frac_str.ends_with('0') {
        frac_str.pop();
    }
    format!("{whole}.{frac_str}")
}

/// Parse a human decimal string (e.g. `"25"`, `"1.5"`) into base units given
/// `decimals`. Rejects more fractional digits than the mint supports rather than
/// silently truncating value.
pub fn parse_amount(input: &str, decimals: u8) -> Result<u128, String> {
    let input = input.trim();
    if input.is_empty() {
        return Err("empty amount".into());
    }
    let (whole, frac) = match input.split_once('.') {
        Some((w, f)) => (w, f),
        None => (input, ""),
    };
    if frac.len() > decimals as usize {
        return Err(format!(
            "amount has {} fractional digits but mint supports {}",
            frac.len(),
            decimals
        ));
    }
    let whole: u128 = if whole.is_empty() {
        0
    } else {
        whole.parse().map_err(|_| "invalid whole part".to_string())?
    };
    let frac_padded = format!("{frac:0<width$}", width = decimals as usize);
    let frac_val: u128 = if frac_padded.is_empty() {
        0
    } else {
        frac_padded
            .parse()
            .map_err(|_| "invalid fractional part".to_string())?
    };
    let divisor = 10u128
        .checked_pow(decimals as u32)
        .ok_or_else(|| format!("unsupported decimals: {decimals}"))?;
    whole
        .checked_mul(divisor)
        .and_then(|w| w.checked_add(frac_val))
        .ok_or_else(|| "amount overflow".into())
}

/// Abbreviate an address for human-readable summaries: `7xKX…gAsU`.
pub fn abbrev(addr: &str) -> String {
    let n = addr.chars().count();
    if n <= 10 {
        return addr.to_string();
    }
    let head: String = addr.chars().take(4).collect();
    let tail: String = addr.chars().skip(n - 4).collect();
    format!("{head}…{tail}")
}

/// Hard cap on a text field so a hostile or oversized value can never blow the
/// context. Truncates on a char boundary and marks the cut.
pub fn clamp_text(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max_chars.saturating_sub(1)).collect();
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_amounts() {
        assert_eq!(render_amount(25_000_000, 6), "25");
        assert_eq!(render_amount(1_500_000, 6), "1.5");
        assert_eq!(render_amount(1, 6), "0.000001");
        assert_eq!(render_amount(42, 0), "42");
    }

    #[test]
    fn parses_amounts() {
        assert_eq!(parse_amount("25", 6).unwrap(), 25_000_000);
        assert_eq!(parse_amount("1.5", 6).unwrap(), 1_500_000);
        assert_eq!(parse_amount("0.000001", 6).unwrap(), 1);
    }

    #[test]
    fn rejects_over_precise_amount() {
        // 7 fractional digits into a 6-decimal mint must fail, not truncate.
        assert!(parse_amount("1.1234567", 6).is_err());
    }

    #[test]
    fn round_trips() {
        for (units, dec) in [(25_000_000u128, 6u8), (1, 9), (123_456, 4)] {
            let s = render_amount(units, dec);
            assert_eq!(parse_amount(&s, dec).unwrap(), units);
        }
    }

    #[test]
    fn abbreviates() {
        assert_eq!(abbrev("7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU"), "7xKX…gAsU");
        assert_eq!(abbrev("short"), "short");
    }

    #[test]
    fn clamps() {
        assert_eq!(clamp_text("hello", 10), "hello");
        assert_eq!(clamp_text("hello world", 5), "hell…");
    }

    #[test]
    fn absurd_decimals_do_not_panic() {
        // A hostile/malformed mint could report huge decimals. Neither path may
        // panic on the 10^n overflow.
        assert_eq!(render_amount(1_000_000, 255), "1000000");
        assert!(parse_amount("1", 255).is_err());
    }
}
