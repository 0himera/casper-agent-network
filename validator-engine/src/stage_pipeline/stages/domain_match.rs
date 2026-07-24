use crate::llm::call_judge_raw;
use crate::prompts::build_stage_domain_match_prompts;
use crate::stage_pipeline::stage_scoring::quality_domain;
use crate::types::{LlmConfig, ValidatorError};

use super::{extract_yes_no, parse_domain_match_mock_response};

#[derive(Debug, Clone, PartialEq)]
pub struct DomainMatchStageEval {
    pub raw_output: String,
    pub domain_matches: bool,
    pub normalized_quality: f32,
    pub passed: bool,
    pub parse_fallback: bool,
}

pub fn parse_domain_match_response(text: String) -> DomainMatchStageEval {
    let raw_output = text.trim().to_string();
    let (domain_matches, parse_fallback) = match extract_yes_no(&raw_output) {
        Some(value) => (value, false),
        None => (true, true),
    };

    DomainMatchStageEval {
        normalized_quality: quality_domain(domain_matches),
        passed: domain_matches,
        raw_output,
        domain_matches,
        parse_fallback,
    }
}

pub async fn evaluate_domain_match_stage(
    config: &LlmConfig,
    domain: &str,
    expected_domain: &str,
    task_prompt: &str,
    agent_output: &str,
) -> Result<DomainMatchStageEval, ValidatorError> {
    if config.mock {
        return Ok(parse_domain_match_mock_response(agent_output));
    }

    let (system, user) =
        build_stage_domain_match_prompts(domain, expected_domain, task_prompt, agent_output)?;
    let text = call_judge_raw(config, "stage_domain_match", &system, &user).await?;
    Ok(parse_domain_match_response(text))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_domain_match_yes_no() {
        let eval = parse_domain_match_response("yes".to_string());
        assert!(eval.domain_matches);
        assert!(eval.passed);

        let eval = parse_domain_match_response("no".to_string());
        assert!(!eval.domain_matches);
        assert!(!eval.passed);
    }

    #[test]
    fn parse_domain_match_russian_tokens() {
        let eval = parse_domain_match_response("нет".to_string());
        assert!(!eval.domain_matches);
    }

    #[test]
    fn parse_domain_match_invalid_uses_neutral_fallback() {
        let eval = parse_domain_match_response("uncertain".to_string());
        assert!(eval.domain_matches);
        assert!(eval.parse_fallback);
    }
}
