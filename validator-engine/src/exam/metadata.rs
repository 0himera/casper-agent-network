use serde_json::Value;

use super::types::AnswerVerificationMode;

/// Resolved per-template verification policy (E6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExamVerificationPolicy {
    pub configured_mode: AnswerVerificationMode,
    pub effective_mode: AnswerVerificationMode,
    /// `llm_first` was configured without `verification_reason` and was downgraded.
    pub policy_degraded: bool,
}

impl Default for ExamVerificationPolicy {
    fn default() -> Self {
        Self {
            configured_mode: AnswerVerificationMode::ExactThenLlm,
            effective_mode: AnswerVerificationMode::ExactThenLlm,
            policy_degraded: false,
        }
    }
}

impl ExamVerificationPolicy {
    pub fn exact_then_llm() -> Self {
        Self::default()
    }

    pub fn llm_first() -> Self {
        Self {
            configured_mode: AnswerVerificationMode::LlmFirst,
            effective_mode: AnswerVerificationMode::LlmFirst,
            policy_degraded: false,
        }
    }
}

fn parse_configured_mode(raw: Option<&str>) -> AnswerVerificationMode {
    match raw {
        Some("llm_first") => AnswerVerificationMode::LlmFirst,
        Some("exact_then_llm") | None => AnswerVerificationMode::ExactThenLlm,
        _ => AnswerVerificationMode::ExactThenLlm,
    }
}

fn has_verification_reason(meta: &Value) -> bool {
    meta.get("verification_reason")
        .and_then(|value| value.as_str())
        .is_some_and(|reason| !reason.trim().is_empty())
}

/// Reads `answer_verification_mode` / `verification_reason` from internal template metadata.
pub fn resolve_exam_verification_policy(source_metadata: Option<&Value>) -> ExamVerificationPolicy {
    let Some(meta) = source_metadata else {
        return ExamVerificationPolicy::default();
    };

    let configured_mode = parse_configured_mode(
        meta.get("answer_verification_mode")
            .and_then(|value| value.as_str()),
    );

    if configured_mode == AnswerVerificationMode::LlmFirst {
        if has_verification_reason(meta) {
            return ExamVerificationPolicy::llm_first();
        }
        return ExamVerificationPolicy {
            configured_mode: AnswerVerificationMode::LlmFirst,
            effective_mode: AnswerVerificationMode::ExactThenLlm,
            policy_degraded: true,
        };
    }

    ExamVerificationPolicy::default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn missing_metadata_defaults_to_exact_then_llm() {
        let policy = resolve_exam_verification_policy(None);
        assert_eq!(policy.effective_mode, AnswerVerificationMode::ExactThenLlm);
        assert!(!policy.policy_degraded);
    }

    #[test]
    fn llm_first_requires_verification_reason() {
        let policy = resolve_exam_verification_policy(Some(&json!({
            "answer_verification_mode": "llm_first"
        })));
        assert_eq!(policy.configured_mode, AnswerVerificationMode::LlmFirst);
        assert_eq!(policy.effective_mode, AnswerVerificationMode::ExactThenLlm);
        assert!(policy.policy_degraded);
    }

    #[test]
    fn llm_first_with_reason_is_effective() {
        let policy = resolve_exam_verification_policy(Some(&json!({
            "answer_verification_mode": "llm_first",
            "verification_reason": "RWA NAV wording varies by issuer report phrasing"
        })));
        assert_eq!(policy.effective_mode, AnswerVerificationMode::LlmFirst);
        assert!(!policy.policy_degraded);
    }
}
