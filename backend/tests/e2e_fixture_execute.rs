use backend::config::Config;
use backend::orchestrator::task_pipeline::{run_fixture_pipeline_with_output, sample_task};
use backend::validator::V2Outcome;
use validator_engine::harness::{load_fixture, load_golden_cases};

fn test_config() -> Config {
    Config {
        database_url: "mysql://localhost".to_string(),
        port: 3000,
        openai_api_key: None,
        claude_api_key: None,
        ollama_url: None,
        ollama_model: None,
        internal_service_key: None,
        cloudflare_account_id: None,
        cloudflare_api_token: None,
        fireworks_api_key: None,
        fireworks_model: None,
        validator_url: None,
        validator_api_key: None,
        validator_model: None,
        validator_provider: None,
        validator_pipeline: backend::config::ValidatorPipeline::Legacy,
        admin_account: String::new(),
    }
}

#[tokio::test]
async fn e2e_injected_fixture_v2_pipeline_defi_yield_routing() {
    temp_env::async_with_vars([("VALIDATOR_MOCK_LLM", Some("1"))], async {
        let fixture = load_fixture("defi_yield_routing.json").expect("fixture");
        let cases = load_golden_cases().expect("golden cases");
        let good_case = cases
            .iter()
            .find(|c| c.id == "defi_yield_routing_good")
            .expect("golden good case");

        let task = sample_task(
            Some("defi_yield_routing"),
            "defi_analysis",
            &good_case.task_prompt,
        );
        let config = test_config();

        let result = run_fixture_pipeline_with_output(
            &task,
            &good_case.agent_output,
            good_case.processing_time_ms,
            fixture.clone(),
            &config,
        )
        .await
        .expect("pipeline ok");

        assert!(result.worker_prompt.contains("<fixture>"));
        assert!(result.worker_prompt.contains("amount_cspr"));
        assert_eq!(result.skill, "defi_yield_routing");

        match result.v2_outcome {
            V2Outcome::Ok(output) => {
                assert_eq!(output.criteria.len(), 5);
                assert_eq!(output.total, 100);
            }
            other => panic!("expected Ok v2 outcome, got {other:?}"),
        }
    })
    .await;
}

#[tokio::test]
async fn e2e_skill_id_resolves_over_domain() {
    temp_env::async_with_vars([("VALIDATOR_MOCK_LLM", Some("1"))], async {
        let fixture = load_fixture("rwa_appraisal.json").expect("fixture");
        let cases = load_golden_cases().expect("golden cases");
        let good_case = cases
            .iter()
            .find(|c| c.id == "rwa_appraisal_good")
            .expect("golden good case");

        let task = sample_task(
            Some("rwa_appraisal"),
            "defi_analysis",
            &good_case.task_prompt,
        );
        let config = test_config();

        let result = run_fixture_pipeline_with_output(
            &task,
            &good_case.agent_output,
            good_case.processing_time_ms,
            fixture,
            &config,
        )
        .await
        .expect("pipeline ok");

        assert_eq!(result.skill, "rwa_appraisal");
        assert!(matches!(result.v2_outcome, V2Outcome::Ok(_)));
    })
    .await;
}

#[tokio::test]
async fn e2e_invalid_fixture_returns_fixture_invalid() {
    let task = sample_task(Some("defi_yield_routing"), "defi_analysis", "Allocate CSPR");
    let config = test_config();
    let invalid_fixture = serde_json::json!({ "amount_cspr": 10000 });

    let result =
        run_fixture_pipeline_with_output(&task, "some output", 1000, invalid_fixture, &config)
            .await
            .expect("pipeline returns result even on invalid fixture");

    assert!(matches!(result.v2_outcome, V2Outcome::FixtureInvalid(_)));
}
