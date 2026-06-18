use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StageId {
    Refusal,
    Gibberish,
    Relevance,
    DomainMatch,
    Factuality,
}

impl StageId {
    pub fn as_str(self) -> &'static str {
        match self {
            StageId::Refusal => "refusal_check",
            StageId::Gibberish => "gibberish_check",
            StageId::Relevance => "relevance_check",
            StageId::DomainMatch => "domain_check",
            StageId::Factuality => "factuality_check",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PipelineVerdict {
    Factual,
    Refusal,
    Gibberish,
    Irrelevant,
    OutOfDomain,
    Hallucinated,
    Unverifiable,
}

impl PipelineVerdict {
    pub fn as_label(self) -> &'static str {
        match self {
            PipelineVerdict::Factual => "factual",
            PipelineVerdict::Refusal => "refusal",
            PipelineVerdict::Gibberish => "gibberish",
            PipelineVerdict::Irrelevant => "irrelevant",
            PipelineVerdict::OutOfDomain => "out_of_domain",
            PipelineVerdict::Hallucinated => "hallucinated",
            PipelineVerdict::Unverifiable => "unverifiable",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StageResult {
    pub id: StageId,
    pub passed: bool,
    pub raw_output: String,
    pub normalized_quality: f32,
    pub weight: u32,
    pub weighted_score: u32,
    pub skipped_due_to_gate: bool,
    pub reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StagePipelineOutput {
    pub verdict: PipelineVerdict,
    pub stages: Vec<StageResult>,
    pub criteria: Vec<crate::types::CriterionEval>,
    pub total: u32,
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StageTiming {
    pub id: StageId,
    pub elapsed_ms: u64,
}

/// Runtime observability for a single stage pipeline evaluation (N4.5).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PipelineRunStats {
    pub pipeline: String,
    pub factuality_enabled: bool,
    pub factuality_ran: bool,
    pub verdict: PipelineVerdict,
    pub total: u32,
    pub llm_calls: u32,
    pub search_cache_hits: u32,
    pub search_cache_misses: u32,
    pub stage_ms: Vec<StageTiming>,
}
