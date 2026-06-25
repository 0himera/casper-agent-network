//! E2 HTTP integration smoke: autonomous exam path via router (no live server).

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
use sqlx::Row;
use tower::ServiceExt;

const TASK_ID: &str = "e2-http-autonomous";
const EXAM_TEMPLATE_ID: &str = "exam-casper-total-stake-block-5000000";
const EXAM_CANONICAL: &str = "2845678901.25 cspr";
const AGENT_PK: &str = "e2-http-agent";
const CREATOR_PK: &str = "e2-http-creator";
const E4_DISPATCH_AGENT: &str = "e4-http-dispatch-agent";
const INTERNAL_SERVICE_KEY: &str = "test-internal-key";

fn database_url() -> String {
    std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "mysql://deagentnet:passw0rd@localhost:3307/deagentnet".to_string())
}

async fn connect_test_pool() -> DbPool {
    let url = database_url();
    init_db(&url).await.unwrap_or_else(|err| {
        panic!(
            "E2 HTTP tests require MySQL at DATABASE_URL ({url}): {err}. \
             Run: DATABASE_URL=... cargo test --test e2_autonomous_http -- --ignored --test-threads=1"
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
    };
    let casper_client = CasperClient::new(
        "http://localhost".to_string(),
        "test-access-key".to_string(),
        "test-package-hash".to_string(),
    );
    create_router(pool, config, casper_client)
}

fn build_dispatch_test_router(pool: DbPool) -> Router {
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
        exam_dispatch_prob_audit: 0.99,
        exam_dispatch_prob_rehab: 0.99,
        exam_max_per_agent_per_period: 1,
        exam_dispatch_period_hours: 24,
        exam_rehab_score_threshold: 0,
        exam_audit_active_jobs_threshold: 2,
        exam_dispatch_budget_motes: 5_000_000_000,
        exam_dispatch_creator_public_key: CREATOR_PK.to_string(),
    };
    let casper_client = CasperClient::new(
        "http://localhost".to_string(),
        "test-access-key".to_string(),
        "test-package-hash".to_string(),
    );
    create_router(pool, config, casper_client)
}

async fn cleanup_e4_dispatch_fixtures(pool: &DbPool) {
    let _ = sqlx::query("DELETE FROM exam_assignments WHERE task_id LIKE 'exam-dispatch-%'")
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM tasks WHERE id LIKE 'exam-dispatch-%'")
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM exam_templates WHERE id = ?")
        .bind(EXAM_TEMPLATE_ID)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM agents WHERE public_key = ?")
        .bind(E4_DISPATCH_AGENT)
        .execute(pool)
        .await;
}

async fn save_and_deactivate_all_templates(pool: &DbPool) -> Vec<String> {
    let rows = sqlx::query("SELECT id FROM exam_templates WHERE status='active'")
        .fetch_all(pool)
        .await
        .unwrap_or_default();
    let ids: Vec<String> = rows.iter().map(|r| r.try_get("id").unwrap()).collect();
    let _ = sqlx::query("UPDATE exam_templates SET status='inactive' WHERE status='active'")
        .execute(pool)
        .await;
    ids
}

async fn restore_templates(pool: &DbPool, ids: &[String]) {
    for id in ids {
        let _ = sqlx::query("UPDATE exam_templates SET status='active' WHERE id=?")
            .bind(id)
            .execute(pool)
            .await;
    }
}

async fn save_and_deactivate_other_agents(pool: &DbPool, keep: &str) -> Vec<String> {
    let rows =
        sqlx::query("SELECT public_key FROM agents WHERE status='active' AND public_key != ?")
            .bind(keep)
            .fetch_all(pool)
            .await
            .unwrap_or_default();
    let pks: Vec<String> = rows
        .iter()
        .map(|r| r.try_get("public_key").unwrap())
        .collect();
    let _ = sqlx::query(
        "UPDATE agents SET status='inactive' WHERE status='active' AND public_key != ?",
    )
    .bind(keep)
    .execute(pool)
    .await;
    pks
}

async fn restore_agents(pool: &DbPool, pks: &[String]) {
    for pk in pks {
        let _ = sqlx::query("UPDATE agents SET status='active' WHERE public_key=?")
            .bind(pk)
            .execute(pool)
            .await;
    }
}

async fn seed_e4_dispatch_fixtures(pool: &DbPool) {
    sqlx::query(
        "INSERT INTO agents (public_key, name, status, active_jobs)
         VALUES (?, 'E4 Dispatch Agent', 'active', 0)
         ON DUPLICATE KEY UPDATE status = 'active', active_jobs = 0",
    )
    .bind(E4_DISPATCH_AGENT)
    .execute(pool)
    .await
    .expect("seed dispatch agent");

    sqlx::query(
        "INSERT INTO reputations (id, agent_public_key, skill, score)
         VALUES ('e4-http-dispatch-rep', ?, 'defi_analysis', 50)
         ON DUPLICATE KEY UPDATE score = VALUES(score)",
    )
    .bind(E4_DISPATCH_AGENT)
    .execute(pool)
    .await
    .expect("seed dispatch agent reputation");

    sqlx::query(
        "INSERT INTO exam_templates (id, prompt, expected_answer_canonical, domain, status)
         VALUES (?, 'Compute stake', ?, 'defi_analysis', 'active')
         ON DUPLICATE KEY UPDATE
           prompt = VALUES(prompt),
           expected_answer_canonical = VALUES(expected_answer_canonical),
           status = 'active'",
    )
    .bind(EXAM_TEMPLATE_ID)
    .bind(EXAM_CANONICAL)
    .execute(pool)
    .await
    .expect("seed exam template for dispatch");
}

async fn cleanup_fixtures(pool: &DbPool) {
    let _ = sqlx::query("DELETE FROM exam_assignments WHERE task_id = ?")
        .bind(TASK_ID)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM tasks WHERE id = ?")
        .bind(TASK_ID)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM exam_templates WHERE id = ?")
        .bind(EXAM_TEMPLATE_ID)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM agents WHERE public_key = ?")
        .bind(AGENT_PK)
        .execute(pool)
        .await;
}

async fn seed_exam_fixtures(pool: &DbPool) {
    sqlx::query(
        "INSERT INTO agents (public_key, name, status) VALUES (?, 'E2 HTTP Agent', 'active')
         ON DUPLICATE KEY UPDATE name = VALUES(name)",
    )
    .bind(AGENT_PK)
    .execute(pool)
    .await
    .expect("seed agent");

    sqlx::query(
        "INSERT INTO exam_templates (id, prompt, expected_answer_canonical, domain, status)
         VALUES (?, 'Compute stake', ?, 'defi_analysis', 'active')
         ON DUPLICATE KEY UPDATE
           prompt = VALUES(prompt),
           expected_answer_canonical = VALUES(expected_answer_canonical)",
    )
    .bind(EXAM_TEMPLATE_ID)
    .bind(EXAM_CANONICAL)
    .execute(pool)
    .await
    .expect("seed exam template");

    sqlx::query(
        "INSERT INTO tasks (
            id, creator_public_key, assigned_agent_public_key, budget_motes, status,
            transaction_hash, domain, prompt, deadline
         ) VALUES (?, ?, ?, 5000000000, 'InProgress', 'tx-e2-http', 'defi_analysis', 'Compute stake', 0)
         ON DUPLICATE KEY UPDATE
           assigned_agent_public_key = VALUES(assigned_agent_public_key),
           status = VALUES(status),
           result = NULL,
           result_hash = NULL,
           validator_audit = NULL",
    )
    .bind(TASK_ID)
    .bind(CREATOR_PK)
    .bind(AGENT_PK)
    .execute(pool)
    .await
    .expect("seed task");

    sqlx::query(
        "INSERT INTO exam_assignments (task_id, template_id, agent_public_key, bucket, status)
         VALUES (?, ?, ?, 'manual', 'assigned')
         ON DUPLICATE KEY UPDATE template_id = VALUES(template_id)",
    )
    .bind(TASK_ID)
    .bind(EXAM_TEMPLATE_ID)
    .bind(AGENT_PK)
    .execute(pool)
    .await
    .expect("seed exam assignment");
}

async fn poll_validator_audit(pool: &DbPool, task_id: &str) -> Option<serde_json::Value> {
    for _ in 0..30 {
        tokio::time::sleep(Duration::from_millis(200)).await;
        let row = sqlx::query("SELECT validator_audit FROM tasks WHERE id = ?")
            .bind(task_id)
            .fetch_optional(pool)
            .await
            .ok()??;
        let audit: Option<serde_json::Value> = row.try_get("validator_audit").ok()?;
        if audit.is_some() {
            return audit;
        }
    }
    None
}

async fn post_raw_result(
    router: &mut Router,
    task_id: &str,
    agent_pubkey: &str,
    body: &str,
) -> StatusCode {
    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/tasks/{task_id}/raw_result"))
        .header(header::CONTENT_TYPE, "application/json")
        .header("X-Agent-Pubkey", agent_pubkey)
        .body(Body::from(body.to_string()))
        .expect("raw_result request");

    let response = router.oneshot(request).await.expect("raw_result response");
    response.status()
}

async fn post_validate(router: &mut Router, task_id: &str) -> StatusCode {
    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/tasks/{task_id}/validate"))
        .header(header::AUTHORIZATION, INTERNAL_SERVICE_KEY)
        .body(Body::empty())
        .expect("validate request");

    let response = router.oneshot(request).await.expect("validate response");
    response.status()
}

async fn post_dispatch(router: &mut Router) -> (StatusCode, serde_json::Value) {
    let request = Request::builder()
        .method("POST")
        .uri("/api/admin/exams/dispatch")
        .header(header::AUTHORIZATION, INTERNAL_SERVICE_KEY)
        .body(Body::empty())
        .expect("dispatch request");

    let response = router.oneshot(request).await.expect("dispatch response");
    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("dispatch body");
    let json: serde_json::Value = serde_json::from_slice(&body).expect("dispatch response json");
    (status, json)
}

fn assert_exam_audit_shape(audit: &serde_json::Value) {
    for key in [
        "exam_id",
        "assignment_hash",
        "expected_answer_hash",
        "actual_answer_hash",
        "hash_algorithm",
        "verdict",
        "pipeline",
        "timestamp",
    ] {
        assert!(audit.get(key).is_some(), "missing exam audit field: {key}");
    }
    assert_eq!(audit["pipeline"], "exam");
    assert_eq!(audit["hash_algorithm"], "sha256");
}

#[tokio::test]
#[ignore = "requires MySQL: DATABASE_URL from backend/.env; cargo test --test e2_autonomous_http -- --ignored --test-threads=1"]
async fn http_autonomous_exam_pass() {
    let pool = connect_test_pool().await;
    cleanup_fixtures(&pool).await;
    seed_exam_fixtures(&pool).await;

    temp_env::async_with_vars(
        [
            ("VALIDATOR_MOCK_LLM", Some("1")),
            ("EXAM_SKIP_ONCHAIN", Some("1")),
        ],
        async {
            let mut router = build_test_router(pool.clone());

            let raw_status = post_raw_result(
                &mut router,
                TASK_ID,
                AGENT_PK,
                r#"{"result":"ANSWER: 2845678901.25 cspr"}"#,
            )
            .await;
            assert_eq!(raw_status, StatusCode::OK);

            let validate_status = post_validate(&mut router, TASK_ID).await;
            assert_eq!(validate_status, StatusCode::ACCEPTED);

            let audit = poll_validator_audit(&pool, TASK_ID)
                .await
                .expect("validator_audit should be persisted after validate");
            assert_exam_audit_shape(&audit);
            assert_eq!(audit["verdict"], "passed");
            assert_eq!(audit["exam_id"], EXAM_TEMPLATE_ID);

            let row = sqlx::query("SELECT result, result_hash FROM tasks WHERE id = ?")
                .bind(TASK_ID)
                .fetch_one(&pool)
                .await
                .expect("fetch task row");
            let result: Option<String> = row.try_get("result").expect("result column");
            assert_eq!(result.as_deref(), Some("ANSWER: 2845678901.25 cspr"));
            let result_hash: Option<String> =
                row.try_get("result_hash").expect("result_hash column");
            assert!(result_hash.is_some());
        },
    )
    .await;

    cleanup_fixtures(&pool).await;
}

#[tokio::test]
#[ignore = "requires MySQL: DATABASE_URL from backend/.env; cargo test --test e2_autonomous_http -- --ignored --test-threads=1"]
async fn http_validate_returns_bad_request_without_raw_result() {
    let pool = connect_test_pool().await;
    cleanup_fixtures(&pool).await;
    seed_exam_fixtures(&pool).await;

    let mut router = build_test_router(pool.clone());
    let status = post_validate(&mut router, TASK_ID).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    cleanup_fixtures(&pool).await;
}

#[tokio::test]
#[ignore = "requires MySQL: DATABASE_URL from backend/.env; cargo test --test e2_autonomous_http -- --ignored --test-threads=1"]
async fn http_e4_dispatch_then_validate_exam_audit() {
    let pool = connect_test_pool().await;
    cleanup_e4_dispatch_fixtures(&pool).await;
    let saved_templates = save_and_deactivate_all_templates(&pool).await;
    let saved_agents = save_and_deactivate_other_agents(&pool, E4_DISPATCH_AGENT).await;
    seed_e4_dispatch_fixtures(&pool).await;

    temp_env::async_with_vars(
        [
            ("VALIDATOR_MOCK_LLM", Some("1")),
            ("EXAM_SKIP_ONCHAIN", Some("1")),
        ],
        async {
            let mut router = build_dispatch_test_router(pool.clone());

            let (dispatch_status, dispatch_json) = post_dispatch(&mut router).await;
            assert_eq!(dispatch_status, StatusCode::OK);
            assert_eq!(dispatch_json["created"], true);

            let task_id = dispatch_json["task_id"]
                .as_str()
                .expect("dispatch task_id")
                .to_string();
            assert!(task_id.starts_with("exam-dispatch-"));

            let raw_status = post_raw_result(
                &mut router,
                &task_id,
                E4_DISPATCH_AGENT,
                r#"{"result":"ANSWER: 2845678901.25 cspr"}"#,
            )
            .await;
            assert_eq!(raw_status, StatusCode::OK);

            let validate_status = post_validate(&mut router, &task_id).await;
            assert_eq!(validate_status, StatusCode::ACCEPTED);

            let audit = poll_validator_audit(&pool, &task_id)
                .await
                .expect("validator_audit after E4 dispatch validate");
            assert_exam_audit_shape(&audit);
            assert_eq!(audit["verdict"], "passed");
            assert_eq!(audit["exam_id"], EXAM_TEMPLATE_ID);

            let assignment =
                sqlx::query("SELECT status, verdict FROM exam_assignments WHERE task_id = ?")
                    .bind(&task_id)
                    .fetch_one(&pool)
                    .await
                    .expect("exam assignment row");
            let status: String = assignment.try_get("status").expect("status");
            let verdict: Option<String> = assignment.try_get("verdict").expect("verdict");
            assert_eq!(status, "validated");
            assert_eq!(verdict.as_deref(), Some("passed"));
        },
    )
    .await;

    cleanup_e4_dispatch_fixtures(&pool).await;
    restore_templates(&pool, &saved_templates).await;
    restore_agents(&pool, &saved_agents).await;
}
