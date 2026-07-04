use backend::config::{Config, ValidatorPipeline};
use backend::validator::exam_adapter::evaluate_exam_task;

fn sample_config() -> Config {
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
        validator_pipeline: ValidatorPipeline::Stage,
        admin_account: String::new(),
        exam_weight: 300,
        exam_dispatch_prob_audit: 0.2,
        exam_dispatch_prob_rehab: 0.5,
        exam_max_per_agent_per_period: 1,
        exam_dispatch_period_hours: 24,
        exam_rehab_score_threshold: 0,
        exam_audit_active_jobs_threshold: 2,
        exam_dispatch_budget_motes: 5_000_000_000,
        exam_dispatch_creator_public_key: String::new(),
        exam_llm_equality: false,
        exam_dispatch_loop_enabled: false,
        exam_dispatch_loop_interval_secs: 300,
        exam_selection_mode: backend::config::ExamSelectionMode::Bucket,
        exam_urgency_base_prob: 0.1,
        exam_urgency_task_weight: 0.05,
        exam_urgency_variance_weight: 0.2,
        exam_urgency_recent_verdicts: 5,
        exam_smoothed_ema_alpha: 0.3,
        exam_leaderboard_use_smoothed: false,
    }
}

#[tokio::test]
async fn evaluate_exam_task_llm_fallback_miss_audit_shape() {
    temp_env::async_with_vars(
        [
            ("VALIDATOR_MOCK_LLM", Some("1")),
            ("EXAM_LLM_EQUALITY", Some("1")),
        ],
        async {
            let mut config = sample_config();
            config.exam_llm_equality = true;
            let result = evaluate_exam_task(
                "exam-template-1",
                "defi_analysis",
                "Compute yield",
                "ANSWER: mock_equality_no about twelve thousand usd",
                "12345.67 usd",
                None,
                4000,
                &config,
            )
            .await
            .expect("exam mock eval");

            assert_eq!(result.total, 0);
            let audit = result.validator_audit.expect("audit");
            assert_eq!(audit["compare_mode"], "llm_fallback_miss");
            assert_eq!(audit["llm_fallback_used"], true);
            assert_eq!(audit["llm_raw"], "NO");
        },
    )
    .await;
}

#[tokio::test]
async fn evaluate_exam_task_returns_err_when_no_llm_provider() {
    temp_env::async_with_vars([("VALIDATOR_MOCK_LLM", Some("0"))], async {
        let mut config = sample_config();
        config.exam_llm_equality = true;
        let result = evaluate_exam_task(
            "exam-template-1",
            "defi_analysis",
            "Compute yield",
            "ANSWER: 999 usd",
            "12345.67 usd",
            None,
            4000,
            &config,
        )
        .await;

        assert!(result.is_err(), "missing provider must not silently pass");
    })
    .await;
}
