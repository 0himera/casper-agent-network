use validator_engine::harness::load_stage_golden_cases;
use validator_engine::{LlmConfig, evaluate_stage_pipeline};

const CASE_IDS: &[&str] = &[
    "good_defi_allocation",
    "refusal_explicit",
    "gibberish_marker",
    "irrelevant_marker",
    "wrong_domain_marker",
];

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let mut config = LlmConfig::from_env();
    config.mock = false;

    let cases = load_stage_golden_cases().expect("stage golden cases");
    println!(
        "Full stage pipeline S0–S3 (real LLM, adaptive delay: start 1s, +0.5s on rate-limit)\n"
    );

    for id in CASE_IDS {
        let case = cases
            .iter()
            .find(|c| c.id == *id)
            .unwrap_or_else(|| panic!("missing case {id}"));

        match evaluate_stage_pipeline(&case.domain, &case.task_prompt, &case.agent_output, &config)
            .await
        {
            Ok(output) => {
                println!("=== {id} ===");
                println!("verdict={:?} total={}", output.verdict, output.total);
                println!("explanation: {}", output.explanation);
                for stage in &output.stages {
                    println!(
                        "  {} passed={} raw={:?} quality={:.2} skipped={}",
                        stage.id.as_str(),
                        stage.passed,
                        stage.raw_output,
                        stage.normalized_quality,
                        stage.skipped_due_to_gate
                    );
                }
            }
            Err(e) => {
                println!("=== {id} === ERROR: {e}");
            }
        }
        println!();
    }
}
