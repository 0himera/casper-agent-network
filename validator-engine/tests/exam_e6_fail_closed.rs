use validator_engine::exam::equality::parse_exam_equality_response;
use validator_engine::{
    ExamVerdict, ExamVerificationPolicy, LlmConfig, call_judge_raw,
    evaluate_exam_pipeline_mock_with_config,
};

const EXAM_ID: &str = "exam-test-1";
const TASK_PROMPT: &str = "Compute the fee-adjusted yield and return ANSWER: <value>";
const EXPECTED_CANONICAL: &str = "12345.67 usd";

#[test]
fn parse_response_empty_payload_fails_closed() {
    let eval = parse_exam_equality_response(String::new());
    assert!(!eval.is_equal);
    assert!(eval.parse_fallback);
    assert!(eval.raw_output.is_empty());
}

#[test]
fn parse_response_whitespace_only_fails_closed() {
    let eval = parse_exam_equality_response("   \n\t  ".to_string());
    assert!(!eval.is_equal);
    assert!(eval.parse_fallback);
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
    assert_eq!(output.audit.compare_mode, "llm_fallback_miss");
    assert!(output.audit.llm_fallback_used);
    assert_eq!(output.audit.llm_raw.as_deref(), Some("NO"));
}

#[tokio::test]
async fn call_judge_raw_no_provider_returns_llm_error() {
    let config = LlmConfig::default();
    let result = call_judge_raw(
        &config,
        "exam_equality",
        "system",
        "Expected answer:\n1 usd\n\nCandidate answer:\n1 usd",
    )
    .await;

    assert!(result.is_err());
    let message = result.unwrap_err().to_string();
    assert!(
        message.contains("no judge LLM provider") || message.contains("LLM"),
        "unexpected error: {message}"
    );
}
