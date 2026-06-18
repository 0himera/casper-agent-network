use crate::llm::call_judge_raw;
use crate::prompts::build_stage_relevance_prompts;
use crate::stage_pipeline::stage_scoring::{quality_relevance, resolved_thresholds};
use crate::types::{LlmConfig, ValidatorError};

use super::{clamp_u32, extract_first_u32, extract_from_json_value, parse_relevance_mock_response};

#[derive(Debug, Clone, PartialEq)]
pub struct RelevanceStageEval {
    pub raw_output: String,
    pub raw_score: u32,
    pub normalized_quality: f32,
    pub passed: bool,
    pub parse_fallback: bool,
}

pub fn parse_relevance_response(text: String) -> RelevanceStageEval {
    let thresholds = resolved_thresholds();
    let raw_output = text.trim().to_string();
    let (raw_score, parse_fallback) =
        match extract_from_json_value(&raw_output, &["score", "value", "answer"])
            .and_then(|value| value.parse::<u32>().ok())
            .or_else(|| extract_first_u32(&raw_output))
        {
            Some(value) => (clamp_u32(value, 0, 10), false),
            None => (thresholds.relevance_min, true),
        };

    RelevanceStageEval {
        normalized_quality: quality_relevance(raw_score),
        passed: raw_score >= thresholds.relevance_min,
        raw_output,
        raw_score,
        parse_fallback,
    }
}

pub async fn evaluate_relevance_stage(
    config: &LlmConfig,
    task_prompt: &str,
    agent_output: &str,
) -> Result<RelevanceStageEval, ValidatorError> {
    if config.mock {
        return Ok(parse_relevance_mock_response(agent_output));
    }

    let (system, user) = build_stage_relevance_prompts(task_prompt, agent_output)?;
    let text = call_judge_raw(config, "stage_relevance", &system, &user).await?;
    Ok(parse_relevance_response(text))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_relevance_plain_number() {
        let eval = parse_relevance_response("8".to_string());
        assert_eq!(eval.raw_score, 8);
        assert!(eval.passed);
    }

    #[test]
    fn parse_relevance_json_number() {
        let eval = parse_relevance_response(r#"{"score":2}"#.to_string());
        assert_eq!(eval.raw_score, 2);
        assert!(!eval.passed);
    }

    #[test]
    fn parse_relevance_invalid_uses_neutral_fallback() {
        let eval = parse_relevance_response("somewhat relevant".to_string());
        assert_eq!(eval.raw_score, resolved_thresholds().relevance_min);
        assert!(eval.parse_fallback);
        assert!(eval.passed);
    }
}
