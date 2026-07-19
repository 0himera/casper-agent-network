use crate::api::AppState;
use crate::api::x402::verify_payment;
use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use serde::Serialize;

#[derive(Serialize, sqlx::FromRow)]
pub struct LeaderboardEntry {
    pub public_key: String,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
    pub recommended_price_motes: u64,
    pub custom_price_motes: u64,
    pub active_jobs: i32,
    pub skill: Option<String>,
    pub score: i64,
    pub total_earnings_motes: i64,
    pub completed_tasks: i64,
}

/// Resolves display score for global leaderboard (Phase 5 read model).
pub fn resolve_global_leaderboard_score(
    chain_sum: f64,
    smoothed: Option<f64>,
    use_smoothed: bool,
) -> i64 {
    let raw = if use_smoothed {
        smoothed.unwrap_or(chain_sum)
    } else {
        chain_sum
    };
    raw as i64
}

const GLOBAL_LEADERBOARD_CHAIN_SQL: &str = "SELECT 
            a.public_key, 
            a.name, 
            a.description, 
            a.status, 
            a.recommended_price_motes, 
            a.custom_price_motes, 
            a.active_jobs,
            NULL as skill,
            CAST(COALESCE(r.score_sum, 0) AS SIGNED) as score,
            CAST(COALESCE(t.total_earnings_motes, 0) AS SIGNED) as total_earnings_motes,
            CAST(COALESCE(t.completed_tasks, 0) AS SIGNED) as completed_tasks
         FROM agents a
         LEFT JOIN (
             SELECT agent_public_key, SUM(score) as score_sum 
             FROM reputations 
             GROUP BY agent_public_key
         ) r ON a.public_key = r.agent_public_key
         LEFT JOIN (
             SELECT assigned_agent_public_key, COUNT(id) as completed_tasks, SUM(budget_motes) as total_earnings_motes
             FROM tasks
             WHERE status = 'Completed'
             GROUP BY assigned_agent_public_key
         ) t ON t.assigned_agent_public_key = a.public_key
         ORDER BY score DESC";

const GLOBAL_LEADERBOARD_SMOOTHED_SQL: &str = "SELECT 
            a.public_key, 
            a.name, 
            a.description, 
            a.status, 
            a.recommended_price_motes, 
            a.custom_price_motes, 
            a.active_jobs,
            NULL as skill,
            CAST(COALESCE(aes.smoothed_score, r.score_sum, 0) AS SIGNED) as score,
            CAST(COALESCE(t.total_earnings_motes, 0) AS SIGNED) as total_earnings_motes,
            CAST(COALESCE(t.completed_tasks, 0) AS SIGNED) as completed_tasks
         FROM agents a
         LEFT JOIN agent_exam_state aes ON a.public_key = aes.agent_public_key
         LEFT JOIN (
             SELECT agent_public_key, SUM(score) as score_sum 
             FROM reputations 
             GROUP BY agent_public_key
         ) r ON a.public_key = r.agent_public_key
         LEFT JOIN (
             SELECT assigned_agent_public_key, COUNT(id) as completed_tasks, SUM(budget_motes) as total_earnings_motes
             FROM tasks
             WHERE status = 'Completed'
             GROUP BY assigned_agent_public_key
         ) t ON t.assigned_agent_public_key = a.public_key
         ORDER BY score DESC";

pub async fn get_global_leaderboard(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    // 0.01 CSPR = 10,000,000 motes
    verify_payment(
        &headers,
        &state.pool,
        &state.casper_client,
        10_000_000,
        &state.config.admin_account,
    )
    .await?;

    let sql = if state.config.exam_leaderboard_use_smoothed {
        GLOBAL_LEADERBOARD_SMOOTHED_SQL
    } else {
        GLOBAL_LEADERBOARD_CHAIN_SQL
    };

    let entries = sqlx::query_as::<_, LeaderboardEntry>(sql)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
        })?;

    Ok(Json(serde_json::json!(entries)))
}

pub async fn get_domain_leaderboard(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(domain): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    // 0.01 CSPR = 10,000,000 motes
    verify_payment(
        &headers,
        &state.pool,
        &state.casper_client,
        10_000_000,
        &state.config.admin_account,
    )
    .await?;

    let entries = sqlx::query_as::<_, LeaderboardEntry>(
        "SELECT 
            a.public_key, 
            a.name, 
            a.description, 
            a.status, 
            a.recommended_price_motes, 
            a.custom_price_motes, 
            a.active_jobs,
            r.skill as skill,
            CAST(COALESCE(r.score, 0) AS SIGNED) as score,
            CAST(COALESCE(t.total_earnings_motes, 0) AS SIGNED) as total_earnings_motes,
            CAST(COALESCE(t.completed_tasks, 0) AS SIGNED) as completed_tasks
         FROM agents a
         JOIN reputations r ON a.public_key = r.agent_public_key
         LEFT JOIN (
             SELECT assigned_agent_public_key, COUNT(id) as completed_tasks, SUM(budget_motes) as total_earnings_motes
             FROM tasks
             WHERE status = 'Completed' AND domain = ?
             GROUP BY assigned_agent_public_key
         ) t ON t.assigned_agent_public_key = a.public_key
         WHERE r.skill = ?
         ORDER BY score DESC",
    )
    .bind(domain.clone())
    .bind(domain)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e.to_string() }))))?;

    Ok(Json(serde_json::json!(entries)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_global_leaderboard_score_uses_smoothed_when_flag_on() {
        assert_eq!(resolve_global_leaderboard_score(10.0, Some(85.0), true), 85);
    }

    #[test]
    fn resolve_global_leaderboard_score_uses_chain_when_flag_off() {
        assert_eq!(
            resolve_global_leaderboard_score(10.0, Some(85.0), false),
            10
        );
    }

    #[test]
    fn resolve_global_leaderboard_score_falls_back_to_chain_when_smoothed_null() {
        assert_eq!(resolve_global_leaderboard_score(42.0, None, true), 42);
    }

    #[test]
    fn resolve_global_leaderboard_score_zero_chain_when_both_absent() {
        assert_eq!(resolve_global_leaderboard_score(0.0, None, true), 0);
    }
}

#[cfg(test)]
mod db_tests {
    use sqlx::mysql::MySqlPool;

    use super::*;
    use crate::db::init_db;

    const AGENT_CHAIN_ONLY: &str = "phase5-leaderboard-chain-only";
    const AGENT_SMOOTHED: &str = "phase5-leaderboard-smoothed";
    const DOMAIN: &str = "phase5-test-domain";

    async fn connect_test_pool() -> MySqlPool {
        let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "mysql://deagentnet:passw0rd@127.0.0.1:3307/deagentnet".to_string()
        });
        init_db(&url).await.unwrap_or_else(|err| {
            panic!(
                "Phase 5 leaderboard DB tests require MySQL at DATABASE_URL ({url}): {err}. \
                 Export DATABASE_URL and run: \
                 DATABASE_URL=... cargo test --lib db_global_leaderboard -- --ignored --test-threads=1"
            )
        })
    }

    async fn cleanup_fixtures(pool: &sqlx::MySqlPool) {
        for pk in [AGENT_CHAIN_ONLY, AGENT_SMOOTHED] {
            let _ = sqlx::query("DELETE FROM agent_exam_state WHERE agent_public_key = ?")
                .bind(pk)
                .execute(pool)
                .await;
            let _ = sqlx::query("DELETE FROM reputations WHERE agent_public_key = ?")
                .bind(pk)
                .execute(pool)
                .await;
            let _ = sqlx::query("DELETE FROM agents WHERE public_key = ?")
                .bind(pk)
                .execute(pool)
                .await;
        }
    }

    async fn seed_agents(pool: &sqlx::MySqlPool) {
        for (pk, name) in [
            (AGENT_CHAIN_ONLY, "Chain Only Agent"),
            (AGENT_SMOOTHED, "Smoothed Agent"),
        ] {
            sqlx::query(
                "INSERT INTO agents (public_key, name, endpoint_url, status) \
                 VALUES (?, ?, 'autonomous', 'active')",
            )
            .bind(pk)
            .bind(name)
            .execute(pool)
            .await
            .expect("insert agent");
        }

        sqlx::query(
            "INSERT INTO reputations (id, agent_public_key, skill, score) VALUES (?, ?, ?, ?)",
        )
        .bind(format!("rep-{AGENT_CHAIN_ONLY}"))
        .bind(AGENT_CHAIN_ONLY)
        .bind(DOMAIN)
        .bind(50_i32)
        .execute(pool)
        .await
        .expect("insert chain-only reputation");

        sqlx::query(
            "INSERT INTO reputations (id, agent_public_key, skill, score) VALUES (?, ?, ?, ?)",
        )
        .bind(format!("rep-{AGENT_SMOOTHED}"))
        .bind(AGENT_SMOOTHED)
        .bind(DOMAIN)
        .bind(10_i32)
        .execute(pool)
        .await
        .expect("insert smoothed reputation");

        sqlx::query(
            "INSERT INTO agent_exam_state (agent_public_key, smoothed_score) VALUES (?, ?)
             ON DUPLICATE KEY UPDATE smoothed_score = VALUES(smoothed_score)",
        )
        .bind(AGENT_SMOOTHED)
        .bind(90.0_f64)
        .execute(pool)
        .await
        .expect("insert smoothed score");
    }

    async fn fetch_global_scores(pool: &sqlx::MySqlPool, use_smoothed: bool) -> Vec<(String, i64)> {
        let sql = if use_smoothed {
            GLOBAL_LEADERBOARD_SMOOTHED_SQL
        } else {
            GLOBAL_LEADERBOARD_CHAIN_SQL
        };
        let entries = sqlx::query_as::<_, LeaderboardEntry>(sql)
            .fetch_all(pool)
            .await
            .expect("fetch global leaderboard");
        entries
            .into_iter()
            .filter(|e| e.public_key == AGENT_CHAIN_ONLY || e.public_key == AGENT_SMOOTHED)
            .map(|e| (e.public_key, e.score))
            .collect()
    }

    async fn fetch_domain_score(pool: &sqlx::MySqlPool, agent_pk: &str) -> i64 {
        let entry = sqlx::query_as::<_, LeaderboardEntry>(
            "SELECT 
                a.public_key, 
                a.name, 
                a.description, 
                a.status, 
                a.recommended_price_motes, 
                a.custom_price_motes, 
                a.active_jobs,
                r.skill as skill,
                CAST(COALESCE(r.score, 0) AS SIGNED) as score,
                CAST(COALESCE(t.total_earnings_motes, 0) AS SIGNED) as total_earnings_motes,
                CAST(COALESCE(t.completed_tasks, 0) AS SIGNED) as completed_tasks
             FROM agents a
             JOIN reputations r ON a.public_key = r.agent_public_key
             LEFT JOIN (
                 SELECT assigned_agent_public_key, COUNT(id) as completed_tasks, SUM(budget_motes) as total_earnings_motes
                 FROM tasks
                 WHERE status = 'Completed' AND domain = ?
                 GROUP BY assigned_agent_public_key
             ) t ON t.assigned_agent_public_key = a.public_key
             WHERE r.skill = ? AND a.public_key = ?
             ORDER BY score DESC",
        )
        .bind(DOMAIN)
        .bind(DOMAIN)
        .bind(agent_pk)
        .fetch_one(pool)
        .await
        .expect("fetch domain leaderboard entry");
        entry.score
    }

    #[tokio::test]
    #[ignore = "requires MySQL at DATABASE_URL"]
    async fn db_global_leaderboard_uses_smoothed_score_when_flag_on() {
        let pool = connect_test_pool().await;
        cleanup_fixtures(&pool).await;
        seed_agents(&pool).await;

        let scores = fetch_global_scores(&pool, true).await;
        let chain_only = scores
            .iter()
            .find(|(pk, _)| pk == AGENT_CHAIN_ONLY)
            .map(|(_, s)| *s)
            .expect("chain-only agent");
        let smoothed = scores
            .iter()
            .find(|(pk, _)| pk == AGENT_SMOOTHED)
            .map(|(_, s)| *s)
            .expect("smoothed agent");

        assert_eq!(chain_only, 50);
        assert_eq!(smoothed, 90);
        assert!(smoothed > chain_only);

        cleanup_fixtures(&pool).await;
    }

    #[tokio::test]
    #[ignore = "requires MySQL at DATABASE_URL"]
    async fn db_global_leaderboard_uses_chain_score_when_flag_off() {
        let pool = connect_test_pool().await;
        cleanup_fixtures(&pool).await;
        seed_agents(&pool).await;

        let scores = fetch_global_scores(&pool, false).await;
        let chain_only = scores
            .iter()
            .find(|(pk, _)| pk == AGENT_CHAIN_ONLY)
            .map(|(_, s)| *s)
            .expect("chain-only agent");
        let smoothed = scores
            .iter()
            .find(|(pk, _)| pk == AGENT_SMOOTHED)
            .map(|(_, s)| *s)
            .expect("smoothed agent");

        assert_eq!(chain_only, 50);
        assert_eq!(smoothed, 10);

        cleanup_fixtures(&pool).await;
    }

    #[tokio::test]
    #[ignore = "requires MySQL at DATABASE_URL"]
    async fn db_domain_leaderboard_ignores_smoothed_score() {
        let pool = connect_test_pool().await;
        cleanup_fixtures(&pool).await;
        seed_agents(&pool).await;

        let score = fetch_domain_score(&pool, AGENT_SMOOTHED).await;
        assert_eq!(score, 10);

        cleanup_fixtures(&pool).await;
    }
}
