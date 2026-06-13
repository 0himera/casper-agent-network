use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillId {
    DefiYieldRouting,
    DefiProtocolRisk,
    RwaAppraisal,
    RwaCompliance,
}

impl SkillId {
    pub fn as_str(self) -> &'static str {
        match self {
            SkillId::DefiYieldRouting => "defi_yield_routing",
            SkillId::DefiProtocolRisk => "defi_protocol_risk",
            SkillId::RwaAppraisal => "rwa_appraisal",
            SkillId::RwaCompliance => "rwa_compliance",
        }
    }
}

impl fmt::Display for SkillId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationInput {
    pub skill: SkillId,
    pub task_prompt: String,
    pub agent_output: String,
    pub fixture: serde_json::Value,
    pub processing_time_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CriterionDef {
    pub id: &'static str,
    pub description: &'static str,
    pub tools: &'static [&'static str],
    pub weight: u32,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Satisfied,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ValidationOutput {
    pub verdict: Verdict,
    pub criteria: Vec<CriterionEval>,
    pub total: u32,
    pub explanation: String,
    pub recommended_price_motes: u64,
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
    pub mock: bool,
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

        Self {
            cloudflare_account_id: env("CLOUDFLARE_ACCOUNT_ID"),
            cloudflare_api_token: env("CLOUDFLARE_API_TOKEN"),
            openai_api_key: env("OPENAI_API_KEY"),
            openai_base_url: env("OPENAI_BASE_URL"),
            claude_api_key: env("CLAUDE_API_KEY"),
            ollama_url: env("OLLAMA_URL"),
            ollama_model: env("OLLAMA_MODEL"),
            mock,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidatorError {
    Llm(String),
    Parse(String),
    Inconsistent(String),
}

impl fmt::Display for ValidatorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidatorError::Llm(msg) => write!(f, "LLM request failed: {msg}"),
            ValidatorError::Parse(msg) => write!(f, "LLM response parse failed: {msg}"),
            ValidatorError::Inconsistent(msg) => write!(f, "consistency check failed: {msg}"),
        }
    }
}

impl std::error::Error for ValidatorError {}
