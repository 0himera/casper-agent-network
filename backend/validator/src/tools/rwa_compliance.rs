use serde_json::{Value, json};

use super::common::{contains_ci, failed, malformed, missing, parse_decimal_fractions, passed};

const TOOL_NEWS: &str = "classify_news";
const TOOL_COLLATERAL: &str = "validate_collateral_logic";

struct NewsItem {
    id: String,
    headline: String,
    severity: String,
    verified: bool,
}

fn parse_news(fixture: &Value) -> Result<Vec<NewsItem>, String> {
    let arr = fixture
        .get("news_items")
        .and_then(|v| v.as_array())
        .ok_or("missing news_items array")?;
    if arr.is_empty() {
        return Err("empty news_items array".into());
    }

    let mut items = Vec::new();
    for item in arr {
        items.push(NewsItem {
            id: item
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or("news item missing id")?
                .to_string(),
            headline: item
                .get("headline")
                .and_then(|v| v.as_str())
                .ok_or("news item missing headline")?
                .to_string(),
            severity: item
                .get("severity")
                .and_then(|v| v.as_str())
                .unwrap_or("low")
                .to_string(),
            verified: item
                .get("verified")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
        });
    }
    Ok(items)
}

fn headline_keywords(headline: &str) -> Vec<String> {
    headline
        .split_whitespace()
        .filter(|w| w.len() >= 4)
        .map(|w| {
            w.trim_matches(|c: char| !c.is_alphanumeric())
                .to_ascii_lowercase()
        })
        .filter(|w| w.len() >= 4)
        .take(4)
        .collect()
}

fn mentions_headline(output: &str, headline: &str) -> bool {
    let keywords = headline_keywords(headline);
    let matched = keywords.iter().filter(|kw| contains_ci(output, kw)).count();
    matched >= 2
        || keywords
            .iter()
            .any(|kw| contains_ci(output, kw) && kw.len() >= 6)
}

pub fn classify_news(fixture: &Value, agent_output: &str) -> crate::types::ToolResult {
    let items = match parse_news(fixture) {
        Ok(v) => v,
        Err(msg) => return malformed(TOOL_NEWS, &msg),
    };

    let mut checks = Vec::new();

    for item in &items {
        let mentioned = mentions_headline(agent_output, &item.headline);
        let (expected_label, passed_check) = if item.verified && item.severity == "high" {
            let treats_as_threat = contains_ci(agent_output, "threat")
                || contains_ci(agent_output, "real")
                || contains_ci(agent_output, "verified")
                || contains_ci(agent_output, "sec");
            let context = threat_context(agent_output, &item.headline);
            let not_classified_as_fud = !contains_ci(&context, "fud");
            (
                "real_threat",
                mentioned && treats_as_threat && not_classified_as_fud,
            )
        } else if !item.verified {
            let treats_as_fud = contains_ci(agent_output, "fud")
                || contains_ci(agent_output, "unverified")
                || contains_ci(agent_output, "unconfirmed");
            ("fud", mentioned && treats_as_fud)
        } else {
            ("neutral", true)
        };

        checks.push(json!({
            "check": format!("classify_{}", item.id),
            "passed": passed_check,
            "expected": expected_label,
            "mentioned": mentioned
        }));
    }

    let required: Vec<_> = checks
        .iter()
        .filter(|c| c["expected"] != "neutral")
        .collect();
    let ok = required.iter().all(|c| c["passed"] == true)
        && required.iter().any(|c| c["mentioned"] == true);

    let expected = json!({
        "verified_high_as_threat": true,
        "unverified_as_fud": true
    });
    let found = json!({ "checks_summary": checks.len() });

    if ok {
        passed(TOOL_NEWS, expected, found, json!(checks))
    } else if required.iter().all(|c| c["mentioned"] == false) {
        missing(TOOL_NEWS, expected, found, json!(checks))
    } else {
        failed(TOOL_NEWS, expected, found, json!(checks))
    }
}

fn threat_context(output: &str, headline: &str) -> String {
    let lower = output.to_ascii_lowercase();
    for keyword in headline_keywords(headline) {
        if let Some(idx) = lower.find(&keyword) {
            let start = idx.saturating_sub(40);
            let end = output.len().min(idx + headline.len() + 40);
            return output[start..end].to_string();
        }
    }
    output.to_string()
}

pub fn validate_collateral_logic(fixture: &Value, agent_output: &str) -> crate::types::ToolResult {
    let current_cf = fixture
        .get("current_collateral_factor")
        .and_then(|v| v.as_f64())
        .ok_or_else(|| "missing current_collateral_factor".to_string());
    let floor = fixture
        .get("collateral_floor")
        .and_then(|v| v.as_f64())
        .ok_or_else(|| "missing collateral_floor".to_string());

    let (current_cf, floor) = match (current_cf, floor) {
        (Ok(c), Ok(f)) => (c, f),
        (Err(msg), _) | (_, Err(msg)) => return malformed(TOOL_COLLATERAL, &msg),
    };

    let has_verified_threat = fixture
        .get("news_items")
        .and_then(|v| v.as_array())
        .map(|items| {
            items.iter().any(|item| {
                item.get("verified").and_then(|v| v.as_bool()) == Some(true)
                    && item.get("severity").and_then(|v| v.as_str()) == Some("high")
            })
        })
        .unwrap_or(false);

    let new_cf = parse_decimal_fractions(agent_output)
        .into_iter()
        .filter(|n| *n < current_cf - 0.01 && *n >= floor)
        .max_by(|a, b| a.partial_cmp(b).unwrap());
    let recommends_reduction = contains_ci(agent_output, "reduce")
        || contains_ci(agent_output, "reduction")
        || contains_ci(agent_output, "lower");

    let new_cf_ok = new_cf
        .map(|nf| nf < current_cf && nf >= floor)
        .unwrap_or(false);
    let drop_ok = new_cf
        .map(|nf| {
            let drop = current_cf - nf;
            drop >= 0.10 && drop <= 0.25
        })
        .unwrap_or(false);

    let ok = has_verified_threat && recommends_reduction && new_cf_ok && drop_ok;

    let expected = json!({
        "current_collateral_factor": current_cf,
        "collateral_floor": floor,
        "requires_reduction": has_verified_threat
    });
    let found = json!({
        "new_collateral_factor": new_cf,
        "recommends_reduction": recommends_reduction,
        "drop_ok": drop_ok
    });

    if ok {
        passed(TOOL_COLLATERAL, expected, found, json!([]))
    } else if new_cf.is_none() && !recommends_reduction {
        missing(TOOL_COLLATERAL, expected, found, json!([]))
    } else {
        failed(TOOL_COLLATERAL, expected, found, json!([]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::common;
    use std::fs;
    use std::path::PathBuf;

    fn fixture() -> Value {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/rwa_compliance.json");
        serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
    }

    const GOLDEN: &str = "SEC inquiry (verified, high severity) is a real threat requiring collateral reduction. Social media default claims are unverified FUD. Recommendation: reduce collateral factor from 0.85 to 0.70. Remediation: monitor SEC proceedings, require additional disclosures, set 30-day review checkpoint.";

    #[test]
    fn classify_news_pass_golden() {
        let r = classify_news(&fixture(), GOLDEN);
        assert!(r.ok, "{:?}", r.details);
    }

    #[test]
    fn classify_news_fail_treats_fud_as_threat() {
        let r = classify_news(
            &fixture(),
            "Social media default claims are a real verified threat. SEC inquiry is FUD.",
        );
        assert!(!r.ok);
    }

    #[test]
    fn classify_news_missing_generic() {
        let r = classify_news(&fixture(), "Market conditions are uncertain today.");
        assert!(!r.ok);
        assert_eq!(r.details["reason"], common::REASON_MISSING);
    }

    #[test]
    fn classify_news_malformed_fixture() {
        let r = classify_news(&json!({}), GOLDEN);
        assert!(!r.ok);
        assert_eq!(r.details["reason"], common::REASON_MALFORMED);
    }

    #[test]
    fn validate_collateral_logic_pass_golden() {
        let r = validate_collateral_logic(&fixture(), GOLDEN);
        assert!(r.ok, "{:?}", r.details);
    }

    #[test]
    fn validate_collateral_logic_fail_increase() {
        let r = validate_collateral_logic(
            &fixture(),
            "Reduce collateral factor from 0.85 to 0.90 due to SEC inquiry threat.",
        );
        assert!(!r.ok);
    }

    #[test]
    fn validate_collateral_logic_missing_generic() {
        let r = validate_collateral_logic(&fixture(), "Collateral should be reviewed.");
        assert!(!r.ok);
        assert_eq!(r.details["reason"], common::REASON_MISSING);
    }

    #[test]
    fn validate_collateral_logic_malformed_fixture() {
        let r = validate_collateral_logic(&json!({}), GOLDEN);
        assert!(!r.ok);
        assert_eq!(r.details["reason"], common::REASON_MALFORMED);
    }
}
