mod fixture_schema;
mod gates;
mod grader;
pub mod harness;
mod llm;
mod prompts;
mod rubric;
mod scoring;
mod tools;
mod types;

pub use fixture_schema::{is_fixture_envelope, validate_fixture};
pub use harness::load_skill_fixture;
pub use types::{
    CriterionDef, CriterionEval, CriterionKind, GraderMode, GraderOptions, JudgeCascadeMode,
    JudgeProvider, LlmConfig, SkillId, SoftLabel, ToolResult, ValidationInput, ValidationOutput,
    ValidatorError, Verdict,
};

pub use crate::llm::{judge_call_count, last_judge_provider_used, reset_judge_call_stats};

pub async fn evaluate(
    input: ValidationInput,
    config: &LlmConfig,
) -> Result<ValidationOutput, ValidatorError> {
    evaluate_with_options(input, config, &GraderOptions::default()).await
}

pub async fn evaluate_with_options(
    input: ValidationInput,
    config: &LlmConfig,
    options: &GraderOptions,
) -> Result<ValidationOutput, ValidatorError> {
    grader::evaluate_with_options(&input, config, options).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_input() -> ValidationInput {
        let fixture = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/defi_yield_routing.json"),
        )
        .expect("fixture");
        ValidationInput {
            skill: SkillId::DefiYieldRouting,
            task_prompt: "Allocate 10k CSPR".to_string(),
            agent_output: "Allocate 4,000 CSPR to cspr-usdt (8.2% APY, high TVL), 3,500 CSPR to cspr-eth (6.1% APY, moderate IL), and 2,500 CSPR to cspr-wbtc (11.4% APY, higher IL risk). Total: 10,000 CSPR. Network gas fees (~2.5 CSPR per swap) included. IL analysis shows cspr-usdt lowest volatility exposure.".to_string(),
            fixture: serde_json::from_str(&fixture).expect("fixture json"),
            processing_time_ms: 10_000,
        }
    }

    #[tokio::test]
    async fn evaluate_smoke_for_defi_yield_routing() {
        let config = LlmConfig::from_env();
        let output = evaluate(sample_input(), &config)
            .await
            .expect("evaluate ok");

        assert_eq!(output.criteria.len(), 5);
        assert!(output.total <= 100);
        if config.mock {
            assert_eq!(output.verdict, Verdict::Satisfied);
            assert_eq!(output.total, 100);
            assert!(output.explanation.contains("F3 mock evaluation"));
        } else {
            assert!(!output.explanation.is_empty());
        }
    }
}
