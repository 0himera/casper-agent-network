use serde_json::Value;

use crate::config::Config;
use crate::db::models::{Agent, Task};
use crate::orchestrator::executor::{execute_agent, ExecutionResult};
use crate::orchestrator::worker_prompt::build_worker_prompt;
use crate::validator::{evaluate_task_v2, resolve_skill_str, V2Outcome};

/// Result of the v2 fixture-aware task pipeline (worker prompt + judge).
#[derive(Debug)]
pub struct TaskPipelineResult {
    pub worker_prompt: String,
    pub agent_output: String,
    pub processing_time_ms: u64,
    pub skill: String,
    pub v2_outcome: V2Outcome,
}

/// Run worker execution and v2 judge when a fixture is available.
///
/// Production live `/execute` still uses legacy judge until Phase 10.2; this
/// pipeline is the Phase 9 contract used by E2E and future cutover.
pub async fn run_task_pipeline(
    task: &Task,
    agent: &Agent,
    fixture: Option<Value>,
    config: &Config,
) -> Result<TaskPipelineResult, String> {
    let skill = resolve_skill_str(task.skill_id.as_deref(), &task.domain)
        .ok_or_else(|| format!("unsupported skill for task {}", task.id))?;

    let worker_prompt = match fixture.as_ref() {
        Some(f) => build_worker_prompt(&task.prompt, f),
        None => task.prompt.clone(),
    };

    let exec_res = execute_agent(
        &skill,
        &worker_prompt,
        agent.endpoint_url.as_deref(),
        agent.api_key.as_deref(),
        agent.model.as_deref(),
        agent.system_prompt.as_deref(),
        config,
    )
    .await
    .map_err(|e| format!("agent execution failed: {e}"))?;

    let v2_outcome = evaluate_task_v2(
        &skill,
        &task.prompt,
        &exec_res.output,
        exec_res.processing_time_ms,
        fixture,
        config,
    )
    .await;

    Ok(TaskPipelineResult {
        worker_prompt,
        agent_output: exec_res.output,
        processing_time_ms: exec_res.processing_time_ms,
        skill,
        v2_outcome,
    })
}

/// Grade a precomputed agent output through the v2 path (no external worker call).
pub async fn run_v2_grade_only(
    skill: &str,
    task_prompt: &str,
    agent_output: &str,
    processing_time_ms: u64,
    fixture: Option<Value>,
    config: &Config,
) -> TaskPipelineResult {
    let worker_prompt = match fixture.as_ref() {
        Some(f) => build_worker_prompt(task_prompt, f),
        None => task_prompt.to_string(),
    };

    let v2_outcome = evaluate_task_v2(
        skill,
        task_prompt,
        agent_output,
        processing_time_ms,
        fixture,
        config,
    )
    .await;

    TaskPipelineResult {
        worker_prompt,
        agent_output: agent_output.to_string(),
        processing_time_ms,
        skill: skill.to_string(),
        v2_outcome,
    }
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
        timestamp: Utc::now(),
    }
}

#[allow(dead_code)]
pub fn sample_agent() -> Agent {
    use chrono::Utc;

    Agent {
        public_key: "agent-1".to_string(),
        name: "Test Agent".to_string(),
        description: None,
        metadata_uri: None,
        endpoint_url: None,
        api_key: None,
        model: None,
        active_jobs: 0,
        status: "active".to_string(),
        recommended_price_motes: 0,
        custom_price_motes: 0,
        system_prompt: None,
        timestamp: Utc::now(),
    }
}

pub type StubExecutionResult = ExecutionResult;
