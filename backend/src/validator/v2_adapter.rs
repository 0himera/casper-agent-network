use std::fs;
use std::path::PathBuf;

use validator_engine::{evaluate_with_options, GraderOptions, LlmConfig, SkillId, ValidationInput, ValidationOutput};

use crate::config::Config;

/// Результат попытки оценить skill через v2-движок.
#[derive(Debug)]
pub enum V2Outcome {
    /// v2 успешно оценил skill.
    Ok(ValidationOutput),
    /// Этот skill не имеет v2-рубрики (например "code_review"). Benchmark пропускает такой skill.
    Unsupported,
    /// Не найден fixture-файл для skill. Benchmark пропускает такой skill.
    FixtureMissing(String),
    /// Реальная ошибка движка (LLM/parse/consistency). Caller обрабатывает как ошибку оценки.
    EngineError(String),
}

fn map_config(config: &Config) -> LlmConfig {
    let mock = std::env::var("VALIDATOR_MOCK_LLM")
        .ok()
        .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"));

    LlmConfig {
        cloudflare_account_id: config.cloudflare_account_id.clone(),
        cloudflare_api_token: config.cloudflare_api_token.clone(),
        openai_api_key: config.openai_api_key.clone(),
        openai_base_url: None,
        claude_api_key: config.claude_api_key.clone(),
        ollama_url: config.ollama_url.clone(),
        ollama_model: config.ollama_model.clone(),
        mock,
    }
}

fn map_skill(skill: &str) -> Option<SkillId> {
    match skill {
        "defi_yield_routing" | "defi_analysis" => Some(SkillId::DefiYieldRouting),
        "defi_protocol_risk" => Some(SkillId::DefiProtocolRisk),
        "rwa_appraisal" => Some(SkillId::RwaAppraisal),
        "rwa_compliance" => Some(SkillId::RwaCompliance),
        _ => None,
    }
}

fn fixture_file(skill: SkillId) -> &'static str {
    match skill {
        SkillId::DefiYieldRouting => "defi_yield_routing.json",
        SkillId::DefiProtocolRisk => "defi_protocol_risk.json",
        SkillId::RwaAppraisal => "rwa_appraisal.json",
        SkillId::RwaCompliance => "rwa_compliance.json",
    }
}

fn load_fixture(skill: SkillId) -> Result<serde_json::Value, String> {
    let path = PathBuf::from("validator").join("fixtures").join(fixture_file(skill));
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("{}: {}", path.display(), e))?;
    serde_json::from_str(&content).map_err(|e| e.to_string())
}

pub async fn evaluate_task_v2(
    skill: &str,
    task_prompt: &str,
    agent_output: &str,
    processing_time_ms: u64,
    config: &Config,
) -> V2Outcome {
    let skill_id = match map_skill(skill) {
        Some(s) => s,
        None => return V2Outcome::Unsupported,
    };

    let fixture = match load_fixture(skill_id) {
        Ok(f) => f,
        Err(e) => return V2Outcome::FixtureMissing(e),
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
            admin_account: String::new(),
        };

        let llm = map_config(&config);
        assert_eq!(llm.openai_api_key.as_deref(), Some("sk-test"));
        assert_eq!(llm.cloudflare_account_id.as_deref(), Some("cf-id"));
        assert_eq!(llm.openai_base_url, None);
    }

    #[tokio::test]
    async fn evaluate_task_v2_returns_unsupported_for_code_review() {
        let config = Config {
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
            admin_account: String::new(),
        };

        let outcome = evaluate_task_v2(
            "code_review",
            "Review contract",
            "Looks good",
            5000,
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

        let config = Config {
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
            admin_account: String::new(),
        };

        let outcome = evaluate_task_v2(
            "defi_yield_routing",
            "Allocate 10,000 CSPR across Casper liquidity pools minimizing impermanent loss risk.",
            "Allocate 4,000 CSPR to cspr-usdt, 3,500 to cspr-eth, 2,500 to cspr-wbtc. Total 10,000 CSPR.",
            4000,
            &config,
        )
        .await;

        match outcome {
            V2Outcome::Ok(output) => {
                assert_eq!(output.criteria.len(), 5);
                assert_eq!(output.total, 100);
            }
            V2Outcome::FixtureMissing(path) => {
                panic!("fixture not found at expected path: {path}");
            }
            other => panic!("expected Ok, got {other:?}"),
        }

        // SAFETY: test-only env cleanup.
        unsafe {
            std::env::remove_var("VALIDATOR_MOCK_LLM");
        }
    }
}
