use serde_json::{Value, json};

use super::common::{
    contains_ci, contains_id, failed, malformed, median, missing, passed, weighted_median,
    within_tolerance,
};

const TOOL_OUTLIERS: &str = "validate_outliers";
const TOOL_SOURCES: &str = "check_sources";
const TOOL_PRICE: &str = "validate_price_derivation";

const RELIABILITY_CUTOFF: f64 = 0.85;
const LOW_RELIABILITY: f64 = 0.5;

struct Source {
    id: String,
    price_usd: f64,
    reliability: f64,
}

fn parse_sources(fixture: &Value) -> Result<Vec<Source>, String> {
    let arr = fixture
        .get("sources")
        .and_then(|v| v.as_array())
        .ok_or("missing sources array")?;
    if arr.is_empty() {
        return Err("empty sources array".into());
    }

    let mut sources = Vec::new();
    for item in arr {
        let id = item
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or("source missing id")?
            .to_string();
        let price_usd = item
            .get("price_usd")
            .and_then(|v| v.as_f64())
            .ok_or("source missing price_usd")?;
        let reliability = item
            .get("reliability")
            .and_then(|v| v.as_f64())
            .ok_or("source missing reliability")?;
        sources.push(Source {
            id,
            price_usd,
            reliability,
        });
    }
    Ok(sources)
}

fn expected_outliers(sources: &[Source], threshold_pct: f64) -> Vec<String> {
    let reliable_prices: Vec<f64> = sources
        .iter()
        .filter(|s| s.reliability >= RELIABILITY_CUTOFF)
        .map(|s| s.price_usd)
        .collect();

    let baseline = median(&reliable_prices).unwrap_or(0.0);

    sources
        .iter()
        .filter(|s| {
            s.reliability < LOW_RELIABILITY
                || (baseline > 0.0
                    && ((s.price_usd - baseline).abs() / baseline * 100.0) > threshold_pct)
        })
        .map(|s| s.id.clone())
        .collect()
}

fn non_outlier_sources<'a>(sources: &'a [Source], outlier_ids: &[String]) -> Vec<&'a Source> {
    sources
        .iter()
        .filter(|s| !outlier_ids.contains(&s.id))
        .collect()
}

pub fn validate_outliers(fixture: &Value, agent_output: &str) -> crate::types::ToolResult {
    let threshold_pct = fixture
        .get("outlier_threshold_pct")
        .and_then(|v| v.as_f64())
        .unwrap_or(3.0);

    let sources = match parse_sources(fixture) {
        Ok(s) => s,
        Err(msg) => return malformed(TOOL_OUTLIERS, &msg),
    };

    let outliers = expected_outliers(&sources, threshold_pct);
    let mut checks = Vec::new();

    for id in &outliers {
        let mentioned = contains_id(agent_output, id);
        checks.push(json!({ "check": format!("mention_outlier_{id}"), "passed": mentioned }));
    }

    let mentions_outlier_concept =
        contains_ci(agent_output, "outlier") || contains_ci(agent_output, "deviation");
    checks.push(json!({
        "check": "mentions_outlier_concept",
        "passed": mentions_outlier_concept
    }));

    let all_mentioned = outliers.iter().all(|id| contains_id(agent_output, id));
    let ok = all_mentioned && mentions_outlier_concept;

    let expected = json!({ "outlier_ids": outliers, "threshold_pct": threshold_pct });
    let found = json!({
        "mentioned_outliers": outliers.iter().filter(|id| contains_id(agent_output, id)).collect::<Vec<_>>(),
        "mentions_outlier_concept": mentions_outlier_concept
    });

    if ok {
        passed(TOOL_OUTLIERS, expected, found, json!(checks))
    } else if !outliers.iter().any(|id| contains_id(agent_output, id)) {
        missing(TOOL_OUTLIERS, expected, found, json!(checks))
    } else {
        failed(TOOL_OUTLIERS, expected, found, json!(checks))
    }
}

pub fn check_sources(fixture: &Value, agent_output: &str) -> crate::types::ToolResult {
    let min_sources = fixture
        .get("min_sources")
        .and_then(|v| v.as_u64())
        .unwrap_or(2) as usize;

    let sources = match parse_sources(fixture) {
        Ok(s) => s,
        Err(msg) => return malformed(TOOL_SOURCES, &msg),
    };

    let credible: Vec<&Source> = sources
        .iter()
        .filter(|s| s.reliability >= RELIABILITY_CUTOFF)
        .collect();

    let mut cited = Vec::new();
    let mut checks = Vec::new();

    for source in &credible {
        let mentioned =
            contains_id(agent_output, &source.id) || canonical_name_match(agent_output, &source.id);
        if mentioned {
            cited.push(source.id.clone());
        }
        checks.push(json!({
            "check": format!("cite_{}", source.id),
            "passed": mentioned
        }));
    }

    let ok = cited.len() >= min_sources;
    let expected = json!({
        "min_sources": min_sources,
        "credible_source_ids": credible.iter().map(|s| &s.id).collect::<Vec<_>>()
    });
    let found = json!({ "cited_source_ids": cited, "cited_count": cited.len() });

    if ok {
        passed(TOOL_SOURCES, expected, found, json!(checks))
    } else if cited.is_empty() {
        missing(TOOL_SOURCES, expected, found, json!(checks))
    } else {
        failed(TOOL_SOURCES, expected, found, json!(checks))
    }
}

fn canonical_name_match(text: &str, id: &str) -> bool {
    match id {
        "lbma" => contains_ci(text, "lbma"),
        "comex" => contains_ci(text, "comex"),
        "ecb_ref" => contains_ci(text, "ecb"),
        _ => false,
    }
}

pub fn validate_price_derivation(fixture: &Value, agent_output: &str) -> crate::types::ToolResult {
    let threshold_pct = fixture
        .get("outlier_threshold_pct")
        .and_then(|v| v.as_f64())
        .unwrap_or(3.0);

    let sources = match parse_sources(fixture) {
        Ok(s) => s,
        Err(msg) => return malformed(TOOL_PRICE, &msg),
    };

    let outlier_ids = expected_outliers(&sources, threshold_pct);
    let clean: Vec<&Source> = non_outlier_sources(&sources, &outlier_ids);

    let pairs: Vec<(f64, f64)> = clean.iter().map(|s| (s.price_usd, s.reliability)).collect();
    let reference = weighted_median(&pairs).unwrap_or(0.0);

    let agent_price = extract_dollar_price(agent_output);
    let algorithm_keywords = ["median", "weighted", "exclude", "outlier", "reliability"];
    let algo_hits: Vec<&str> = algorithm_keywords
        .iter()
        .filter(|kw| contains_ci(agent_output, kw))
        .copied()
        .collect();

    let price_ok = agent_price
        .map(|p| within_tolerance(p, reference, 0.005))
        .unwrap_or(false);
    let algo_ok = algo_hits.len() >= 2;

    let checks = json!([
        { "check": "price_within_tolerance", "passed": price_ok, "reference": reference, "agent_price": agent_price },
        { "check": "algorithm_described", "passed": algo_ok, "keywords_found": algo_hits }
    ]);

    let expected = json!({ "reference_price_usd": reference, "tolerance_pct": 0.5 });
    let found = json!({ "agent_price_usd": agent_price, "algorithm_keywords": algo_hits });

    if price_ok && algo_ok {
        passed(TOOL_PRICE, expected, found, checks)
    } else if agent_price.is_none() {
        missing(TOOL_PRICE, expected, found, checks)
    } else {
        failed(TOOL_PRICE, expected, found, checks)
    }
}

fn extract_dollar_price(text: &str) -> Option<f64> {
    let lower = text.to_ascii_lowercase();

    if let Some(idx) = lower.find("fair price") {
        let window = &text[idx..text.len().min(idx + 60)];
        if let Some(price) = first_dollar_amount(window) {
            return Some(price);
        }
    }

    let mut prices = Vec::new();
    for (idx, _) in text.match_indices('$') {
        if let Some(price) = first_dollar_amount(&text[idx..]) {
            prices.push(price);
        }
    }

    if prices.is_empty() {
        for num in super::common::parse_numbers(text) {
            if (2000.0..=3000.0).contains(&num) {
                prices.push(num);
            }
        }
    }

    prices.into_iter().reduce(|a, b| {
        if (a - 2346.0).abs() <= (b - 2346.0).abs() {
            a
        } else {
            b
        }
    })
}

fn first_dollar_amount(text: &str) -> Option<f64> {
    let after = text
        .strip_prefix('$')
        .or_else(|| text.find('$').map(|idx| &text[idx + 1..]))?;
    super::common::parse_numbers(after).first().copied()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::common;
    use std::fs;
    use std::path::PathBuf;

    fn fixture() -> Value {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/rwa_appraisal.json");
        serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
    }

    const GOLDEN: &str = "Filtered retail_feed ($2410) as outlier (>3% deviation). Cross-checked LBMA, COMEX, ECB sources. Weighted median fair price: $2,346.50 USD/oz based on reliability scores. Algorithm: exclude outliers, weight by source reliability, compute median.";

    #[test]
    fn validate_outliers_pass_golden() {
        let r = validate_outliers(&fixture(), GOLDEN);
        assert!(r.ok, "{:?}", r.details);
    }

    #[test]
    fn validate_outliers_fail_wrong_outlier() {
        let r = validate_outliers(
            &fixture(),
            "Used lbma, comex, and retail_feed equally in the final price.",
        );
        assert!(!r.ok);
        assert_eq!(r.details["reason"], common::REASON_FAILED);
    }

    #[test]
    fn validate_outliers_missing_generic() {
        let r = validate_outliers(&fixture(), "Fair price analysis complete.");
        assert!(!r.ok);
        assert_eq!(r.details["reason"], common::REASON_MISSING);
    }

    #[test]
    fn validate_outliers_malformed_fixture() {
        let r = validate_outliers(&json!({}), "outlier retail_feed");
        assert!(!r.ok);
        assert_eq!(r.details["reason"], common::REASON_MALFORMED);
    }

    #[test]
    fn check_sources_pass_golden() {
        let r = check_sources(&fixture(), GOLDEN);
        assert!(r.ok, "{:?}", r.details);
    }

    #[test]
    fn check_sources_fail_retail_only() {
        let r = check_sources(&fixture(), "Only retail_feed source used at $2410.");
        assert!(!r.ok);
    }

    #[test]
    fn check_sources_missing_generic() {
        let r = check_sources(&fixture(), "Price looks reasonable.");
        assert!(!r.ok);
        assert_eq!(r.details["reason"], common::REASON_MISSING);
    }

    #[test]
    fn check_sources_malformed_fixture() {
        let r = check_sources(&json!({ "sources": [] }), GOLDEN);
        assert!(!r.ok);
        assert_eq!(r.details["reason"], common::REASON_MALFORMED);
    }

    #[test]
    fn validate_price_derivation_pass_golden() {
        let r = validate_price_derivation(&fixture(), GOLDEN);
        assert!(r.ok, "{:?}", r.details);
    }

    #[test]
    fn validate_price_derivation_fail_wrong_price() {
        let r = validate_price_derivation(
            &fixture(),
            "Filtered retail_feed as outlier. Weighted median fair price: $2,500.00 USD/oz. Algorithm: exclude outliers, compute median.",
        );
        assert!(!r.ok);
    }

    #[test]
    fn validate_price_derivation_missing_no_price() {
        let r = validate_price_derivation(
            &fixture(),
            "Filtered retail_feed as outlier. Algorithm: exclude outliers.",
        );
        assert!(!r.ok);
        assert_eq!(r.details["reason"], common::REASON_MISSING);
    }

    #[test]
    fn validate_price_derivation_malformed_fixture() {
        let r = validate_price_derivation(&json!({}), GOLDEN);
        assert!(!r.ok);
        assert_eq!(r.details["reason"], common::REASON_MALFORMED);
    }
}
