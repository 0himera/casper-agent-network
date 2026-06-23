use crate::api::AppState;
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
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

pub async fn get_global_leaderboard(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let entries = sqlx::query_as::<_, LeaderboardEntry>(
        "SELECT 
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
         ORDER BY score DESC"
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(entries))
}

pub async fn get_domain_leaderboard(
    State(state): State<AppState>,
    Path(domain): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
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
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(entries))
}
