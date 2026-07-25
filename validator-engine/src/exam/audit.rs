use sha2::{Digest, Sha256};

use super::types::{ExamAudit, ExamVerdict};

pub(crate) fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

fn current_timestamp_unix() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

#[allow(clippy::too_many_arguments)]
pub fn build_exam_audit(
    exam_id: &str,
    task_prompt: &str,
    canonical_expected: &str,
    canonical_actual: &str,
    verdict: ExamVerdict,
    compare_mode: &str,
    llm_fallback_used: bool,
    answer_verification_mode: &str,
    llm_raw: Option<String>,
) -> ExamAudit {
    ExamAudit {
        exam_id: exam_id.to_string(),
        assignment_hash: sha256_hex(task_prompt),
        expected_answer_hash: sha256_hex(canonical_expected),
        actual_answer_hash: sha256_hex(canonical_actual),
        hash_algorithm: "sha256".to_string(),
        verdict,
        pipeline: "exam".to_string(),
        timestamp: current_timestamp_unix(),
        compare_mode: compare_mode.to_string(),
        llm_fallback_used,
        answer_verification_mode: answer_verification_mode.to_string(),
        llm_raw,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exam::types::compare_mode;
    use serde_json::Value;

    #[test]
    fn audit_uses_sha256_and_exam_pipeline() {
        let audit = build_exam_audit(
            "exam-1",
            "Compute yield",
            "12345.67 usd",
            "999 usd",
            ExamVerdict::Failed,
            compare_mode::EXACT_MATCH,
            false,
            "exact_then_llm",
            None,
        );

        assert_eq!(audit.exam_id, "exam-1");
        assert_eq!(audit.hash_algorithm, "sha256");
        assert_eq!(audit.pipeline, "exam");
        assert_eq!(audit.verdict, ExamVerdict::Failed);
        assert_ne!(audit.expected_answer_hash, audit.actual_answer_hash);
        assert_eq!(audit.expected_answer_hash.len(), 64);
        assert_eq!(audit.actual_answer_hash.len(), 64);
        assert_eq!(audit.assignment_hash.len(), 64);
        assert!(!audit.timestamp.is_empty());
    }

    #[test]
    fn audit_json_contains_required_fields() {
        let audit = build_exam_audit(
            "exam-1",
            "Compute yield",
            "12345.67 usd",
            "12345.67 usd",
            ExamVerdict::Passed,
            compare_mode::EXACT_MATCH,
            false,
            "exact_then_llm",
            None,
        );
        let json: Value = serde_json::to_value(&audit).expect("serialize audit");

        for key in [
            "exam_id",
            "assignment_hash",
            "expected_answer_hash",
            "actual_answer_hash",
            "hash_algorithm",
            "verdict",
            "pipeline",
            "timestamp",
            "compare_mode",
            "llm_fallback_used",
            "answer_verification_mode",
        ] {
            assert!(json.get(key).is_some(), "missing audit field: {key}");
        }

        assert_eq!(json["hash_algorithm"], "sha256");
        assert_eq!(json["pipeline"], "exam");
        assert_eq!(json["verdict"], "passed");
        assert_eq!(json["compare_mode"], "exact_match");
        assert_eq!(json["llm_fallback_used"], false);
        assert_eq!(json["answer_verification_mode"], "exact_then_llm");
        assert!(json.get("llm_raw").is_none());
    }

    #[test]
    fn audit_json_includes_llm_fallback_fields_when_used() {
        let audit = build_exam_audit(
            "exam-1",
            "Compute yield",
            "12345.67 usd",
            "about twelve thousand usd",
            ExamVerdict::Passed,
            compare_mode::LLM_FALLBACK_MATCH,
            true,
            "exact_then_llm",
            Some("YES".to_string()),
        );
        let json: Value = serde_json::to_value(&audit).expect("serialize audit");
        assert_eq!(json["compare_mode"], "llm_fallback_match");
        assert_eq!(json["llm_fallback_used"], true);
        assert_eq!(json["llm_raw"], "YES");
    }

    #[test]
    fn audit_json_includes_llm_first_fields() {
        let audit = build_exam_audit(
            "exam-rwa",
            "Compute NAV",
            "100.47 usd",
            "nav per share 100.47 usd",
            ExamVerdict::Passed,
            compare_mode::LLM_FIRST_MATCH,
            false,
            "llm_first",
            Some("YES".to_string()),
        );
        let json: Value = serde_json::to_value(&audit).expect("serialize audit");
        assert_eq!(json["compare_mode"], "llm_first_match");
        assert_eq!(json["llm_fallback_used"], false);
        assert_eq!(json["answer_verification_mode"], "llm_first");
    }
}
