use validator_engine::{
    ExamPipelineOutput, ExamVerificationPolicy, evaluate_exam_pipeline,
    resolve_exam_verification_policy,
};

use crate::config::Config;

use super::llm_judge::{EvaluationResult, RubricScores, recommended_price_motes};

/// Maps backend `Config` to `validator-engine` `LlmConfig` for the exam pipeline.
pub fn map_config(config: &Config) -> validator_engine::LlmConfig {
    super::map_base_config(config)
}

fn placeholder_rubric_scores() -> RubricScores {
    RubricScores {
        accuracy_or_safety: 0,
        depth_or_quality: 0,
        sources_or_testing: 0,
        actionability_or_explanation: 0,
        presentation: 0,
    }
}

pub fn build_validator_audit(output: &ExamPipelineOutput) -> Option<serde_json::Value> {
    serde_json::to_value(&output.audit).ok()
}

pub fn format_validator_eval_log(exam_id: &str, output: &ExamPipelineOutput) -> String {
    format!(
        "validator_eval pipeline=exam exam_id={} verdict={} total={} compare_mode={} llm_fallback_used={}",
        exam_id,
        output.verdict.as_label(),
        output.total,
        output.audit.compare_mode,
        output.audit.llm_fallback_used
    )
}

pub fn map_exam_output_to_evaluation(
    output: ExamPipelineOutput,
    domain: &str,
    processing_time_ms: u64,
) -> EvaluationResult {
    let total = output.total;
    let reasoning = output.explanation.clone();
    let validator_audit = build_validator_audit(&output);

    EvaluationResult {
        scores: placeholder_rubric_scores(),
        total,
        reasoning,
        recommended_price_motes: recommended_price_motes(domain, total, processing_time_ms),
        validator_audit,
    }
}

/// Exam pipeline path for live validation when `exam_assignments` exists.
pub async fn evaluate_exam_task(
    exam_id: &str,
    domain: &str,
    task_prompt: &str,
    agent_result: &str,
    expected_answer_canonical: &str,
    source_metadata: Option<&serde_json::Value>,
    processing_time_ms: u64,
    config: &Config,
) -> Result<EvaluationResult, Box<dyn std::error::Error + Send + Sync>> {
    let llm_config = map_config(config);
    let verification_policy = resolve_exam_verification_policy(source_metadata);

    let output = evaluate_exam_pipeline(
        &llm_config,
        exam_id,
        task_prompt,
        agent_result,
        expected_answer_canonical,
        verification_policy,
    )
    .await
    .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })?;

    tracing::info!("{}", format_validator_eval_log(exam_id, &output));

    Ok(map_exam_output_to_evaluation(
        output,
        domain,
        processing_time_ms,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ValidatorPipeline;
    use validator_engine::{ExamVerdict, evaluate_exam_pipeline_mock};

    fn sample_config() -> Config {
        Config {
            database_url: "mysql://localhost".to_string(),
            port: 3000,
            openai_api_key: None,
            claude_api_key: None,
            ollama_url: None,
            ollama_model: None,
            internal_service_key: None,
            cloudflare_account_id: None,
            cloudflare_api_token: None,
            fireworks_api_key: None,
            fireworks_model: None,
            validator_url: None,
            validator_api_key: None,
            validator_model: None,
            validator_provider: None,
            validator_pipeline: ValidatorPipeline::Stage,
            admin_account: String::new(),
            exam_weight: 300,
            exam_dispatch_prob_audit: 0.2,
            exam_dispatch_prob_rehab: 0.5,
            exam_max_per_agent_per_period: 1,
            exam_dispatch_period_hours: 24,
            exam_rehab_score_threshold: 0,
            exam_audit_active_jobs_threshold: 2,
            exam_dispatch_budget_motes: 5_000_000_000,
            exam_dispatch_creator_public_key: String::new(),
            exam_llm_equality: false,
        }
    }

    #[test]
    fn map_exam_output_passed_total_is_100() {
        let output = evaluate_exam_pipeline_mock(
            "exam-template-1",
            "Compute yield",
            "ANSWER: 2845678901.25 cspr",
            "2845678901.25 cspr",
        );
        assert_eq!(output.verdict, ExamVerdict::Passed);
        assert_eq!(output.total, 100);

        let eval = map_exam_output_to_evaluation(output, "defi_analysis", 4000);
        assert_eq!(eval.total, 100);
        assert!(!eval.reasoning.is_empty());
        assert!(eval.validator_audit.is_some());
    }

    #[test]
    fn map_exam_output_failed_total_is_0() {
        let output = evaluate_exam_pipeline_mock(
            "exam-template-1",
            "Compute yield",
            "ANSWER: 1 usd",
            "2845678901.25 cspr",
        );
        assert_eq!(output.verdict, ExamVerdict::Failed);

        let eval = map_exam_output_to_evaluation(output, "defi_analysis", 4000);
        assert_eq!(eval.total, 0);
    }

    #[test]
    fn build_validator_audit_contains_exam_pipeline_and_verdict() {
        let output = evaluate_exam_pipeline_mock(
            "exam-template-1",
            "Compute yield",
            "ANSWER: 2845678901.25 cspr",
            "2845678901.25 cspr",
        );
        let audit = build_validator_audit(&output).expect("audit json");
        assert_eq!(audit["pipeline"], "exam");
        assert_eq!(audit["verdict"], "passed");
        assert_eq!(audit["exam_id"], "exam-template-1");
        assert_eq!(audit["compare_mode"], "exact_match");
        assert_eq!(audit["llm_fallback_used"], false);
        assert_eq!(audit["answer_verification_mode"], "exact_then_llm");
    }

    #[tokio::test]
    async fn evaluate_exam_task_llm_fallback_audit_shape() {
        temp_env::async_with_vars(
            [
                ("VALIDATOR_MOCK_LLM", Some("1")),
                ("EXAM_LLM_EQUALITY", Some("1")),
            ],
            async {
                let mut config = sample_config();
                config.exam_llm_equality = true;
                let result = evaluate_exam_task(
                    "exam-template-1",
                    "defi_analysis",
                    "Compute yield",
                    "ANSWER: mock_equality_yes about twelve thousand usd",
                    "12345.67 usd",
                    None,
                    4000,
                    &config,
                )
                .await
                .expect("exam mock eval");

                assert_eq!(result.total, 100);
                let audit = result.validator_audit.expect("audit");
                assert_eq!(audit["compare_mode"], "llm_fallback_match");
                assert_eq!(audit["llm_fallback_used"], true);
                assert_eq!(audit["answer_verification_mode"], "exact_then_llm");
            },
        )
        .await;
    }

    #[tokio::test]
    async fn evaluate_exam_task_llm_fallback_miss_audit_shape() {
        temp_env::async_with_vars(
            [
                ("VALIDATOR_MOCK_LLM", Some("1")),
                ("EXAM_LLM_EQUALITY", Some("1")),
            ],
            async {
                let mut config = sample_config();
                config.exam_llm_equality = true;
                let result = evaluate_exam_task(
                    "exam-template-1",
                    "defi_analysis",
                    "Compute yield",
                    "ANSWER: mock_equality_no about twelve thousand usd",
                    "12345.67 usd",
                    None,
                    4000,
                    &config,
                )
                .await
                .expect("exam mock eval");

                assert_eq!(result.total, 0);
                let audit = result.validator_audit.expect("audit");
                assert_eq!(audit["compare_mode"], "llm_fallback_miss");
                assert_eq!(audit["llm_fallback_used"], true);
                assert_eq!(audit["llm_raw"], "NO");
            },
        )
        .await;
    }

    #[tokio::test]
    async fn evaluate_exam_task_returns_err_when_no_llm_provider() {
        temp_env::async_with_vars([("VALIDATOR_MOCK_LLM", Some("0"))], async {
            let mut config = sample_config();
            config.exam_llm_equality = true;
            let result = evaluate_exam_task(
                "exam-template-1",
                "defi_analysis",
                "Compute yield",
                "ANSWER: 999 usd",
                "12345.67 usd",
                None,
                4000,
                &config,
            )
            .await;

            assert!(result.is_err(), "missing provider must not silently pass");
        })
        .await;
    }

    #[tokio::test]
    async fn evaluate_exam_task_llm_first_audit_shape() {
        temp_env::async_with_vars(
            [
                ("VALIDATOR_MOCK_LLM", Some("1")),
                ("EXAM_LLM_EQUALITY", Some("1")),
            ],
            async {
                let mut config = sample_config();
                config.exam_llm_equality = true;
                let metadata = serde_json::json!({
                    "answer_verification_mode": "llm_first",
                    "verification_reason": "RWA NAV wording varies"
                });
                let result = evaluate_exam_task(
                    "exam-rwa-tokenized-tbill-nav-2024-q3",
                    "rwa_valuation",
                    "RWA NAV memo",
                    "ANSWER: mock_equality_yes one hundred dollars and forty-seven cents per share",
                    "100.47 usd",
                    Some(&metadata),
                    4000,
                    &config,
                )
                .await
                .expect("exam mock eval");

                assert_eq!(result.total, 100);
                let audit = result.validator_audit.expect("audit");
                assert_eq!(audit["compare_mode"], "llm_first_match");
                assert_eq!(audit["llm_fallback_used"], false);
                assert_eq!(audit["answer_verification_mode"], "llm_first");
            },
        )
        .await;
    }
}
