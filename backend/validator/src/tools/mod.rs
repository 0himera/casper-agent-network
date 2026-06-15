mod common;
mod defi_protocol;
mod defi_yield;
mod rwa_appraisal;
mod rwa_compliance;

use crate::types::ToolResult;

const KNOWN_TOOLS: &[&str] = &[
    "check_allocation_sum",
    "validate_apy",
    "check_fees",
    "validate_il",
    "validate_revert_rate",
    "check_risk_thresholds",
    "validate_outliers",
    "check_sources",
    "validate_price_derivation",
    "classify_news",
    "validate_collateral_logic",
];

pub fn run_tool(name: &str, fixture: &serde_json::Value, agent_output: &str) -> ToolResult {
    match name {
        "validate_outliers" => rwa_appraisal::validate_outliers(fixture, agent_output),
        "check_sources" => rwa_appraisal::check_sources(fixture, agent_output),
        "validate_price_derivation" => {
            rwa_appraisal::validate_price_derivation(fixture, agent_output)
        }
        "check_allocation_sum" => defi_yield::check_allocation_sum(fixture, agent_output),
        "validate_apy" => defi_yield::validate_apy(fixture, agent_output),
        "check_fees" => defi_yield::check_fees(fixture, agent_output),
        "validate_il" => defi_yield::validate_il(fixture, agent_output),
        "validate_revert_rate" => defi_protocol::validate_revert_rate(fixture, agent_output),
        "check_risk_thresholds" => defi_protocol::check_risk_thresholds(fixture, agent_output),
        "classify_news" => rwa_compliance::classify_news(fixture, agent_output),
        "validate_collateral_logic" => {
            rwa_compliance::validate_collateral_logic(fixture, agent_output)
        }
        _ => {
            debug_assert!(!KNOWN_TOOLS.contains(&name));
            ToolResult {
                tool: name.to_string(),
                ok: false,
                details: serde_json::json!({ "error": "unknown tool" }),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn load_fixture(name: &str) -> serde_json::Value {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures").join(name);
        serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
    }

    #[test]
    fn dispatch_routes_known_tools() {
        let fixture = load_fixture("defi_yield_routing.json");
        let output = "Allocate 4,000 CSPR to cspr-usdt (8.2% APY), 3,500 CSPR to cspr-eth (6.1% APY), 2,500 CSPR to cspr-wbtc (11.4% APY). Total: 10,000 CSPR. Network gas fees (~2.5 CSPR per swap) included. IL analysis shows cspr-usdt lowest volatility exposure.";
        let result = run_tool("check_allocation_sum", &fixture, output);
        assert_eq!(result.tool, "check_allocation_sum");
        assert!(result.ok);
        assert_eq!(result.details["reason"], common::REASON_PASSED);
    }

    #[test]
    fn unknown_tool_returns_error_without_panic() {
        let result = run_tool("does_not_exist", &serde_json::json!({}), "output");
        assert!(!result.ok);
        assert_eq!(result.details["error"], "unknown tool");
    }

    #[test]
    fn all_known_tools_are_routed() {
        for name in KNOWN_TOOLS {
            let result = run_tool(name, &serde_json::json!({}), "output");
            assert_ne!(result.details.get("error"), Some(&serde_json::json!("unknown tool")));
            assert_eq!(result.tool, *name);
        }
    }
}
