use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::env as std_env;
use std::process::Command;

use crate::api::AppState;
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
) -> Result<impl IntoResponse, (StatusCode, String)> {
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

    // 4. Spawn background execution task
    tokio::spawn(async move {
        println!("Background execution started for task {}", task.id);

        // Execute the agent task
        let exec_res = match execute_agent(
            &task.domain,
            &task.prompt,
            agent.endpoint_url.as_deref(),
            agent.api_key.as_deref(),
            agent.model.as_deref(),
            agent.system_prompt.as_deref(),
            &state.config,
        )
        .await
        {
            Ok(res) => res,
            Err(err) => {
                eprintln!("Failed to execute agent for task {}: {}", task.id, err);
                return;
            }
        };

        // Evaluate results via LLM-as-Judge
        let eval_res = match evaluate_task(
            &task.domain,
            &task.prompt,
            &exec_res.output,
            exec_res.processing_time_ms,
            &state.config,
        )
        .await
        {
            Ok(res) => res,
            Err(err) => {
                eprintln!("Failed to evaluate task {}: {}", task.id, err);
                return;
            }
        };

        let score = eval_res.total;

        // Calculate weight based on reputation.md multi-dimensional weight formula
        let base_price = match task.domain.as_str() {
            "code_review" => 10_000_000_000f64,
            "rwa_valuation" => 15_000_000_000f64,
            "data_analysis" => 2_000_000_000f64,
            _ => 5_000_000_000f64, // defi_analysis
        };
        let ratio = (task.budget_motes as f64) / base_price;
        let economic_weight = (ratio + 1.0).log2() + 1.0;

        let complexity_weight = match task.domain.as_str() {
            "code_review" => 3.0,
            "rwa_valuation" => 2.5,
            "defi_analysis" => 2.0,
            "data_analysis" => 1.5,
            _ => 1.0,
        };

        let competition_weight = 1.0;
        let client_rep_weight = 1.0;
        let recency_weight = 1.0;

        let final_weight = economic_weight * 0.40
            + complexity_weight * 0.25
            + competition_weight * 0.15
            + client_rep_weight * 0.15
            + recency_weight * 0.05;

        // Scale by 100 for integer precision on-chain
        let weight = (final_weight * 100.0).round() as u32;
        let weight = if weight == 0 { 1 } else { weight };

        // Generate SHA-256 result hash
        let mut hasher = Sha256::new();
        hasher.update(exec_res.output.as_bytes());
        let result_hash = hex::encode(hasher.finalize());

        // Platform Proxy signature
        let signature = format!("sig:platform_proxy:{}", result_hash);

        // Update database with output, hash and signature
        let _ = sqlx::query(
            "UPDATE tasks SET result_hash = ?, result_signature = ?, result = ?, validator_audit = ? WHERE id = ?",
        )
        .bind(&result_hash)
        .bind(&signature)
        .bind(&exec_res.output)
        .bind(&eval_res.validator_audit)
        .bind(&task.id)
        .execute(&state.pool)
        .await;

        println!(
            "Task {} executed. Score: {}, Weight: {}, Result Hash: {}, submitting to chain...",
            task.id, score, weight, result_hash
        );

        // Call the on-chain CLI tool
        let bin_path =
            if std::path::Path::new("/usr/local/bin/agent_network_submit_complete").exists() {
                "/usr/local/bin/agent_network_submit_complete"
            } else {
                "cargo"
            };

        let mut cmd = Command::new(bin_path);
        if bin_path == "cargo" {
            cmd.args(&[
                "run",
                "--bin",
                "agent_network_submit_complete",
                "--features",
                "livenet",
                "--",
                &task.id,
                &result_hash,
                &task.domain,
                &score.to_string(),
                &weight.to_string(),
            ])
            .current_dir("../smart-contract");
        } else {
            cmd.args(&[
                &task.id,
                &result_hash,
                &task.domain,
                &score.to_string(),
                &weight.to_string(),
            ]);
        }

        // Pass CONTRACT_HASH env if configured
        if let Ok(hash) = std_env::var("CONTRACT_HASH") {
            cmd.env("CONTRACT_HASH", hash);
        } else if let Ok(package_hash) = std_env::var("CONTRACT_PACKAGE_HASH") {
            // Set hash address for livenet env
            cmd.env("CONTRACT_HASH", format!("hash-{}", package_hash));
        }

        match cmd.status() {
            Ok(status) => {
                if status.success() {
                    println!("✅ Successfully completed task {} on-chain!", task.id);
                } else {
                    eprintln!(
                        "❌ On-chain transaction failed for task {} with status: {:?}",
                        task.id, status
                    );
                }
            }
            Err(e) => {
                eprintln!("❌ Failed to execute on-chain CLI tool: {}", e);
            }
        }
    });

    Ok(StatusCode::ACCEPTED)
}
