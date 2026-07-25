//! Wave 4 G25: A2A parent_task_id backend HTTP round-trip.
//!
//! Run:
//! ```bash
//! DATABASE_URL='mysql://deagentnet:passw0rd@127.0.0.1:3307/deagentnet' \
//!   cargo test -p backend --test wave4_parent_task_roundtrip -- --ignored --test-threads=1 --nocapture
//! ```

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

const PARENT_ID: &str = "w4-parent-abc_1";
const CHILD_ID: &str = "w4-child-xyz_1";
const CREATOR_PK: &str = "w4-parent-creator";

fn database_url() -> String {
    std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "mysql://deagentnet:passw0rd@127.0.0.1:3307/deagentnet".to_string())
}

async fn connect_test_pool() -> DbPool {
    let url = database_url();
    init_db(&url)
        .await
        .unwrap_or_else(|err| panic!("MySQL required at {url}: {err}"))
}

fn build_test_router(pool: DbPool) -> Router {
    let config = Config {
        database_url: database_url(),
        port: 3000,
        openai_api_key: None,
        claude_api_key: None,
        ollama_url: None,
        ollama_model: None,
        internal_service_key: Some("w4-parent-internal".to_string()),
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

async fn cleanup(pool: &DbPool) {
    let _ = sqlx::query("DELETE FROM tasks WHERE id IN (?, ?)")
        .bind(PARENT_ID)
        .bind(CHILD_ID)
        .execute(pool)
        .await;
}

/// G25: POST parent + child with parent_task_id → GET child returns the same field.
#[tokio::test]
#[ignore]
async fn test_w4_parent_task_id_http_roundtrip() {
    let pool = connect_test_pool().await;
    cleanup(&pool).await;

    let app = build_test_router(pool.clone());

    let parent_payload = json!({
        "id": PARENT_ID,
        "creator_public_key": CREATOR_PK,
        "budget_motes": 5_000_000_000u64,
        "transaction_hash": "w4-parent-tx",
        "domain": "defi_analysis",
        "prompt": "parent task",
        "deadline": 999999u64
    });

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/tasks")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&parent_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK, "parent create");

    let child_payload = json!({
        "id": CHILD_ID,
        "creator_public_key": CREATOR_PK,
        "budget_motes": 5_000_000_000u64,
        "transaction_hash": "w4-child-tx",
        "domain": "defi_analysis",
        "prompt": "child task",
        "deadline": 999999u64,
        "parent_task_id": PARENT_ID
    });

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/tasks")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&child_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK, "child create");

    let res = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/tasks/{CHILD_ID}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let task: serde_json::Value = serde_json::from_slice(&body).expect("task json");
    assert_eq!(
        task["parent_task_id"].as_str(),
        Some(PARENT_ID),
        "parent_task_id must round-trip via GET /api/tasks/{{id}}: {task}"
    );

    println!("[PASS] G25 backend HTTP: parent_task_id round-trip");
    cleanup(&pool).await;
}
