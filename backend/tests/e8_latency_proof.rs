use axum::http::StatusCode;
use backend::{
    api::create_router,
    casper::contract::CasperClient,
    config::{Config, ValidatorPipeline},
    db::{DbPool, init_db},
};
use std::time::{Duration, Instant};
use tower::ServiceExt;

const TASK_ID: &str = "e8-latency-task";
const EXAM_TEMPLATE_ID: &str = "exam-casper-latency";
const AGENT_PK: &str = "e8-latency-agent";

fn database_url() -> String {
    std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "mysql://deagentnet:passw0rd@127.0.0.1:3307/deagentnet".to_string())
}

async fn connect_test_pool() -> DbPool {
    let url = database_url();
    init_db(&url).await.expect("init_db")
}

async fn cleanup_fixtures(pool: &DbPool) {
    let _ = sqlx::query("DELETE FROM tasks WHERE id = ?")
        .bind(TASK_ID)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM exam_assignments WHERE task_id = ?")
        .bind(TASK_ID)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM agent_exam_state WHERE agent_public_key = ?")
        .bind(AGENT_PK)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM agents WHERE public_key = ?")
        .bind(AGENT_PK)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM exam_templates WHERE id = ?")
        .bind(EXAM_TEMPLATE_ID)
        .execute(pool)
        .await;
}

#[tokio::test]
#[ignore = "requires MySQL: DATABASE_URL from backend/.env; cargo test --test e8_latency_proof -- --ignored --test-threads=1"]
async fn latency_proof_recalc_does_not_block_main_path() {
    let pool = connect_test_pool().await;
    cleanup_fixtures(&pool).await;

    // 1. Seed agent, task, exam, etc
    sqlx::query("INSERT INTO agents (public_key, name, status) VALUES (?, 'Agent 8', 'active')")
        .bind(AGENT_PK)
        .execute(&pool)
        .await
        .expect("seed agent");

    sqlx::query(
        "INSERT INTO tasks (id, domain, prompt, budget_motes, status, creator_public_key, assigned_agent_public_key, transaction_hash, deadline)
         VALUES (?, 'defi_analysis', 'Prompt', 5000000000, 'InProgress', 'test-creator', ?, 'tx-mock', 0)",
    )
    .bind(TASK_ID)
    .bind(AGENT_PK)
    .execute(&pool)
    .await
    .expect("seed task");

    sqlx::query(
        "INSERT INTO exam_templates (id, prompt, expected_answer_canonical, domain, status) VALUES (?, 't', 'a', 'defi_analysis', 'active')",
    )
    .bind(EXAM_TEMPLATE_ID)
    .execute(&pool)
    .await
    .expect("seed template");

    sqlx::query(
        "INSERT INTO exam_assignments (task_id, template_id, agent_public_key, bucket, status)
         VALUES (?, ?, ?, 'audit', 'assigned')",
    )
    .bind(TASK_ID)
    .bind(EXAM_TEMPLATE_ID)
    .bind(AGENT_PK)
    .execute(&pool)
    .await
    .expect("seed exam assignment");

    sqlx::query(
        "INSERT INTO agent_exam_state (agent_public_key, exam_urgency)
         VALUES (?, 0.0)",
    )
    .bind(AGENT_PK)
    .execute(&pool)
    .await
    .expect("seed agent exam state");

    // We will bypass LLM call and on-chain by using MOCK LLM and SKIP ONCHAIN flags
    let mut config = Config::from_env();
    config.validator_pipeline = ValidatorPipeline::Stage;
    config.exam_dispatch_loop_enabled = false;
    config.internal_service_key = Some("test-internal-key".to_string());

    let casper_client = CasperClient::new(
        "http://localhost".to_string(),
        "key".to_string(),
        "hash".to_string(),
    );
    let mut router = create_router(pool.clone(), config.clone(), casper_client);

    // Provide raw result
    let request = axum::http::Request::builder()
        .method("POST")
        .uri(format!("/api/tasks/{}/raw_result", TASK_ID))
        .header("Content-Type", "application/json")
        .header("X-Agent-Pubkey", AGENT_PK)
        .body(axum::body::Body::from(
            r#"{"result":"ANSWER: test answer"}"#,
        ))
        .unwrap();

    let raw_res = router.clone().oneshot(request).await.unwrap();
    assert_eq!(raw_res.status(), StatusCode::OK);

    // 2. Start a separate transaction and LOCK the agent_exam_state row.
    // This will artificially slow down the `spawn_exam_urgency_recalc` which tries to upsert this row.
    let mut lock_tx = pool.begin().await.unwrap();
    sqlx::query("SELECT * FROM agent_exam_state WHERE agent_public_key = ? FOR UPDATE")
        .bind(AGENT_PK)
        .fetch_one(&mut *lock_tx)
        .await
        .unwrap();

    // 3. Trigger /validate (mocked out via env var)
    temp_env::async_with_vars(
        [
            ("VALIDATOR_MOCK_LLM", Some("1")),
            ("EXAM_SKIP_ONCHAIN", Some("1")),
        ],
        async {
            let req = axum::http::Request::builder()
                .method("POST")
                .uri(format!("/api/tasks/{}/validate", TASK_ID))
                .header("Content-Type", "application/json")
                .header("Authorization", config.internal_service_key.as_deref().unwrap_or("test"))
                .body(axum::body::Body::empty())
                .unwrap();

            let validate_start = Instant::now();
            let validate_res = router.clone().oneshot(req).await.unwrap();
            let status = validate_res.status();

            let elapsed = validate_start.elapsed();

            // /validate HTTP call should return ACCEPTED immediately
            assert_eq!(status, StatusCode::ACCEPTED);
            assert!(elapsed < Duration::from_millis(500), "HTTP call should be non-blocking");

            // 4. Wait for the main path (validate_and_complete) to finish.
            // Since we mocked LLM and skipped onchain, it should finish very fast.
            // It will update task status to 'Completed'.
            let mut audited = false;
            let wait_start = Instant::now();
            while wait_start.elapsed() < Duration::from_secs(5) {
                let row = sqlx::query("SELECT validator_audit FROM tasks WHERE id = ?")
                    .bind(TASK_ID)
                    .fetch_optional(&pool)
                    .await
                    .unwrap()
                    .unwrap();

                let audit: Option<serde_json::Value> = sqlx::Row::try_get(&row, "validator_audit").unwrap();
                if audit.is_some() {
                    audited = true;
                    break;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }

            assert!(audited, "Main validation path must complete without waiting for async recalc (which is blocked by our lock)");

            // Release the lock
            lock_tx.rollback().await.unwrap();
        }
    ).await;

    cleanup_fixtures(&pool).await;
}
