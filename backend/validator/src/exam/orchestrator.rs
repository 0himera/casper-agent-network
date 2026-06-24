use crate::stage_pipeline::stages::refusal::evaluate_refusal_stage;
use crate::types::{LlmConfig, ValidatorError};

use super::audit::build_exam_audit;
use super::canonicalize::canonicalize_exam_answer;
use super::compare::compare_type_h;
use super::gates::check_exam_input_gate;
use super::parse::extract_answer_contract;
use super::types::{ExamPipelineOutput, ExamVerdict, exam_total_for_verdict};

fn assemble_output(
    exam_id: &str,
    task_prompt: &str,
    canonical_expected: &str,
    canonical_actual: &str,
    verdict: ExamVerdict,
    explanation: String,
) -> ExamPipelineOutput {
    ExamPipelineOutput {
        verdict,
        total: exam_total_for_verdict(verdict),
        explanation,
        audit: build_exam_audit(
            exam_id,
            task_prompt,
            canonical_expected,
            canonical_actual,
            verdict,
        ),
    }
}

async fn run_exam_pipeline(
    config: &LlmConfig,
    exam_id: &str,
    task_prompt: &str,
    agent_output: &str,
    expected_answer: &str,
) -> Result<ExamPipelineOutput, ValidatorError> {
    let canonical_expected = canonicalize_exam_answer(expected_answer);

    if let Err(failure) = check_exam_input_gate(agent_output) {
        return Ok(assemble_output(
            exam_id,
            task_prompt,
            &canonical_expected,
            "",
            ExamVerdict::GateFailed,
            format!("Exam input gate failed: {}", failure.reason()),
        ));
    }

    let refusal = evaluate_refusal_stage(config, task_prompt, agent_output).await?;
    if refusal.is_refusal {
        return Ok(assemble_output(
            exam_id,
            task_prompt,
            &canonical_expected,
            "",
            ExamVerdict::Refusal,
            "Exam pipeline early exit: agent output classified as refusal.".to_string(),
        ));
    }

    let raw_answer = match extract_answer_contract(agent_output) {
        Some(value) => value,
        None => {
            return Ok(assemble_output(
                exam_id,
                task_prompt,
                &canonical_expected,
                "",
                ExamVerdict::Failed,
                "Exam pipeline failed: missing ANSWER: contract in agent output.".to_string(),
            ));
        }
    };

    let canonical_actual = canonicalize_exam_answer(&raw_answer);
    let passed = compare_type_h(&canonical_actual, &canonical_expected);

    if passed {
        Ok(assemble_output(
            exam_id,
            task_prompt,
            &canonical_expected,
            &canonical_actual,
            ExamVerdict::Passed,
            format!("Exam answer matched expected canonical value `{canonical_expected}`."),
        ))
    } else {
        Ok(assemble_output(
            exam_id,
            task_prompt,
            &canonical_expected,
            &canonical_actual,
            ExamVerdict::Failed,
            format!(
                "Exam answer `{canonical_actual}` did not match expected `{canonical_expected}`."
            ),
        ))
    }
}

/// Live exam pipeline with real refusal stage (S0 reuse).
pub async fn evaluate_exam_pipeline(
    config: &LlmConfig,
    exam_id: &str,
    task_prompt: &str,
    agent_output: &str,
    expected_answer: &str,
) -> Result<ExamPipelineOutput, ValidatorError> {
    run_exam_pipeline(config, exam_id, task_prompt, agent_output, expected_answer).await
}

/// Mock exam pipeline — no network; refusal uses mock markers via `LlmConfig::mock`.
pub fn evaluate_exam_pipeline_mock(
    exam_id: &str,
    task_prompt: &str,
    agent_output: &str,
    expected_answer: &str,
) -> ExamPipelineOutput {
    let config = LlmConfig {
        mock: true,
        ..Default::default()
    };

    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("exam mock runtime")
        .block_on(run_exam_pipeline(
            &config,
            exam_id,
            task_prompt,
            agent_output,
            expected_answer,
        ))
        .expect("exam mock pipeline")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exam::audit::sha256_hex;

    const EXAM_ID: &str = "exam-test-1";
    const TASK_PROMPT: &str = "Compute the fee-adjusted yield and return ANSWER: <value>";
    const EXPECTED_CANONICAL: &str = "12345.67 usd";
    const EXPECTED_RAW: &str = "12345.67 USD";

    fn empty_actual_hash() -> String {
        sha256_hex("")
    }

    fn assert_audit_basics(output: &ExamPipelineOutput, verdict: ExamVerdict) {
        assert_eq!(output.audit.exam_id, EXAM_ID);
        assert_eq!(output.audit.pipeline, "exam");
        assert_eq!(output.audit.hash_algorithm, "sha256");
        assert_eq!(output.audit.verdict, verdict);
        assert_eq!(output.audit.assignment_hash, sha256_hex(TASK_PROMPT));
        assert!(!output.audit.timestamp.is_empty());
        assert!(!output.explanation.is_empty());
    }

    #[test]
    fn mock_pipeline_passes_on_exact_match() {
        let output = evaluate_exam_pipeline_mock(
            EXAM_ID,
            TASK_PROMPT,
            "ANSWER: 12345.67 USD",
            EXPECTED_CANONICAL,
        );

        assert_eq!(output.verdict, ExamVerdict::Passed);
        assert_eq!(output.total, 100);
        assert_audit_basics(&output, ExamVerdict::Passed);
        assert_eq!(
            output.audit.expected_answer_hash,
            output.audit.actual_answer_hash
        );
    }

    #[test]
    fn mock_pipeline_passes_with_non_canonical_expected_input() {
        let output =
            evaluate_exam_pipeline_mock(EXAM_ID, TASK_PROMPT, "ANSWER: 12345.67 USD", EXPECTED_RAW);

        assert_eq!(output.verdict, ExamVerdict::Passed);
        assert_eq!(output.total, 100);
        assert_audit_basics(&output, ExamVerdict::Passed);
    }

    #[test]
    fn mock_pipeline_fails_on_wrong_answer() {
        let output = evaluate_exam_pipeline_mock(
            EXAM_ID,
            TASK_PROMPT,
            "ANSWER: 999 usd",
            EXPECTED_CANONICAL,
        );

        assert_eq!(output.verdict, ExamVerdict::Failed);
        assert_eq!(output.total, 0);
        assert_audit_basics(&output, ExamVerdict::Failed);
        assert_ne!(
            output.audit.expected_answer_hash,
            output.audit.actual_answer_hash
        );
        assert!(output.explanation.contains("did not match expected"));
    }

    #[test]
    fn mock_pipeline_refusal_early_exit() {
        let output = evaluate_exam_pipeline_mock(
            EXAM_ID,
            TASK_PROMPT,
            "mock_refusal: I cannot fulfill this request",
            EXPECTED_CANONICAL,
        );

        assert_eq!(output.verdict, ExamVerdict::Refusal);
        assert_eq!(output.total, 0);
        assert_audit_basics(&output, ExamVerdict::Refusal);
        assert_eq!(output.audit.actual_answer_hash, empty_actual_hash());
        assert!(output.explanation.contains("refusal"));
    }

    #[test]
    fn mock_pipeline_gate_fails_on_empty_output() {
        let output = evaluate_exam_pipeline_mock(EXAM_ID, TASK_PROMPT, "   ", EXPECTED_CANONICAL);

        assert_eq!(output.verdict, ExamVerdict::GateFailed);
        assert_eq!(output.total, 0);
        assert_audit_basics(&output, ExamVerdict::GateFailed);
        assert_eq!(output.audit.actual_answer_hash, empty_actual_hash());
        assert!(output.explanation.contains("Exam input gate failed"));
    }

    #[test]
    fn mock_pipeline_gate_fails_on_error_marker() {
        let output = evaluate_exam_pipeline_mock(
            EXAM_ID,
            TASK_PROMPT,
            "execution error while computing",
            EXPECTED_CANONICAL,
        );

        assert_eq!(output.verdict, ExamVerdict::GateFailed);
        assert_eq!(output.total, 0);
        assert_audit_basics(&output, ExamVerdict::GateFailed);
        assert_eq!(output.audit.actual_answer_hash, empty_actual_hash());
    }

    #[test]
    fn mock_pipeline_fails_when_answer_missing() {
        let output = evaluate_exam_pipeline_mock(
            EXAM_ID,
            TASK_PROMPT,
            "Here is my analysis without the required contract.",
            EXPECTED_CANONICAL,
        );

        assert_eq!(output.verdict, ExamVerdict::Failed);
        assert_eq!(output.total, 0);
        assert_audit_basics(&output, ExamVerdict::Failed);
        assert_eq!(output.audit.actual_answer_hash, empty_actual_hash());
        assert!(output.explanation.contains("missing ANSWER:"));
    }

    #[test]
    fn mock_pipeline_passes_short_answer_under_twenty_chars() {
        let output = evaluate_exam_pipeline_mock(EXAM_ID, TASK_PROMPT, "ANSWER: 1 usd", "1 usd");

        assert_eq!(output.verdict, ExamVerdict::Passed);
        assert_eq!(output.total, 100);
    }

    #[test]
    fn mock_pipeline_handles_format_noise() {
        let output = evaluate_exam_pipeline_mock(
            EXAM_ID,
            TASK_PROMPT,
            "**Analysis**\n\nANSWER:  12345.67   USD.\n",
            EXPECTED_RAW,
        );

        assert_eq!(output.verdict, ExamVerdict::Passed);
        assert_eq!(output.total, 100);
    }

    #[tokio::test]
    async fn live_pipeline_mock_config_passes_without_network() {
        let config = LlmConfig {
            mock: true,
            ..Default::default()
        };

        let output = evaluate_exam_pipeline(
            &config,
            EXAM_ID,
            TASK_PROMPT,
            "ANSWER: 12345.67 USD",
            EXPECTED_RAW,
        )
        .await
        .expect("live mock exam pipeline");

        assert_eq!(output.verdict, ExamVerdict::Passed);
        assert_eq!(output.total, 100);
        assert_audit_basics(&output, ExamVerdict::Passed);
    }
}
