use validator_engine::harness::{
    load_exam_equality_real_smoke_cases, verification_policy_for_real_smoke_case,
};
use validator_engine::{LlmConfig, evaluate_exam_pipeline};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let mut config = LlmConfig::from_env();
    config.mock = false;
    config.exam_llm_equality = true;

    let cases = load_exam_equality_real_smoke_cases().expect("exam equality real smoke cases");

    println!("Exam LLM-equality manual smoke (real LLM, EXAM_LLM_EQUALITY=1)\n");
    println!(
        "  VALIDATOR_PROVIDER={}",
        std::env::var("VALIDATOR_PROVIDER").unwrap_or_else(|_| "<auto>".to_string())
    );
    println!(
        "  VALIDATOR_LLM_MODEL={}",
        std::env::var("VALIDATOR_LLM_MODEL")
            .or_else(|_| std::env::var("OPENAI_MODEL"))
            .unwrap_or_else(|_| "<provider default>".to_string())
    );
    println!();

    let mut pass_count = 0usize;
    let mut fail_count = 0usize;

    for case in &cases {
        let verification_policy = verification_policy_for_real_smoke_case(case);
        let agent_output = format!("ANSWER: {}", case.candidate_answer);

        match evaluate_exam_pipeline(
            &config,
            &case.id,
            &case.task_prompt,
            &agent_output,
            &case.expected_answer,
            verification_policy,
        )
        .await
        {
            Ok(output) => {
                let predicted_pass = output.verdict == validator_engine::ExamVerdict::Passed;
                let label_pass = case.label.eq_ignore_ascii_case("pass");
                let ok = predicted_pass == label_pass;
                if ok {
                    pass_count += 1;
                } else {
                    fail_count += 1;
                }

                println!("=== {} ===", case.id);
                println!(
                    "label={} verdict={} total={} compare_mode={} verification_mode={} ok={}",
                    case.label,
                    output.verdict.as_label(),
                    output.total,
                    output.audit.compare_mode,
                    output.audit.answer_verification_mode,
                    ok
                );
                println!("explanation: {}", output.explanation);
                if let Some(raw) = &output.audit.llm_raw {
                    println!("llm_raw: {raw}");
                }
            }
            Err(err) => {
                fail_count += 1;
                println!("=== {} === ERROR: {err}", case.id);
            }
        }
        println!();
    }

    println!("Summary: {pass_count} matched label, {fail_count} mismatched or errored");
    if fail_count > 0 {
        std::process::exit(1);
    }
}
