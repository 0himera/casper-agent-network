use serde_json::{json, Value};

use super::common::{
    contains_ci, failed, find_percent_near, malformed, missing, passed, within_abs,
};

const TOOL_REVERT: &str = "validate_revert_rate";
const TOOL_RISK: &str = "check_risk_thresholds";

pub fn validate_revert_rate(fixture: &Value, agent_output: &str) -> crate::types::ToolResult {
    let expected_rate = fixture
        .get("revert_rate")
        .and_then(|v| v.as_f64())
        .ok_or_else(|| "missing revert_rate".to_string());

    let threshold = fixture
        .get("anomaly_threshold")
        .and_then(|v| v.as_f64());

    let expected_rate = match expected_rate {
        Ok(r) => r,
        Err(msg) => return malformed(TOOL_REVERT, &msg),
    };

    let found_rate = find_percent_near(agent_output, "revert")
        .or_else(|| find_percent_near(agent_output, "rate"));

    let mentions_threshold = threshold
        .map(|t| {
            let threshold_pct = t * 100.0;
            agent_output.contains(&format!("{threshold_pct:.0}%"))
                || agent_output.contains(&format!("{:.0}%", t * 100.0))
                || within_abs(
                    find_percent_near(agent_output, "threshold").unwrap_or(-1.0),
                    t,
                    0.01,
                )
        })
        .unwrap_or(true);

    let rate_ok = found_rate
        .map(|r| within_abs(r, expected_rate, 0.01))
        .unwrap_or(false);

    let ok = rate_ok && mentions_threshold;

    let expected = json!({
        "revert_rate": expected_rate,
        "anomaly_threshold": threshold
    });
    let found = json!({
        "found_revert_rate": found_rate,
        "mentions_threshold": mentions_threshold
    });

    if ok {
        passed(TOOL_REVERT, expected, found, json!([]))
    } else if found_rate.is_none() {
        missing(TOOL_REVERT, expected, found, json!([]))
    } else {
        failed(TOOL_REVERT, expected, found, json!([]))
    }
}

pub fn check_risk_thresholds(fixture: &Value, agent_output: &str) -> crate::types::ToolResult {
    let revert_rate = fixture
        .get("revert_rate")
        .and_then(|v| v.as_f64())
        .ok_or_else(|| "missing revert_rate".to_string());
    let anomaly_threshold = fixture
        .get("anomaly_threshold")
        .and_then(|v| v.as_f64())
        .ok_or_else(|| "missing anomaly_threshold".to_string());

    let (revert_rate, anomaly_threshold) = match (revert_rate, anomaly_threshold) {
        (Ok(r), Ok(t)) => (r, t),
        (Err(msg), _) | (_, Err(msg)) => return malformed(TOOL_RISK, &msg),
    };

    let is_high_risk = revert_rate > anomaly_threshold;
    let says_high = contains_ci(agent_output, "high risk");
    let says_safe = contains_ci(agent_output, "safe");

    let ok = if is_high_risk {
        says_high && !says_safe
    } else {
        says_safe || contains_ci(agent_output, "low risk")
    };

    let expected = json!({
        "is_high_risk": is_high_risk,
        "revert_rate": revert_rate,
        "anomaly_threshold": anomaly_threshold
    });
    let found = json!({
        "says_high_risk": says_high,
        "says_safe": says_safe
    });

    if ok {
        passed(TOOL_RISK, expected, found, json!([]))
    } else if !says_high && !says_safe {
        missing(TOOL_RISK, expected, found, json!([]))
    } else {
        failed(TOOL_RISK, expected, found, json!([]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::common;
    use std::fs;
    use std::path::PathBuf;

    fn fixture() -> Value {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/defi_protocol_risk.json");
        serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
    }

    const GOLDEN: &str = "Revert rate is 15% over the last 6 hours, exceeding the 10% anomaly threshold. Classification: High Risk. Recommended mitigation: pause new deposits, increase monitoring frequency, alert users to withdraw idle funds, and prepare emergency circuit breaker activation.";

    #[test]
    fn validate_revert_rate_pass_golden() {
        let r = validate_revert_rate(&fixture(), GOLDEN);
        assert!(r.ok, "{:?}", r.details);
    }

    #[test]
    fn validate_revert_rate_fail_wrong_rate() {
        let r = validate_revert_rate(
            &fixture(),
            "Revert rate is 5% over the last 6 hours, below the 10% anomaly threshold.",
        );
        assert!(!r.ok);
    }

    #[test]
    fn validate_revert_rate_missing_generic() {
        let r = validate_revert_rate(&fixture(), "Protocol seems unstable lately.");
        assert!(!r.ok);
        assert_eq!(r.details["reason"], common::REASON_MISSING);
    }

    #[test]
    fn validate_revert_rate_malformed_fixture() {
        let r = validate_revert_rate(&json!({}), GOLDEN);
        assert!(!r.ok);
        assert_eq!(r.details["reason"], common::REASON_MALFORMED);
    }

    #[test]
    fn check_risk_thresholds_pass_golden() {
        let r = check_risk_thresholds(&fixture(), GOLDEN);
        assert!(r.ok, "{:?}", r.details);
    }

    #[test]
    fn check_risk_thresholds_fail_safe_when_high() {
        let r = check_risk_thresholds(
            &fixture(),
            "Revert rate is 15%, classification: Safe.",
        );
        assert!(!r.ok);
    }

    #[test]
    fn check_risk_thresholds_missing_generic() {
        let r = check_risk_thresholds(&fixture(), "Many transactions reverted recently.");
        assert!(!r.ok);
        assert_eq!(r.details["reason"], common::REASON_MISSING);
    }

    #[test]
    fn check_risk_thresholds_malformed_fixture() {
        let r = check_risk_thresholds(&json!({}), GOLDEN);
        assert!(!r.ok);
        assert_eq!(r.details["reason"], common::REASON_MALFORMED);
    }
}
