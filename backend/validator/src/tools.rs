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

fn is_known_tool(name: &str) -> bool {
    KNOWN_TOOLS.contains(&name)
}

pub fn run_tool(name: &str, _fixture: &serde_json::Value, _agent_output: &str) -> ToolResult {
    if is_known_tool(name) {
        ToolResult {
            tool: name.to_string(),
            ok: true,
            details: serde_json::json!({ "stub": true }),
        }
    } else {
        ToolResult {
            tool: name.to_string(),
            ok: false,
            details: serde_json::json!({ "error": "unknown tool" }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_tool_returns_stub_ok() {
        let result = run_tool("check_allocation_sum", &serde_json::json!({}), "output");
        assert!(result.ok);
        assert_eq!(result.tool, "check_allocation_sum");
        assert_eq!(result.details, serde_json::json!({ "stub": true }));
    }

    #[test]
    fn unknown_tool_returns_error_without_panic() {
        let result = run_tool("does_not_exist", &serde_json::json!({}), "output");
        assert!(!result.ok);
        assert_eq!(result.details["error"], "unknown tool");
    }
}
