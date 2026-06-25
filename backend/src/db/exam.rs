//! Internal read paths for exam tables (E1). Not exposed via REST/MCP.

use chrono::{DateTime, Utc};

use super::DbPool;
use super::models::{ExamAssignment, ExamTemplate};

/// Agent eligible for exam dispatch (internal only).
#[derive(Clone, Debug, sqlx::FromRow)]
pub struct DispatchCandidate {
    pub public_key: String,
    pub active_jobs: i32,
    pub reputation_score: i64,
}

/// Active Type-H templates from the curated seed pool.
pub async fn list_active_exam_templates(pool: &DbPool) -> Result<Vec<ExamTemplate>, sqlx::Error> {
    sqlx::query_as::<_, ExamTemplate>(
        "SELECT id, prompt, expected_answer_canonical, domain, status, source_metadata, \
         created_at, updated_at \
         FROM exam_templates WHERE status = 'active' ORDER BY id",
    )
    .fetch_all(pool)
    .await
}

/// Lookup exam assignment by live task id (used by E2 validation routing).
pub async fn get_exam_assignment_by_task_id(
    pool: &DbPool,
    task_id: &str,
) -> Result<Option<ExamAssignment>, sqlx::Error> {
    sqlx::query_as::<_, ExamAssignment>(
        "SELECT task_id, template_id, agent_public_key, bucket, status, verdict, \
         created_at, validated_at \
         FROM exam_assignments WHERE task_id = ?",
    )
    .bind(task_id)
    .fetch_optional(pool)
    .await
}

/// Load internal template row including canonical expected answer.
pub async fn get_exam_template_by_id(
    pool: &DbPool,
    template_id: &str,
) -> Result<Option<ExamTemplate>, sqlx::Error> {
    sqlx::query_as::<_, ExamTemplate>(
        "SELECT id, prompt, expected_answer_canonical, domain, status, source_metadata, \
         created_at, updated_at \
         FROM exam_templates WHERE id = ?",
    )
    .bind(template_id)
    .fetch_optional(pool)
    .await
}

/// Mark exam assignment validated after E3 completion path (internal only).
pub async fn update_exam_assignment_validation(
    pool: &DbPool,
    task_id: &str,
    verdict: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE exam_assignments \
         SET status = 'validated', verdict = ?, validated_at = NOW() \
         WHERE task_id = ?",
    )
    .bind(verdict)
    .bind(task_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Active agents with global reputation sum (same aggregation as leaderboard).
pub async fn list_dispatch_candidates(
    pool: &DbPool,
) -> Result<Vec<DispatchCandidate>, sqlx::Error> {
    sqlx::query_as::<_, DispatchCandidate>(
        "SELECT
            a.public_key,
            a.active_jobs,
            CAST(COALESCE(SUM(r.score), 0) AS SIGNED) AS reputation_score
         FROM agents a
         LEFT JOIN reputations r ON a.public_key = r.agent_public_key
         WHERE a.status = 'active'
         GROUP BY a.public_key, a.active_jobs
         ORDER BY a.public_key",
    )
    .fetch_all(pool)
    .await
}

/// Random active Type-H template from the curated seed pool.
pub async fn pick_random_active_exam_template(
    pool: &DbPool,
) -> Result<Option<ExamTemplate>, sqlx::Error> {
    sqlx::query_as::<_, ExamTemplate>(
        "SELECT id, prompt, expected_answer_canonical, domain, status, source_metadata, \
         created_at, updated_at \
         FROM exam_templates \
         WHERE status = 'active' \
         ORDER BY RAND() \
         LIMIT 1",
    )
    .fetch_optional(pool)
    .await
}

/// Count exam assignments for an agent since the given timestamp (frequency cap).
pub async fn count_recent_exam_assignments(
    pool: &DbPool,
    agent_public_key: &str,
    since: DateTime<Utc>,
) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM exam_assignments \
         WHERE agent_public_key = ? AND created_at >= ?",
    )
    .bind(agent_public_key)
    .bind(since)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

/// Insert a new exam assignment row (E4 dispatch).
pub async fn insert_exam_assignment(
    pool: &DbPool,
    task_id: &str,
    template_id: &str,
    agent_public_key: &str,
    bucket: &str,
    status: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO exam_assignments (task_id, template_id, agent_public_key, bucket, status) \
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(task_id)
    .bind(template_id)
    .bind(agent_public_key)
    .bind(bucket)
    .bind(status)
    .execute(pool)
    .await?;
    Ok(())
}

/// Parameters for inserting a dispatched live exam task.
pub struct DispatchedExamTaskParams<'a> {
    pub task_id: &'a str,
    pub creator_public_key: &'a str,
    pub assigned_agent_public_key: &'a str,
    pub budget_motes: u64,
    pub transaction_hash: &'a str,
    pub domain: &'a str,
    pub prompt: &'a str,
}

/// Insert a dispatched live exam task already assigned and InProgress.
pub async fn insert_dispatched_exam_task(
    pool: &DbPool,
    params: DispatchedExamTaskParams<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO tasks (
            id, creator_public_key, assigned_agent_public_key, budget_motes, status,
            transaction_hash, domain, prompt, deadline
         ) VALUES (?, ?, ?, ?, 'InProgress', ?, ?, ?, 0)",
    )
    .bind(params.task_id)
    .bind(params.creator_public_key)
    .bind(params.assigned_agent_public_key)
    .bind(params.budget_motes)
    .bind(params.transaction_hash)
    .bind(params.domain)
    .bind(params.prompt)
    .execute(pool)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use validator_engine::exam::canonicalize::canonicalize_exam_answer;

    /// Seed values from scripts/seed_exam_pool.sql must already match E0 canonicalize.
    #[test]
    fn seed_canonical_answers_are_pre_normalized() {
        let seeds = [
            "2845678901.25 cspr",
            "412345678.90 usd",
            "9876543210.00 usd",
            "100.47 usd",
            "523456789.12 usd",
        ];
        for seed in seeds {
            assert_eq!(
                canonicalize_exam_answer(seed),
                seed,
                "seed answer must be pre-canonicalized: {seed}"
            );
        }
    }
}
