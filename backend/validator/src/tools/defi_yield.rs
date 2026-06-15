use serde_json::{json, Value};

use super::common::{
    contains_ci, contains_id, failed, malformed, missing, motes_to_cspr, parse_numbers, passed,
    within_abs,
};

const TOOL_ALLOC: &str = "check_allocation_sum";
const TOOL_APY: &str = "validate_apy";
const TOOL_FEES: &str = "check_fees";
const TOOL_IL: &str = "validate_il";

struct Pool {
    id: String,
    apy: f64,
    fee_bps: u64,
}

fn format_fee_bps(fee_bps: u64) -> String {
    format!("{fee_bps} bps")
}

fn parse_pools(fixture: &Value) -> Result<(u64, i64, Vec<Pool>), String> {
    let amount_cspr = fixture
        .get("amount_cspr")
        .and_then(|v| v.as_u64())
        .ok_or("missing amount_cspr")?;
    let gas_price_motes = fixture
        .get("gas_price_motes")
        .and_then(|v| v.as_i64())
        .ok_or("missing gas_price_motes")?;

    let arr = fixture
        .get("pools")
        .and_then(|v| v.as_array())
        .ok_or("missing pools array")?;

    let mut pools = Vec::new();
    for item in arr {
        pools.push(Pool {
            id: item
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or("pool missing id")?
                .to_string(),
            apy: item
                .get("apy")
                .and_then(|v| v.as_f64())
                .ok_or("pool missing apy")?,
            fee_bps: item
                .get("fee_bps")
                .and_then(|v| v.as_u64())
                .unwrap_or(0),
        });
    }
    Ok((amount_cspr, gas_price_motes, pools))
}

pub fn check_allocation_sum(fixture: &Value, agent_output: &str) -> crate::types::ToolResult {
    let (target, _, pools) = match parse_pools(fixture) {
        Ok(v) => v,
        Err(msg) => return malformed(TOOL_ALLOC, &msg),
    };

    let per_pool = extract_pool_allocations(agent_output, &pools);
    let explicit_total = extract_explicit_total(agent_output);

    let sum_from_pools: Option<f64> = if per_pool.is_empty() {
        None
    } else {
        Some(per_pool.values().sum())
    };

    let matched_sum = sum_from_pools.or(explicit_total);
    let ok = matched_sum
        .map(|s| within_abs(s, target as f64, 1.0))
        .unwrap_or(false);

    let expected = json!({ "target_cspr": target });
    let found = json!({
        "per_pool_allocations": per_pool,
        "explicit_total": explicit_total,
        "computed_sum": matched_sum
    });

    if ok {
        passed(TOOL_ALLOC, expected, found, json!([]))
    } else if matched_sum.is_none() {
        missing(TOOL_ALLOC, expected, found, json!([]))
    } else {
        failed(TOOL_ALLOC, expected, found, json!([]))
    }
}

fn extract_pool_allocations(output: &str, pools: &[Pool]) -> std::collections::HashMap<String, f64> {
    let mut result = std::collections::HashMap::new();
    let lower = output.to_ascii_lowercase();

    for pool in pools {
        if let Some(idx) = lower.find(&pool.id.to_ascii_lowercase()) {
            let window_start = idx.saturating_sub(40);
            let window = &output[window_start..idx];
            if let Some(amount) = parse_numbers(window).into_iter().rev().find(|n| *n >= 100.0) {
                result.insert(pool.id.clone(), amount);
            }
        }
    }
    result
}

fn extract_explicit_total(output: &str) -> Option<f64> {
    let lower = output.to_ascii_lowercase();
    if let Some(idx) = lower.find("total") {
        let window = &output[idx..output.len().min(idx + 40)];
        return parse_numbers(window)
            .into_iter()
            .find(|n| *n >= 1000.0);
    }
    None
}

pub fn validate_apy(fixture: &Value, agent_output: &str) -> crate::types::ToolResult {
    let (_, _, pools) = match parse_pools(fixture) {
        Ok(v) => v,
        Err(msg) => return malformed(TOOL_APY, &msg),
    };

    let mut checks = Vec::new();
    let mut matched = 0;
    let mut mentioned = 0;

    for pool in &pools {
        if !contains_id(agent_output, &pool.id) {
            continue;
        }
        mentioned += 1;
        let expected_pct = pool.apy * 100.0;
        let found_pct = extract_apy_near_pool(agent_output, &pool.id);
        let ok = found_pct
            .map(|p| within_abs(p, expected_pct, 0.5))
            .unwrap_or(false);
        if ok {
            matched += 1;
        }
        checks.push(json!({
            "check": format!("apy_{}", pool.id),
            "passed": ok,
            "expected_pct": expected_pct,
            "found_pct": found_pct
        }));
    }

    let ok = matched >= 2;
    let expected = json!({ "min_matched_pools": 2 });
    let found = json!({ "mentioned_pools": mentioned, "matched_pools": matched });

    if ok {
        passed(TOOL_APY, expected, found, json!(checks))
    } else if mentioned == 0 {
        missing(TOOL_APY, expected, found, json!(checks))
    } else {
        failed(TOOL_APY, expected, found, json!(checks))
    }
}

fn extract_apy_near_pool(output: &str, pool_id: &str) -> Option<f64> {
    let lower = output.to_ascii_lowercase();
    let id_lower = pool_id.to_ascii_lowercase();
    let idx = lower.find(&id_lower)?;
    let window = &output[idx..output.len().min(idx + 40)];
    for num in parse_numbers(window) {
        if (1.0..=30.0).contains(&num) {
            return Some(num);
        }
    }
    None
}

pub fn check_fees(fixture: &Value, agent_output: &str) -> crate::types::ToolResult {
    let (_, gas_price_motes, pools) = match parse_pools(fixture) {
        Ok(v) => v,
        Err(msg) => return malformed(TOOL_FEES, &msg),
    };

    let mentions_fees = contains_ci(agent_output, "fee")
        || contains_ci(agent_output, "gas")
        || contains_ci(agent_output, "commission");

    let expected_gas_cspr = motes_to_cspr(gas_price_motes);
    let gas_mentioned = parse_numbers(agent_output)
        .into_iter()
        .any(|n| within_abs(n, expected_gas_cspr, expected_gas_cspr * 0.3));

    let mentions_pool_fee = pools.iter().any(|p| {
        contains_id(agent_output, &p.id)
            && (contains_ci(agent_output, "fee") || contains_ci(agent_output, &format_fee_bps(p.fee_bps)))
    });

    let ok = mentions_fees && (gas_mentioned || mentions_pool_fee);

    let expected = json!({ "expected_gas_cspr": expected_gas_cspr });
    let found = json!({
        "mentions_fees": mentions_fees,
        "gas_mentioned": gas_mentioned,
        "mentions_pool_fee": mentions_pool_fee
    });

    if ok {
        passed(TOOL_FEES, expected, found, json!([]))
    } else if !mentions_fees {
        missing(TOOL_FEES, expected, found, json!([]))
    } else {
        failed(TOOL_FEES, expected, found, json!([]))
    }
}

pub fn validate_il(fixture: &Value, agent_output: &str) -> crate::types::ToolResult {
    let (_, _, pools) = match parse_pools(fixture) {
        Ok(v) => v,
        Err(msg) => return malformed(TOOL_IL, &msg),
    };

    let il_keywords = ["il", "impermanent loss", "volatility"];
    let has_il_concept = il_keywords.iter().any(|kw| contains_ci(agent_output, kw));

    let mentioned_pools: Vec<&str> = pools
        .iter()
        .filter(|p| contains_id(agent_output, &p.id))
        .map(|p| p.id.as_str())
        .collect();

    let ok = has_il_concept && mentioned_pools.len() >= 2;

    let expected = json!({ "min_pools": 2, "requires_il_concept": true });
    let found = json!({
        "mentioned_pools": mentioned_pools,
        "has_il_concept": has_il_concept
    });

    if ok {
        passed(TOOL_IL, expected, found, json!([]))
    } else if mentioned_pools.is_empty() && !has_il_concept {
        missing(TOOL_IL, expected, found, json!([]))
    } else {
        failed(TOOL_IL, expected, found, json!([]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::common;
    use std::fs;
    use std::path::PathBuf;

    fn fixture() -> Value {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/defi_yield_routing.json");
        serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
    }

    const GOLDEN: &str = "Allocate 4,000 CSPR to cspr-usdt (8.2% APY, high TVL), 3,500 CSPR to cspr-eth (6.1% APY, moderate IL), and 2,500 CSPR to cspr-wbtc (11.4% APY, higher IL risk). Total: 10,000 CSPR. Network gas fees (~2.5 CSPR per swap) included. IL analysis shows cspr-usdt lowest volatility exposure.";

    #[test]
    fn check_allocation_sum_pass_golden() {
        let r = check_allocation_sum(&fixture(), GOLDEN);
        assert!(r.ok, "{:?}", r.details);
    }

    #[test]
    fn check_allocation_sum_fail_wrong_total() {
        let r = check_allocation_sum(
            &fixture(),
            "Allocate 6,000 CSPR to cspr-usdt and 5,000 CSPR to cspr-eth. Total: 11,000 CSPR.",
        );
        assert!(!r.ok);
    }

    #[test]
    fn check_allocation_sum_missing_generic() {
        let r = check_allocation_sum(&fixture(), "Good yield routing recommendation.");
        assert!(!r.ok);
        assert_eq!(r.details["reason"], common::REASON_MISSING);
    }

    #[test]
    fn check_allocation_sum_malformed_fixture() {
        let r = check_allocation_sum(&json!({}), GOLDEN);
        assert!(!r.ok);
        assert_eq!(r.details["reason"], common::REASON_MALFORMED);
    }

    #[test]
    fn validate_apy_pass_golden() {
        let r = validate_apy(&fixture(), GOLDEN);
        assert!(r.ok, "{:?}", r.details);
    }

    #[test]
    fn validate_apy_fail_wrong_apy() {
        let r = validate_apy(
            &fixture(),
            "cspr-usdt at 20% APY, cspr-eth at 25% APY, cspr-wbtc at 30% APY.",
        );
        assert!(!r.ok);
    }

    #[test]
    fn validate_apy_missing_no_pools() {
        let r = validate_apy(&fixture(), "APY looks good overall.");
        assert!(!r.ok);
        assert_eq!(r.details["reason"], common::REASON_MISSING);
    }

    #[test]
    fn validate_apy_malformed_fixture() {
        let r = validate_apy(&json!({}), GOLDEN);
        assert!(!r.ok);
    }

    #[test]
    fn check_fees_pass_golden() {
        let r = check_fees(&fixture(), GOLDEN);
        assert!(r.ok, "{:?}", r.details);
    }

    #[test]
    fn check_fees_fail_no_fees() {
        let r = check_fees(
            &fixture(),
            "Allocate 4000 to cspr-usdt and 6000 to cspr-eth with no cost analysis.",
        );
        assert!(!r.ok);
    }

    #[test]
    fn check_fees_missing_generic() {
        let r = check_fees(&fixture(), "Pools selected by APY only.");
        assert!(!r.ok);
        assert_eq!(r.details["reason"], common::REASON_MISSING);
    }

    #[test]
    fn check_fees_malformed_fixture() {
        let r = check_fees(&json!({}), GOLDEN);
        assert!(!r.ok);
    }

    #[test]
    fn validate_il_pass_golden() {
        let r = validate_il(&fixture(), GOLDEN);
        assert!(r.ok, "{:?}", r.details);
    }

    #[test]
    fn validate_il_fail_one_pool() {
        let r = validate_il(
            &fixture(),
            "cspr-usdt has low impermanent loss exposure.",
        );
        assert!(!r.ok);
    }

    #[test]
    fn validate_il_missing_generic() {
        let r = validate_il(&fixture(), "Allocate across multiple pools.");
        assert!(!r.ok);
        assert_eq!(r.details["reason"], common::REASON_MISSING);
    }

    #[test]
    fn validate_il_malformed_fixture() {
        let r = validate_il(&json!({}), GOLDEN);
        assert!(!r.ok);
    }
}
