use std::collections::HashMap;

use validator_engine::harness::load_stage_golden_cases;
use validator_engine::stage_pipeline::stages::refusal::parse_refusal_response;
use validator_engine::{LlmConfig, build_stage_refusal_prompts_version, call_judge_raw};

const PROMPT_VERSIONS: &[&str] = &["v1", "v2"];

const CASE_IDS: &[&str] = &[
    "good_defi_allocation",
    "refusal_explicit",
    "gibberish_marker",
    "irrelevant_marker",
    "wrong_domain_marker",
];

fn expected_refusal(case_id: &str) -> bool {
    matches!(case_id, "refusal_explicit")
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let mut config = LlmConfig::from_env();
    config.mock = false;

    let cases = load_stage_golden_cases().expect("stage golden cases");
    let selected: Vec<_> = CASE_IDS
        .iter()
        .map(|id| {
            cases
                .iter()
                .find(|c| c.id == *id)
                .unwrap_or_else(|| panic!("missing case {id}"))
        })
        .collect();

    let mut summary: HashMap<&str, u32> = HashMap::new();

    println!("Refusal prompt benchmark (real LLM, stage S0 only)");
    println!("Expected: good/gibberish/irrelevant/wrong_domain => no; refusal_explicit => yes\n");

    for version in PROMPT_VERSIONS {
        let mut correct = 0u32;
        println!("=== {version} ===");

        for case in &selected {
            let (system, user) = match build_stage_refusal_prompts_version(
                &case.task_prompt,
                &case.agent_output,
                Some(version),
            ) {
                Ok(p) => p,
                Err(e) => {
                    println!("  {} BUILD_ERR: {e} MISS", case.id);
                    continue;
                }
            };

            let text = match call_judge_raw(&config, "stage_refusal", &system, &user).await {
                Ok(t) => t,
                Err(e) => {
                    println!("  {} LLM_ERR: {e} MISS", case.id);
                    continue;
                }
            };

            let eval = parse_refusal_response(text);
            let exp = expected_refusal(&case.id);
            let ok = eval.is_refusal == exp;
            if ok {
                correct += 1;
            }

            println!(
                "  {} expect_refusal={} got_refusal={} raw={:?} {}",
                case.id,
                exp,
                eval.is_refusal,
                eval.raw_output,
                if ok { "OK" } else { "MISS" }
            );
        }

        summary.insert(version, correct);
        println!("  score: {correct}/5\n");
    }

    println!("=== RANKING ===");
    let mut ranked: Vec<_> = summary.iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));

    for (version, score) in ranked {
        let label = if *score == 5 {
            "ACCEPTABLE"
        } else if *score >= 4 {
            "near-miss"
        } else {
            "reject"
        };
        println!("{version}: {score}/5 ({label})");
    }
}
