//! Wave 4 C12/C13: audit DB-down (via lazy pool) and multi-instance /validate race.
//!
//! Run:
//! ```bash
//! DATABASE_URL='mysql://deagentnet:passw0rd@127.0.0.1:3307/deagentnet' \
//!   VALIDATOR_MOCK_LLM=1 EXAM_SKIP_ONCHAIN=1 \
//!   cargo test -p backend --test wave4_evil_http -- --ignored --test-threads=1 --nocapture
//! ```

use std::time::Duration;

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
use tower::ServiceExt;

const TASK_ID: &str = "w4-evil-validate-race";
const AGENT_PK: &str = "w4-evil-agent";
const CREATOR_PK: &str = "w4-evil-creator";
const INTERNAL_SERVICE_KEY: &str = "w4-evil-internal-key";

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

async fn cleanup(pool: &DbPool) {
    let _ = sqlx::query("DELETE FROM validate_leases WHERE task_id = ?")
        .bind(TASK_ID)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM validations WHERE task_id = ?")
        .bind(TASK_ID)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM tasks WHERE id = ?")
        .bind(TASK_ID)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM agents WHERE public_key = ?")
        .bind(AGENT_PK)
        .execute(pool)
        .await;
}

async fn seed_inprogress_with_result(pool: &DbPool) {
    sqlx::query(
        "INSERT INTO agents (public_key, name, status, active_jobs)
         VALUES (?, 'W4 Evil Agent', 'active', 0)
         ON DUPLICATE KEY UPDATE status='active'",
    )
    .bind(AGENT_PK)
    .execute(pool)
    .await
    .expect("seed agent");

    sqlx::query(
        "INSERT INTO tasks (
            id, creator_public_key, assigned_agent_public_key, budget_motes, status,
            transaction_hash, domain, prompt, deadline, result_hash, result, timestamp
         ) VALUES (?, ?, ?, 100, 'InProgress', 'w4-tx', 'defi_analysis', 'prompt', 999999,
                   'rh', 'Recommended allocation across pools with fee-adjusted APY detail.', NOW())
         ON DUPLICATE KEY UPDATE status='InProgress', result='Recommended allocation across pools with fee-adjusted APY detail.',
           validator_audit=NULL",
    )
    .bind(TASK_ID)
    .bind(CREATOR_PK)
    .bind(AGENT_PK)
    .execute(pool)
    .await
    .expect("seed task");
}

/// Wave 4 scenario 13: two independent routers share one DB; DB lease allows only one acceptor.
#[tokio::test]
#[ignore]
async fn test_w4_validate_multi_instance_inflight_gap() {
    let pool = connect_test_pool().await;
    cleanup(&pool).await;
    seed_inprogress_with_result(&pool).await;

    // Two separate AppStates → two ValidateInflight maps; coordination is via validate_leases.
    let router_a = build_test_router(pool.clone());
    let router_b = build_test_router(pool.clone());

    let req = || {
        Request::builder()
            .method("POST")
            .uri(format!("/api/tasks/{TASK_ID}/validate"))
            .header(header::AUTHORIZATION, INTERNAL_SERVICE_KEY)
            .body(Body::empty())
            .unwrap()
    };

    let (res_a, res_b) = tokio::join!(router_a.oneshot(req()), router_b.oneshot(req()));

    let res_a = res_a.expect("a");
    let res_b = res_b.expect("b");
    let status_a = res_a.status();
    let status_b = res_b.status();
    assert_eq!(status_a, StatusCode::ACCEPTED, "a={status_a:?}");
    assert_eq!(status_b, StatusCode::ACCEPTED, "b={status_b:?}");

    let body_a = axum::body::to_bytes(res_a.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let body_b = axum::body::to_bytes(res_b.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let json_a: serde_json::Value = serde_json::from_slice(&body_a).expect("json a");
    let json_b: serde_json::Value = serde_json::from_slice(&body_b).expect("json b");
    let status_str_a = json_a["status"].as_str().unwrap_or("");
    let status_str_b = json_b["status"].as_str().unwrap_or("");

    let accepted_count = [status_str_a, status_str_b]
        .iter()
        .filter(|s| **s == "accepted")
        .count();
    let in_progress_count = [status_str_a, status_str_b]
        .iter()
        .filter(|s| **s == "in_progress")
        .count();

    assert_eq!(
        accepted_count, 1,
        "exactly one instance must accept: a={status_str_a:?} b={status_str_b:?}"
    );
    assert_eq!(
        in_progress_count, 1,
        "the other instance must report in_progress: a={status_str_a:?} b={status_str_b:?}"
    );

    println!(
        "[PASS] scenario 13: distributed validate lease — one accepted, one in_progress (a={status_str_a}, b={status_str_b})"
    );

    // Let spawned tasks settle briefly
    tokio::time::sleep(Duration::from_millis(500)).await;
    cleanup(&pool).await;
}

/// Wave 4 scenario 12 (HTTP): GET /api/audit/logs with dead pool → 500, no password in body.
#[tokio::test]
#[ignore]
async fn test_w4_audit_http_db_down() {
    let pool = sqlx::mysql::MySqlPoolOptions::new()
        .acquire_timeout(Duration::from_secs(1))
        .connect_lazy("mysql://deagentnet:passw0rd@127.0.0.1:1/deagentnet")
        .expect("lazy");

    let app = build_test_router(pool);
    let res = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/audit/logs")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("oneshot");

    assert_eq!(res.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let body = axum::body::to_bytes(res.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let text = String::from_utf8_lossy(&body);
    assert!(!text.contains("passw0rd"), "no password in body: {text}");
    println!("[PASS] scenario 12 (HTTP): audit 500 without DSN leak");
}
