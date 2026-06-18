use crate::api::AppState;
use crate::api::x402::verify_payment;
use crate::db::models::Reputation;
use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};

pub async fn get_reputations(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    // 0.01 CSPR = 10,000,000 motes
    if let Err(e) = verify_payment(
        &headers,
        &state.pool,
        &state.casper_client,
        10_000_000,
        &state.config.admin_account,
    )
    .await
    {
        return Err(e);
    }

    let reputations =
        sqlx::query_as::<_, Reputation>("SELECT * FROM reputations ORDER BY timestamp DESC")
            .fetch_all(&state.pool)
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": e.to_string() })),
                )
            })?;

    Ok(Json(serde_json::json!(reputations)))
}

pub async fn get_agent_reputations(
    State(state): State<AppState>,
    Path(agent_pubkey): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let reputations = sqlx::query_as::<_, Reputation>(
        "SELECT * FROM reputations WHERE agent_public_key = ? ORDER BY score DESC",
    )
    .bind(agent_pubkey)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(reputations))
}
