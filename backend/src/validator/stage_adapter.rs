use validator_engine::{
    LlmConfig, PipelineRunStats, StagePipelineOutput, evaluate_stage_pipeline_with_stats,
    judge_call_count, reset_judge_call_stats,
};

use crate::config::Config;

use super::llm_judge::{EvaluationResult, RubricScores, recommended_price_motes};

/// Maps backend `Config` to `validator-engine` `LlmConfig` for the stage pipeline.
pub fn map_config(config: &Config) -> LlmConfig {
    let mock = std::env::var("VALIDATOR_MOCK_LLM")
        .ok()
        .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"));

    fn env(key: &str) -> Option<String> {
        std::env::var(key).ok().filter(|v| !v.is_empty())
    }

    let judge_cascade = env("VALIDATOR_JUDGE_CASCADE").and_then(|v| match v.as_str() {
        "local_first" => Some(validator_engine::JudgeCascadeMode::LocalFirst),
        "api_first" => Some(validator_engine::JudgeCascadeMode::ApiFirst),
        _ => None,
    });

    let judge_timeout_ms = env("VALIDATOR_JUDGE_TIMEOUT_MS").and_then(|v| v.parse().ok());

    let judge_self_consistency =
        env("VALIDATOR_JUDGE_SELF_CONSISTENCY").map(|v| v == "1" || v.eq_ignore_ascii_case("true"));

    let factuality_enabled =
        env("VALIDATOR_FACTUALITY").map(|v| v == "1" || v.eq_ignore_ascii_case("true"));

    let mut custom_url = config.validator_url.clone();
    let custom_api_key = config
        .validator_api_key
        .clone()
        .or(config.fireworks_api_key.clone());
    let custom_model = config
        .validator_model
        .clone()
        .or(config.fireworks_model.clone());

    if custom_url.is_none() && custom_api_key.is_some() {
        custom_url = Some("https://api.fireworks.ai/inference/v1".to_string());
    }

    LlmConfig {
        cloudflare_account_id: config.cloudflare_account_id.clone(),
        cloudflare_api_token: config.cloudflare_api_token.clone(),
        openai_api_key: config.openai_api_key.clone(),
        openai_base_url: env("OPENAI_BASE_URL"),
        claude_api_key: config.claude_api_key.clone(),
        ollama_url: config.ollama_url.clone(),
        ollama_model: config.ollama_model.clone(),
        custom_url,
        custom_api_key,
        custom_model,
        provider: config.validator_provider.clone(),
        mock,
        factuality_enabled,
        serpapi_api_key: env("SERPAPI_API_KEY"),
        judge_cascade,
        judge_timeout_ms,
        judge_self_consistency,
    }
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

    println!("{}", format_validator_eval_log(&stats));
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
        }
    }

    #[test]
    fn map_config_reads_factuality_flag_from_env() {
        unsafe {
            std::env::set_var("VALIDATOR_FACTUALITY", "1");
        }

        let llm = map_config(&sample_config(ValidatorPipeline::Stage));
        assert_eq!(llm.factuality_enabled, Some(true));

        unsafe {
            std::env::remove_var("VALIDATOR_FACTUALITY");
        }
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
        unsafe {
            std::env::set_var("VALIDATOR_MOCK_LLM", "1");
        }

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

        unsafe {
            std::env::remove_var("VALIDATOR_MOCK_LLM");
        }
    }

    #[tokio::test]
    async fn evaluate_task_stage_factuality_mock_includes_factuality_stage() {
        unsafe {
            std::env::set_var("VALIDATOR_MOCK_LLM", "1");
            std::env::set_var("VALIDATOR_FACTUALITY", "1");
        }

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

        unsafe {
            std::env::remove_var("VALIDATOR_MOCK_LLM");
            std::env::remove_var("VALIDATOR_FACTUALITY");
        }
    }
}
