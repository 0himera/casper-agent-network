use std::collections::HashMap;

use validator_engine::harness::load_stage_golden_cases;
use validator_engine::stage_pipeline::stages::relevance::parse_relevance_response;
use validator_engine::{LlmConfig, build_stage_relevance_prompts_version, call_judge_raw};

const PROMPT_VERSIONS: &[&str] = &["v1", "v2"];

const CASE_IDS: &[&str] = &[
    "good_defi_allocation",
    "wrong_domain_marker",
    "wrong_domain_code_in_defi",
    "irrelevant_marker",
    "irrelevant_off_topic",
];

fn expected_relevant(case_id: &str) -> bool {
    matches!(
        case_id,
        "good_defi_allocation" | "wrong_domain_marker" | "wrong_domain_code_in_defi"
    )
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

    println!("Relevance prompt benchmark (real LLM, stage S2 only)");
    println!("Expected: good/wrong_domain* => relevant (>=6); irrelevant* => not relevant (<6)\n");

    for version in PROMPT_VERSIONS {
        let mut correct = 0u32;
        println!("=== {version} ===");

        for case in &selected {
            let (system, user) = match build_stage_relevance_prompts_version(
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

            let text = match call_judge_raw(&config, "stage_relevance", &system, &user).await {
                Ok(t) => t,
                Err(e) => {
                    println!("  {} LLM_ERR: {e} MISS", case.id);
                    continue;
                }
            };

            let eval = parse_relevance_response(text);
            let exp = expected_relevant(&case.id);
            let got_relevant = eval.passed;
            let ok = got_relevant == exp;
            if ok {
                correct += 1;
            }

            println!(
                "  {} expect_relevant={} got_relevant={} raw={:?} {}",
                case.id,
                exp,
                got_relevant,
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
