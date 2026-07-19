//! E2E full lifecycle integration test for Casper Agent Network (CAN).
//! Exercises: Create Task -> Assign -> Submit Result -> 3x Submit Validation -> Finalize -> Check Reputation.

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
};
use backend::{
    api::create_router,
    casper::contract::CasperClient,
    config::{Config, ValidatorPipeline},
    db::{DbPool, init_db},
};
use serde_json::json;
use tower::ServiceExt;

const INTERNAL_SERVICE_KEY: &str = "test-internal-key";
const TASK_ID: &str = "e2e-task-123";
const AGENT_PK: &str = "01d0a514d79d989f67a2176b66d6c97a7372b05ffe40cdcd9e473d4a2176be600";
const CREATOR_PK: &str = "01a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2";

fn database_url() -> String {
    std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "mysql://deagentnet:passw0rd@localhost:3307/deagentnet".to_string())
}

async fn connect_test_pool() -> DbPool {
    let url = database_url();
    init_db(&url).await.unwrap_or_else(|err| {
        panic!(
            "E2E tests require MySQL at DATABASE_URL ({url}): {err}. \
             Run: DATABASE_URL=... cargo test --test e2e_full_lifecycle -- --ignored"
        )
    })
}

fn build_test_router(pool: DbPool) -> Router {
    let config = Config {
        database_url: database_url(),
        port: 3000,
        openai_api_key: None,
        claude_api_key: None,
        ollama_url: None,
        ollama_model: None,
        internal_service_key: Some(INTERNAL_SERVICE_KEY.to_string()),
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
    };
    let casper_client = CasperClient::new(
        "http://localhost".to_string(),
        "test-access-key".to_string(),
        "test-package-hash".to_string(),
    );
    create_router(pool, config, casper_client)
}

#[tokio::test]
#[ignore]
async fn test_full_e2e_lifecycle() {
    let pool = connect_test_pool().await;

    // Clean up fixtures
    let _ = sqlx::query("DELETE FROM validations WHERE task_id = ?").bind(TASK_ID).execute(&pool).await;
    let _ = sqlx::query("DELETE FROM tasks WHERE id = ?").bind(TASK_ID).execute(&pool).await;
    let _ = sqlx::query("DELETE FROM agents WHERE public_key = ?").bind(AGENT_PK).execute(&pool).await;
    let _ = sqlx::query("DELETE FROM reputations WHERE agent_public_key = ?").bind(AGENT_PK).execute(&pool).await;

    let app = build_test_router(pool.clone());

    // 1. Register Agent
    let register_payload = json!({
        "public_key": AGENT_PK,
        "name": "E2E Test Agent",
        "description": "DeFi Valuation capabilities",
        "metadata_uri": "https://test-agent.dev",
        "endpoint_url": "http://localhost:5000/execute",
        "api_key": "some-agent-api-key",
        "model": "gpt-4o",
        "custom_price_motes": 5_000_000_000u64,
        "system_prompt": "You are a valuation agent.",
        "delegated_signer": AGENT_PK
    });

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/agents/register")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&register_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);

    // 2. Create Task
    let task_payload = json!({
        "id": TASK_ID,
        "creator_public_key": CREATOR_PK,
        "budget_motes": 10_000_000_000u64,
        "transaction_hash": "e2e-create-tx",
        "domain": "defi_analysis",
        "prompt": "Evaluate Casper yield curves",
        "deadline": 1781281517u64
    });

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/tasks")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&task_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);

    // 3. Assign Task
    let assign_payload = json!({
        "task_id": TASK_ID,
        "assigned_agent_public_key": AGENT_PK,
        "transaction_hash": "e2e-assign-tx"
    });

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/tasks/assign")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&assign_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);

    // 4. Submit Result (with verified delegated signature)
    let submit_payload = json!({
        "task_id": TASK_ID,
        "result_hash": "0xabc",
        "result": "DeFi yield analysis output",
        "signature": "01d0a514d79d989f67a2176b66d6c97a7372b05ffe40cdcd9e473d4a2176be600abc"
    });

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/tasks/submit")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&submit_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);

    // 5. Submit 3 Validator Gradings
    for i in 1..=3 {
        let val_pk = format!("01-validator-{}", i);
        let val_payload = json!({
            "task_id": TASK_ID,
            "validator_public_key": val_pk,
            "verdict": "pass",
            "score": 95,
            "reason": format!("Validator {} approved", i),
            "signature": format!("signature-val-{}", i)
        });

        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tasks/validate")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(serde_json::to_vec(&val_payload).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(res.status(), StatusCode::OK);
    }

    // 6. Finalize Task (reaches quorum and releases funds)
    let finalize_payload = json!({
        "task_id": TASK_ID,
        "transaction_hash": "e2e-finalize-tx"
    });

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/tasks/finalize")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&finalize_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);

    // 7. Check Database reputation update
    let row: Option<(i32,)> = sqlx::query_as("SELECT score FROM reputations WHERE agent_public_key = ? AND skill = ?")
        .bind(AGENT_PK)
        .bind("defi_analysis")
        .fetch_optional(&pool)
        .await
        .unwrap();

    assert!(row.is_some(), "Reputation should be updated");
}
