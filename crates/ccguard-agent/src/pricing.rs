/// Approximate USD per 1M tokens (input, output) by model family. Rough estimate for cost display.
pub fn price_per_mtok(model: &str) -> (f64, f64) {
    let m = model.to_ascii_lowercase();
    if m.contains("opus") {
        (15.0, 75.0)
    } else if m.contains("haiku") {
        (1.0, 5.0)
    } else {
        // sonnet and unknown default
        (3.0, 15.0)
    }
}

pub fn estimate_cost(model: &str, tokens_in: i64, tokens_out: i64) -> f64 {
    let (pin, pout) = price_per_mtok(model);
    (tokens_in as f64 / 1_000_000.0) * pin + (tokens_out as f64 / 1_000_000.0) * pout
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opus_cost_matches_table() {
        // 1M in @ $15 + 1M out @ $75 = $90
        assert!((estimate_cost("claude-opus-4-8", 1_000_000, 1_000_000) - 90.0).abs() < 1e-9);
    }

    #[test]
    fn unknown_model_defaults_to_sonnet_pricing() {
        assert_eq!(price_per_mtok("mystery"), (3.0, 15.0));
    }
}
