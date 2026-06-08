use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use crate::api::AppState;
use crate::db::models::Task;

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
