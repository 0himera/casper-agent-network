use validator_engine::GraderOptions;
use validator_engine::LlmConfig;
use validator_engine::harness::{load_golden_cases, run_determinism, run_regression};

fn mock_config() -> LlmConfig {
    LlmConfig {
        mock: true,
        ..Default::default()
    }
}

#[tokio::test]
async fn golden_case_regression_all_match() {
    let cases = load_golden_cases().expect("golden manifest must load");
    let options = GraderOptions::default();
    let metrics = run_regression(&cases, &mock_config(), &options)
        .await
        .expect("regression run ok");

    assert_eq!(
        metrics.accuracy,
        1.0,
        "regression failures: {:?}",
        metrics
            .case_results
            .iter()
            .filter(|r| !r.matched)
            .collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn golden_case_determinism_r5_flip_rate_zero() {
    let cases = load_golden_cases().expect("golden manifest must load");
    let options = GraderOptions::default();
    let metrics = run_determinism(&cases, &mock_config(), &options, 5)
        .await
        .expect("determinism run ok");

    assert_eq!(metrics.flip_rate, 0.0);
    assert!(metrics.case_results.iter().all(|r| r.matched));
}
