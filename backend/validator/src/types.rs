use std::collections::HashMap;
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
pub enum CriterionKind {
    Hard,
    Soft,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SoftLabel {
    Strong,
    Partial,
    Missing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CriterionDef {
    pub id: &'static str,
    pub description: &'static str,
    pub tools: &'static [&'static str],
    pub weight: u32,
    pub kind: CriterionKind,
    pub critical: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraderMode {
    F3,
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelfConsistencyTrigger {
    PartialOnly,
}

#[derive(Debug, Clone)]
pub struct SelfConsistencyConfig {
    pub enabled: bool,
    pub samples: u32,
    pub trigger: SelfConsistencyTrigger,
}

#[derive(Debug, Clone)]
pub struct SkillJudgeConfig {
    pub provider: Option<JudgeProvider>,
    pub model: Option<String>,
    pub self_consistency: Option<SelfConsistencyConfig>,
}

#[derive(Debug, Clone)]
pub struct JudgeRoutingConfig {
    pub cascade: JudgeCascadeMode,
    pub default_timeout_ms: u64,
    pub skills: HashMap<SkillId, SkillJudgeConfig>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GraderOptions {
    pub mode: GraderMode,
    pub pass_threshold: u32,
    /// Prompt version override (`v1`, `v2`). `None` uses YAML `current_version`.
    pub prompt_version: Option<&'static str>,
    /// When false, few-shot exemplars are omitted (A/B baseline).
    pub few_shot_enabled: bool,
    /// Override YAML self-consistency per skill. `None` uses YAML per skill.
    pub self_consistency_enabled: Option<bool>,
}

impl Default for GraderOptions {
    fn default() -> Self {
        Self::f3()
    }
}

impl GraderOptions {
    pub const DEFAULT_PASS_THRESHOLD: u32 = 70;

    pub fn f3() -> Self {
        Self {
            mode: GraderMode::F3,
            pass_threshold: Self::DEFAULT_PASS_THRESHOLD,
            prompt_version: None,
            few_shot_enabled: true,
            self_consistency_enabled: None,
        }
    }

    pub fn f3_baseline() -> Self {
        Self {
            mode: GraderMode::F3,
            pass_threshold: Self::DEFAULT_PASS_THRESHOLD,
            prompt_version: Some("v1"),
            few_shot_enabled: false,
            self_consistency_enabled: None,
        }
    }

    pub fn f3_few_shot() -> Self {
        Self {
            mode: GraderMode::F3,
            pass_threshold: Self::DEFAULT_PASS_THRESHOLD,
            prompt_version: Some("v2"),
            few_shot_enabled: true,
            self_consistency_enabled: None,
        }
    }
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
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidatorError {
    Llm(String),
    RateLimited(String),
    Parse(String),
    Inconsistent(String),
    Fixture(String),
    Search(String),
}

impl fmt::Display for ValidatorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidatorError::Llm(msg) => write!(f, "LLM request failed: {msg}"),
            ValidatorError::RateLimited(msg) => write!(f, "LLM rate limited: {msg}"),
            ValidatorError::Parse(msg) => write!(f, "LLM response parse failed: {msg}"),
            ValidatorError::Inconsistent(msg) => write!(f, "consistency check failed: {msg}"),
            ValidatorError::Fixture(msg) => write!(f, "fixture validation failed: {msg}"),
            ValidatorError::Search(msg) => write!(f, "search provider failed: {msg}"),
        }
    }
}

impl std::error::Error for ValidatorError {}
