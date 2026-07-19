//! Time-Weighted Reputation Decay Module (§2.3)

pub const HALF_LIFE_MS: u64 = 30 * 86_400 * 1000; // 30 days in milliseconds

/// Calculates decayed reputation values based on exponential half-life decay.
/// `decay_ratio = 0.5 ^ ((now_ms - last_update_ms) / HALF_LIFE_MS)`
pub fn calculate_decay(
    weighted_sum: u64,
    total_weight: u64,
    last_update_ms: u64,
    now_ms: u64,
) -> (u64, u64) {
    if now_ms <= last_update_ms || total_weight == 0 {
        return (weighted_sum, total_weight);
    }

    let elapsed_ms = now_ms - last_update_ms;
    let elapsed_periods = elapsed_ms as f64 / HALF_LIFE_MS as f64;
    let decay_ratio = 0.5_f64.powf(elapsed_periods);

    let decayed_weighted_sum = (weighted_sum as f64 * decay_ratio).round() as u64;
    let decayed_total_weight = (total_weight as f64 * decay_ratio).round() as u64;

    (decayed_weighted_sum, decayed_total_weight)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero_elapsed_returns_same() {
        let (ws, tw) = calculate_decay(1000, 10, 100, 100);
        assert_eq!(ws, 1000);
        assert_eq!(tw, 10);
    }

    #[test]
    fn test_half_life_decay_halves_values() {
        let now = 100 + HALF_LIFE_MS;
        let (ws, tw) = calculate_decay(1000, 10, 100, now);
        assert_eq!(ws, 500);
        assert_eq!(tw, 5);
    }

    #[test]
    fn test_two_half_lives_quarters_values() {
        let now = 100 + HALF_LIFE_MS * 2;
        let (ws, tw) = calculate_decay(1000, 20, 100, now);
        assert_eq!(ws, 250);
        assert_eq!(tw, 5);
    }
}
