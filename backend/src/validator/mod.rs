pub mod benchmark_adapter;
pub mod exam_adapter;
pub mod llm_judge;
pub mod stage_adapter;

use crate::config::Config;
use validator_engine::LlmConfig;

// Live `/execute` uses `evaluate_task()`; switch via `VALIDATOR_PIPELINE=stage|legacy`.
pub use benchmark_adapter::{
    BenchmarkSkillEval, build_benchmark_llm_config, evaluate_benchmark_skill_stage,
    warn_serpapi_if_needed,
};
pub use exam_adapter::evaluate_exam_task;
pub use llm_judge::evaluate_task;

/// Maps backend `Config` to `validator-engine` `LlmConfig`.
pub fn map_base_config(config: &Config) -> LlmConfig {
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
