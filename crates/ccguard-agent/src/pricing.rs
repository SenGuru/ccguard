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

/// Full cost including cached-input tokens. Cache reads bill at ~0.1x the input rate,
/// cache writes (creation) at ~1.25x (Claude's default 5-min TTL).
pub fn estimate_cost_full(
    model: &str,
    tokens_in: i64,
    tokens_out: i64,
    cache_read: i64,
    cache_creation: i64,
) -> f64 {
    let (pin, pout) = price_per_mtok(model);
    let m = 1_000_000.0;
    (tokens_in as f64 / m) * pin
        + (tokens_out as f64 / m) * pout
        + (cache_read as f64 / m) * (pin * 0.1)
        + (cache_creation as f64 / m) * (pin * 1.25)
}

/// Cost from uncached input + output only (no cache tokens).
pub fn estimate_cost(model: &str, tokens_in: i64, tokens_out: i64) -> f64 {
    estimate_cost_full(model, tokens_in, tokens_out, 0, 0)
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

    #[test]
    fn cache_tokens_priced_at_reduced_rates() {
        // opus input $5/M → cache_read @ 0.1x = $0.50/M, cache_creation @ 1.25x = $6.25/M.
        // 1M cache_read + 1M cache_creation = 0.50 + 6.25 = $6.75
        let c = estimate_cost_full("claude-opus-4-8", 0, 0, 1_000_000, 1_000_000);
        assert!((c - 6.75).abs() < 1e-9);
    }
}
