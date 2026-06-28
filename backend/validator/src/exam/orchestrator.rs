use crate::stage_pipeline::stages::refusal::evaluate_refusal_stage;
use crate::types::{LlmConfig, ValidatorError};

use super::audit::build_exam_audit;
use super::canonicalize::canonicalize_exam_answer;
use super::compare::compare_type_h;
use super::equality::{ExamEqualityEval, evaluate_exam_equality, parse_exam_equality_response};
use super::gates::check_exam_input_gate;
use super::metadata::ExamVerificationPolicy;
use super::parse::extract_answer_contract;
use super::types::{
    AnswerVerificationMode, ExamPipelineOutput, ExamVerdict, compare_mode, exam_total_for_verdict,
};

struct AuditMeta {
    compare_mode: &'static str,
    llm_fallback_used: bool,
    answer_verification_mode: &'static str,
    llm_raw: Option<String>,
}

fn assemble_output(
    exam_id: &str,
    task_prompt: &str,
    canonical_expected: &str,
    canonical_actual: &str,
    verdict: ExamVerdict,
    explanation: String,
    audit_meta: AuditMeta,
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
            audit_meta.compare_mode,
            audit_meta.llm_fallback_used,
            audit_meta.answer_verification_mode,
            audit_meta.llm_raw,
        ),
    }
}

fn exact_only_fail_output(
    exam_id: &str,
    task_prompt: &str,
    canonical_expected: &str,
    canonical_actual: &str,
    verification_mode: AnswerVerificationMode,
    suffix: &str,
) -> ExamPipelineOutput {
    assemble_output(
        exam_id,
        task_prompt,
        canonical_expected,
        canonical_actual,
        ExamVerdict::Failed,
        format!(
            "Exam answer `{canonical_actual}` did not match expected `{canonical_expected}`{suffix}."
        ),
        AuditMeta {
            compare_mode: compare_mode::EXACT_MATCH,
            llm_fallback_used: false,
            answer_verification_mode: verification_mode.as_label(),
            llm_raw: None,
        },
    )
}

fn output_from_llm_eval(
    exam_id: &str,
    task_prompt: &str,
    canonical_expected: &str,
    canonical_actual: &str,
    equality: ExamEqualityEval,
    verification_mode: AnswerVerificationMode,
    llm_first: bool,
) -> ExamPipelineOutput {
    let (compare, llm_used) = if llm_first {
        if equality.is_equal {
            (compare_mode::LLM_FIRST_MATCH, false)
        } else {
            (compare_mode::LLM_FIRST_MISS, false)
        }
    } else if equality.is_equal {
        (compare_mode::LLM_FALLBACK_MATCH, true)
    } else {
        (compare_mode::LLM_FALLBACK_MISS, true)
    };

    let verdict = if equality.is_equal {
        ExamVerdict::Passed
    } else {
        ExamVerdict::Failed
    };

    let explanation = if equality.is_equal {
        if llm_first {
            format!(
                "Exam answer `{canonical_actual}` matched expected `{canonical_expected}` via LLM-first verification."
            )
        } else {
            format!(
                "Exam answer `{canonical_actual}` matched expected `{canonical_expected}` via LLM equality fallback."
            )
        }
    } else if llm_first {
        format!(
            "Exam answer `{canonical_actual}` did not match expected `{canonical_expected}` (LLM-first verification)."
        )
    } else {
        format!(
            "Exam answer `{canonical_actual}` did not match expected `{canonical_expected}` (exact and LLM fallback)."
        )
    };

    assemble_output(
        exam_id,
        task_prompt,
        canonical_expected,
        canonical_actual,
        verdict,
        explanation,
        AuditMeta {
            compare_mode: compare,
            llm_fallback_used: llm_used,
            answer_verification_mode: verification_mode.as_label(),
            llm_raw: Some(equality.raw_output),
        },
    )
}

async fn run_llm_verification(
    config: &LlmConfig,
    agent_output: &str,
    canonical_expected: &str,
    canonical_actual: &str,
) -> ExamEqualityEval {
    match evaluate_exam_equality(config, agent_output, canonical_actual, canonical_expected).await {
        Ok(eval) => eval,
        Err(err) => ExamEqualityEval {
            is_equal: false,
            raw_output: format!("LLM_ERROR: {err}"),
            parse_fallback: true,
        },
    }
}

async fn resolve_mismatch_with_optional_llm(
    config: &LlmConfig,
    exam_id: &str,
    task_prompt: &str,
    agent_output: &str,
    canonical_expected: &str,
    canonical_actual: &str,
    verification_mode: AnswerVerificationMode,
) -> Result<ExamPipelineOutput, ValidatorError> {
    if !config.exam_llm_equality {
        return Ok(exact_only_fail_output(
            exam_id,
            task_prompt,
            canonical_expected,
            canonical_actual,
            verification_mode,
            "",
        ));
    }

    let equality =
        run_llm_verification(config, agent_output, canonical_expected, canonical_actual).await;

    Ok(output_from_llm_eval(
        exam_id,
        task_prompt,
        canonical_expected,
        canonical_actual,
        equality,
        verification_mode,
        false,
    ))
}

async fn run_llm_first_verification(
    config: &LlmConfig,
    exam_id: &str,
    task_prompt: &str,
    agent_output: &str,
    canonical_expected: &str,
    canonical_actual: &str,
    verification_mode: AnswerVerificationMode,
) -> Result<ExamPipelineOutput, ValidatorError> {
    if !config.exam_llm_equality {
        if compare_type_h(canonical_actual, canonical_expected) {
            return Ok(assemble_output(
                exam_id,
                task_prompt,
                canonical_expected,
                canonical_actual,
                ExamVerdict::Passed,
                format!("Exam answer matched expected canonical value `{canonical_expected}`."),
                AuditMeta {
                    compare_mode: compare_mode::EXACT_MATCH,
                    llm_fallback_used: false,
                    answer_verification_mode: verification_mode.as_label(),
                    llm_raw: None,
                },
            ));
        }
        return Ok(exact_only_fail_output(
            exam_id,
            task_prompt,
            canonical_expected,
            canonical_actual,
            verification_mode,
            " (LLM-first disabled; exact-only fail-safe)",
        ));
    }

    let equality =
        run_llm_verification(config, agent_output, canonical_expected, canonical_actual).await;

    Ok(output_from_llm_eval(
        exam_id,
        task_prompt,
        canonical_expected,
        canonical_actual,
        equality,
        verification_mode,
        true,
    ))
}

async fn run_exam_pipeline(
    config: &LlmConfig,
    exam_id: &str,
    task_prompt: &str,
    agent_output: &str,
    expected_answer: &str,
    verification_policy: ExamVerificationPolicy,
) -> Result<ExamPipelineOutput, ValidatorError> {
    let canonical_expected = canonicalize_exam_answer(expected_answer);
    let verification_mode = verification_policy.effective_mode;
    let mode_label = verification_mode.as_label();

    if let Err(failure) = check_exam_input_gate(agent_output) {
        return Ok(assemble_output(
            exam_id,
            task_prompt,
            &canonical_expected,
            "",
            ExamVerdict::GateFailed,
            format!("Exam input gate failed: {}", failure.reason()),
            AuditMeta {
                compare_mode: compare_mode::GATE_FAILED,
                llm_fallback_used: false,
                answer_verification_mode: mode_label,
                llm_raw: None,
            },
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
            AuditMeta {
                compare_mode: compare_mode::REFUSAL,
                llm_fallback_used: false,
                answer_verification_mode: mode_label,
                llm_raw: None,
            },
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
                AuditMeta {
                    compare_mode: compare_mode::ANSWER_MISSING,
                    llm_fallback_used: false,
                    answer_verification_mode: mode_label,
                    llm_raw: None,
                },
            ));
        }
    };

    let canonical_actual = canonicalize_exam_answer(&raw_answer);

    if verification_mode == AnswerVerificationMode::LlmFirst {
        return run_llm_first_verification(
            config,
            exam_id,
            task_prompt,
            agent_output,
            &canonical_expected,
            &canonical_actual,
            verification_mode,
        )
        .await;
    }

    if compare_type_h(&canonical_actual, &canonical_expected) {
        Ok(assemble_output(
            exam_id,
            task_prompt,
            &canonical_expected,
            &canonical_actual,
            ExamVerdict::Passed,
            format!("Exam answer matched expected canonical value `{canonical_expected}`."),
            AuditMeta {
                compare_mode: compare_mode::EXACT_MATCH,
                llm_fallback_used: false,
                answer_verification_mode: mode_label,
                llm_raw: None,
            },
        ))
    } else {
        resolve_mismatch_with_optional_llm(
            config,
            exam_id,
            task_prompt,
            agent_output,
            &canonical_expected,
            &canonical_actual,
            verification_mode,
        )
        .await
    }
}

/// Live exam pipeline with real refusal stage (S0 reuse).
pub async fn evaluate_exam_pipeline(
    config: &LlmConfig,
    exam_id: &str,
    task_prompt: &str,
    agent_output: &str,
    expected_answer: &str,
    verification_policy: ExamVerificationPolicy,
) -> Result<ExamPipelineOutput, ValidatorError> {
    run_exam_pipeline(
        config,
        exam_id,
        task_prompt,
        agent_output,
        expected_answer,
        verification_policy,
    )
    .await
}

/// Mock exam pipeline — no network; refusal uses mock markers via `LlmConfig::mock`.
pub fn evaluate_exam_pipeline_mock(
    exam_id: &str,
    task_prompt: &str,
    agent_output: &str,
    expected_answer: &str,
) -> ExamPipelineOutput {
    evaluate_exam_pipeline_mock_with_config(
        exam_id,
        task_prompt,
        agent_output,
        expected_answer,
        LlmConfig {
            mock: true,
            ..Default::default()
        },
        ExamVerificationPolicy::default(),
    )
}

/// Mock exam pipeline with explicit config (for E6 fallback tests).
pub fn evaluate_exam_pipeline_mock_with_config(
    exam_id: &str,
    task_prompt: &str,
    agent_output: &str,
    expected_answer: &str,
    config: LlmConfig,
    verification_policy: ExamVerificationPolicy,
) -> ExamPipelineOutput {
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
            verification_policy,
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
        assert_eq!(output.audit.compare_mode, compare_mode::EXACT_MATCH);
        assert!(!output.audit.llm_fallback_used);
        assert_eq!(output.audit.answer_verification_mode, "exact_then_llm");
        assert_eq!(
            output.audit.expected_answer_hash,
            output.audit.actual_answer_hash
        );
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
        assert_eq!(output.audit.compare_mode, compare_mode::EXACT_MATCH);
        assert!(!output.audit.llm_fallback_used);
    }

    #[test]
    fn mock_pipeline_llm_fallback_passes_on_semantic_match() {
        let config = LlmConfig {
            mock: true,
            exam_llm_equality: true,
            ..Default::default()
        };
        let output = evaluate_exam_pipeline_mock_with_config(
            EXAM_ID,
            TASK_PROMPT,
            "ANSWER: mock_equality_yes about twelve thousand usd",
            EXPECTED_CANONICAL,
            config,
            ExamVerificationPolicy::default(),
        );

        assert_eq!(output.verdict, ExamVerdict::Passed);
        assert_eq!(output.audit.compare_mode, compare_mode::LLM_FALLBACK_MATCH);
        assert!(output.audit.llm_fallback_used);
    }

    #[test]
    fn mock_pipeline_llm_first_passes_when_flag_on() {
        let config = LlmConfig {
            mock: true,
            exam_llm_equality: true,
            ..Default::default()
        };
        let output = evaluate_exam_pipeline_mock_with_config(
            EXAM_ID,
            TASK_PROMPT,
            "ANSWER: mock_equality_yes nav per share 100.47 usd",
            "100.47 usd",
            config,
            ExamVerificationPolicy::llm_first(),
        );

        assert_eq!(output.verdict, ExamVerdict::Passed);
        assert_eq!(output.audit.compare_mode, compare_mode::LLM_FIRST_MATCH);
        assert!(!output.audit.llm_fallback_used);
        assert_eq!(output.audit.answer_verification_mode, "llm_first");
    }

    #[test]
    fn mock_pipeline_llm_first_fail_safe_when_flag_off() {
        let config = LlmConfig {
            mock: true,
            exam_llm_equality: false,
            ..Default::default()
        };
        let output = evaluate_exam_pipeline_mock_with_config(
            EXAM_ID,
            TASK_PROMPT,
            "ANSWER: mock_equality_yes nav per share 100.47 usd",
            "100.47 usd",
            config,
            ExamVerificationPolicy::llm_first(),
        );

        assert_eq!(output.verdict, ExamVerdict::Failed);
        assert_eq!(output.audit.compare_mode, compare_mode::EXACT_MATCH);
        assert!(!output.audit.llm_fallback_used);
    }

    #[test]
    fn mock_pipeline_refusal_skips_llm() {
        let config = LlmConfig {
            mock: true,
            exam_llm_equality: true,
            ..Default::default()
        };
        let output = evaluate_exam_pipeline_mock_with_config(
            EXAM_ID,
            TASK_PROMPT,
            "mock_refusal: I cannot fulfill this request",
            EXPECTED_CANONICAL,
            config,
            ExamVerificationPolicy::llm_first(),
        );

        assert_eq!(output.verdict, ExamVerdict::Refusal);
        assert_eq!(output.audit.compare_mode, compare_mode::REFUSAL);
        assert!(!output.audit.llm_fallback_used);
    }

    #[test]
    fn mock_pipeline_llm_fallback_miss_on_mock_no() {
        let config = LlmConfig {
            mock: true,
            exam_llm_equality: true,
            ..Default::default()
        };
        let output = evaluate_exam_pipeline_mock_with_config(
            EXAM_ID,
            TASK_PROMPT,
            "ANSWER: mock_equality_no about twelve thousand usd",
            EXPECTED_CANONICAL,
            config,
            ExamVerificationPolicy::default(),
        );

        assert_eq!(output.verdict, ExamVerdict::Failed);
        assert_eq!(output.total, 0);
        assert_eq!(output.audit.compare_mode, compare_mode::LLM_FALLBACK_MISS);
        assert!(output.audit.llm_fallback_used);
        assert_eq!(output.audit.llm_raw.as_deref(), Some("NO"));
    }

    #[test]
    fn pipeline_output_fails_closed_on_unparseable_llm_response() {
        let equality = parse_exam_equality_response("perhaps".to_string());
        let output = output_from_llm_eval(
            EXAM_ID,
            TASK_PROMPT,
            EXPECTED_CANONICAL,
            "999 usd",
            equality,
            AnswerVerificationMode::ExactThenLlm,
            false,
        );

        assert_eq!(output.verdict, ExamVerdict::Failed);
        assert_eq!(output.audit.compare_mode, compare_mode::LLM_FALLBACK_MISS);
        assert!(output.audit.llm_fallback_used);
        assert_eq!(output.audit.llm_raw.as_deref(), Some("perhaps"));
    }

    #[test]
    fn pipeline_output_fails_closed_on_llm_transport_error() {
        let equality = ExamEqualityEval {
            is_equal: false,
            raw_output: "LLM_ERROR: no judge LLM provider available in cascade chain".to_string(),
            parse_fallback: true,
        };
        let output = output_from_llm_eval(
            EXAM_ID,
            TASK_PROMPT,
            EXPECTED_CANONICAL,
            "999 usd",
            equality,
            AnswerVerificationMode::ExactThenLlm,
            false,
        );

        assert_eq!(output.verdict, ExamVerdict::Failed);
        assert_eq!(output.audit.compare_mode, compare_mode::LLM_FALLBACK_MISS);
        assert!(output.audit.llm_fallback_used);
        assert!(
            output
                .audit
                .llm_raw
                .as_deref()
                .unwrap_or("")
                .starts_with("LLM_ERROR:")
        );
    }

    #[tokio::test]
    async fn llm_verification_missing_provider_fails_closed() {
        let config = LlmConfig {
            mock: false,
            ..Default::default()
        };
        let eval = run_llm_verification(
            &config,
            "ANSWER: 999 usd",
            EXPECTED_CANONICAL,
            "999 usd",
        )
        .await;

        assert!(!eval.is_equal);
        assert!(eval.parse_fallback);
        assert!(eval.raw_output.starts_with("LLM_ERROR:"));
    }
}
