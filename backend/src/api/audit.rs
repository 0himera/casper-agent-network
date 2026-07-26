use crate::api::AppState;
use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
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
    use crate::api::AppState;
    use crate::api::ValidateInflight;
    use crate::casper::contract::CasperClient;
    use crate::config::Config;
    use sqlx::MySqlPool;

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

    async fn connect_test_pool() -> Option<MySqlPool> {
        let url = std::env::var("DATABASE_URL").ok()?;
        if url.is_empty() {
            return None;
        }
        MySqlPool::connect(&url).await.ok()
    }

    async fn cleanup_task(pool: &MySqlPool, task_id: &str) {
        let _ = sqlx::query("DELETE FROM tasks WHERE id = ?")
            .bind(task_id)
            .execute(pool)
            .await;
    }

    #[tokio::test]
    #[ignore]
    async fn test_get_audit_logs_db_integration() {
        let pool = match connect_test_pool().await {
            Some(p) => p,
            None => {
                println!(
                    "Skipping test_get_audit_logs_db_integration: DATABASE_URL not set or unreachable"
                );
                return;
            }
        };

        let task_id_audit = "test-audit-has-log";
        let task_id_no_audit = "test-audit-no-log";

        cleanup_task(&pool, task_id_audit).await;
        cleanup_task(&pool, task_id_no_audit).await;

        // Seed agents first to satisfy foreign key (since we used 'test-agent-pk')
        let _ = sqlx::query(
            "INSERT INTO agents (public_key, name, status, active_jobs)
             VALUES ('test-agent-pk', 'Test Agent', 'active', 0)
             ON DUPLICATE KEY UPDATE status = 'active'",
        )
        .execute(&pool)
        .await;

        // 1. Seed a task WITH validator_audit
        let audit_data = serde_json::json!({
            "prompt_sha256": "sha1",
            "result_sha256": "sha2",
            "temperature": 0.5,
            "total": 85
        });

        sqlx::query(
            "INSERT INTO tasks (
                id, creator_public_key, assigned_agent_public_key, budget_motes, status,
                transaction_hash, domain, prompt, deadline, validator_audit, timestamp
            ) VALUES (?, 'test-creator-pk', 'test-agent-pk', 100, 'Completed',
                      'test-tx-hash-1', 'defi_analysis', 'test prompt', 123456, ?, NOW())",
        )
        .bind(task_id_audit)
        .bind(&audit_data)
        .execute(&pool)
        .await
        .expect("seed task 1");

        // 2. Seed a task WITHOUT validator_audit
        sqlx::query(
            "INSERT INTO tasks (
                id, creator_public_key, assigned_agent_public_key, budget_motes, status,
                transaction_hash, domain, prompt, deadline, validator_audit, timestamp
            ) VALUES (?, 'test-creator-pk', 'test-agent-pk', 100, 'Completed',
                      'test-tx-hash-2', 'defi_analysis', 'test prompt', 123456, NULL, NOW())",
        )
        .bind(task_id_no_audit)
        .execute(&pool)
        .await
        .expect("seed task 2");

        // Construct AppState
        let state = AppState {
            pool: pool.clone(),
            config: Config::from_env(),
            casper_client: CasperClient::new("".to_string(), "".to_string(), "".to_string()),
            validate_inflight: ValidateInflight::default(),
        };

        // Call handler
        let response = get_audit_logs(State(state))
            .await
            .expect("get_audit_logs handler failed")
            .into_response();

        assert_eq!(response.status(), StatusCode::OK);

        // Parse response body
        let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
            .await
            .expect("read body");
        let logs: Vec<AuditLogRow> = serde_json::from_slice(&body_bytes).expect("deserialize logs");

        // Assertions:
        // - Should contain the seeded task with audit
        // - Should NOT contain the seeded task without audit
        let found_audit = logs.iter().find(|l| l.id == task_id_audit);
        let found_no_audit = logs.iter().find(|l| l.id == task_id_no_audit);

        assert!(
            found_audit.is_some(),
            "Seeded task with audit must be present in logs"
        );
        assert_eq!(found_audit.unwrap().domain, "defi_analysis");
        assert_eq!(
            found_audit.unwrap().validator_audit.as_ref().unwrap()["total"],
            85
        );

        assert!(
            found_no_audit.is_none(),
            "Seeded task without audit must NOT be present in logs"
        );

        // Clean up
        cleanup_task(&pool, task_id_audit).await;
        cleanup_task(&pool, task_id_no_audit).await;
    }

    /// Wave 4 scenario 11: mix of NULL / valid JSON in validator_audit — endpoint stays stable.
    #[tokio::test]
    #[ignore]
    async fn test_w4_audit_logs_mixed_payloads() {
        let pool = match connect_test_pool().await {
            Some(p) => p,
            None => return,
        };

        let id_valid = "w4-audit-valid";
        let id_null = "w4-audit-null";
        cleanup_task(&pool, id_valid).await;
        cleanup_task(&pool, id_null).await;

        let _ = sqlx::query(
            "INSERT INTO agents (public_key, name, status, active_jobs)
             VALUES ('w4-audit-agent', 'A', 'active', 0)
             ON DUPLICATE KEY UPDATE status = 'active'",
        )
        .execute(&pool)
        .await;

        let audit = serde_json::json!({"total": 77, "pipeline": "stage"});
        sqlx::query(
            "INSERT INTO tasks (
                id, creator_public_key, assigned_agent_public_key, budget_motes, status,
                transaction_hash, domain, prompt, deadline, validator_audit, timestamp
            ) VALUES (?, 'c', 'w4-audit-agent', 100, 'Completed',
                      'tx1', 'defi_analysis', 'p', 1, ?, NOW())",
        )
        .bind(id_valid)
        .bind(&audit)
        .execute(&pool)
        .await
        .expect("seed valid");

        sqlx::query(
            "INSERT INTO tasks (
                id, creator_public_key, assigned_agent_public_key, budget_motes, status,
                transaction_hash, domain, prompt, deadline, validator_audit, timestamp
            ) VALUES (?, 'c', 'w4-audit-agent', 100, 'Completed',
                      'tx2', 'defi_analysis', 'p', 1, NULL, NOW())",
        )
        .bind(id_null)
        .execute(&pool)
        .await
        .expect("seed null");

        let state = AppState {
            pool: pool.clone(),
            config: Config::from_env(),
            casper_client: CasperClient::new("".into(), "".into(), "".into()),
            validate_inflight: ValidateInflight::default(),
        };

        let response = get_audit_logs(State(state))
            .await
            .expect("handler must not fail on mixed rows")
            .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let logs: Vec<AuditLogRow> = serde_json::from_slice(&body).expect("json array");
        assert!(logs.iter().any(|l| l.id == id_valid));
        assert!(logs.iter().all(|l| l.id != id_null));
        println!("[PASS] scenario 11: mixed validator_audit rows — stable 200, skips NULL");

        cleanup_task(&pool, id_valid).await;
        cleanup_task(&pool, id_null).await;
    }

    /// Wave 4 scenario 12: DB down → controlled 500, body must not contain DSN secrets.
    #[tokio::test]
    #[ignore]
    async fn test_w4_audit_logs_db_down_no_secret_leak() {
        // Lazy pool to closed port — query fails without needing docker stop.
        let pool = sqlx::mysql::MySqlPoolOptions::new()
            .acquire_timeout(std::time::Duration::from_secs(1))
            .connect_lazy("mysql://deagentnet:passw0rd@127.0.0.1:1/deagentnet")
            .expect("lazy");

        let state = AppState {
            pool,
            config: Config::from_env(),
            casper_client: CasperClient::new("".into(), "".into(), "".into()),
            validate_inflight: ValidateInflight::default(),
        };

        match get_audit_logs(State(state)).await {
            Ok(_) => panic!("must be Err when DB is down"),
            Err((status, Json(body))) => {
                assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
                let text = body.to_string();
                assert!(
                    !text.contains("passw0rd"),
                    "must not leak password: {}",
                    text
                );
                assert!(
                    !text.contains("mysql://deagentnet:passw0rd"),
                    "must not leak DSN: {}",
                    text
                );
                println!("[PASS] scenario 12: audit DB failure → 500 without DSN leak");
            }
        }
    }
}
