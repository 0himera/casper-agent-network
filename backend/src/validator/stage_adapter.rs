use validator_engine::{
    LlmConfig, PipelineRunStats, StagePipelineOutput, evaluate_stage_pipeline_with_stats,
    judge_call_count, reset_judge_call_stats,
};

use crate::config::Config;

use super::llm_judge::{EvaluationResult, RubricScores, recommended_price_motes};

/// Maps backend `Config` to `validator-engine` `LlmConfig` for the stage pipeline.
pub fn map_config(config: &Config) -> LlmConfig {
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

pub fn build_validator_audit(
    output: &StagePipelineOutput,
    stats: &PipelineRunStats,
) -> Option<serde_json::Value> {
    Some(serde_json::json!({
        "pipeline": "stage",
        "stats": stats,
        "output": output,
    }))
}

pub fn format_validator_eval_log(stats: &PipelineRunStats) -> String {
    let stages = stats
        .stage_ms
        .iter()
        .map(|timing| format!("{}:{}ms", timing.id.as_str(), timing.elapsed_ms))
        .collect::<Vec<_>>()
        .join(",");

    format!(
        "validator_eval pipeline=stage factuality_enabled={} factuality_ran={} verdict={} total={} llm_calls={} search_hits={} search_misses={} stages={}",
        stats.factuality_enabled,
        stats.factuality_ran,
        stats.verdict.as_label(),
        stats.total,
        stats.llm_calls,
        stats.search_cache_hits,
        stats.search_cache_misses,
        stages
    )
}

pub fn map_stage_output_to_evaluation(
    output: StagePipelineOutput,
    stats: &PipelineRunStats,
    domain: &str,
    processing_time_ms: u64,
) -> EvaluationResult {
    let validator_audit = build_validator_audit(&output, stats);
    let total = output.total;
    let reasoning = output.explanation;

    EvaluationResult {
        scores: placeholder_rubric_scores(),
        total,
        reasoning,
        recommended_price_motes: recommended_price_motes(domain, total, processing_time_ms),
        validator_audit,
    }
}

/// Stage pipeline path for live `/execute` (N4 cutover).
pub async fn evaluate_task_stage(
    domain: &str,
    task_prompt: &str,
    agent_result: &str,
    processing_time_ms: u64,
    config: &Config,
) -> Result<EvaluationResult, Box<dyn std::error::Error + Send + Sync>> {
    reset_judge_call_stats();
    let llm_config = map_config(config);

    let (output, stats) =
        evaluate_stage_pipeline_with_stats(domain, task_prompt, agent_result, &llm_config)
            .await
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })?;

    tracing::info!("{}", format_validator_eval_log(&stats));
    let _ = judge_call_count();

    Ok(map_stage_output_to_evaluation(
        output,
        &stats,
        domain,
        processing_time_ms,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ValidatorPipeline;
    use validator_engine::{PipelineVerdict, StageId, StageTiming};

    fn sample_config(pipeline: ValidatorPipeline) -> Config {
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
            validator_pipeline: pipeline,
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
            exam_dispatch_loop_enabled: false,
            exam_dispatch_loop_interval_secs: 300,
            exam_selection_mode: crate::config::ExamSelectionMode::Bucket,
            exam_urgency_base_prob: 0.1,
            exam_urgency_task_weight: 0.05,
            exam_urgency_variance_weight: 0.2,
            exam_urgency_recent_verdicts: 5,
            exam_smoothed_ema_alpha: 0.3,
            exam_leaderboard_use_smoothed: false,
        }
    }

    #[test]
    fn map_config_reads_factuality_flag_from_env() {
        temp_env::with_var("VALIDATOR_FACTUALITY", Some("1"), || {
            let llm = map_config(&sample_config(ValidatorPipeline::Stage));
            assert_eq!(llm.factuality_enabled, Some(true));
        });
    }

    #[test]
    fn map_stage_output_preserves_pricing_formula() {
        let output = StagePipelineOutput {
            verdict: PipelineVerdict::Factual,
            stages: vec![],
            criteria: vec![],
            total: 80,
            explanation: "Good answer".to_string(),
        };
        let stats = PipelineRunStats {
            pipeline: "stage".to_string(),
            factuality_enabled: false,
            factuality_ran: false,
            verdict: PipelineVerdict::Factual,
            total: 80,
            llm_calls: 4,
            search_cache_hits: 0,
            search_cache_misses: 0,
            stage_ms: vec![StageTiming {
                id: StageId::Refusal,
                elapsed_ms: 10,
            }],
        };

        let eval = map_stage_output_to_evaluation(output, &stats, "defi_analysis", 4000);
        assert_eq!(eval.total, 80);
        assert_eq!(eval.reasoning, "Good answer");
        assert!(eval.recommended_price_motes > 0);
        assert!(eval.validator_audit.is_some());
    }

    #[test]
    fn build_validator_audit_contains_output_and_stats() {
        let output = StagePipelineOutput {
            verdict: PipelineVerdict::Factual,
            stages: vec![],
            criteria: vec![],
            total: 90,
            explanation: "ok".to_string(),
        };
        let stats = PipelineRunStats {
            pipeline: "stage".to_string(),
            factuality_enabled: true,
            factuality_ran: false,
            verdict: PipelineVerdict::Factual,
            total: 90,
            llm_calls: 3,
            search_cache_hits: 1,
            search_cache_misses: 2,
            stage_ms: vec![],
        };

        let audit = build_validator_audit(&output, &stats).expect("audit json");
        assert_eq!(audit["pipeline"], "stage");
        assert_eq!(audit["output"]["total"], 90);
        assert_eq!(audit["stats"]["llm_calls"], 3);
    }

    #[tokio::test]
    async fn evaluate_task_stage_mock_returns_audit_json() {
        temp_env::async_with_vars([("VALIDATOR_MOCK_LLM", Some("1"))], async {
            let config = sample_config(ValidatorPipeline::Stage);
            let result = evaluate_task_stage(
                "defi_analysis",
                "Analyze yield",
                "Recommended allocation across cspr-usdt and cspr-eth pools with fee-adjusted APY.",
                4000,
                &config,
            )
            .await
            .expect("stage mock eval");

            assert!(result.total <= 100);
            assert!(!result.reasoning.is_empty());
            assert!(result.validator_audit.is_some());
        })
        .await;
    }

    #[tokio::test]
    async fn evaluate_task_stage_factuality_mock_includes_factuality_stage() {
        temp_env::async_with_vars(
            [
                ("VALIDATOR_MOCK_LLM", Some("1")),
                ("VALIDATOR_FACTUALITY", Some("1")),
            ],
            async {
                let config = sample_config(ValidatorPipeline::Stage);
                let result = evaluate_task_stage(
                    "defi_analysis",
                    "Analyze yield",
                    "MOCK_FACT_SUPPORTED: CSPR can be staked on the network. DeFi pools expose users to smart contract risk and should be evaluated carefully before allocation because capital can be lost due to exploits or market volatility in live markets.",
                    4000,
                    &config,
                )
                .await
                .expect("stage factuality mock eval");

                let audit = result.validator_audit.expect("audit json");
                assert_eq!(audit["stats"]["factuality_enabled"], true);
            },
        )
        .await;
    }
}
