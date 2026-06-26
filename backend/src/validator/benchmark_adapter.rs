//! Benchmark evaluation via the same stage pipeline used by live validation.

use validator_engine::{
    LlmConfig, PipelineRunStats, StagePipelineOutput, evaluate_stage_pipeline_with_stats,
    reset_judge_call_stats,
};

use crate::config::Config;

use super::stage_adapter::{format_validator_eval_log, map_config};

const BASE_DEFI_PRICE_MOTES: u64 = 5_000_000_000;
const BASE_RWA_PRICE_MOTES: u64 = 15_000_000_000;

/// Platform domain group for benchmark pricing and documentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BenchmarkDomainGroup {
    DeFi,
    Rwa,
    Other,
}

/// Maps a domain string to one of three platform groups: DeFi, RWA, or other.
pub(crate) fn benchmark_domain_group(domain: &str) -> BenchmarkDomainGroup {
    match domain {
        "rwa" => BenchmarkDomainGroup::Rwa,
        "defi" => BenchmarkDomainGroup::DeFi,
        _ => BenchmarkDomainGroup::Other,
    }
}

/// Result of evaluating one benchmark skill through the stage pipeline.
#[derive(Debug, Clone)]
pub struct BenchmarkSkillEval {
    pub score: u32,
    pub recommended_price_motes: u64,
    pub rubric_json: serde_json::Value,
}

fn benchmark_base_price_motes(domain: &str) -> u64 {
    match benchmark_domain_group(domain) {
        BenchmarkDomainGroup::Rwa => BASE_RWA_PRICE_MOTES,
        BenchmarkDomainGroup::DeFi | BenchmarkDomainGroup::Other => BASE_DEFI_PRICE_MOTES,
    }
}

fn speed_multiplier(processing_time_ms: u64) -> f64 {
    if processing_time_ms < 5_000 {
        1.2
    } else if processing_time_ms < 15_000 {
        1.0
    } else if processing_time_ms < 30_000 {
        0.8
    } else {
        0.6
    }
}

pub(crate) fn benchmark_recommended_price_motes(
    domain: &str,
    total: u32,
    processing_time_ms: u64,
) -> u64 {
    let base = benchmark_base_price_motes(domain) as f64;
    let score_factor = total as f64 / 100.0;
    let multiplier = speed_multiplier(processing_time_ms);
    (base * score_factor * multiplier) as u64
}

/// Stage pipeline config for benchmark, intentionally identical to live stage validation.
pub fn build_benchmark_llm_config(config: &Config) -> LlmConfig {
    map_config(config)
}

fn build_rubric_json(output: &StagePipelineOutput, stats: &PipelineRunStats) -> serde_json::Value {
    serde_json::json!({
        "pipeline": "stage",
        "verdict": output.verdict.as_label(),
        "total": output.total,
        "explanation": output.explanation,
        "criteria": output.criteria,
        "stages": output.stages,
        "stats": stats,
    })
}

/// Log once per benchmark run if SerpAPI is missing in non-mock mode.
pub fn warn_serpapi_if_needed(llm: &LlmConfig) {
    if llm.mock {
        return;
    }
    if !llm.factuality_enabled.unwrap_or(false) {
        return;
    }
    if llm.serpapi_api_key.as_ref().is_some_and(|k| !k.is_empty()) {
        return;
    }
    eprintln!(
        "benchmark warning: SERPAPI_API_KEY is unset; factuality will use empty search snippets and scores may be depressed"
    );
}

/// Evaluate a benchmark domain via the shared stage pipeline.
pub async fn evaluate_benchmark_skill_stage(
    domain: &str,
    prompt: &str,
    agent_output: &str,
    processing_time_ms: u64,
    config: &Config,
) -> Option<BenchmarkSkillEval> {
    reset_judge_call_stats();
    let llm = build_benchmark_llm_config(config);
    warn_serpapi_if_needed(&llm);

    match evaluate_stage_pipeline_with_stats(domain, prompt, agent_output, &llm).await {
        Ok((output, stats)) => {
            println!("{}", format_validator_eval_log(&stats));
            Some(BenchmarkSkillEval {
                score: output.total,
                recommended_price_motes: benchmark_recommended_price_motes(
                    domain,
                    output.total,
                    processing_time_ms,
                ),
                rubric_json: build_rubric_json(&output, &stats),
            })
        }
        Err(err) => {
            eprintln!(
                "stage benchmark eval failed for skill '{}': {}",
                domain, err
            );
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ValidatorPipeline;

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
            validator_pipeline: ValidatorPipeline::Legacy,
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
    fn benchmark_domain_group_classifies_defi_rwa_other() {
        assert_eq!(benchmark_domain_group("defi"), BenchmarkDomainGroup::DeFi);
        assert_eq!(benchmark_domain_group("rwa"), BenchmarkDomainGroup::Rwa);
        assert_eq!(benchmark_domain_group("other"), BenchmarkDomainGroup::Other);
    }

    #[test]
    fn benchmark_base_price_motes_rwa_vs_defi() {
        assert_eq!(benchmark_base_price_motes("rwa"), BASE_RWA_PRICE_MOTES);
        assert_eq!(benchmark_base_price_motes("defi"), BASE_DEFI_PRICE_MOTES);
        assert_eq!(benchmark_base_price_motes("other"), BASE_DEFI_PRICE_MOTES);
    }

    #[test]
    fn benchmark_recommended_price_applies_score_and_speed() {
        let fast = benchmark_recommended_price_motes("defi", 100, 4_000);
        assert_eq!(fast, 6_000_000_000);

        let rwa = benchmark_recommended_price_motes("rwa", 100, 10_000);
        assert_eq!(rwa, BASE_RWA_PRICE_MOTES);
    }

    #[test]
    fn build_benchmark_llm_config_matches_live_stage_policy() {
        unsafe {
            std::env::set_var("VALIDATOR_MOCK_LLM", "1");
            std::env::set_var("VALIDATOR_FACTUALITY", "1");
        }
        let llm = build_benchmark_llm_config(&sample_config());
        assert!(llm.mock);
        assert_eq!(llm.factuality_enabled, Some(true));
        unsafe {
            std::env::remove_var("VALIDATOR_MOCK_LLM");
            std::env::remove_var("VALIDATOR_FACTUALITY");
        }
    }

    #[tokio::test]
    async fn evaluate_benchmark_skill_stage_mock_good_output() {
        unsafe {
            std::env::set_var("VALIDATOR_MOCK_LLM", "1");
        }

        let config = sample_config();
        let prompt = "Allocate 10,000 CSPR across Casper liquidity pools.";
        let agent_output = "Recommended allocation across cspr-usdt and cspr-eth pools with fee-adjusted APY analysis and impermanent loss reasoning.";

        let eval = evaluate_benchmark_skill_stage("defi", prompt, agent_output, 4_000, &config)
            .await
            .expect("mock eval");

        assert!(eval.score <= 100);
        assert_eq!(eval.rubric_json["pipeline"], "stage");
        assert!(eval.rubric_json["criteria"].is_array());
        assert!(eval.rubric_json["stages"].is_array());
        assert!(eval.rubric_json["verdict"].is_string());
        assert!(eval.recommended_price_motes > 0);

        unsafe {
            std::env::remove_var("VALIDATOR_MOCK_LLM");
        }
    }

    #[tokio::test]
    async fn evaluate_benchmark_skill_stage_early_exit_still_returns_result() {
        unsafe {
            std::env::set_var("VALIDATOR_MOCK_LLM", "1");
        }

        let config = sample_config();
        let eval = evaluate_benchmark_skill_stage("defi", "task", "short", 1_000, &config)
            .await
            .expect("gate failure still returns Some");

        assert_eq!(eval.score, 0);
        assert_ne!(eval.rubric_json["verdict"], "factual");

        unsafe {
            std::env::remove_var("VALIDATOR_MOCK_LLM");
        }
    }

    #[tokio::test]
    async fn evaluate_benchmark_skill_stage_refusal_mock() {
        unsafe {
            std::env::set_var("VALIDATOR_MOCK_LLM", "1");
        }

        let config = sample_config();
        let eval = evaluate_benchmark_skill_stage(
            "defi",
            "task",
            "MOCK_REFUSAL: I cannot fulfill this request.",
            5_000,
            &config,
        )
        .await
        .expect("refusal eval");

        assert_eq!(eval.score, 0);
        assert_eq!(eval.rubric_json["verdict"], "refusal");

        unsafe {
            std::env::remove_var("VALIDATOR_MOCK_LLM");
        }
    }
}
