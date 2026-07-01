//! Internal DB row types and public API DTOs.
//!
//! Exam secrets (`expected_answer_canonical`, assignment linkage) live only in
//! `ExamTemplate` / `ExamAssignment`. Never expose those structs on REST or MCP routes.
//! Use [`TaskPublic`] for agent-facing task responses.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Agent profile row as stored in `agents`.
#[derive(Clone, Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Agent {
    pub public_key: String,
    pub name: String,
    pub description: Option<String>,
    pub metadata_uri: Option<String>,
    pub endpoint_url: Option<String>,
    #[serde(skip_serializing)]
    pub api_key: Option<String>,
    pub model: Option<String>,
    pub active_jobs: i32,
    pub status: String,
    pub recommended_price_motes: u64,
    pub custom_price_motes: u64,
    pub system_prompt: Option<String>,
    pub timestamp: DateTime<Utc>,
    #[sqlx(default)]
    pub is_available: bool,
    #[sqlx(default)]
    pub completed_tasks: i64,
    #[sqlx(default)]
    pub total_earnings_motes: i64,
    #[sqlx(default)]
    pub reputation_score: i64,
    #[sqlx(default)]
    pub skills: Option<String>,
}

/// Live task row as stored in `tasks`. Internal use; prefer [`TaskPublic`] for HTTP responses.
#[derive(Clone, Debug, sqlx::FromRow)]
pub struct Task {
    pub id: String,
    pub creator_public_key: String,
    pub assigned_agent_public_key: Option<String>,
    pub budget_motes: u64,
    pub status: String,
    pub result_hash: Option<String>,
    pub result: Option<String>,
    pub metadata_uri: Option<String>,
    pub transaction_hash: String,
    pub domain: String,
    /// Legacy F3-era field; persisted for API/DB compatibility. Stage scoring uses `domain` only.
    pub skill_id: Option<String>,
    pub prompt: String,
    pub deadline: u64,
    pub result_signature: Option<String>,
    pub validator_audit: Option<serde_json::Value>,
    pub timestamp: DateTime<Utc>,
    pub parent_task_id: Option<String>,
}

/// Agent-facing task shape for REST/MCP. Explicit allowlist — no exam table fields.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TaskPublic {
    pub id: String,
    pub creator_public_key: String,
    pub assigned_agent_public_key: Option<String>,
    pub budget_motes: u64,
    pub status: String,
    pub result_hash: Option<String>,
    pub result: Option<String>,
    pub metadata_uri: Option<String>,
    pub transaction_hash: String,
    pub domain: String,
    pub skill_id: Option<String>,
    pub prompt: String,
    pub deadline: u64,
    pub result_signature: Option<String>,
    pub validator_audit: Option<serde_json::Value>,
    pub timestamp: DateTime<Utc>,
    pub parent_task_id: Option<String>,
}

impl From<Task> for TaskPublic {
    fn from(task: Task) -> Self {
        Self {
            id: task.id,
            creator_public_key: task.creator_public_key,
            assigned_agent_public_key: task.assigned_agent_public_key,
            budget_motes: task.budget_motes,
            status: task.status,
            result_hash: task.result_hash,
            result: task.result,
            metadata_uri: task.metadata_uri,
            transaction_hash: task.transaction_hash,
            domain: task.domain,
            skill_id: task.skill_id,
            prompt: task.prompt,
            deadline: task.deadline,
            result_signature: task.result_signature,
            validator_audit: task.validator_audit,
            timestamp: task.timestamp,
            parent_task_id: task.parent_task_id,
        }
    }
}

/// Internal exam template pool row. `expected_answer_canonical` must match E0 canonicalize.
#[derive(Clone, Debug, sqlx::FromRow)]
pub struct ExamTemplate {
    pub id: String,
    pub prompt: String,
    pub expected_answer_canonical: String,
    pub domain: String,
    pub status: String,
    pub source_metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Internal linkage: live task → exam template → assigned agent.
#[derive(Clone, Debug, sqlx::FromRow)]
pub struct ExamAssignment {
    pub task_id: String,
    pub template_id: String,
    pub agent_public_key: String,
    pub bucket: String,
    pub status: String,
    pub verdict: Option<String>,
    pub created_at: DateTime<Utc>,
    pub validated_at: Option<DateTime<Utc>>,
}

/// Explicit column list for public task reads (no `SELECT *`).
pub const TASK_PUBLIC_COLUMNS: &str = "\
    id, creator_public_key, assigned_agent_public_key, budget_motes, status, \
    result_hash, result, metadata_uri, transaction_hash, domain, skill_id, \
    prompt, deadline, result_signature, validator_audit, timestamp, parent_task_id";

#[derive(Clone, Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Reputation {
    pub id: String,
    pub agent_public_key: String,
    pub skill: String,
    pub score: i32,
    pub timestamp: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct BenchmarkRun {
    pub id: i32,
    pub agent_public_key: String,
    pub domain: String,
    pub score: i32,
    pub result: String,
    pub rubric_scores: serde_json::Value, // For JSON storage
    pub timestamp: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_public_excludes_exam_only_field_names() {
        let json = serde_json::to_value(TaskPublic {
            id: "t1".into(),
            creator_public_key: "creator".into(),
            assigned_agent_public_key: None,
            budget_motes: 1,
            status: "Open".into(),
            result_hash: None,
            result: None,
            metadata_uri: None,
            transaction_hash: "tx".into(),
            domain: "defi_analysis".into(),
            skill_id: None,
            prompt: "ANSWER: 42 usd".into(),
            deadline: 0,
            result_signature: None,
            validator_audit: None,
            timestamp: Utc::now(),
            parent_task_id: None,
        })
        .expect("serialize TaskPublic");

        let obj = json.as_object().expect("object");
        for forbidden in [
            "expected_answer_canonical",
            "template_id",
            "is_exam",
            "exam_id",
        ] {
            assert!(
                !obj.contains_key(forbidden),
                "TaskPublic must not expose exam field: {forbidden}"
            );
        }
    }
}
