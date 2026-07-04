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
use crate::db::exam::{
    get_agent_exam_state, get_exam_assignment_by_task_id, get_exam_template_by_id,
    on_exam_validated, on_ordinary_task_completed, update_exam_assignment_validation,
    upsert_agent_exam_state,
};
use crate::db::models::{
    Agent, AgentExamState, ExamAssignment, ExamTemplate, TASK_PUBLIC_COLUMNS, Task, TaskPublic,
};
use crate::orchestrator::executor::execute_agent;
use crate::validator::llm_judge::EvaluationResult;
use crate::validator::{evaluate_exam_task, evaluate_task};

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
    let query = format!("SELECT {TASK_PUBLIC_COLUMNS} FROM tasks ORDER BY timestamp DESC");
    let tasks = sqlx::query_as::<_, Task>(&query)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let public: Vec<TaskPublic> = tasks.into_iter().map(TaskPublic::from).collect();
    Ok(Json(public))
}

pub async fn get_task(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let query = format!("SELECT {TASK_PUBLIC_COLUMNS} FROM tasks WHERE id = ?");
    let task = sqlx::query_as::<_, Task>(&query)
        .bind(id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    match task {
        Some(task) => Ok(Json(TaskPublic::from(task))),
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
        tracing::info!(
            "Agent is autonomous, skipping backend execution for task {}",
            id
        );
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
        .ok_or((
            StatusCode::BAD_REQUEST,
            "Missing X-Agent-Pubkey header".into(),
        ))?;

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
        _ => {
            return Err((
                StatusCode::FORBIDDEN,
                "Agent pubkey does not match assigned agent".into(),
            ));
        }
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

    tracing::info!(
        "Raw result saved for task {} from agent {}",
        id,
        agent_pubkey
    );
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

    let result = task.result.as_ref().ok_or((
        StatusCode::BAD_REQUEST,
        "No raw result saved for task".into(),
    ))?;

    if is_validate_noop(&task) {
        return Ok(validate_http_response(
            StatusCode::OK,
            "noop",
            "Task already validated",
        ));
    }

    if !state.validate_inflight.try_start(&id) {
        return Ok(validate_http_response(
            StatusCode::ACCEPTED,
            "in_progress",
            "Validation already in progress",
        ));
    }

    let pool = state.pool.clone();
    let config = state.config.clone();
    let inflight = state.validate_inflight.clone();
    let result = result.clone();
    let task_id = task.id.clone();
    let domain = task.domain.clone();
    let prompt = task.prompt.clone();
    let budget_motes = task.budget_motes;

    tokio::spawn(async move {
        validate_and_complete(
            &pool,
            &config,
            &task_id,
            &domain,
            &prompt,
            budget_motes,
            &result,
            0,
        )
        .await;
        inflight.finish(&task_id);
    });

    Ok(validate_http_response(
        StatusCode::ACCEPTED,
        "accepted",
        "Validation started",
    ))
}

struct ExamEvalContext {
    #[allow(dead_code)]
    assignment: ExamAssignment,
    template: ExamTemplate,
}

enum ExamLoadOutcome {
    NotExam,
    Ready(Box<ExamEvalContext>),
    Error,
}

async fn load_exam_context(pool: &DbPool, task_id: &str) -> ExamLoadOutcome {
    let assignment = match get_exam_assignment_by_task_id(pool, task_id).await {
        Ok(Some(assignment)) => assignment,
        Ok(None) => return ExamLoadOutcome::NotExam,
        Err(err) => {
            tracing::error!(
                "Failed to load exam assignment for task {}: {}",
                task_id,
                err
            );
            return ExamLoadOutcome::Error;
        }
    };

    match get_exam_template_by_id(pool, &assignment.template_id).await {
        Ok(Some(template)) => ExamLoadOutcome::Ready(Box::new(ExamEvalContext {
            assignment,
            template,
        })),
        Ok(None) => {
            tracing::error!(
                "Exam assignment for task {} references missing template {}",
                task_id,
                assignment.template_id
            );
            ExamLoadOutcome::Error
        }
        Err(err) => {
            tracing::error!(
                "Failed to load exam template {} for task {}: {}",
                assignment.template_id,
                task_id,
                err
            );
            ExamLoadOutcome::Error
        }
    }
}

async fn evaluate_task_validation_with_context(
    exam_ctx: Option<Box<ExamEvalContext>>,
    _task_id: &str,
    domain: &str,
    prompt: &str,
    output: &str,
    processing_time_ms: u64,
    config: &Config,
) -> Result<EvaluationResult, Box<dyn std::error::Error + Send + Sync>> {
    if let Some(ctx) = exam_ctx {
        return evaluate_exam_task(
            &ctx.template.id,
            domain,
            prompt,
            output,
            &ctx.template.expected_answer_canonical,
            ctx.template.source_metadata.as_ref(),
            processing_time_ms,
            config,
        )
        .await;
    }

    evaluate_task(domain, prompt, output, processing_time_ms, config).await
}

#[cfg(test)]
async fn evaluate_task_validation(
    pool: &DbPool,
    config: &Config,
    task_id: &str,
    domain: &str,
    prompt: &str,
    output: &str,
    processing_time_ms: u64,
) -> Result<EvaluationResult, ()> {
    let exam_ctx = match load_exam_context(pool, task_id).await {
        ExamLoadOutcome::NotExam => None,
        ExamLoadOutcome::Ready(ctx) => Some(ctx),
        ExamLoadOutcome::Error => return Err(()),
    };

    evaluate_task_validation_with_context(
        exam_ctx,
        task_id,
        domain,
        prompt,
        output,
        processing_time_ms,
        config,
    )
    .await
    .map_err(|err| {
        tracing::error!("Failed to evaluate task {}: {}", task_id, err);
    })
}

fn compute_ordinary_task_weight(domain: &str, budget_motes: u64) -> u32 {
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

    let final_weight =
        economic_weight * 0.40 + complexity_weight * 0.25 + 1.0 * 0.15 + 1.0 * 0.15 + 1.0 * 0.05;

    let weight = (final_weight * 100.0).round() as u32;
    if weight == 0 { 1 } else { weight }
}

fn resolve_completion_weight(
    is_exam: bool,
    config: &Config,
    domain: &str,
    budget_motes: u64,
) -> u32 {
    if is_exam {
        config.exam_weight
    } else {
        compute_ordinary_task_weight(domain, budget_motes)
    }
}

/// CLI args passed to `agent_network_submit_complete` after `--` (or directly for installed binary).
fn submit_complete_cli_args(
    creator_address: &str,
    task_id: &str,
    result_hash: &str,
    domain: &str,
    score: u32,
    weight: u32,
) -> [String; 6] {
    [
        creator_address.to_string(),
        task_id.to_string(),
        result_hash.to_string(),
        domain.to_string(),
        score.to_string(),
        weight.to_string(),
    ]
}

fn exam_verdict_from_audit(audit: &Option<serde_json::Value>) -> Option<String> {
    audit
        .as_ref()
        .and_then(|value| value.get("verdict"))
        .and_then(|value| value.as_str())
        .map(str::to_string)
}

fn compute_result_hash(output: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(output.as_bytes());
    hex::encode(hasher.finalize())
}

fn needs_submit_retry(task: &Task) -> bool {
    task.validator_audit.is_some() && task.status != "Completed" && !should_skip_onchain_submit()
}

fn is_validate_noop(task: &Task) -> bool {
    if task.status == "Completed" {
        return true;
    }
    task.validator_audit.is_some() && !needs_submit_retry(task)
}

fn score_from_validator_audit(audit: &serde_json::Value) -> Option<u32> {
    match audit.get("pipeline").and_then(|value| value.as_str()) {
        Some("exam") => match audit.get("verdict").and_then(|value| value.as_str()) {
            Some("passed") => Some(100),
            Some("failed") | Some("refusal") | Some("gate_failed") => Some(0),
            _ => None,
        },
        Some("stage") => audit
            .get("output")
            .and_then(|output| output.get("total"))
            .and_then(|total| total.as_u64())
            .map(|total| total as u32),
        _ => audit
            .get("total")
            .and_then(|total| total.as_u64())
            .map(|total| total as u32),
    }
}

fn validate_http_response(
    status: StatusCode,
    body_status: &str,
    message: &str,
) -> impl IntoResponse {
    (
        status,
        Json(serde_json::json!({
            "status": body_status,
            "message": message,
        })),
    )
}

async fn fetch_task_row(pool: &DbPool, task_id: &str) -> Option<Task> {
    match sqlx::query_as::<_, Task>("SELECT * FROM tasks WHERE id = ?")
        .bind(task_id)
        .fetch_optional(pool)
        .await
    {
        Ok(task) => task,
        Err(err) => {
            tracing::error!("Failed to load task {}: {}", task_id, err);
            None
        }
    }
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
    let Some(task_row) = fetch_task_row(pool, task_id).await else {
        tracing::error!("Task {} not found during validate_and_complete", task_id);
        return;
    };

    let exam_ctx = match load_exam_context(pool, task_id).await {
        ExamLoadOutcome::NotExam => None,
        ExamLoadOutcome::Ready(ctx) => Some(ctx),
        ExamLoadOutcome::Error => return,
    };
    let is_exam = exam_ctx.is_some();

    let (score, weight, result_hash) = if let Some(existing_audit) = &task_row.validator_audit {
        if !needs_submit_retry(&task_row) {
            tracing::info!(
                "Task {} already validated; skipping duplicate validate_and_complete",
                task_id
            );
            return;
        }

        let score = match score_from_validator_audit(existing_audit) {
            Some(score) => score,
            None => {
                tracing::error!(
                    "Task {} has validator_audit but score could not be derived",
                    task_id
                );
                return;
            }
        };
        let weight = resolve_completion_weight(is_exam, config, domain, budget_motes);
        let result_hash = task_row
            .result_hash
            .clone()
            .unwrap_or_else(|| compute_result_hash(output));

        tracing::info!(
            "Task {} retrying submit path only (score={}, weight={})",
            task_id,
            score,
            weight
        );

        (score, weight, result_hash)
    } else {
        let eval_res = match evaluate_task_validation_with_context(
            exam_ctx,
            task_id,
            domain,
            prompt,
            output,
            processing_time_ms,
            config,
        )
        .await
        {
            Ok(res) => res,
            Err(err) => {
                tracing::error!("Failed to evaluate task {}: {}", task_id, err);
                return;
            }
        };

        let score = eval_res.total;
        let weight = resolve_completion_weight(is_exam, config, domain, budget_motes);
        let result_hash = compute_result_hash(output);
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

        if is_exam {
            if let Some(verdict) = exam_verdict_from_audit(&eval_res.validator_audit) {
                tracing::info!(
                    "exam_eval verdict={} score={} weight={} task_id={}",
                    verdict,
                    score,
                    weight,
                    task_id
                );
                if let Err(err) = update_exam_assignment_validation(pool, task_id, &verdict).await {
                    tracing::error!(
                        "Failed to update exam assignment for task {}: {}",
                        task_id,
                        err
                    );
                } else if let Some(agent_pk) = task_row.assigned_agent_public_key.clone() {
                    spawn_exam_urgency_recalc(pool.clone(), config.clone(), agent_pk);
                }
            } else {
                tracing::warn!(
                    "Exam task {} validated but verdict missing from validator_audit",
                    task_id
                );
            }
        }

        (score, weight, result_hash)
    };

    tracing::info!(
        "Task {} validated. Score: {}, Weight: {}, Result Hash: {}, submitting to chain...",
        task_id,
        score,
        weight,
        result_hash
    );

    if should_skip_onchain_submit() {
        tracing::info!(
            "Skipping on-chain submit for task {} (EXAM_SKIP_ONCHAIN=1)",
            task_id
        );
        return;
    }

    let bin_path = if std::path::Path::new("/usr/local/bin/agent_network_submit_complete").exists()
    {
        "/usr/local/bin/agent_network_submit_complete"
    } else {
        "cargo"
    };

    let mut cmd = Command::new(bin_path);
    let cli_args = submit_complete_cli_args(
        &task_row.creator_public_key,
        task_id,
        &result_hash,
        domain,
        score,
        weight,
    );
    if bin_path == "cargo" {
        cmd.args([
            "run",
            "--bin",
            "agent_network_submit_complete",
            "--features",
            "livenet",
            "--",
            &cli_args[0],
            &cli_args[1],
            &cli_args[2],
            &cli_args[3],
            &cli_args[4],
            &cli_args[5],
        ])
        .current_dir("../smart-contract");
    } else {
        cmd.args([
            &cli_args[0],
            &cli_args[1],
            &cli_args[2],
            &cli_args[3],
            &cli_args[4],
            &cli_args[5],
        ]);
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
                if !is_exam && let Some(agent_pk) = task_row.assigned_agent_public_key.clone() {
                    spawn_ordinary_task_urgency_recalc(pool.clone(), config.clone(), agent_pk);
                }
            } else {
                tracing::error!(
                    "❌ On-chain transaction failed for task {}: {:?}",
                    task_id,
                    status
                );
            }
        }
        Err(e) => {
            tracing::error!("❌ Failed to execute on-chain CLI tool: {}", e);
        }
    }
}

fn should_skip_onchain_submit() -> bool {
    std_env::var("EXAM_SKIP_ONCHAIN")
        .ok()
        .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
}

fn spawn_exam_urgency_recalc(pool: DbPool, config: Config, agent_public_key: String) {
    tokio::spawn(async move {
        if let Err(err) = on_exam_validated(&pool, &agent_public_key, &config).await {
            tracing::error!(
                "Failed to recalculate exam urgency after exam validation for {}: {}",
                agent_public_key,
                err
            );
        }
    });
}

fn spawn_ordinary_task_urgency_recalc(pool: DbPool, config: Config, agent_public_key: String) {
    tokio::spawn(async move {
        if let Err(err) = on_ordinary_task_completed(&pool, &agent_public_key, &config).await {
            tracing::error!(
                "Failed to recalculate exam urgency after ordinary task for {}: {}",
                agent_public_key,
                err
            );
        }
    });
}

#[cfg(test)]
mod validation_tests {
    use super::*;
    use crate::config::ValidatorPipeline;
    use crate::db::init_db;
    use chrono::Utc;

    const EXAM_TEMPLATE_ID: &str = "exam-casper-total-stake-block-5000000";
    const EXAM_CANONICAL: &str = "2845678901.25 cspr";
    const E2_AGENT_PK: &str = "e2-test-agent";
    const E2_CREATOR_PK: &str = "e2-test-creator";

    fn sample_config() -> Config {
        Config {
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "mysql://root:password@127.0.0.1:3306/deagentnet".to_string()),
            port: 3000,
            openai_api_key: None,
            claude_api_key: None,
            ollama_url: None,
            ollama_model: None,
            internal_service_key: None,
            cloudflare_account_id: None,
            cloudflare_api_token: None,
            fireworks_api_key: None,
            fireworks_model: None,
            validator_url: None,
            validator_api_key: None,
            validator_model: None,
            validator_provider: None,
            validator_pipeline: ValidatorPipeline::Stage,
            admin_account: String::new(),
            exam_weight: 300,
            exam_dispatch_prob_audit: 0.2,
            exam_dispatch_prob_rehab: 0.5,
            exam_max_per_agent_per_period: 1,
            exam_dispatch_period_hours: 24,
            exam_rehab_score_threshold: 0,
            exam_audit_active_jobs_threshold: 2,
            exam_dispatch_budget_motes: 5_000_000_000,
            exam_dispatch_creator_public_key: String::new(),
            exam_llm_equality: false,
            exam_dispatch_loop_enabled: false,
            exam_dispatch_loop_interval_secs: 300,
            exam_selection_mode: crate::config::ExamSelectionMode::Bucket,
            exam_urgency_base_prob: 0.1,
            exam_urgency_task_weight: 0.05,
            exam_urgency_variance_weight: 0.2,
            exam_urgency_recent_verdicts: 5,
            exam_smoothed_ema_alpha: 0.3,
            exam_leaderboard_use_smoothed: false,
        }
    }

    fn sample_exam_context() -> ExamEvalContext {
        ExamEvalContext {
            assignment: ExamAssignment {
                task_id: "task-exam-1".to_string(),
                template_id: EXAM_TEMPLATE_ID.to_string(),
                agent_public_key: E2_AGENT_PK.to_string(),
                bucket: "manual".to_string(),
                status: "assigned".to_string(),
                verdict: None,
                created_at: Utc::now(),
                validated_at: None,
            },
            template: ExamTemplate {
                id: EXAM_TEMPLATE_ID.to_string(),
                prompt: "Compute stake".to_string(),
                expected_answer_canonical: EXAM_CANONICAL.to_string(),
                domain: "defi_analysis".to_string(),
                status: "active".to_string(),
                source_metadata: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
        }
    }

    async fn connect_test_pool() -> DbPool {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "mysql://root:password@127.0.0.1:3306/deagentnet".to_string());
        init_db(&database_url).await.unwrap_or_else(|err| {
            panic!(
                "E2 DB tests require MySQL at DATABASE_URL ({database_url}): {err}. \
                 Export DATABASE_URL from backend/.env (see DOCKER_MYSQL_PORT_3307.md). \
                 Run: DATABASE_URL=... cargo test --lib db_ -- --ignored --test-threads=1"
            )
        })
    }

    async fn cleanup_e2_fixtures(pool: &DbPool, task_id: &str) {
        let _ = sqlx::query("DELETE FROM exam_assignments WHERE task_id = ?")
            .bind(task_id)
            .execute(pool)
            .await;
        let _ = sqlx::query("DELETE FROM tasks WHERE id = ?")
            .bind(task_id)
            .execute(pool)
            .await;
        let _ = sqlx::query("DELETE FROM exam_templates WHERE id = ?")
            .bind(EXAM_TEMPLATE_ID)
            .execute(pool)
            .await;
        let _ = sqlx::query("DELETE FROM agent_exam_state WHERE agent_public_key = ?")
            .bind(E2_AGENT_PK)
            .execute(pool)
            .await;
        let _ = sqlx::query("DELETE FROM agents WHERE public_key = ?")
            .bind(E2_AGENT_PK)
            .execute(pool)
            .await;
    }

    async fn seed_agent(pool: &DbPool) {
        sqlx::query(
            "INSERT INTO agents (public_key, name, status) VALUES (?, 'E2 Test Agent', 'active')
             ON DUPLICATE KEY UPDATE name = VALUES(name)",
        )
        .bind(E2_AGENT_PK)
        .execute(pool)
        .await
        .expect("seed agent");
    }

    async fn seed_exam_template(pool: &DbPool) {
        sqlx::query(
            "INSERT INTO exam_templates (id, prompt, expected_answer_canonical, domain, status)
             VALUES (?, 'Compute stake', ?, 'defi_analysis', 'active')
             ON DUPLICATE KEY UPDATE
               prompt = VALUES(prompt),
               expected_answer_canonical = VALUES(expected_answer_canonical),
               domain = VALUES(domain),
               status = VALUES(status)",
        )
        .bind(EXAM_TEMPLATE_ID)
        .bind(EXAM_CANONICAL)
        .execute(pool)
        .await
        .expect("seed exam template");
    }

    async fn seed_task(pool: &DbPool, task_id: &str) {
        sqlx::query(
            "INSERT INTO tasks (
                id, creator_public_key, assigned_agent_public_key, budget_motes, status,
                transaction_hash, domain, prompt, deadline
             ) VALUES (?, ?, ?, 5000000000, 'InProgress', 'tx-e2', 'defi_analysis', 'Compute stake', 0)
             ON DUPLICATE KEY UPDATE
               assigned_agent_public_key = VALUES(assigned_agent_public_key),
               status = VALUES(status),
               prompt = VALUES(prompt)",
        )
        .bind(task_id)
        .bind(E2_CREATOR_PK)
        .bind(E2_AGENT_PK)
        .execute(pool)
        .await
        .expect("seed task");
    }

    async fn seed_exam_assignment(pool: &DbPool, task_id: &str) {
        sqlx::query(
            "INSERT INTO exam_assignments (task_id, template_id, agent_public_key, bucket, status)
             VALUES (?, ?, ?, 'manual', 'assigned')
             ON DUPLICATE KEY UPDATE
               template_id = VALUES(template_id),
               agent_public_key = VALUES(agent_public_key),
               status = VALUES(status)",
        )
        .bind(task_id)
        .bind(EXAM_TEMPLATE_ID)
        .bind(E2_AGENT_PK)
        .execute(pool)
        .await
        .expect("seed exam assignment");
    }

    async fn seed_exam_task_fixture(pool: &DbPool, task_id: &str, with_assignment: bool) {
        seed_agent(pool).await;
        seed_exam_template(pool).await;
        seed_task(pool, task_id).await;
        if with_assignment {
            seed_exam_assignment(pool, task_id).await;
        }
    }

    async fn fetch_task_row(pool: &DbPool, task_id: &str) -> Task {
        sqlx::query_as::<_, Task>("SELECT * FROM tasks WHERE id = ?")
            .bind(task_id)
            .fetch_one(pool)
            .await
            .expect("fetch task row")
    }

    async fn fetch_exam_assignment_row(pool: &DbPool, task_id: &str) -> ExamAssignment {
        get_exam_assignment_by_task_id(pool, task_id)
            .await
            .expect("fetch exam assignment")
            .expect("exam assignment row")
    }

    async fn seed_agent_exam_state(pool: &DbPool, tasks_since_last_exam: i32, exam_urgency: f64) {
        upsert_agent_exam_state(
            pool,
            &AgentExamState {
                agent_public_key: E2_AGENT_PK.into(),
                exam_urgency,
                smoothed_score: None,
                last_exam_at: None,
                tasks_since_last_exam,
                updated_at: Utc::now(),
            },
        )
        .await
        .expect("seed agent exam state");
    }

    async fn poll_agent_exam_state_until<F>(
        pool: &DbPool,
        predicate: F,
        max_attempts: u32,
    ) -> AgentExamState
    where
        F: Fn(&AgentExamState) -> bool,
    {
        for _ in 0..max_attempts {
            if let Ok(Some(state)) = get_agent_exam_state(pool, E2_AGENT_PK).await {
                if predicate(&state) {
                    return state;
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        panic!("agent_exam_state did not reach expected condition within poll window");
    }

    fn assert_exam_audit_shape(audit: &serde_json::Value) {
        for key in [
            "exam_id",
            "assignment_hash",
            "expected_answer_hash",
            "actual_answer_hash",
            "hash_algorithm",
            "verdict",
            "pipeline",
            "timestamp",
            "compare_mode",
            "llm_fallback_used",
            "answer_verification_mode",
        ] {
            assert!(audit.get(key).is_some(), "missing exam audit field: {key}");
        }
        assert_eq!(audit["pipeline"], "exam");
        assert_eq!(audit["hash_algorithm"], "sha256");
    }

    #[test]
    fn is_validate_noop_when_completed_or_audit_without_submit_retry() {
        temp_env::with_vars([("EXAM_SKIP_ONCHAIN", Some("1"))], || {
            let mut task = Task {
                id: "t1".to_string(),
                creator_public_key: "c".to_string(),
                assigned_agent_public_key: None,
                budget_motes: 0,
                status: "Completed".to_string(),
                result_hash: None,
                result: None,
                metadata_uri: None,
                transaction_hash: "tx".to_string(),
                domain: "defi_analysis".to_string(),
                skill_id: None,
                prompt: "p".to_string(),
                deadline: 0,
                result_signature: None,
                validator_audit: None,
                timestamp: Utc::now(),
                parent_task_id: None,
            };
            assert!(is_validate_noop(&task));

            task.status = "InProgress".to_string();
            task.validator_audit = Some(serde_json::json!({"pipeline":"exam","verdict":"passed"}));
            assert!(is_validate_noop(&task));
            assert!(!needs_submit_retry(&task));
        });
    }

    #[test]
    fn needs_submit_retry_when_audit_present_and_not_completed_without_skip() {
        temp_env::with_vars([("EXAM_SKIP_ONCHAIN", None::<&str>)], || {
            let task = Task {
                id: "t1".to_string(),
                creator_public_key: "c".to_string(),
                assigned_agent_public_key: None,
                budget_motes: 0,
                status: "InProgress".to_string(),
                result_hash: Some("abc".to_string()),
                result: Some("output".to_string()),
                metadata_uri: None,
                transaction_hash: "tx".to_string(),
                domain: "defi_analysis".to_string(),
                skill_id: None,
                prompt: "p".to_string(),
                deadline: 0,
                result_signature: None,
                validator_audit: Some(serde_json::json!({"pipeline":"exam","verdict":"passed"})),
                timestamp: Utc::now(),
                parent_task_id: None,
            };
            assert!(!is_validate_noop(&task));
            assert!(needs_submit_retry(&task));
        });
    }

    #[test]
    fn score_from_validator_audit_supports_exam_and_stage() {
        let exam = serde_json::json!({"pipeline":"exam","verdict":"passed"});
        assert_eq!(score_from_validator_audit(&exam), Some(100));

        let stage = serde_json::json!({
            "pipeline": "stage",
            "output": { "total": 82 }
        });
        assert_eq!(score_from_validator_audit(&stage), Some(82));
    }

    #[test]
    fn submit_complete_cli_args_uses_domain_score_and_weight() {
        let args = submit_complete_cli_args(
            "0203abc...",
            "task-exam-1",
            "abc123",
            "defi_analysis",
            100,
            300,
        );
        assert_eq!(args[0], "0203abc...");
        assert_eq!(args[1], "task-exam-1");
        assert_eq!(args[2], "abc123");
        assert_eq!(args[3], "defi_analysis");
        assert_eq!(args[4], "100");
        assert_eq!(args[5], "300");
    }

    #[test]
    fn resolve_completion_weight_uses_exam_weight_for_exam_tasks() {
        let config = sample_config();
        assert_eq!(
            resolve_completion_weight(true, &config, "defi_analysis", 5_000_000_000),
            300
        );
    }

    #[test]
    fn resolve_completion_weight_uses_economic_formula_for_ordinary_tasks() {
        let config = sample_config();
        let exam_weight = resolve_completion_weight(true, &config, "defi_analysis", 5_000_000_000);
        let ordinary_weight =
            resolve_completion_weight(false, &config, "defi_analysis", 5_000_000_000);
        assert_eq!(exam_weight, 300);
        assert_ne!(ordinary_weight, exam_weight);
        assert!(ordinary_weight >= 1);
    }

    #[test]
    fn exam_fail_and_refusal_submit_args_use_zero_score_and_exam_weight() {
        let config = sample_config();
        let weight = resolve_completion_weight(true, &config, "defi_analysis", 5_000_000_000);

        for score in [0u32, 0u32] {
            let args = submit_complete_cli_args(
                "0203abc...",
                "task-fail",
                "deadbeef",
                "defi_analysis",
                score,
                weight,
            );
            assert_eq!(args[3], "defi_analysis");
            assert_eq!(args[4], "0");
            assert_eq!(args[5], "300");
        }
    }

    /// Shared by hosted `/execute` and autonomous `/validate` via `validate_and_complete`.
    #[tokio::test]
    async fn shared_validation_branching_covers_exam_and_ordinary_paths() {
        temp_env::async_with_vars([("VALIDATOR_MOCK_LLM", Some("1"))], async {
            let config = sample_config();

            let exam_pass = evaluate_task_validation_with_context(
                Some(Box::new(sample_exam_context())),
                "task-exam-1",
                "defi_analysis",
                "Compute stake",
                "ANSWER: 2845678901.25 cspr",
                4000,
                &config,
            )
            .await
            .expect("exam pass validation");
            assert_eq!(exam_pass.total, 100);
            let pass_audit = exam_pass.validator_audit.expect("pass audit");
            assert_eq!(pass_audit["pipeline"], "exam");
            assert_eq!(pass_audit["verdict"], "passed");

            let exam_fail = evaluate_task_validation_with_context(
                Some(Box::new(sample_exam_context())),
                "task-exam-1",
                "defi_analysis",
                "Compute stake",
                "ANSWER: 1 cspr",
                4000,
                &config,
            )
            .await
            .expect("exam fail validation");
            assert_eq!(exam_fail.total, 0);
            let fail_audit = exam_fail.validator_audit.expect("fail audit");
            assert_eq!(fail_audit["pipeline"], "exam");
            assert_eq!(fail_audit["verdict"], "failed");

            let ordinary = evaluate_task_validation_with_context(
                None,
                "task-regular-1",
                "defi_analysis",
                "Analyze yield",
                "Recommended allocation across cspr-usdt and cspr-eth pools with fee-adjusted APY.",
                4000,
                &config,
            )
            .await
            .expect("ordinary validation");
            assert!(ordinary.total <= 100);
            let stage_audit = ordinary.validator_audit.expect("stage audit");
            assert_eq!(stage_audit["pipeline"], "stage");
        })
        .await;
    }

    #[tokio::test]
    #[ignore = "requires MySQL: DATABASE_URL from backend/.env; cargo test --lib db_ -- --ignored --test-threads=1"]
    async fn db_exam_routing_passes_when_assignment_exists() {
        let task_id = "e2-db-pass";
        let pool = connect_test_pool().await;
        cleanup_e2_fixtures(&pool, task_id).await;
        seed_exam_task_fixture(&pool, task_id, true).await;

        temp_env::async_with_vars([("VALIDATOR_MOCK_LLM", Some("1"))], async {
            let config = sample_config();
            let result = evaluate_task_validation(
                &pool,
                &config,
                task_id,
                "defi_analysis",
                "Compute stake",
                "ANSWER: 2845678901.25 cspr",
                4000,
            )
            .await
            .expect("exam routing pass");

            assert_eq!(result.total, 100);
            let audit = result.validator_audit.expect("exam audit");
            assert_exam_audit_shape(&audit);
            assert_eq!(audit["verdict"], "passed");
            assert_eq!(audit["exam_id"], EXAM_TEMPLATE_ID);
        })
        .await;

        cleanup_e2_fixtures(&pool, task_id).await;
    }

    #[tokio::test]
    #[ignore = "requires MySQL: DATABASE_URL from backend/.env; cargo test --lib db_ -- --ignored --test-threads=1"]
    async fn db_exam_routing_fails_on_wrong_answer() {
        let task_id = "e2-db-fail";
        let pool = connect_test_pool().await;
        cleanup_e2_fixtures(&pool, task_id).await;
        seed_exam_task_fixture(&pool, task_id, true).await;

        temp_env::async_with_vars([("VALIDATOR_MOCK_LLM", Some("1"))], async {
            let config = sample_config();
            let result = evaluate_task_validation(
                &pool,
                &config,
                task_id,
                "defi_analysis",
                "Compute stake",
                "ANSWER: 1 cspr",
                4000,
            )
            .await
            .expect("exam routing fail");

            assert_eq!(result.total, 0);
            let audit = result.validator_audit.expect("exam audit");
            assert_exam_audit_shape(&audit);
            assert_eq!(audit["verdict"], "failed");
        })
        .await;

        cleanup_e2_fixtures(&pool, task_id).await;
    }

    #[tokio::test]
    #[ignore = "requires MySQL: DATABASE_URL from backend/.env; cargo test --lib db_ -- --ignored --test-threads=1"]
    async fn db_exam_routing_refusal() {
        let task_id = "e2-db-refusal";
        let pool = connect_test_pool().await;
        cleanup_e2_fixtures(&pool, task_id).await;
        seed_exam_task_fixture(&pool, task_id, true).await;

        temp_env::async_with_vars([("VALIDATOR_MOCK_LLM", Some("1"))], async {
            let config = sample_config();
            let result = evaluate_task_validation(
                &pool,
                &config,
                task_id,
                "defi_analysis",
                "Compute stake",
                "mock_refusal: I cannot fulfill this request",
                4000,
            )
            .await
            .expect("exam routing refusal");

            assert_eq!(result.total, 0);
            let audit = result.validator_audit.expect("exam audit");
            assert_exam_audit_shape(&audit);
            assert_eq!(audit["verdict"], "refusal");
        })
        .await;

        cleanup_e2_fixtures(&pool, task_id).await;
    }

    #[tokio::test]
    #[ignore = "requires MySQL: DATABASE_URL from backend/.env; cargo test --lib db_ -- --ignored --test-threads=1"]
    async fn db_ordinary_task_uses_stage_pipeline_without_assignment() {
        let task_id = "e2-db-ordinary";
        let pool = connect_test_pool().await;
        cleanup_e2_fixtures(&pool, task_id).await;
        seed_exam_task_fixture(&pool, task_id, false).await;

        temp_env::async_with_vars(
            [
                ("VALIDATOR_MOCK_LLM", Some("1")),
                ("VALIDATOR_PIPELINE", Some("stage")),
            ],
            async {
                let config = sample_config();
                let result = evaluate_task_validation(
                    &pool,
                    &config,
                    task_id,
                    "defi_analysis",
                    "Analyze yield",
                    "Recommended allocation across cspr-usdt and cspr-eth pools with fee-adjusted APY.",
                    4000,
                )
                .await
                .expect("ordinary routing");

                assert!(result.total <= 100);
                let audit = result.validator_audit.expect("stage audit");
                assert_eq!(audit["pipeline"], "stage");
            },
        )
        .await;

        cleanup_e2_fixtures(&pool, task_id).await;
    }

    #[tokio::test]
    #[ignore = "requires MySQL: DATABASE_URL from backend/.env; cargo test --lib db_ -- --ignored --test-threads=1"]
    async fn db_validate_and_complete_persists_exam_audit() {
        let task_id = "e2-db-persist";
        let output = "ANSWER: 2845678901.25 cspr";
        let pool = connect_test_pool().await;
        cleanup_e2_fixtures(&pool, task_id).await;
        seed_exam_task_fixture(&pool, task_id, true).await;

        temp_env::async_with_vars(
            [
                ("VALIDATOR_MOCK_LLM", Some("1")),
                ("EXAM_SKIP_ONCHAIN", Some("1")),
            ],
            async {
                let config = sample_config();
                validate_and_complete(
                    &pool,
                    &config,
                    task_id,
                    "defi_analysis",
                    "Compute stake",
                    5_000_000_000,
                    output,
                    4000,
                )
                .await;

                let task = fetch_task_row(&pool, task_id).await;
                assert_eq!(task.result.as_deref(), Some(output));
                assert!(task.result_hash.is_some());
                assert!(task.result_signature.is_some());

                let audit = task.validator_audit.expect("persisted validator_audit");
                assert_exam_audit_shape(&audit);
                assert_eq!(audit["verdict"], "passed");
                assert_eq!(audit["exam_id"], EXAM_TEMPLATE_ID);

                let assignment = fetch_exam_assignment_row(&pool, task_id).await;
                assert_eq!(assignment.status, "validated");
                assert_eq!(assignment.verdict.as_deref(), Some("passed"));
                assert!(assignment.validated_at.is_some());
            },
        )
        .await;

        cleanup_e2_fixtures(&pool, task_id).await;
    }

    #[tokio::test]
    #[ignore = "requires MySQL: DATABASE_URL from backend/.env; cargo test --lib db_completion_state -- --ignored --test-threads=1"]
    async fn db_validate_and_complete_updates_agent_exam_state_on_exam_path() {
        let task_id = "e2-db-exam-urgency-state";
        let output = "ANSWER: 2845678901.25 cspr";
        let pool = connect_test_pool().await;
        cleanup_e2_fixtures(&pool, task_id).await;
        seed_exam_task_fixture(&pool, task_id, true).await;
        seed_agent_exam_state(&pool, 5, 0.4).await;

        temp_env::async_with_vars(
            [
                ("VALIDATOR_MOCK_LLM", Some("1")),
                ("EXAM_SKIP_ONCHAIN", Some("1")),
            ],
            async {
                let config = sample_config();
                validate_and_complete(
                    &pool,
                    &config,
                    task_id,
                    "defi_analysis",
                    "Compute stake",
                    5_000_000_000,
                    output,
                    4000,
                )
                .await;

                let state = poll_agent_exam_state_until(
                    &pool,
                    |state| state.tasks_since_last_exam == 0 && state.last_exam_at.is_some(),
                    40,
                )
                .await;

                assert_eq!(state.tasks_since_last_exam, 0);
                assert!(state.last_exam_at.is_some());
                assert!(
                    state.exam_urgency < 0.4,
                    "exam validation hook should recalculate urgency downward after recent exam"
                );
                assert_eq!(
                    state.smoothed_score,
                    Some(100.0),
                    "exam validation should persist smoothed_score for pass verdict"
                );

                let assignment = fetch_exam_assignment_row(&pool, task_id).await;
                assert_eq!(assignment.status, "validated");
            },
        )
        .await;

        cleanup_e2_fixtures(&pool, task_id).await;
    }

    #[tokio::test]
    #[ignore = "requires MySQL: DATABASE_URL from backend/.env; cargo test --lib db_completion_state -- --ignored --test-threads=1"]
    async fn db_validate_and_complete_skip_onchain_does_not_update_ordinary_state() {
        let task_id = "e2-db-ordinary-urgency-skip";
        let output =
            "Recommended allocation across cspr-usdt and cspr-eth pools with fee-adjusted APY.";
        let pool = connect_test_pool().await;
        cleanup_e2_fixtures(&pool, task_id).await;
        seed_exam_task_fixture(&pool, task_id, false).await;
        seed_agent_exam_state(&pool, 2, 0.15).await;

        temp_env::async_with_vars(
            [
                ("VALIDATOR_MOCK_LLM", Some("1")),
                ("VALIDATOR_PIPELINE", Some("stage")),
                ("EXAM_SKIP_ONCHAIN", Some("1")),
            ],
            async {
                let config = sample_config();
                validate_and_complete(
                    &pool,
                    &config,
                    task_id,
                    "defi_analysis",
                    "Analyze yield",
                    5_000_000_000,
                    output,
                    4000,
                )
                .await;

                tokio::time::sleep(std::time::Duration::from_millis(250)).await;

                let state = get_agent_exam_state(&pool, E2_AGENT_PK)
                    .await
                    .expect("get agent exam state")
                    .expect("baseline row");

                assert_eq!(state.tasks_since_last_exam, 2);
                assert!((state.exam_urgency - 0.15).abs() < f64::EPSILON);
                assert!(
                    state.smoothed_score.is_none(),
                    "ordinary task path must not update smoothed_score"
                );
            },
        )
        .await;

        cleanup_e2_fixtures(&pool, task_id).await;
    }

    #[tokio::test]
    #[ignore = "requires MySQL: DATABASE_URL from backend/.env; cargo test --lib db_ -- --ignored --test-threads=1"]
    async fn db_validate_and_complete_uses_exam_weight_and_updates_fail_assignment() {
        let task_id = "e2-db-e3-fail";
        let output = "ANSWER: 1 cspr";
        let pool = connect_test_pool().await;
        cleanup_e2_fixtures(&pool, task_id).await;
        seed_exam_task_fixture(&pool, task_id, true).await;

        temp_env::async_with_vars(
            [
                ("VALIDATOR_MOCK_LLM", Some("1")),
                ("EXAM_SKIP_ONCHAIN", Some("1")),
            ],
            async {
                let config = sample_config();
                validate_and_complete(
                    &pool,
                    &config,
                    task_id,
                    "defi_analysis",
                    "Compute stake",
                    5_000_000_000,
                    output,
                    4000,
                )
                .await;

                let task = fetch_task_row(&pool, task_id).await;
                let audit = task.validator_audit.expect("persisted validator_audit");
                assert_eq!(audit["verdict"], "failed");

                let assignment = fetch_exam_assignment_row(&pool, task_id).await;
                assert_eq!(assignment.status, "validated");
                assert_eq!(assignment.verdict.as_deref(), Some("failed"));
                assert!(assignment.validated_at.is_some());
            },
        )
        .await;

        cleanup_e2_fixtures(&pool, task_id).await;
    }

    #[tokio::test]
    #[ignore = "requires MySQL: DATABASE_URL from backend/.env; cargo test --lib db_ -- --ignored --test-threads=1"]
    async fn db_validate_and_complete_updates_refusal_assignment() {
        let task_id = "e2-db-e3-refusal";
        let output = "mock_refusal: I cannot fulfill this request";
        let pool = connect_test_pool().await;
        cleanup_e2_fixtures(&pool, task_id).await;
        seed_exam_task_fixture(&pool, task_id, true).await;

        temp_env::async_with_vars(
            [
                ("VALIDATOR_MOCK_LLM", Some("1")),
                ("EXAM_SKIP_ONCHAIN", Some("1")),
            ],
            async {
                let config = sample_config();
                validate_and_complete(
                    &pool,
                    &config,
                    task_id,
                    "defi_analysis",
                    "Compute stake",
                    5_000_000_000,
                    output,
                    4000,
                )
                .await;

                let task = fetch_task_row(&pool, task_id).await;
                assert_eq!(task.result.as_deref(), Some(output));
                assert!(task.result_hash.is_some());
                assert!(task.result_signature.is_some());

                let audit = task.validator_audit.expect("persisted validator_audit");
                assert_exam_audit_shape(&audit);
                assert_eq!(audit["verdict"], "refusal");

                let assignment = fetch_exam_assignment_row(&pool, task_id).await;
                assert_eq!(assignment.status, "validated");
                assert_eq!(assignment.verdict.as_deref(), Some("refusal"));
                assert!(assignment.validated_at.is_some());
            },
        )
        .await;

        cleanup_e2_fixtures(&pool, task_id).await;
    }

    #[tokio::test]
    #[ignore = "requires MySQL: DATABASE_URL from backend/.env; cargo test --lib db_ -- --ignored --test-threads=1"]
    async fn db_validate_and_complete_retry_is_idempotent() {
        let task_id = "e2-db-retry-idempotent";
        let output = "ANSWER: 2845678901.25 cspr";
        let pool = connect_test_pool().await;
        cleanup_e2_fixtures(&pool, task_id).await;
        seed_exam_task_fixture(&pool, task_id, true).await;

        temp_env::async_with_vars(
            [
                ("VALIDATOR_MOCK_LLM", Some("1")),
                ("EXAM_SKIP_ONCHAIN", Some("1")),
            ],
            async {
                let config = sample_config();
                validate_and_complete(
                    &pool,
                    &config,
                    task_id,
                    "defi_analysis",
                    "Compute stake",
                    5_000_000_000,
                    output,
                    4000,
                )
                .await;

                let task_after_first = fetch_task_row(&pool, task_id).await;
                let audit_first = task_after_first
                    .validator_audit
                    .clone()
                    .expect("audit after first validate");
                let assignment_first = fetch_exam_assignment_row(&pool, task_id).await;

                validate_and_complete(
                    &pool,
                    &config,
                    task_id,
                    "defi_analysis",
                    "Compute stake",
                    5_000_000_000,
                    output,
                    4000,
                )
                .await;

                let task_after_second = fetch_task_row(&pool, task_id).await;
                let assignment_second = fetch_exam_assignment_row(&pool, task_id).await;

                assert_eq!(task_after_second.validator_audit, Some(audit_first));
                assert_eq!(
                    assignment_first.validated_at,
                    assignment_second.validated_at
                );
                assert_eq!(assignment_second.status, "validated");
                assert_eq!(assignment_second.verdict.as_deref(), Some("passed"));
            },
        )
        .await;

        cleanup_e2_fixtures(&pool, task_id).await;
    }

    #[tokio::test]
    #[ignore = "requires MySQL: DATABASE_URL from backend/.env; cargo test --lib db_ -- --ignored --test-threads=1"]
    async fn db_validate_retry_reuses_audit_for_submit_path() {
        let task_id = "e2-db-retry-submit";
        let output = "ANSWER: 2845678901.25 cspr";
        let pool = connect_test_pool().await;
        cleanup_e2_fixtures(&pool, task_id).await;
        seed_exam_task_fixture(&pool, task_id, true).await;

        temp_env::async_with_vars([("VALIDATOR_MOCK_LLM", Some("1"))], async {
            let config = sample_config();

            temp_env::async_with_vars([("EXAM_SKIP_ONCHAIN", Some("1"))], async {
                validate_and_complete(
                    &pool,
                    &config,
                    task_id,
                    "defi_analysis",
                    "Compute stake",
                    5_000_000_000,
                    output,
                    4000,
                )
                .await;
            })
            .await;

            let task_after_first = fetch_task_row(&pool, task_id).await;
            let audit_first = task_after_first
                .validator_audit
                .clone()
                .expect("audit after first validate");
            let assignment_first = fetch_exam_assignment_row(&pool, task_id).await;
            assert_eq!(task_after_first.status, "InProgress");

            validate_and_complete(
                &pool,
                &config,
                task_id,
                "defi_analysis",
                "Compute stake",
                5_000_000_000,
                output,
                4000,
            )
            .await;

            let task_after_retry = fetch_task_row(&pool, task_id).await;
            let assignment_after_retry = fetch_exam_assignment_row(&pool, task_id).await;

            assert_eq!(task_after_retry.validator_audit, Some(audit_first));
            assert_eq!(
                assignment_first.validated_at,
                assignment_after_retry.validated_at
            );
        })
        .await;

        cleanup_e2_fixtures(&pool, task_id).await;
    }

    /// Gap 3 substitute: proves backend reaches submit-path attempt when `EXAM_SKIP_ONCHAIN` is unset.
    #[tokio::test]
    #[ignore = "requires MySQL: prod-path gap sanity; cargo test prod_path_branch_sanity -- --ignored --test-threads=1"]
    async fn prod_path_branch_sanity_reaches_submit_attempt() {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
            )
            .with_test_writer()
            .try_init();

        assert!(
            !should_skip_onchain_submit(),
            "EXAM_SKIP_ONCHAIN must be unset for prod-path sanity check"
        );

        let task_id = "e2-prod-path-sanity";
        let output = "ANSWER: 2845678901.25 cspr";
        let pool = connect_test_pool().await;
        cleanup_e2_fixtures(&pool, task_id).await;
        seed_exam_task_fixture(&pool, task_id, true).await;

        temp_env::async_with_vars([("VALIDATOR_MOCK_LLM", Some("1"))], async {
            let config = sample_config();

            validate_and_complete(
                &pool,
                &config,
                task_id,
                "defi_analysis",
                "Compute stake",
                5_000_000_000,
                output,
                4000,
            )
            .await;

            let task = fetch_task_row(&pool, task_id).await;
            assert!(
                task.validator_audit.is_some(),
                "validation must persist audit before submit attempt"
            );
        })
        .await;

        cleanup_e2_fixtures(&pool, task_id).await;
    }
}
