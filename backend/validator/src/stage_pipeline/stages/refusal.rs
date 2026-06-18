use crate::llm::call_judge_raw;
use crate::prompts::build_stage_refusal_prompts;
use crate::stage_pipeline::stage_scoring::quality_refusal;
use crate::types::{LlmConfig, ValidatorError};

use super::{extract_yes_no, parse_refusal_mock_response};

#[derive(Debug, Clone, PartialEq)]
pub struct RefusalStageEval {
    pub raw_output: String,
    pub is_refusal: bool,
    pub normalized_quality: f32,
    pub passed: bool,
    pub parse_fallback: bool,
}

pub fn parse_refusal_response(text: String) -> RefusalStageEval {
    let raw_output = text.trim().to_string();
    let (is_refusal, parse_fallback) = match extract_yes_no(&raw_output) {
        Some(true) => (true, false),
        Some(false) => (false, false),
        None => (false, true),
    };

    RefusalStageEval {
        normalized_quality: quality_refusal(is_refusal),
        passed: !is_refusal,
        raw_output,
        is_refusal,
        parse_fallback,
    }
}

pub async fn evaluate_refusal_stage(
    config: &LlmConfig,
    task_prompt: &str,
    agent_output: &str,
) -> Result<RefusalStageEval, ValidatorError> {
    if config.mock {
        return Ok(parse_refusal_mock_response(agent_output));
    }

    let (system, user) = build_stage_refusal_prompts(task_prompt, agent_output)?;
    let text = call_judge_raw(config, "stage_refusal", &system, &user).await?;
    Ok(parse_refusal_response(text))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_refusal_yes_no() {
        let eval = parse_refusal_response("yes".to_string());
        assert!(eval.is_refusal);
        assert!(!eval.passed);
        assert!(!eval.parse_fallback);

        let eval = parse_refusal_response("no".to_string());
        assert!(!eval.is_refusal);
        assert!(eval.passed);
    }

    #[test]
    fn parse_refusal_russian_tokens() {
        let eval = parse_refusal_response("да".to_string());
        assert!(eval.is_refusal);
    }

    #[test]
    fn parse_refusal_json_fallback() {
        let eval = parse_refusal_response(r#"{"answer":"no"}"#.to_string());
        assert!(!eval.is_refusal);
        assert!(eval.passed);
    }

    #[test]
    fn parse_refusal_invalid_uses_neutral_fallback() {
        let eval = parse_refusal_response("maybe later".to_string());
        assert!(!eval.is_refusal);
        assert!(eval.parse_fallback);
    }
}
