//! Wave 4 G23/G24: reputation snapshot HTTP/DB surface.
//!
//! Run:
//! ```bash
//! DATABASE_URL='mysql://deagentnet:passw0rd@127.0.0.1:3307/deagentnet' \
//!   cargo test -p backend --test wave4_reputation_snapshot -- --ignored --test-threads=1 --nocapture
//! ```

use std::time::Duration;

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use backend::{
    api::{
        create_router,
        reputations::{ReputationSnapshot, verify_reputation_snapshot},
    },
    casper::contract::CasperClient,
    config::{Config, ValidatorPipeline},
    db::{DbPool, init_db},
};
use tower::ServiceExt;

const AGENT_EMPTY: &str = "w4-snap-empty";
const AGENT_MULTI: &str = "w4-snap-multi";
const AGENT_DIRTY: &str = "w4-snap-dirty";
const SIGNER: &str = "w4-snapshot-signer";

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
        internal_service_key: Some("w4-snap-internal".to_string()),
        cloudflare_account_id: None,
        cloudflare_api_token: None,
        fireworks_api_key: None,
        fireworks_model: None,
        validator_url: None,
        validator_api_key: None,
        validator_model: None,
        validator_provider: None,
        validator_pipeline: ValidatorPipeline::Stage,
        admin_account: SIGNER.to_string(),
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
    let _ = sqlx::query("DELETE FROM reputations WHERE agent_public_key IN (?, ?, ?)")
        .bind(AGENT_EMPTY)
        .bind(AGENT_MULTI)
        .bind(AGENT_DIRTY)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM agents WHERE public_key IN (?, ?, ?)")
        .bind(AGENT_EMPTY)
        .bind(AGENT_MULTI)
        .bind(AGENT_DIRTY)
        .execute(pool)
        .await;
}

async fn seed_agent(pool: &DbPool, pk: &str) {
    sqlx::query(
        "INSERT INTO agents (public_key, name, status, active_jobs)
         VALUES (?, 'W4 Snap Agent', 'active', 0)
         ON DUPLICATE KEY UPDATE status='active'",
    )
    .bind(pk)
    .execute(pool)
    .await
    .expect("seed agent");
}

async fn get_snapshot(app: Router, agent: &str) -> (StatusCode, ReputationSnapshot) {
    let res = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/reputations/snapshot/{agent}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("oneshot");
    let status = res.status();
    let body = axum::body::to_bytes(res.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let snapshot: ReputationSnapshot = serde_json::from_slice(&body)
        .unwrap_or_else(|e| panic!("snapshot json: {e} body={body:?}"));
    (status, snapshot)
}

/// G23: empty reputations → 200 + [] + verifiable signature.
#[tokio::test]
#[ignore]
async fn test_w4_reputation_snapshot_empty_agent_ok() {
    let pool = connect_test_pool().await;
    cleanup(&pool).await;
    seed_agent(&pool, AGENT_EMPTY).await;

    let app = build_test_router(pool.clone());
    let (status, snapshot) = get_snapshot(app, AGENT_EMPTY).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(snapshot.agent_public_key, AGENT_EMPTY);
    assert!(snapshot.skills_reputation.is_empty());
    assert_eq!(snapshot.signer, SIGNER);
    assert!(
        verify_reputation_snapshot(&snapshot),
        "empty snapshot must verify"
    );
    println!("[PASS] G23 empty-state: 200 + [] + valid signature");
    cleanup(&pool).await;
}

/// G23: multi-skill ordering is stable across back-to-back requests.
/// Note: signature may differ because timestamp_ms is wall-clock; skills must not drift.
#[tokio::test]
#[ignore]
async fn test_w4_reputation_snapshot_repeat_stable_skills() {
    let pool = connect_test_pool().await;
    cleanup(&pool).await;
    seed_agent(&pool, AGENT_MULTI).await;

    // Distinct scores + equal-score tie (skill ASC secondary) for determinism.
    for (id, skill, score) in [
        ("w4-snap-r1", "zeta_skill", 50),
        ("w4-snap-r2", "alpha_skill", 90),
        ("w4-snap-r3", "beta_skill", 70),
        ("w4-snap-r4", "gamma_skill", 70),
    ] {
        sqlx::query(
            "INSERT INTO reputations (id, agent_public_key, skill, score) VALUES (?, ?, ?, ?)",
        )
        .bind(id)
        .bind(AGENT_MULTI)
        .bind(skill)
        .bind(score)
        .execute(&pool)
        .await
        .expect("seed reputation");
    }

    let app_a = build_test_router(pool.clone());
    let (status_a, snap_a) = get_snapshot(app_a, AGENT_MULTI).await;
    let app_b = build_test_router(pool.clone());
    let (status_b, snap_b) = get_snapshot(app_b, AGENT_MULTI).await;

    assert_eq!(status_a, StatusCode::OK);
    assert_eq!(status_b, StatusCode::OK);
    assert!(verify_reputation_snapshot(&snap_a));
    assert!(verify_reputation_snapshot(&snap_b));

    let skills_a: Vec<(String, i32)> = snap_a
        .skills_reputation
        .iter()
        .map(|s| (s.skill.clone(), s.score))
        .collect();
    let skills_b: Vec<(String, i32)> = snap_b
        .skills_reputation
        .iter()
        .map(|s| (s.skill.clone(), s.score))
        .collect();

    assert_eq!(
        skills_a,
        vec![
            ("alpha_skill".to_string(), 90),
            ("beta_skill".to_string(), 70),
            ("gamma_skill".to_string(), 70),
            ("zeta_skill".to_string(), 50),
        ]
    );
    assert_eq!(skills_a, skills_b, "skills_reputation must not drift");
    assert_eq!(snap_a.signer, SIGNER);
    // timestamp/signature may differ — do not assert equality
    println!(
        "[PASS] G23 repeat: skills stable (signatures may differ: {} vs {})",
        snap_a.signature, snap_b.signature
    );
    cleanup(&pool).await;
}

/// G24: DB-down → controlled 500, no DSN/password leak.
#[tokio::test]
#[ignore]
async fn test_w4_reputation_snapshot_db_down_no_secret_leak() {
    let pool = sqlx::mysql::MySqlPoolOptions::new()
        .acquire_timeout(Duration::from_secs(1))
        .connect_lazy("mysql://deagentnet:passw0rd@127.0.0.1:1/deagentnet")
        .expect("lazy");

    let app = build_test_router(pool);
    let res = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/reputations/snapshot/{AGENT_EMPTY}"))
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
    assert!(
        !text.contains("mysql://deagentnet:"),
        "no full DSN in body: {text}"
    );
    println!("[PASS] G24 DB-down: 500 without DSN leak");
}

/// G24: duplicate skills + noisy skill names → 200, no crash.
/// Product semantics (keep-as-is): handler does NOT dedupe; both rows are returned
/// so snapshot signatures reflect raw DB state rather than silent merges.
#[tokio::test]
#[ignore]
async fn test_w4_reputation_snapshot_dirty_rows_no_500() {
    let pool = connect_test_pool().await;
    cleanup(&pool).await;
    seed_agent(&pool, AGENT_DIRTY).await;

    // Keep under reputations.skill VARCHAR(100).
    let noisy = "noisy_skill_!@#$%^&*()[]{};:,.<>/?`~_tail_xx";
    assert!(noisy.len() <= 100);
    sqlx::query("INSERT INTO reputations (id, agent_public_key, skill, score) VALUES (?, ?, ?, ?)")
        .bind("w4-snap-d1")
        .bind(AGENT_DIRTY)
        .bind("dup_skill")
        .bind(80)
        .execute(&pool)
        .await
        .expect("seed d1");
    sqlx::query("INSERT INTO reputations (id, agent_public_key, skill, score) VALUES (?, ?, ?, ?)")
        .bind("w4-snap-d2")
        .bind(AGENT_DIRTY)
        .bind("dup_skill")
        .bind(60)
        .execute(&pool)
        .await
        .expect("seed d2");
    sqlx::query("INSERT INTO reputations (id, agent_public_key, skill, score) VALUES (?, ?, ?, ?)")
        .bind("w4-snap-d3")
        .bind(AGENT_DIRTY)
        .bind(&noisy)
        .bind(40)
        .execute(&pool)
        .await
        .expect("seed d3");

    let app = build_test_router(pool.clone());
    let (status, snapshot) = get_snapshot(app, AGENT_DIRTY).await;

    assert_eq!(status, StatusCode::OK);
    assert!(verify_reputation_snapshot(&snapshot));
    assert_eq!(
        snapshot.skills_reputation.len(),
        3,
        "duplicates are not merged (current semantics)"
    );
    let dup_count = snapshot
        .skills_reputation
        .iter()
        .filter(|s| s.skill == "dup_skill")
        .count();
    assert_eq!(dup_count, 2, "both duplicate skill rows returned");
    assert!(
        snapshot.skills_reputation.iter().any(|s| s.skill == noisy),
        "noisy skill preserved"
    );
    println!("[PASS] G24 dirty rows: 200, duplicates kept, no crash");
    cleanup(&pool).await;
}
