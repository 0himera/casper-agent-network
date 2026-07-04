//! Phase 5.1 compressed smoke: accelerated E7 background dispatch loop.

use std::time::Duration;

use backend::{
    config::{Config, ExamSelectionMode, ValidatorPipeline},
    db::{DbPool, init_db},
    exam_dispatch_loop::{shutdown, spawn_if_enabled},
};
use sqlx::Row;
use tracing_test::traced_test;

const E7_AGENT: &str = "e7-loop-smoke-agent";
const E7_TEMPLATE: &str = "exam-casper-total-stake-block-5000000";
const E7_CANONICAL: &str = "2845678901.25 cspr";
const E7_CREATOR: &str = "e7-loop-smoke-creator";

fn database_url() -> String {
    std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "mysql://deagentnet:passw0rd@localhost:3307/deagentnet".to_string())
}

async fn connect_test_pool() -> DbPool {
    let url = database_url();
    init_db(&url).await.unwrap_or_else(|err| {
        panic!(
            "E7 loop smoke tests require MySQL at DATABASE_URL ({url}): {err}. \
             Run: DATABASE_URL=... cargo test --test e7_dispatch_loop_smoke -- --ignored --test-threads=1"
        )
    })
}

fn loop_smoke_config(enabled: bool) -> Config {
    Config {
        database_url: database_url(),
        port: 3000,
        openai_api_key: None,
        claude_api_key: None,
        ollama_url: None,
        ollama_model: None,
        cloudflare_account_id: None,
        cloudflare_api_token: None,
        fireworks_api_key: None,
        fireworks_model: None,
        validator_url: None,
        validator_api_key: None,
        validator_model: None,
        validator_provider: None,
        validator_pipeline: ValidatorPipeline::Legacy,
        admin_account: E7_CREATOR.to_string(),
        internal_service_key: None,
        exam_weight: 300,
        exam_dispatch_prob_audit: 0.99,
        exam_dispatch_prob_rehab: 0.99,
        exam_max_per_agent_per_period: 1,
        exam_dispatch_period_hours: 24,
        exam_rehab_score_threshold: 0,
        exam_audit_active_jobs_threshold: 2,
        exam_dispatch_budget_motes: 5_000_000_000,
        exam_dispatch_creator_public_key: E7_CREATOR.to_string(),
        exam_llm_equality: false,
        exam_dispatch_loop_enabled: enabled,
        exam_dispatch_loop_interval_secs: 1,
        exam_selection_mode: ExamSelectionMode::Bucket,
        exam_urgency_base_prob: 0.1,
        exam_urgency_task_weight: 0.05,
        exam_urgency_variance_weight: 0.2,
        exam_urgency_recent_verdicts: 5,
        exam_smoothed_ema_alpha: 0.3,
        exam_leaderboard_use_smoothed: false,
    }
}

async fn cleanup_fixtures(pool: &DbPool) {
    let _ = sqlx::query("DELETE FROM exam_assignments WHERE task_id LIKE 'exam-dispatch-%'")
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM tasks WHERE id LIKE 'exam-dispatch-%'")
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM exam_templates WHERE id = ?")
        .bind(E7_TEMPLATE)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM agents WHERE public_key = ?")
        .bind(E7_AGENT)
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

async fn seed_loop_fixtures(pool: &DbPool) {
    sqlx::query(
        "INSERT INTO agents (public_key, name, status, active_jobs)
         VALUES (?, 'E7 Loop Agent', 'active', 0)
         ON DUPLICATE KEY UPDATE status = 'active', active_jobs = 0",
    )
    .bind(E7_AGENT)
    .execute(pool)
    .await
    .expect("seed loop agent");

    sqlx::query(
        "INSERT INTO reputations (id, agent_public_key, skill, score)
         VALUES ('e7-loop-smoke-rep', ?, 'defi_analysis', 50)
         ON DUPLICATE KEY UPDATE score = VALUES(score)",
    )
    .bind(E7_AGENT)
    .execute(pool)
    .await
    .expect("seed loop agent reputation");

    sqlx::query(
        "INSERT INTO exam_templates (id, prompt, expected_answer_canonical, domain, status)
         VALUES (?, 'Compute stake', ?, 'defi_analysis', 'active')
         ON DUPLICATE KEY UPDATE
           prompt = VALUES(prompt),
           expected_answer_canonical = VALUES(expected_answer_canonical),
           status = 'active'",
    )
    .bind(E7_TEMPLATE)
    .bind(E7_CANONICAL)
    .execute(pool)
    .await
    .expect("seed exam template for loop smoke");
}

async fn count_exam_dispatch_tasks(pool: &DbPool) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM tasks WHERE id LIKE 'exam-dispatch-%'")
        .fetch_one(pool)
        .await
        .expect("count exam dispatch tasks")
}

#[tokio::test]
#[ignore = "requires MySQL at DATABASE_URL"]
async fn loop_smoke_spawn_disabled_returns_none() {
    let pool = connect_test_pool().await;
    let config = loop_smoke_config(false);
    assert!(spawn_if_enabled(pool, config).is_none());
}

#[tokio::test]
#[traced_test]
#[ignore = "requires MySQL at DATABASE_URL"]
async fn loop_smoke_creates_dispatch_logs_and_shuts_down() {
    let pool = connect_test_pool().await;
    cleanup_fixtures(&pool).await;

    let saved_templates = save_and_deactivate_all_templates(&pool).await;
    let saved_agents = save_and_deactivate_other_agents(&pool, E7_AGENT).await;
    seed_loop_fixtures(&pool).await;

    let config = loop_smoke_config(true);
    let Some((stop_tx, handle)) = spawn_if_enabled(pool.clone(), config) else {
        panic!("expected loop to spawn when enabled");
    };

    tokio::time::sleep(Duration::from_millis(3500)).await;

    let task_count = count_exam_dispatch_tasks(&pool).await;
    assert!(
        task_count >= 1,
        "expected at least one exam-dispatch task, got {task_count}"
    );

    let assignment_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM exam_assignments WHERE task_id LIKE 'exam-dispatch-%'",
    )
    .fetch_one(&pool)
    .await
    .expect("count exam assignments");
    assert!(
        assignment_count >= 1,
        "expected at least one exam assignment, got {assignment_count}"
    );

    logs_contain("exam dispatch loop iteration");
    logs_contain("outcome=created");

    shutdown(stop_tx, handle, Duration::from_secs(2)).await;
    assert!(
        task_count <= 2,
        "frequency cap should prevent runaway dispatch; got {task_count} tasks"
    );

    cleanup_fixtures(&pool).await;
    restore_templates(&pool, &saved_templates).await;
    restore_agents(&pool, &saved_agents).await;
}

#[tokio::test]
#[traced_test]
#[ignore = "requires MySQL at DATABASE_URL"]
async fn loop_smoke_no_runaway_dispatch_under_frequency_cap() {
    let pool = connect_test_pool().await;
    cleanup_fixtures(&pool).await;

    let saved_templates = save_and_deactivate_all_templates(&pool).await;
    let saved_agents = save_and_deactivate_other_agents(&pool, E7_AGENT).await;
    seed_loop_fixtures(&pool).await;

    let config = loop_smoke_config(true);
    let Some((stop_tx, handle)) = spawn_if_enabled(pool.clone(), config) else {
        panic!("expected loop to spawn when enabled");
    };

    tokio::time::sleep(Duration::from_millis(3500)).await;

    let task_count = count_exam_dispatch_tasks(&pool).await;
    assert_eq!(
        task_count, 1,
        "frequency cap should allow at most one dispatch task across multiple ticks"
    );

    logs_contain("exam dispatch loop iteration");

    shutdown(stop_tx, handle, Duration::from_secs(2)).await;

    cleanup_fixtures(&pool).await;
    restore_templates(&pool, &saved_templates).await;
    restore_agents(&pool, &saved_agents).await;
}
