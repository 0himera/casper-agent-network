use validator_engine::harness::{load_stage_golden_cases, load_stage_llm_real_smoke_cases};
use validator_engine::{LlmConfig, evaluate_stage_pipeline};

/// Legacy subset from stage_golden_cases.json when STAGE_LLM_USE_GOLDEN=1.
const LEGACY_CASE_IDS: &[&str] = &[
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

    let use_golden = std::env::var("STAGE_LLM_USE_GOLDEN")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    let (label, cases) = if use_golden {
        (
            "stage_golden_cases.json (legacy subset)",
            load_stage_golden_cases().expect("stage golden cases"),
        )
    } else {
        (
            "stage_llm_real_smoke_cases.json (Wave 5 B4/B5)",
            load_stage_llm_real_smoke_cases().expect("stage llm real smoke cases"),
        )
    };

    println!("Full stage pipeline S0–S3 (real LLM, adaptive delay: start 1s, +0.5s on rate-limit)");
    println!("Cases: {label}\n");

    let selected: Vec<_> = if use_golden {
        LEGACY_CASE_IDS
            .iter()
            .filter_map(|id| cases.iter().find(|c| c.id == *id).cloned())
            .collect()
    } else {
        cases
    };

    let mut matched = 0usize;
    let mut mismatched = 0usize;

    for case in &selected {
        match evaluate_stage_pipeline(&case.domain, &case.task_prompt, &case.agent_output, &config)
            .await
        {
            Ok(output) => {
                let actual = output.verdict.as_label();
                let expected = case.expect.verdict.as_str();
                let mut ok = actual == expected;
                if expected == "factual" {
                    if let Some(gte) = case.expect.total_gte {
                        ok = ok && output.total >= gte;
                    }
                } else if let Some(lt) = case.expect.total_lt {
                    ok = ok && output.total < lt;
                }

                if ok {
                    matched += 1;
                } else {
                    mismatched += 1;
                }

                println!("=== {} ===", case.id);
                println!(
                    "expect_verdict={} verdict={} total={} ok={}",
                    expected, actual, output.total, ok
                );
                println!("explanation: {}", output.explanation);
                for stage in &output.stages {
                    let pf = stage
                        .details
                        .as_ref()
                        .and_then(|d| d.get("parse_fallback"))
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    println!(
                        "  {} passed={} raw={:?} quality={:.2} skipped={} parse_fallback={}",
                        stage.id.as_str(),
                        stage.passed,
                        stage.raw_output,
                        stage.normalized_quality,
                        stage.skipped_due_to_gate,
                        pf
                    );
                }
            }
            Err(e) => {
                mismatched += 1;
                println!("=== {} === ERROR: {e}", case.id);
            }
        }
        println!();
    }

    println!("Summary: {matched} matched expect band, {mismatched} mismatched or errored");
    if mismatched > 0 {
        std::process::exit(1);
    }
}
