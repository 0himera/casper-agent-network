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

pub fn build_exam_audit(
    exam_id: &str,
    task_prompt: &str,
    canonical_expected: &str,
    canonical_actual: &str,
    verdict: ExamVerdict,
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn audit_uses_sha256_and_exam_pipeline() {
        let audit = build_exam_audit(
            "exam-1",
            "Compute yield",
            "12345.67 usd",
            "999 usd",
            ExamVerdict::Failed,
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
        ] {
            assert!(json.get(key).is_some(), "missing audit field: {key}");
        }

        assert_eq!(json["hash_algorithm"], "sha256");
        assert_eq!(json["pipeline"], "exam");
        assert_eq!(json["verdict"], "passed");
    }
}
