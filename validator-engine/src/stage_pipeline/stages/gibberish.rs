use crate::llm::call_judge_raw;
use crate::prompts::build_stage_gibberish_prompts;
use crate::stage_pipeline::stage_scoring::{quality_gibberish, resolved_thresholds};
use crate::types::{LlmConfig, ValidatorError};

use super::{clamp_u32, extract_first_u32, extract_from_json_value, parse_gibberish_mock_response};

#[derive(Debug, Clone, PartialEq)]
pub struct GibberishStageEval {
    pub raw_output: String,
    pub raw_score: u32,
    pub normalized_quality: f32,
    pub passed: bool,
    pub parse_fallback: bool,
}

pub fn parse_gibberish_response(text: String) -> GibberishStageEval {
    let thresholds = resolved_thresholds();
    let raw_output = text.trim().to_string();
    let (raw_score, parse_fallback) =
        match extract_from_json_value(&raw_output, &["score", "value", "answer"])
            .and_then(|value| value.parse::<u32>().ok())
            .or_else(|| extract_first_u32(&raw_output))
        {
            Some(value) => (clamp_u32(value, 1, 5), false),
            None => (thresholds.gibberish_min, true),
        };

    GibberishStageEval {
        normalized_quality: quality_gibberish(raw_score),
        passed: raw_score >= thresholds.gibberish_min,
        raw_output,
        raw_score,
        parse_fallback,
    }
}

pub async fn evaluate_gibberish_stage(
    config: &LlmConfig,
    task_prompt: &str,
    agent_output: &str,
) -> Result<GibberishStageEval, ValidatorError> {
    if config.mock {
        return Ok(parse_gibberish_mock_response(agent_output));
    }

    let (system, user) = build_stage_gibberish_prompts(task_prompt, agent_output)?;
    let text = call_judge_raw(config, "stage_gibberish", &system, &user).await?;
    Ok(parse_gibberish_response(text))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_gibberish_plain_number() {
        let eval = parse_gibberish_response("4".to_string());
        assert_eq!(eval.raw_score, 4);
        assert!(eval.passed);
    }

    #[test]
    fn parse_gibberish_json_number() {
        let eval = parse_gibberish_response(r#"{"score":1}"#.to_string());
        assert_eq!(eval.raw_score, 1);
        assert!(!eval.passed);
    }

    #[test]
    fn parse_gibberish_prose_with_number() {
        let eval = parse_gibberish_response("Meaningfulness score: 2".to_string());
        assert_eq!(eval.raw_score, 2);
    }

    #[test]
    fn parse_gibberish_invalid_uses_neutral_fallback() {
        let eval = parse_gibberish_response("unclear".to_string());
        assert_eq!(eval.raw_score, resolved_thresholds().gibberish_min);
        assert!(eval.parse_fallback);
        assert!(eval.passed);
    }
}
