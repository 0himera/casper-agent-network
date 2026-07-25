use crate::stage_pipeline::stage_scoring::{
    self, quality_domain, quality_gibberish, quality_refusal, quality_relevance,
};

pub mod claims;
pub mod domain_match;
pub mod factuality;
pub mod gibberish;
pub mod refusal;
pub mod relevance;

pub fn clamp_u32(value: u32, min: u32, max: u32) -> u32 {
    value.clamp(min, max)
}

pub fn extract_first_u32(text: &str) -> Option<u32> {
    let mut digits = String::new();
    let mut started = false;
    for ch in text.chars() {
        if ch.is_ascii_digit() {
            digits.push(ch);
            started = true;
        } else if started {
            break;
        }
    }
    if digits.is_empty() {
        return None;
    }
    digits.parse().ok()
}

pub fn extract_from_json_value(text: &str, keys: &[&str]) -> Option<String> {
    let trimmed = text.trim();
    let json_str = if trimmed.starts_with('{') {
        trimmed
    } else {
        trimmed
            .find('{')
            .and_then(|start| trimmed.rfind('}').map(|end| &trimmed[start..=end]))?
    };

    let parsed: serde_json::Value = serde_json::from_str(json_str).ok()?;
    for key in keys {
        if let Some(value) = parsed.get(*key) {
            if let Some(text_value) = value.as_str() {
                return Some(text_value.to_string());
            }
            if value.is_number() || value.is_boolean() {
                return Some(value.to_string());
            }
        }
    }
    None
}

pub fn extract_yes_no(text: &str) -> Option<bool> {
    if let Some(json_value) =
        extract_from_json_value(text, &["answer", "result", "response", "value"])
        && let Some(parsed) = parse_yes_no_token(&json_value)
    {
        return Some(parsed);
    }

    let normalized = text.trim().to_ascii_lowercase();
    for token in normalized.split(|c: char| !c.is_alphanumeric()) {
        if let Some(parsed) = parse_yes_no_token(token) {
            return Some(parsed);
        }
    }

    parse_yes_no_token(&normalized)
}

fn parse_yes_no_token(token: &str) -> Option<bool> {
    match token.trim().trim_matches(|c: char| !c.is_alphanumeric()) {
        "yes" | "y" | "true" | "да" => Some(true),
        "no" | "n" | "false" | "нет" => Some(false),
        _ => None,
    }
}

pub fn parse_refusal_mock_response(agent_output: &str) -> refusal::RefusalStageEval {
    let lower = agent_output.to_ascii_lowercase();
    let is_refusal = lower.contains("mock_refusal")
        || lower.contains("i cannot fulfill")
        || lower.contains("i can't fulfill this request");
    refusal::RefusalStageEval {
        raw_output: if is_refusal {
            "yes".to_string()
        } else {
            "no".to_string()
        },
        is_refusal,
        normalized_quality: quality_refusal(is_refusal),
        passed: !is_refusal,
        parse_fallback: false,
    }
}

pub fn parse_gibberish_mock_response(agent_output: &str) -> gibberish::GibberishStageEval {
    let thresholds = stage_scoring::resolved_thresholds();
    let lower = agent_output.to_ascii_lowercase();
    let parse_noisy = lower.contains("mock_parse_noisy");
    let raw_score = if lower.contains("mock_gibberish") {
        1
    } else {
        5
    };
    gibberish::GibberishStageEval {
        raw_output: raw_score.to_string(),
        raw_score,
        normalized_quality: quality_gibberish(raw_score),
        passed: raw_score >= thresholds.gibberish_min,
        parse_fallback: parse_noisy,
    }
}

pub fn parse_relevance_mock_response(agent_output: &str) -> relevance::RelevanceStageEval {
    let thresholds = stage_scoring::resolved_thresholds();
    let lower = agent_output.to_ascii_lowercase();
    let parse_noisy = lower.contains("mock_parse_noisy");
    let raw_score = if lower.contains("mock_irrelevant") {
        2
    } else {
        8
    };
    relevance::RelevanceStageEval {
        raw_output: raw_score.to_string(),
        raw_score,
        normalized_quality: quality_relevance(raw_score),
        passed: raw_score >= thresholds.relevance_min,
        parse_fallback: parse_noisy,
    }
}

pub fn parse_domain_match_mock_response(agent_output: &str) -> domain_match::DomainMatchStageEval {
    let lower = agent_output.to_ascii_lowercase();
    let domain_matches = !lower.contains("mock_wrong_domain");
    domain_match::DomainMatchStageEval {
        raw_output: if domain_matches {
            "yes".to_string()
        } else {
            "no".to_string()
        },
        domain_matches,
        normalized_quality: quality_domain(domain_matches),
        passed: domain_matches,
        parse_fallback: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_noisy_mock_sets_parse_fallback_flags() {
        let gibberish = parse_gibberish_mock_response(
            "MOCK_PARSE_NOISY: Recommended allocation across cspr-usdt pools.",
        );
        let relevance = parse_relevance_mock_response(
            "MOCK_PARSE_NOISY: Recommended allocation across cspr-usdt pools.",
        );
        assert!(gibberish.parse_fallback);
        assert!(relevance.parse_fallback);
        assert!(gibberish.passed);
        assert!(relevance.passed);
    }

    #[test]
    fn extract_yes_no_from_plain_text() {
        assert_eq!(extract_yes_no("yes"), Some(true));
        assert_eq!(extract_yes_no("No"), Some(false));
        assert_eq!(extract_yes_no("Answer: да"), Some(true));
    }

    #[test]
    fn extract_yes_no_from_json() {
        assert_eq!(extract_yes_no(r#"{"answer":"no"}"#), Some(false));
    }

    #[test]
    fn extract_first_u32_from_prose() {
        assert_eq!(extract_first_u32("Score: 7 out of 10"), Some(7));
        assert_eq!(extract_first_u32("no digits"), None);
    }

    #[test]
    fn extract_from_json_value_reads_numeric_field() {
        assert_eq!(
            extract_from_json_value(r#"{"score":4}"#, &["score"]),
            Some("4".to_string())
        );
    }

    #[test]
    fn clamp_u32_limits_range() {
        assert_eq!(clamp_u32(99, 1, 5), 5);
        assert_eq!(clamp_u32(0, 1, 5), 1);
    }
}
