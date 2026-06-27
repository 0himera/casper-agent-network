use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JudgeProvider {
    Cloudflare,
    Openai,
    Claude,
    Ollama,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum JudgeCascadeMode {
    #[default]
    ApiFirst,
    LocalFirst,
}

#[derive(Debug, Clone)]
pub struct JudgeRoutingConfig {
    pub cascade: JudgeCascadeMode,
    pub default_timeout_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolResult {
    pub tool: String,
    pub ok: bool,
    pub details: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CriterionEval {
    pub id: String,
    pub passed: bool,
    pub score: u32,
    pub gap: Option<String>,
    pub evidence: Vec<ToolResult>,
}

#[derive(Debug, Clone, Default)]
pub struct LlmConfig {
    pub cloudflare_account_id: Option<String>,
    pub cloudflare_api_token: Option<String>,
    pub openai_api_key: Option<String>,
    pub openai_base_url: Option<String>,
    pub claude_api_key: Option<String>,
    pub ollama_url: Option<String>,
    pub ollama_model: Option<String>,
    pub custom_url: Option<String>,
    pub custom_api_key: Option<String>,
    pub custom_model: Option<String>,
    pub provider: Option<String>,
    pub mock: bool,
    pub factuality_enabled: Option<bool>,
    pub serpapi_api_key: Option<String>,
    pub judge_cascade: Option<JudgeCascadeMode>,
    pub judge_timeout_ms: Option<u64>,
    pub judge_self_consistency: Option<bool>,
    /// Post-MVP (E6): LLM semantic equality fallback after exact mismatch.
    pub exam_llm_equality: bool,
}

impl LlmConfig {
    pub fn from_env() -> Self {
        let _ = dotenvy::dotenv();

        fn env(key: &str) -> Option<String> {
            std::env::var(key).ok().filter(|v| !v.is_empty())
        }

        let mock = std::env::var("VALIDATOR_MOCK_LLM")
            .ok()
            .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"));

        let mut custom_url = env("VALIDATOR_LLM_URL");
        let custom_api_key = env("VALIDATOR_LLM_API_KEY").or(env("FIREWORKS_API_KEY"));
        let custom_model = env("VALIDATOR_LLM_MODEL").or(env("FIREWORKS_MODEL"));

        if custom_url.is_none() && custom_api_key.is_some() {
            custom_url = Some("https://api.fireworks.ai/inference/v1".to_string());
        }

        let judge_cascade = env("VALIDATOR_JUDGE_CASCADE").and_then(|v| match v.as_str() {
            "local_first" => Some(JudgeCascadeMode::LocalFirst),
            "api_first" => Some(JudgeCascadeMode::ApiFirst),
            _ => None,
        });

        let judge_timeout_ms = env("VALIDATOR_JUDGE_TIMEOUT_MS").and_then(|v| v.parse().ok());

        let judge_self_consistency = env("VALIDATOR_JUDGE_SELF_CONSISTENCY")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"));

        let factuality_enabled =
            env("VALIDATOR_FACTUALITY").map(|v| v == "1" || v.eq_ignore_ascii_case("true"));

        Self {
            cloudflare_account_id: env("CLOUDFLARE_ACCOUNT_ID"),
            cloudflare_api_token: env("CLOUDFLARE_API_TOKEN"),
            openai_api_key: env("OPENAI_API_KEY"),
            openai_base_url: env("OPENAI_BASE_URL"),
            claude_api_key: env("CLAUDE_API_KEY"),
            ollama_url: env("OLLAMA_URL"),
            ollama_model: env("OLLAMA_MODEL"),
            custom_url,
            custom_api_key,
            custom_model,
            provider: env("VALIDATOR_PROVIDER"),
            mock,
            factuality_enabled,
            serpapi_api_key: env("SERPAPI_API_KEY"),
            judge_cascade,
            judge_timeout_ms,
            judge_self_consistency,
            exam_llm_equality: env("EXAM_LLM_EQUALITY")
                .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidatorError {
    Llm(String),
    RateLimited(String),
    Parse(String),
    Inconsistent(String),
    Search(String),
}

impl fmt::Display for ValidatorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidatorError::Llm(msg) => write!(f, "LLM request failed: {msg}"),
            ValidatorError::RateLimited(msg) => write!(f, "LLM rate limited: {msg}"),
            ValidatorError::Parse(msg) => write!(f, "LLM response parse failed: {msg}"),
            ValidatorError::Inconsistent(msg) => write!(f, "consistency check failed: {msg}"),
            ValidatorError::Search(msg) => write!(f, "search provider failed: {msg}"),
        }
    }
}

impl std::error::Error for ValidatorError {}
