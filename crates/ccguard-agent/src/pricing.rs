/// Approximate USD per 1M tokens (input, output) by model family. Rough estimate for cost display.
/// Rates are uncached input + output for Claude 4.x (Opus 4.x $5/$25, Sonnet 4.6 $3/$15,
/// Haiku 4.5 $1/$5). NOTE: cached-input tokens (cache_read ~0.1x, cache_creation ~1.25x) are
/// not yet captured by the agent, so input cost is a floor for cache-heavy tools like Claude Code.
pub fn price_per_mtok(model: &str) -> (f64, f64) {
    let m = model.to_ascii_lowercase();
    if m.contains("opus") {
        (5.0, 25.0)
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
        // Opus 4.x: 1M in @ $5 + 1M out @ $25 = $30
        assert!((estimate_cost("claude-opus-4-8", 1_000_000, 1_000_000) - 30.0).abs() < 1e-9);
    }

    #[test]
    fn unknown_model_defaults_to_sonnet_pricing() {
        assert_eq!(price_per_mtok("mystery"), (3.0, 15.0));
    }
}
