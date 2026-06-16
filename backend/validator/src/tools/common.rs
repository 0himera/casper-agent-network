use serde_json::{json, Value};

pub const REASON_PASSED: &str = "passed";
pub const REASON_FAILED: &str = "failed";
pub const REASON_MISSING: &str = "missing_data";
pub const REASON_MALFORMED: &str = "malformed_fixture";

pub fn contains_ci(text: &str, needle: &str) -> bool {
    text.to_ascii_lowercase()
        .contains(&needle.to_ascii_lowercase())
}

pub fn contains_id(text: &str, id: &str) -> bool {
    contains_ci(text, id)
}

pub fn parse_numbers(text: &str) -> Vec<f64> {
    let mut numbers = Vec::new();
    let mut current = String::new();

    for ch in text.chars() {
        if ch.is_ascii_digit() || ch == '.' || ch == ',' {
            current.push(ch);
        } else if !current.is_empty() {
            if let Some(n) = parse_number_token(&current) {
                numbers.push(n);
            }
            current.clear();
        }
    }

    if !current.is_empty() {
        if let Some(n) = parse_number_token(&current) {
            numbers.push(n);
        }
    }

    numbers
}

fn parse_number_token(token: &str) -> Option<f64> {
    let cleaned: String = token
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    if cleaned.is_empty() {
        return None;
    }
    cleaned.parse().ok()
}

pub fn parse_percent(text: &str) -> Option<f64> {
    let lower = text.to_ascii_lowercase();

    for token in lower.split_whitespace() {
        if let Some(num_str) = token.strip_suffix('%') {
            if let Ok(v) = num_str.parse::<f64>() {
                return Some(v / 100.0);
            }
        }
    }

    for (i, _window) in lower.match_indices("percent") {
        let prefix = &lower[..i];
        if let Some(num) = prefix
            .split_whitespace()
            .last()
            .and_then(|s| s.parse::<f64>().ok())
        {
            return Some(num / 100.0);
        }
    }

    if lower.contains("15%") {
        return Some(0.15);
    }
    if lower.contains("10%") {
        return Some(0.10);
    }

    None
}

pub fn find_percent_near(text: &str, keyword: &str) -> Option<f64> {
    let lower = text.to_ascii_lowercase();
    let keyword_lower = keyword.to_ascii_lowercase();

    if let Some(idx) = lower.find(&keyword_lower) {
        let window = &text[idx.saturating_sub(30)..text.len().min(idx + 60)];
        if let Some(p) = parse_percent(window) {
            return Some(p);
        }
        for num in parse_numbers(window) {
            if (0.0..=1.0).contains(&num) {
                return Some(num);
            }
            if num > 1.0 && num <= 100.0 {
                return Some(num / 100.0);
            }
        }
    }

    parse_percent(text)
}

pub fn within_tolerance(actual: f64, expected: f64, pct: f64) -> bool {
    if expected == 0.0 {
        return actual.abs() < f64::EPSILON;
    }
    ((actual - expected).abs() / expected) <= pct
}

pub fn within_abs(actual: f64, expected: f64, abs_tol: f64) -> bool {
    (actual - expected).abs() <= abs_tol
}

pub fn median(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = sorted.len() / 2;
    if sorted.len() % 2 == 0 {
        Some((sorted[mid - 1] + sorted[mid]) / 2.0)
    } else {
        Some(sorted[mid])
    }
}

pub fn weighted_median(prices_weights: &[(f64, f64)]) -> Option<f64> {
    if prices_weights.is_empty() {
        return None;
    }
    let mut sorted = prices_weights.to_vec();
    sorted.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    let total: f64 = sorted.iter().map(|(_, w)| w).sum();
    if total <= 0.0 {
        return None;
    }
    let half = total / 2.0;
    let mut cumulative = 0.0;
    for &(price, weight) in &sorted {
        cumulative += weight;
        if cumulative >= half {
            return Some(price);
        }
    }
    sorted.last().map(|(p, _)| *p)
}

pub fn result(tool: &str, ok: bool, reason: &str, expected: Value, found: Value, checks: Value) -> crate::types::ToolResult {
    crate::types::ToolResult {
        tool: tool.to_string(),
        ok,
        details: json!({
            "reason": reason,
            "expected": expected,
            "found": found,
            "checks": checks,
        }),
    }
}

pub fn malformed(tool: &str, message: &str) -> crate::types::ToolResult {
    result(
        tool,
        false,
        REASON_MALFORMED,
        json!(null),
        json!({ "error": message }),
        json!([]),
    )
}

pub fn missing(tool: &str, expected: Value, found: Value, checks: Value) -> crate::types::ToolResult {
    result(tool, false, REASON_MISSING, expected, found, checks)
}

pub fn failed(tool: &str, expected: Value, found: Value, checks: Value) -> crate::types::ToolResult {
    result(tool, false, REASON_FAILED, expected, found, checks)
}

pub fn passed(tool: &str, expected: Value, found: Value, checks: Value) -> crate::types::ToolResult {
    result(tool, true, REASON_PASSED, expected, found, checks)
}

pub fn parse_decimal_fractions(text: &str) -> Vec<f64> {
    let mut values = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i + 2 < chars.len() {
        if chars[i] == '0' && chars[i + 1] == '.' {
            let start = i;
            i += 2;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            let token: String = chars[start..i].iter().collect();
            if let Ok(v) = token.parse::<f64>() {
                if v > 0.0 && v <= 1.0 {
                    values.push(v);
                }
            }
        } else {
            i += 1;
        }
    }
    values
}

pub fn motes_to_cspr(motes: i64) -> f64 {
    motes as f64 / 1_000_000_000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_numbers_handles_commas() {
        let nums = parse_numbers("Allocate 4,000 CSPR and 3,500 CSPR");
        assert!(nums.contains(&4000.0));
        assert!(nums.contains(&3500.0));
    }

    #[test]
    fn within_tolerance_works() {
        assert!(within_tolerance(2346.50, 2346.10, 0.005));
        assert!(!within_tolerance(2400.0, 2346.10, 0.005));
    }
}
