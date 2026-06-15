use validator_engine::{
    evaluate_with_options, load_skill_fixture, validate_fixture, GraderOptions, LlmConfig,
    SkillId, ValidationInput, ValidationOutput,
};

use crate::config::Config;

use super::skill::map_skill;

/// Результат попытки оценить skill через v2-движок.
#[derive(Debug)]
pub enum V2Outcome {
    /// v2 успешно оценил skill.
    Ok(ValidationOutput),
    /// Этот skill не имеет v2-рубрики (например "code_review"). Benchmark пропускает такой skill.
    Unsupported,
    /// Inline fixture не прошёл JSON Schema.
    FixtureInvalid(String),
    /// Не найден fixture-файл для skill. Benchmark пропускает такой skill.
    FixtureMissing(String),
    /// Реальная ошибка движка (LLM/parse/consistency). Caller обрабатывает как ошибку оценки.
    EngineError(String),
}

fn map_config(config: &Config) -> LlmConfig {
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

    let judge_self_consistency = env("VALIDATOR_JUDGE_SELF_CONSISTENCY").map(|v| {
        v == "1" || v.eq_ignore_ascii_case("true")
    });

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
        openai_base_url: None,
        claude_api_key: config.claude_api_key.clone(),
        ollama_url: config.ollama_url.clone(),
        ollama_model: config.ollama_model.clone(),
        custom_url,
        custom_api_key,
        custom_model,
        provider: config.validator_provider.clone(),
        mock,
        judge_cascade,
        judge_timeout_ms,
        judge_self_consistency,
    }
}

fn resolve_fixture(
    skill_id: SkillId,
    fixture: Option<serde_json::Value>,
) -> Result<serde_json::Value, V2Outcome> {
    match fixture {
        Some(value) => validate_fixture(skill_id, &value)
            .map_err(|e| V2Outcome::FixtureInvalid(e.to_string())),
        None => load_skill_fixture(skill_id).map_err(V2Outcome::FixtureMissing),
    }
}

pub async fn evaluate_task_v2(
    skill: &str,
    task_prompt: &str,
    agent_output: &str,
    processing_time_ms: u64,
    fixture: Option<serde_json::Value>,
    config: &Config,
) -> V2Outcome {
    let skill_id = match map_skill(skill) {
        Some(s) => s,
        None => return V2Outcome::Unsupported,
    };

    let fixture = match resolve_fixture(skill_id, fixture) {
        Ok(f) => f,
        Err(outcome) => return outcome,
    };

    let input = ValidationInput {
        skill: skill_id,
        task_prompt: task_prompt.to_string(),
        agent_output: agent_output.to_string(),
        fixture,
        processing_time_ms,
    };

    match evaluate_with_options(input, &map_config(config), &GraderOptions::f3()).await {
        Ok(output) => V2Outcome::Ok(output),
        Err(e) => V2Outcome::EngineError(e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_skill_supports_v2_and_legacy_alias() {
        assert_eq!(map_skill("defi_yield_routing"), Some(SkillId::DefiYieldRouting));
        assert_eq!(map_skill("defi_analysis"), Some(SkillId::DefiYieldRouting));
        assert_eq!(map_skill("defi_protocol_risk"), Some(SkillId::DefiProtocolRisk));
        assert_eq!(map_skill("rwa_appraisal"), Some(SkillId::RwaAppraisal));
        assert_eq!(map_skill("rwa_compliance"), Some(SkillId::RwaCompliance));
        assert_eq!(map_skill("code_review"), None);
    }

    #[test]
    fn map_config_copies_fields_from_backend_config() {
        let config = Config {
            database_url: "mysql://localhost".to_string(),
            port: 3000,
            openai_api_key: Some("sk-test".to_string()),
            claude_api_key: None,
            ollama_url: None,
            ollama_model: None,
            cloudflare_account_id: Some("cf-id".to_string()),
            cloudflare_api_token: Some("cf-token".to_string()),
            fireworks_api_key: None,
            fireworks_model: None,
            validator_url: None,
            validator_api_key: None,
            validator_model: None,
            validator_provider: None,
            admin_account: String::new(),
        };

        let llm = map_config(&config);
        assert_eq!(llm.openai_api_key.as_deref(), Some("sk-test"));
        assert_eq!(llm.cloudflare_account_id.as_deref(), Some("cf-id"));
        assert_eq!(llm.openai_base_url, None);
    }

    #[tokio::test]
    async fn evaluate_task_v2_returns_unsupported_for_code_review() {
        let config = sample_config();

        let outcome = evaluate_task_v2(
            "code_review",
            "Review contract",
            "Looks good",
            5000,
            None,
            &config,
        )
        .await;

        assert!(matches!(outcome, V2Outcome::Unsupported));
    }

    #[tokio::test]
    async fn evaluate_task_v2_mock_llm_for_supported_skill() {
        // SAFETY: test-only env mutation; no concurrent tests read this var.
        unsafe {
            std::env::set_var("VALIDATOR_MOCK_LLM", "1");
        }

        let config = sample_config();
        let agent_output = "Allocate 4,000 CSPR to cspr-usdt (8.2% APY, high TVL), 3,500 CSPR to cspr-eth (6.1% APY, moderate IL), and 2,500 CSPR to cspr-wbtc (11.4% APY, higher IL risk). Total: 10,000 CSPR. Network gas fees (~2.5 CSPR per swap) included. IL analysis shows cspr-usdt lowest volatility exposure.";

        let outcome = evaluate_task_v2(
            "defi_yield_routing",
            "Allocate 10,000 CSPR across Casper liquidity pools minimizing impermanent loss risk.",
            agent_output,
            4000,
            None,
            &config,
        )
        .await;

        match outcome {
            V2Outcome::Ok(output) => {
                assert_eq!(output.criteria.len(), 5);
                assert_eq!(output.total, 100);
            }
            V2Outcome::FixtureMissing(path) => {
                panic!("fixture not found: {path}");
            }
            other => panic!("expected Ok, got {other:?}"),
        }

        // SAFETY: test-only env cleanup.
        unsafe {
            std::env::remove_var("VALIDATOR_MOCK_LLM");
        }
    }

    #[tokio::test]
    async fn evaluate_task_v2_accepts_inline_fixture() {
        unsafe {
            std::env::set_var("VALIDATOR_MOCK_LLM", "1");
        }

        let config = sample_config();
        let fixture = serde_json::json!({
            "amount_cspr": 10000,
            "gas_price_motes": 2500000000u64,
            "pools": [
                { "id": "cspr-usdt", "apy": 0.082, "fee_bps": 30 }
            ]
        });
        let agent_output = "Allocate 4,000 CSPR to cspr-usdt (8.2% APY, high TVL), 3,500 CSPR to cspr-eth (6.1% APY, moderate IL), and 2,500 CSPR to cspr-wbtc (11.4% APY, higher IL risk). Total: 10,000 CSPR. Network gas fees (~2.5 CSPR per swap) included. IL analysis shows cspr-usdt lowest volatility exposure.";

        let outcome = evaluate_task_v2(
            "defi_yield_routing",
            "Allocate 10,000 CSPR",
            agent_output,
            4000,
            Some(fixture),
            &config,
        )
        .await;

        assert!(matches!(outcome, V2Outcome::Ok(_)));

        unsafe {
            std::env::remove_var("VALIDATOR_MOCK_LLM");
        }
    }

    #[tokio::test]
    async fn evaluate_task_v2_rejects_invalid_inline_fixture() {
        let config = sample_config();
        let invalid = serde_json::json!({ "amount_cspr": 10000 });

        let outcome = evaluate_task_v2(
            "defi_yield_routing",
            "Allocate",
            "output",
            1000,
            Some(invalid),
            &config,
        )
        .await;

        assert!(matches!(outcome, V2Outcome::FixtureInvalid(_)));
    }

    fn sample_config() -> Config {
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
            admin_account: String::new(),
        }
    }
}
