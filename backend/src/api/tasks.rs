use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use crate::api::AppState;
use crate::db::models::Task;

#[derive(Deserialize)]
pub struct CreateOrUpdateTaskPayload {
    pub id: String,
    pub creator_public_key: String,
    pub budget_motes: u64,
    pub transaction_hash: String,
    pub domain: String,
    pub prompt: String,
}

pub async fn get_tasks(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let tasks = sqlx::query_as::<_, Task>(
        "SELECT * FROM tasks ORDER BY timestamp DESC"
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(tasks))
}

pub async fn get_task(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let task = sqlx::query_as::<_, Task>(
        "SELECT * FROM tasks WHERE id = ?"
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    match task {
        Some(task) => Ok(Json(task)),
        None => Err((StatusCode::NOT_FOUND, "Task not found".to_string())),
    }
}

pub async fn create_or_update_task(
    State(state): State<AppState>,
    Json(payload): Json<CreateOrUpdateTaskPayload>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    sqlx::query(
        "INSERT INTO tasks (id, creator_public_key, budget_motes, status, transaction_hash, domain, prompt)
         VALUES (?, ?, ?, 'Open', ?, ?, ?)
         ON DUPLICATE KEY UPDATE domain = ?, prompt = ?"
    )
    .bind(&payload.id)
    .bind(&payload.creator_public_key)
    .bind(payload.budget_motes)
    .bind(&payload.transaction_hash)
    .bind(&payload.domain)
    .bind(&payload.prompt)
    .bind(&payload.domain)
    .bind(&payload.prompt)
    .execute(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::OK)
}
