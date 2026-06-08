use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use crate::api::AppState;
use crate::db::models::Reputation;

pub async fn get_reputations(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let reputations = sqlx::query_as::<_, Reputation>(
        "SELECT * FROM reputations ORDER BY timestamp DESC"
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(reputations))
}

pub async fn get_agent_reputations(
    State(state): State<AppState>,
    Path(agent_pubkey): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let reputations = sqlx::query_as::<_, Reputation>(
        "SELECT * FROM reputations WHERE agent_public_key = ? ORDER BY score DESC"
    )
    .bind(agent_pubkey)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(reputations))
}
