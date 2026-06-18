use validator_engine::GraderOptions;
use validator_engine::LlmConfig;
use validator_engine::harness::{compare_few_shot_uplift, load_soft_calibration};

fn real_llm_config() -> LlmConfig {
    LlmConfig::from_env()
}

/// Real-LLM A/B: baseline (v1, no few-shot) vs treatment (v2, few-shot).
/// Run manually: `./scripts/few_shot_uplift.sh` or `cargo test --test soft_uplift -- --ignored --nocapture`
#[tokio::test]
#[ignore = "requires real LLM API keys; mock always returns strong labels"]
async fn few_shot_uplift_real_llm_ab() {
    let config = real_llm_config();
    assert!(
        !config.mock,
        "set VALIDATOR_MOCK_LLM=0 and configure a judge LLM provider"
    );

    let cases = load_soft_calibration().expect("soft calibration must load");
    let report = compare_few_shot_uplift(&cases, &config)
        .await
        .expect("uplift compare ok");

    println!(
        "Baseline accuracy: {:.1}%",
        report.baseline.accuracy * 100.0
    );
    println!(
        "Treatment accuracy: {:.1}%",
        report.treatment.accuracy * 100.0
    );
    println!("Delta accuracy: {:+.1}pp", report.delta_accuracy * 100.0);

    for result in &report.baseline.case_results {
        if !result.matched {
            println!(
                "baseline mismatch {}: {}",
                result.case_id,
                result.mismatch_reason.as_deref().unwrap_or("")
            );
        }
    }
    for result in &report.treatment.case_results {
        if !result.matched {
            println!(
                "treatment mismatch {}: {}",
                result.case_id,
                result.mismatch_reason.as_deref().unwrap_or("")
            );
        }
    }

    assert!(
        report.treatment.accuracy >= report.baseline.accuracy,
        "few-shot treatment should not regress vs baseline"
    );
}

#[tokio::test]
async fn default_options_use_few_shot_v2() {
    let options = GraderOptions::default();
    assert!(options.few_shot_enabled);
    assert!(options.prompt_version.is_none());

    let baseline = GraderOptions::f3_baseline();
    assert!(!baseline.few_shot_enabled);
    assert_eq!(baseline.prompt_version, Some("v1"));

    let treatment = GraderOptions::f3_few_shot();
    assert!(treatment.few_shot_enabled);
    assert_eq!(treatment.prompt_version, Some("v2"));
}
