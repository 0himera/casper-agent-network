use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExamVerdict {
    Passed,
    Failed,
    Refusal,
    GateFailed,
}

impl ExamVerdict {
    pub fn as_label(self) -> &'static str {
        match self {
            ExamVerdict::Passed => "passed",
            ExamVerdict::Failed => "failed",
            ExamVerdict::Refusal => "refusal",
            ExamVerdict::GateFailed => "gate_failed",
        }
    }
}

/// Per-template verification policy (E6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnswerVerificationMode {
    #[default]
    ExactThenLlm,
    LlmFirst,
}

impl AnswerVerificationMode {
    pub fn as_label(self) -> &'static str {
        match self {
            AnswerVerificationMode::ExactThenLlm => "exact_then_llm",
            AnswerVerificationMode::LlmFirst => "llm_first",
        }
    }
}

/// How the exam answer was compared (E6 audit contract).
pub mod compare_mode {
    pub const EXACT_MATCH: &str = "exact_match";
    pub const LLM_FALLBACK_MATCH: &str = "llm_fallback_match";
    pub const LLM_FALLBACK_MISS: &str = "llm_fallback_miss";
    pub const LLM_FIRST_MATCH: &str = "llm_first_match";
    pub const LLM_FIRST_MISS: &str = "llm_first_miss";
    pub const ANSWER_MISSING: &str = "answer_missing";
    pub const REFUSAL: &str = "refusal";
    pub const GATE_FAILED: &str = "gate_failed";
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExamAudit {
    pub exam_id: String,
    pub assignment_hash: String,
    pub expected_answer_hash: String,
    pub actual_answer_hash: String,
    pub hash_algorithm: String,
    pub verdict: ExamVerdict,
    pub pipeline: String,
    pub timestamp: String,
    pub compare_mode: String,
    pub llm_fallback_used: bool,
    pub answer_verification_mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm_raw: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExamPipelineOutput {
    pub verdict: ExamVerdict,
    pub total: u32,
    pub explanation: String,
    pub audit: ExamAudit,
}

pub fn exam_total_for_verdict(verdict: ExamVerdict) -> u32 {
    match verdict {
        ExamVerdict::Passed => 100,
        ExamVerdict::Failed | ExamVerdict::Refusal | ExamVerdict::GateFailed => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exam_total_for_verdict_mapping() {
        assert_eq!(exam_total_for_verdict(ExamVerdict::Passed), 100);
        assert_eq!(exam_total_for_verdict(ExamVerdict::Failed), 0);
        assert_eq!(exam_total_for_verdict(ExamVerdict::Refusal), 0);
        assert_eq!(exam_total_for_verdict(ExamVerdict::GateFailed), 0);
    }

    #[test]
    fn exam_verdict_as_label() {
        assert_eq!(ExamVerdict::Passed.as_label(), "passed");
        assert_eq!(ExamVerdict::Failed.as_label(), "failed");
        assert_eq!(ExamVerdict::Refusal.as_label(), "refusal");
        assert_eq!(ExamVerdict::GateFailed.as_label(), "gate_failed");
    }

    #[test]
    fn exam_verdict_serde_round_trip() {
        for verdict in [
            ExamVerdict::Passed,
            ExamVerdict::Failed,
            ExamVerdict::Refusal,
            ExamVerdict::GateFailed,
        ] {
            let json = serde_json::to_string(&verdict).expect("serialize verdict");
            let parsed: ExamVerdict = serde_json::from_str(&json).expect("deserialize verdict");
            assert_eq!(parsed, verdict);
        }
    }
}
