use serde_json::Value;

use crate::config::Config;
use crate::db::models::Task;
use crate::orchestrator::executor::ExecutionResult;
use crate::orchestrator::worker_prompt::build_worker_prompt;
use crate::validator::{V2Outcome, evaluate_task_v2, resolve_skill_str};

/// Result of the v2 fixture-aware task pipeline (worker prompt + judge).
#[derive(Debug)]
pub struct TaskPipelineResult {
    pub worker_prompt: String,
    pub agent_output: String,
    pub processing_time_ms: u64,
    pub skill: String,
    pub v2_outcome: V2Outcome,
}

/// Test helper: pipeline with stubbed worker output (no external API).
pub async fn run_fixture_pipeline_with_output(
    task: &Task,
    agent_output: &str,
    processing_time_ms: u64,
    fixture: Value,
    config: &Config,
) -> Result<TaskPipelineResult, String> {
    let skill = resolve_skill_str(task.skill_id.as_deref(), &task.domain)
        .ok_or_else(|| format!("unsupported skill for task {}", task.id))?;

    let worker_prompt = build_worker_prompt(&task.prompt, &fixture);
    let v2_outcome = evaluate_task_v2(
        &skill,
        &task.prompt,
        agent_output,
        processing_time_ms,
        Some(fixture),
        config,
    )
    .await;

    Ok(TaskPipelineResult {
        worker_prompt,
        agent_output: agent_output.to_string(),
        processing_time_ms,
        skill,
        v2_outcome,
    })
}

pub fn sample_task(skill_id: Option<&str>, domain: &str, prompt: &str) -> Task {
    use chrono::Utc;

    Task {
        id: "task-e2e-1".to_string(),
        creator_public_key: "creator".to_string(),
        assigned_agent_public_key: Some("agent-1".to_string()),
        budget_motes: 10_000_000_000,
        status: "InProgress".to_string(),
        result_hash: None,
        result: None,
        metadata_uri: None,
        transaction_hash: "tx-1".to_string(),
        domain: domain.to_string(),
        skill_id: skill_id.map(str::to_string),
        prompt: prompt.to_string(),
        deadline: 0,
        result_signature: None,
        validator_audit: None,
        timestamp: Utc::now(),
    }
}

pub type StubExecutionResult = ExecutionResult;
