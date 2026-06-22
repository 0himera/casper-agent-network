use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::env as std_env;
use tokio::process::Command;

use crate::api::AppState;
use crate::config::Config;
use crate::db::DbPool;
use crate::db::models::{Agent, Task};
use crate::orchestrator::executor::execute_agent;
use crate::validator::evaluate_task;

#[derive(Deserialize)]
pub struct CreateOrUpdateTaskPayload {
    pub id: String,
    pub creator_public_key: String,
    pub budget_motes: u64,
    pub transaction_hash: String,
    pub domain: String,
    /// Legacy F3-era field; persisted for API/DB compatibility. Stage scoring uses `domain` only.
    pub skill_id: Option<String>,
    pub prompt: String,
    pub deadline: Option<u64>,
}

pub async fn get_tasks(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let tasks = sqlx::query_as::<_, Task>("SELECT * FROM tasks ORDER BY timestamp DESC")
        .fetch_all(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(tasks))
}

pub async fn get_task(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let task = sqlx::query_as::<_, Task>("SELECT * FROM tasks WHERE id = ?")
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
    let deadline_val = payload.deadline.unwrap_or(0);

    sqlx::query(
        "INSERT INTO tasks (id, creator_public_key, budget_motes, status, transaction_hash, domain, skill_id, prompt, deadline)
         VALUES (?, ?, ?, 'Open', ?, ?, ?, ?, ?)
         ON DUPLICATE KEY UPDATE domain = ?, skill_id = ?, prompt = ?, deadline = ?"
    )
    .bind(&payload.id)
    .bind(&payload.creator_public_key)
    .bind(payload.budget_motes)
    .bind(&payload.transaction_hash)
    .bind(&payload.domain)
    .bind(&payload.skill_id)
    .bind(&payload.prompt)
    .bind(deadline_val)
    .bind(&payload.domain)
    .bind(&payload.skill_id)
    .bind(&payload.prompt)
    .bind(deadline_val)
    .execute(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::OK)
}

pub async fn execute_task_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    if let Some(expected_key) = &state.config.internal_service_key {
        let auth_header = headers.get("Authorization").and_then(|h| h.to_str().ok());
        if auth_header != Some(expected_key.as_str()) {
            return Err((StatusCode::UNAUTHORIZED, "Unauthorized".to_string()));
        }
    }

    // 1. Fetch task details
    let task = sqlx::query_as::<_, Task>("SELECT * FROM tasks WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Task not found".to_string()))?;

    // 2. We only execute tasks that are InProgress and have an assigned agent
    if task.status != "InProgress" || task.assigned_agent_public_key.is_none() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Task is not in progress or has no assigned agent".to_string(),
        ));
    }

    let agent_pubkey = task.assigned_agent_public_key.clone().unwrap();

    // 3. Fetch agent details
    let agent = sqlx::query_as::<_, Agent>("SELECT * FROM agents WHERE public_key = ?")
        .bind(&agent_pubkey)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((
            StatusCode::NOT_FOUND,
            "Assigned agent not found".to_string(),
        ))?;

    // Skip autonomous agents — they execute locally and submit_result on-chain themselves
    if agent.endpoint_url.as_deref() == Some("autonomous") {
        tracing::info!("Agent is autonomous, skipping backend execution for task {}", id);
        return Ok(StatusCode::OK);
    }

    // Spawn background execution for hosted/external agents
    let pool = state.pool.clone();
    let config = state.config.clone();
    tokio::spawn(async move {
        tracing::info!("Background execution started for task {}", task.id);

        let exec_res = match execute_agent(
            &task.domain,
            &task.prompt,
            agent.endpoint_url.as_deref(),
            agent.api_key.as_deref(),
            agent.model.as_deref(),
            agent.system_prompt.as_deref(),
            &config,
        )
        .await
        {
            Ok(res) => res,
            Err(err) => {
                tracing::error!("Failed to execute agent for task {}: {}", task.id, err);
                return;
            }
        };

        validate_and_complete(
            &pool,
            &config,
            &task.id,
            &task.domain,
            &task.prompt,
            task.budget_motes,
            &exec_res.output,
            exec_res.processing_time_ms,
        )
        .await;
    });

    Ok(StatusCode::ACCEPTED)
}

#[derive(Deserialize)]
pub struct RawResultPayload {
    pub result: String,
}

pub async fn raw_result_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Json(payload): Json<RawResultPayload>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let agent_pubkey = headers
        .get("X-Agent-Pubkey")
        .and_then(|v| v.to_str().ok())
        .ok_or((StatusCode::BAD_REQUEST, "Missing X-Agent-Pubkey header".into()))?;

    let task = sqlx::query_as::<_, Task>("SELECT * FROM tasks WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Task not found".into()))?;

    if task.status != "InProgress" {
        return Err((StatusCode::BAD_REQUEST, "Task is not in progress".into()));
    }

    match &task.assigned_agent_public_key {
        Some(key) if key == agent_pubkey => {}
        _ => return Err((StatusCode::FORBIDDEN, "Agent pubkey does not match assigned agent".into())),
    }

    let result_hash = {
        let mut hasher = Sha256::new();
        hasher.update(payload.result.as_bytes());
        hex::encode(hasher.finalize())
    };

    let _ = sqlx::query("UPDATE tasks SET result = ?, result_hash = ? WHERE id = ?")
        .bind(&payload.result)
        .bind(&result_hash)
        .bind(&id)
        .execute(&state.pool)
        .await;

    tracing::info!("Raw result saved for task {} from agent {}", id, agent_pubkey);
    Ok(StatusCode::OK)
}

pub async fn validate_task_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    if let Some(expected_key) = &state.config.internal_service_key {
        let auth_header = headers.get("Authorization").and_then(|h| h.to_str().ok());
        if auth_header != Some(expected_key.as_str()) {
            return Err((StatusCode::UNAUTHORIZED, "Unauthorized".to_string()));
        }
    }

    let task = sqlx::query_as::<_, Task>("SELECT * FROM tasks WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Task not found".into()))?;

    let result = task
        .result
        .as_ref()
        .ok_or((StatusCode::BAD_REQUEST, "No raw result saved for task".into()))?;

    let pool = state.pool.clone();
    let config = state.config.clone();
    let result = result.clone();
    tokio::spawn(async move {
        validate_and_complete(
            &pool,
            &config,
            &task.id,
            &task.domain,
            &task.prompt,
            task.budget_motes,
            &result,
            0,
        )
        .await;
    });

    Ok(StatusCode::ACCEPTED)
}

async fn validate_and_complete(
    pool: &DbPool,
    config: &Config,
    task_id: &str,
    domain: &str,
    prompt: &str,
    budget_motes: u64,
    output: &str,
    processing_time_ms: u64,
) {
    let eval_res = match evaluate_task(domain, prompt, output, processing_time_ms, config).await {
        Ok(res) => res,
        Err(err) => {
            tracing::error!("Failed to evaluate task {}: {}", task_id, err);
            return;
        }
    };

    let score = eval_res.total;

    let base_price = match domain {
        "code_review" => 10_000_000_000f64,
        "rwa_valuation" => 15_000_000_000f64,
        "data_analysis" => 2_000_000_000f64,
        _ => 5_000_000_000f64,
    };
    let ratio = (budget_motes as f64) / base_price;
    let economic_weight = (ratio + 1.0).log2() + 1.0;

    let complexity_weight = match domain {
        "code_review" => 3.0,
        "rwa_valuation" => 2.5,
        "defi_analysis" => 2.0,
        "data_analysis" => 1.5,
        _ => 1.0,
    };

    let final_weight = economic_weight * 0.40
        + complexity_weight * 0.25
        + 1.0 * 0.15
        + 1.0 * 0.15
        + 1.0 * 0.05;

    let weight = (final_weight * 100.0).round() as u32;
    let weight = if weight == 0 { 1 } else { weight };

    let mut hasher = Sha256::new();
    hasher.update(output.as_bytes());
    let result_hash = hex::encode(hasher.finalize());

    let signature = format!("sig:platform_proxy:{}", result_hash);

    let _ = sqlx::query(
        "UPDATE tasks SET result_hash = ?, result_signature = ?, result = ?, validator_audit = ? WHERE id = ?",
    )
    .bind(&result_hash)
    .bind(&signature)
    .bind(output)
    .bind(&eval_res.validator_audit)
    .bind(task_id)
    .execute(pool)
    .await;

    tracing::info!(
        "Task {} validated. Score: {}, Weight: {}, Result Hash: {}, submitting to chain...",
        task_id, score, weight, result_hash
    );

    let bin_path =
        if std::path::Path::new("/usr/local/bin/agent_network_submit_complete").exists() {
            "/usr/local/bin/agent_network_submit_complete"
        } else {
            "cargo"
        };

    let mut cmd = Command::new(bin_path);
    if bin_path == "cargo" {
        cmd.args([
            "run", "--bin", "agent_network_submit_complete", "--features", "livenet", "--",
            task_id, &result_hash, domain, &score.to_string(), &weight.to_string(),
        ])
        .current_dir("../smart-contract");
    } else {
        cmd.args([task_id, &result_hash, domain, &score.to_string(), &weight.to_string()]);
    }

    if let Ok(hash) = std_env::var("CONTRACT_HASH") {
        cmd.env("CONTRACT_HASH", hash);
    } else if let Ok(package_hash) = std_env::var("CONTRACT_PACKAGE_HASH") {
        cmd.env("CONTRACT_HASH", format!("hash-{}", package_hash));
    }

    match cmd.status().await {
        Ok(status) => {
            if status.success() {
                tracing::info!("✅ Successfully completed task {} on-chain!", task_id);
                let _ = sqlx::query("UPDATE tasks SET status = 'Completed' WHERE id = ?")
                    .bind(task_id)
                    .execute(pool)
                    .await;
            } else {
                tracing::error!("❌ On-chain transaction failed for task {}: {:?}", task_id, status);
            }
        }
        Err(e) => {
            tracing::error!("❌ Failed to execute on-chain CLI tool: {}", e);
        }
    }
}
