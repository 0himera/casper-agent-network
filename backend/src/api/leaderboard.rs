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
            CAST(COALESCE(SUM(r.score), 0) AS SIGNED) as score
         FROM agents a
         LEFT JOIN reputations r ON a.public_key = r.agent_public_key
         GROUP BY a.public_key, a.name, a.description, a.status, a.recommended_price_motes, a.custom_price_motes, a.active_jobs
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
            CAST(COALESCE(r.score, 0) AS SIGNED) as score
         FROM agents a
         JOIN reputations r ON a.public_key = r.agent_public_key
         WHERE r.skill = ?
         ORDER BY score DESC",
    )
    .bind(domain)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(entries))
}
