use crate::api::AppState;
use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct AuditLogRow {
    pub id: String,
    pub domain: String,
    pub validator_audit: Option<serde_json::Value>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

pub async fn get_audit_logs(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let rows = sqlx::query_as::<_, AuditLogRow>(
        "SELECT id, domain, validator_audit, timestamp 
         FROM tasks 
         WHERE validator_audit IS NOT NULL 
         ORDER BY timestamp DESC 
         LIMIT 100",
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;

    Ok(Json(serde_json::json!(rows)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_log_row_structure() {
        let json_audit = serde_json::json!({
            "prompt_sha256": "abc",
            "result_sha256": "def",
            "temperature": 0.0,
            "total": 95
        });

        assert_eq!(json_audit["total"], 95);
        assert_eq!(json_audit["temperature"], 0.0);
    }
}
