mod gates;
mod grader;
pub mod harness;
mod llm;
mod rubric;
mod scoring;
mod tools;
mod types;

pub use types::{
    CriterionDef, CriterionEval, CriterionKind, GraderMode, GraderOptions, LlmConfig, SkillId,
    SoftLabel, ToolResult, ValidationInput, ValidationOutput, ValidatorError, Verdict,
};

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
        ValidationInput {
            skill: SkillId::DefiYieldRouting,
            task_prompt: "Allocate 10k CSPR".to_string(),
            agent_output:
                "Recommended allocation across cspr-usdt and cspr-eth pools with fee-adjusted APY."
                    .to_string(),
            fixture: serde_json::json!({
                "amount_cspr": 10000,
                "gas_price_motes": 2_500_000_000_i64,
                "pools": []
            }),
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
