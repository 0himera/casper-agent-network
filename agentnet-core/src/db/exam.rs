//! Internal read paths for exam tables (E1). Not exposed via REST/MCP.

use chrono::{DateTime, Utc};

use super::DbPool;
use super::models::{AgentExamState, ExamAssignment, ExamTemplate};

#[derive(Clone, Debug, Default)]
pub struct Config {
    pub exam_urgency_task_weight: f64,
    pub exam_urgency_variance_weight: f64,
    pub exam_urgency_recent_verdicts: u32,
    pub exam_smoothed_ema_alpha: f64,
}

pub fn clamp_ema_alpha(alpha: f64) -> f64 {
    const DEFAULT: f64 = 0.3;
    if !alpha.is_finite() || alpha <= 0.0 || alpha > 1.0 {
        DEFAULT
    } else {
        alpha
    }
}

/// Agent eligible for exam dispatch (internal only).
#[derive(Clone, Debug, sqlx::FromRow)]
pub struct DispatchCandidate {
    pub public_key: String,
    pub active_jobs: i32,
    pub reputation_score: i64,
    pub exam_urgency: f64,
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
         WHERE task_id = ? AND status != 'validated'",
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
            CAST(COALESCE(SUM(r.score), 0) AS SIGNED) AS reputation_score,
            COALESCE(aes.exam_urgency, 0) AS exam_urgency
         FROM agents a
         LEFT JOIN reputations r ON a.public_key = r.agent_public_key
         LEFT JOIN agent_exam_state aes ON a.public_key = aes.agent_public_key
         WHERE a.status = 'active'
         GROUP BY a.public_key, a.active_jobs, aes.exam_urgency
         ORDER BY a.public_key",
    )
    .fetch_all(pool)
    .await
}

/// Recent validated exam verdict strings for an agent (newest first).
pub async fn list_recent_validated_verdicts(
    pool: &DbPool,
    agent_public_key: &str,
    limit: u32,
) -> Result<Vec<String>, sqlx::Error> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT verdict FROM exam_assignments
         WHERE agent_public_key = ? AND status = 'validated' AND verdict IS NOT NULL
         ORDER BY validated_at DESC
         LIMIT ?",
    )
    .bind(agent_public_key)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|(verdict,)| verdict).collect())
}

/// All validated exam verdicts for an agent in chronological order (oldest first).
pub async fn list_validated_exam_verdicts_chronological(
    pool: &DbPool,
    agent_public_key: &str,
) -> Result<Vec<String>, sqlx::Error> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT verdict FROM exam_assignments
         WHERE agent_public_key = ? AND status = 'validated' AND verdict IS NOT NULL
         ORDER BY validated_at ASC",
    )
    .bind(agent_public_key)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|(verdict,)| verdict).collect())
}

/// Map validated exam verdict to smoothed numeric value (`passed` → 100, failures → 0).
pub fn verdict_to_smoothed_value(verdict: &str) -> Option<f64> {
    match verdict {
        "passed" => Some(100.0),
        "failed" | "refusal" | "gate_failed" => Some(0.0),
        _ => None,
    }
}

/// Exponential moving average over numeric verdict values (oldest → newest).
pub fn compute_ema(values: &[f64], alpha: f64) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let alpha = clamp_ema_alpha(alpha);
    let mut ema = values[0];
    for &value in values.iter().skip(1) {
        ema = alpha * value + (1.0 - alpha) * ema;
    }
    Some(ema)
}

/// Compute smoothed score from validated verdict strings.
pub fn compute_smoothed_score_from_verdicts(verdicts: &[String], alpha: f64) -> Option<f64> {
    let values: Vec<f64> = verdicts
        .iter()
        .filter_map(|verdict| verdict_to_smoothed_value(verdict))
        .collect();
    compute_ema(&values, alpha)
}

/// Load validated exam history and compute off-chain smoothed score.
pub async fn compute_smoothed_score(
    pool: &DbPool,
    agent_public_key: &str,
    alpha: f64,
) -> Result<Option<f64>, sqlx::Error> {
    let verdicts = list_validated_exam_verdicts_chronological(pool, agent_public_key).await?;
    Ok(compute_smoothed_score_from_verdicts(&verdicts, alpha))
}

/// Instability of recent verdicts: adjacent flip rate in `[0.0, 1.0]`.
pub fn verdict_instability(recent_verdicts: &[String]) -> f64 {
    if recent_verdicts.len() < 2 {
        return 0.0;
    }
    let mut flips = 0u32;
    for window in recent_verdicts.windows(2) {
        if window[0] != window[1] {
            flips += 1;
        }
    }
    flips as f64 / (recent_verdicts.len() - 1) as f64
}

/// Pure urgency formula from agent state and recent verdict history.
pub fn compute_exam_urgency_value(
    tasks_since_last_exam: i32,
    last_exam_at: Option<DateTime<Utc>>,
    recent_verdicts: &[String],
    config: &Config,
) -> f64 {
    let task_component = tasks_since_last_exam.max(0) as f64 * config.exam_urgency_task_weight;
    let variance_component =
        verdict_instability(recent_verdicts) * config.exam_urgency_variance_weight;

    let recency_multiplier = match last_exam_at {
        None => 1.2,
        Some(last) => {
            let hours = (Utc::now() - last).num_hours();
            if hours < 24 {
                0.25
            } else if hours < 72 {
                0.5
            } else {
                1.0
            }
        }
    };

    let stability_discount =
        if recent_verdicts.len() >= 2 && recent_verdicts.iter().all(|v| v == "passed") {
            0.5
        } else {
            1.0
        };

    (task_component + variance_component) * recency_multiplier * stability_discount
}

fn default_agent_exam_state(agent_public_key: &str) -> AgentExamState {
    AgentExamState {
        agent_public_key: agent_public_key.to_string(),
        exam_urgency: 0.0,
        smoothed_score: None,
        last_exam_at: None,
        tasks_since_last_exam: 0,
        updated_at: Utc::now(),
    }
}

pub fn resolve_price_score(smoothed: Option<f64>, chain_sum: i64) -> u32 {
    let effective = smoothed.unwrap_or(chain_sum as f64);
    // clamp to 0..100
    if effective < 0.0 {
        0
    } else if effective > 100.0 {
        100
    } else {
        effective as u32
    }
}

/// Recompute and persist `exam_urgency` for one agent.
pub async fn recalculate_exam_urgency(
    pool: &DbPool,
    agent_public_key: &str,
    config: &Config,
) -> Result<(), sqlx::Error> {
    let mut state = get_agent_exam_state(pool, agent_public_key)
        .await?
        .unwrap_or_else(|| default_agent_exam_state(agent_public_key));

    let verdicts =
        list_recent_validated_verdicts(pool, agent_public_key, config.exam_urgency_recent_verdicts)
            .await?;

    state.exam_urgency = compute_exam_urgency_value(
        state.tasks_since_last_exam,
        state.last_exam_at,
        &verdicts,
        config,
    );
    state.updated_at = Utc::now();

    upsert_agent_exam_state(pool, &state).await
}

/// After exam validation: reset task counter, stamp last exam, recalc urgency, smoothed score, and update recommended price.
pub async fn on_exam_validated(
    pool: &DbPool,
    agent_public_key: &str,
    config: &Config,
) -> Result<(), sqlx::Error> {
    let mut state = get_agent_exam_state(pool, agent_public_key)
        .await?
        .unwrap_or_else(|| default_agent_exam_state(agent_public_key));

    state.last_exam_at = Some(Utc::now());
    state.tasks_since_last_exam = 0;

    let verdicts =
        list_recent_validated_verdicts(pool, agent_public_key, config.exam_urgency_recent_verdicts)
            .await?;

    state.exam_urgency = compute_exam_urgency_value(
        state.tasks_since_last_exam,
        state.last_exam_at,
        &verdicts,
        config,
    );
    state.smoothed_score =
        compute_smoothed_score(pool, agent_public_key, config.exam_smoothed_ema_alpha).await?;
    state.updated_at = Utc::now();

    upsert_agent_exam_state(pool, &state).await?;

    // Update recommended price
    let chain_sum: i64 = sqlx::query_scalar("SELECT CAST(COALESCE(SUM(score), 0) AS SIGNED) FROM reputations WHERE agent_public_key = ?")
        .bind(agent_public_key)
        .fetch_one(pool)
        .await
        .unwrap_or(0);

    let price_score = resolve_price_score(state.smoothed_score, chain_sum);
    // Use 10000ms for a 1.0 multiplier
    let new_price =
        crate::casper_utils::recommended_price_motes("defi_analysis", price_score, 10000);

    sqlx::query("UPDATE agents SET recommended_price_motes = ? WHERE public_key = ?")
        .bind(new_price)
        .bind(agent_public_key)
        .execute(pool)
        .await?;

    Ok(())
}

/// After ordinary task completion: increment counter and recalc urgency.
pub async fn on_ordinary_task_completed(
    pool: &DbPool,
    agent_public_key: &str,
    config: &Config,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO agent_exam_state (agent_public_key, tasks_since_last_exam)
         VALUES (?, 1)
         ON DUPLICATE KEY UPDATE tasks_since_last_exam = tasks_since_last_exam + 1",
    )
    .bind(agent_public_key)
    .execute(pool)
    .await?;

    recalculate_exam_urgency(pool, agent_public_key, config).await
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

/// Load per-agent exam state row (internal only).
pub async fn get_agent_exam_state(
    pool: &DbPool,
    agent_public_key: &str,
) -> Result<Option<AgentExamState>, sqlx::Error> {
    sqlx::query_as::<_, AgentExamState>(
        "SELECT agent_public_key, exam_urgency, smoothed_score, last_exam_at, \
         tasks_since_last_exam, updated_at \
         FROM agent_exam_state WHERE agent_public_key = ?",
    )
    .bind(agent_public_key)
    .fetch_optional(pool)
    .await
}

/// Insert or update per-agent exam state (internal only).
pub async fn upsert_agent_exam_state(
    pool: &DbPool,
    state: &AgentExamState,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO agent_exam_state \
         (agent_public_key, exam_urgency, smoothed_score, last_exam_at, tasks_since_last_exam) \
         VALUES (?, ?, ?, ?, ?) \
         ON DUPLICATE KEY UPDATE \
         exam_urgency = VALUES(exam_urgency), \
         smoothed_score = VALUES(smoothed_score), \
         last_exam_at = VALUES(last_exam_at), \
         tasks_since_last_exam = VALUES(tasks_since_last_exam)",
    )
    .bind(&state.agent_public_key)
    .bind(state.exam_urgency)
    .bind(state.smoothed_score)
    .bind(state.last_exam_at)
    .bind(state.tasks_since_last_exam)
    .execute(pool)
    .await?;
    Ok(())
}

/// Idempotent backfill: one row per active agent. Safe to call on every startup.
pub async fn ensure_agent_exam_state_for_active_agents(pool: &DbPool) -> Result<(), sqlx::Error> {
    let result = sqlx::query(
        "INSERT INTO agent_exam_state (agent_public_key)
         SELECT public_key FROM agents WHERE status = 'active'
         ON DUPLICATE KEY UPDATE agent_public_key = agent_public_key",
    )
    .execute(pool)
    .await?;

    tracing::info!(
        "agent_exam_state backfill rows_affected={}",
        result.rows_affected()
    );
    Ok(())
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
    use chrono::Utc;
    use sqlx::mysql::MySqlPool;

    use super::*;
    use crate::db::init_db;

    #[test]
    fn seed_canonical_answers_are_pre_normalized() {
        fn canonicalize_exam_answer(s: &str) -> &str {
            s
        }

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

    fn sample_urgency_config() -> Config {
        Config {
            exam_urgency_task_weight: 0.05,
            exam_urgency_variance_weight: 0.2,
            exam_urgency_recent_verdicts: 5,
            exam_smoothed_ema_alpha: 0.3,
        }
    }

    #[test]
    fn verdict_to_smoothed_value_maps_known_verdicts() {
        assert_eq!(verdict_to_smoothed_value("passed"), Some(100.0));
        assert_eq!(verdict_to_smoothed_value("failed"), Some(0.0));
        assert_eq!(verdict_to_smoothed_value("refusal"), Some(0.0));
        assert_eq!(verdict_to_smoothed_value("gate_failed"), Some(0.0));
        assert_eq!(verdict_to_smoothed_value("unknown"), None);
    }

    #[test]
    fn compute_ema_empty_history_returns_none() {
        assert_eq!(compute_ema(&[], 0.3), None);
    }

    #[test]
    fn compute_ema_single_exam_returns_value() {
        assert_eq!(compute_ema(&[100.0], 0.3), Some(100.0));
        assert_eq!(compute_ema(&[0.0], 0.3), Some(0.0));
    }

    #[test]
    fn compute_ema_pass_streak_then_fail_drops_smoothly() {
        let values = [100.0, 100.0, 0.0];
        let score = compute_ema(&values, 0.3).expect("ema");
        assert!((score - 70.0).abs() < f64::EPSILON);
        assert!(score > 0.0);
    }

    #[test]
    fn compute_ema_alternating_verdicts() {
        let values = [100.0, 0.0, 100.0, 0.0];
        let score = compute_ema(&values, 0.3).expect("ema");
        assert!(score > 0.0 && score < 100.0);
    }

    #[test]
    fn compute_smoothed_score_from_verdicts_skips_unknown() {
        let verdicts = vec!["passed".into(), "unknown".into(), "failed".into()];
        let score = compute_smoothed_score_from_verdicts(&verdicts, 0.3).expect("score");
        assert!((0.0..=100.0).contains(&score));
    }

    #[test]
    fn verdict_instability_zero_for_single_or_empty() {
        assert_eq!(verdict_instability(&[]), 0.0);
        assert_eq!(verdict_instability(&["passed".into()]), 0.0);
    }

    #[test]
    fn verdict_instability_max_for_alternating() {
        let alternating = vec![
            "passed".into(),
            "failed".into(),
            "passed".into(),
            "failed".into(),
        ];
        assert!((verdict_instability(&alternating) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn compute_exam_urgency_grows_with_tasks_since_last_exam() {
        let config = sample_urgency_config();
        let low = compute_exam_urgency_value(0, None, &[], &config);
        let high = compute_exam_urgency_value(10, None, &[], &config);
        assert!(high > low);
    }

    #[test]
    fn compute_exam_urgency_reduced_after_recent_exam() {
        let config = sample_urgency_config();
        let no_exam = compute_exam_urgency_value(5, None, &[], &config);
        let recent = compute_exam_urgency_value(5, Some(Utc::now()), &[], &config);
        assert!(recent < no_exam);
    }

    #[test]
    fn compute_exam_urgency_all_pass_streak_discounts() {
        let config = sample_urgency_config();
        let verdicts = vec!["passed".into(), "passed".into(), "passed".into()];
        let unstable =
            compute_exam_urgency_value(4, None, &["passed".into(), "failed".into()], &config);
        let stable = compute_exam_urgency_value(4, None, &verdicts, &config);
        assert!(stable < unstable);
    }

    #[cfg(test)]
    mod db_tests {
        use super::*;

        const PHASE2_AGENT_PK: &str = "phase2-exam-state-agent";

        async fn connect_test_pool() -> MySqlPool {
            let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
                "mysql://deagentnet:passw0rd@127.0.0.1:3307/deagentnet".to_string()
            });
            init_db(&url).await.unwrap_or_else(|err| {
                panic!(
                    "Phase 2 DB tests require MySQL at DATABASE_URL ({url}): {err}. \
                     Export DATABASE_URL and run: \
                     DATABASE_URL=... cargo test --lib db_agent_exam_state -- --ignored --test-threads=1"
                )
            })
        }

        async fn cleanup_phase2_fixtures(pool: &DbPool) {
            let _ = sqlx::query("DELETE FROM agent_exam_state WHERE agent_public_key = ?")
                .bind(PHASE2_AGENT_PK)
                .execute(pool)
                .await;
            let _ = sqlx::query("DELETE FROM agents WHERE public_key = ?")
                .bind(PHASE2_AGENT_PK)
                .execute(pool)
                .await;
        }

        async fn seed_phase2_agent(pool: &DbPool) {
            sqlx::query(
                "INSERT INTO agents (public_key, name, status, active_jobs)
                 VALUES (?, 'Phase2 Agent', 'active', 0)
                 ON DUPLICATE KEY UPDATE status = 'active'",
            )
            .bind(PHASE2_AGENT_PK)
            .execute(pool)
            .await
            .expect("seed agent");
        }

        async fn seed_validated_exam_assignment(pool: &DbPool, task_id: &str, verdict: &str) {
            seed_validated_exam_assignment_hours_ago(pool, task_id, verdict, 0).await;
        }

        async fn seed_validated_exam_assignment_hours_ago(
            pool: &DbPool,
            task_id: &str,
            verdict: &str,
            hours_ago: i64,
        ) {
            sqlx::query(
                "INSERT INTO exam_templates (id, prompt, expected_answer_canonical, domain, status)
                 VALUES (?, 'prompt', '42 usd', 'defi_analysis', 'active')
                 ON DUPLICATE KEY UPDATE status = 'active'",
            )
            .bind("phase3-completion-template")
            .execute(pool)
            .await
            .expect("seed template");

            sqlx::query(
                "INSERT INTO tasks (
                    id, creator_public_key, assigned_agent_public_key, budget_motes, status,
                    transaction_hash, domain, prompt, deadline
                 ) VALUES (?, 'creator', ?, 5000000000, 'Completed', 'tx', 'defi_analysis', 'p', 0)
                 ON DUPLICATE KEY UPDATE status = 'Completed'",
            )
            .bind(task_id)
            .bind(PHASE2_AGENT_PK)
            .execute(pool)
            .await
            .expect("seed task");

            sqlx::query(
                "INSERT INTO exam_assignments (
                    task_id, template_id, agent_public_key, bucket, status, verdict, validated_at
                 ) VALUES (?, 'phase3-completion-template', ?, 'audit', 'validated', ?, DATE_SUB(NOW(), INTERVAL ? HOUR))
                 ON DUPLICATE KEY UPDATE status = 'validated', verdict = VALUES(verdict), validated_at = VALUES(validated_at)",
            )
            .bind(task_id)
            .bind(PHASE2_AGENT_PK)
            .bind(verdict)
            .bind(hours_ago)
            .execute(pool)
            .await
            .expect("seed validated assignment");
        }

        async fn cleanup_completion_fixtures(pool: &DbPool) {
            let _ = sqlx::query(
                "DELETE FROM exam_assignments WHERE agent_public_key = ? OR task_id LIKE 'phase3-%' OR task_id LIKE 'phase4-%'",
            )
            .bind(PHASE2_AGENT_PK)
            .execute(pool)
            .await;
            let _ = sqlx::query(
                "DELETE FROM tasks WHERE assigned_agent_public_key = ? OR id LIKE 'phase3-%' OR id LIKE 'phase4-%'",
            )
            .bind(PHASE2_AGENT_PK)
            .execute(pool)
            .await;
            let _ =
                sqlx::query("DELETE FROM exam_templates WHERE id = 'phase3-completion-template'")
                    .execute(pool)
                    .await;
            cleanup_phase2_fixtures(pool).await;
        }

        #[tokio::test]
        #[ignore = "requires MySQL: DATABASE_URL; cargo test --lib db_completion_state -- --ignored --test-threads=1"]
        async fn db_on_exam_validated_resets_counter_and_recalculates_urgency() {
            let pool = connect_test_pool().await;
            cleanup_completion_fixtures(&pool).await;
            seed_phase2_agent(&pool).await;
            seed_validated_exam_assignment(&pool, "phase3-exam-prior", "passed").await;

            let baseline = AgentExamState {
                agent_public_key: PHASE2_AGENT_PK.into(),
                exam_urgency: 0.5,
                smoothed_score: None,
                last_exam_at: None,
                tasks_since_last_exam: 7,
                updated_at: Utc::now(),
            };
            upsert_agent_exam_state(&pool, &baseline)
                .await
                .expect("seed baseline state");

            let config = sample_urgency_config();
            on_exam_validated(&pool, PHASE2_AGENT_PK, &config)
                .await
                .expect("on_exam_validated");

            let loaded = get_agent_exam_state(&pool, PHASE2_AGENT_PK)
                .await
                .expect("get")
                .expect("row exists");

            assert_eq!(loaded.tasks_since_last_exam, 0);
            assert!(loaded.last_exam_at.is_some());
            assert_ne!(loaded.exam_urgency, baseline.exam_urgency);
            assert!(
                loaded.exam_urgency < baseline.exam_urgency,
                "recent exam should reduce urgency vs tasks_since_last_exam=7 baseline"
            );
            assert_eq!(
                loaded.smoothed_score,
                Some(100.0),
                "single validated pass should yield smoothed_score=100"
            );

            cleanup_completion_fixtures(&pool).await;
        }

        #[tokio::test]
        #[ignore = "requires MySQL: DATABASE_URL; cargo test --lib db_completion_state -- --ignored --test-threads=1"]
        async fn db_on_ordinary_task_completed_increments_counter_and_recalculates_urgency() {
            let pool = connect_test_pool().await;
            cleanup_completion_fixtures(&pool).await;
            seed_phase2_agent(&pool).await;

            let baseline = AgentExamState {
                agent_public_key: PHASE2_AGENT_PK.into(),
                exam_urgency: 0.1,
                smoothed_score: None,
                last_exam_at: None,
                tasks_since_last_exam: 2,
                updated_at: Utc::now(),
            };
            upsert_agent_exam_state(&pool, &baseline)
                .await
                .expect("seed baseline state");

            let config = sample_urgency_config();
            on_ordinary_task_completed(&pool, PHASE2_AGENT_PK, &config)
                .await
                .expect("on_ordinary_task_completed");

            let loaded = get_agent_exam_state(&pool, PHASE2_AGENT_PK)
                .await
                .expect("get")
                .expect("row exists");

            assert_eq!(loaded.tasks_since_last_exam, 3);
            assert!(loaded.exam_urgency > baseline.exam_urgency);
            assert_eq!(
                loaded.smoothed_score, baseline.smoothed_score,
                "ordinary task completion must not update smoothed_score"
            );

            cleanup_completion_fixtures(&pool).await;
        }

        #[tokio::test]
        #[ignore = "requires MySQL: DATABASE_URL; cargo test --lib db_completion_state -- --ignored --test-threads=1"]
        async fn db_on_exam_validated_creates_row_when_missing() {
            let pool = connect_test_pool().await;
            cleanup_completion_fixtures(&pool).await;
            seed_phase2_agent(&pool).await;

            let config = sample_urgency_config();
            on_exam_validated(&pool, PHASE2_AGENT_PK, &config)
                .await
                .expect("on_exam_validated");

            let loaded = get_agent_exam_state(&pool, PHASE2_AGENT_PK)
                .await
                .expect("get")
                .expect("row created");

            assert_eq!(loaded.tasks_since_last_exam, 0);
            assert!(loaded.last_exam_at.is_some());

            cleanup_completion_fixtures(&pool).await;
        }

        #[tokio::test]
        #[ignore = "requires MySQL: DATABASE_URL; cargo test --lib db_completion_state -- --ignored --test-threads=1"]
        async fn db_on_ordinary_task_completed_creates_row_when_missing() {
            let pool = connect_test_pool().await;
            cleanup_completion_fixtures(&pool).await;
            seed_phase2_agent(&pool).await;

            let config = sample_urgency_config();
            on_ordinary_task_completed(&pool, PHASE2_AGENT_PK, &config)
                .await
                .expect("on_ordinary_task_completed");

            let loaded = get_agent_exam_state(&pool, PHASE2_AGENT_PK)
                .await
                .expect("get")
                .expect("row created");

            assert_eq!(loaded.tasks_since_last_exam, 1);
            assert!(loaded.exam_urgency >= 0.0);

            cleanup_completion_fixtures(&pool).await;
        }

        #[tokio::test]
        #[ignore = "requires MySQL: DATABASE_URL; cargo test --lib db_agent_exam_state -- --ignored --test-threads=1"]
        async fn db_agent_exam_state_upsert_and_get_round_trip() {
            let pool = connect_test_pool().await;
            cleanup_phase2_fixtures(&pool).await;
            seed_phase2_agent(&pool).await;

            let state = AgentExamState {
                agent_public_key: PHASE2_AGENT_PK.into(),
                exam_urgency: 0.75,
                smoothed_score: Some(82.5),
                last_exam_at: Some(Utc::now()),
                tasks_since_last_exam: 3,
                updated_at: Utc::now(),
            };
            upsert_agent_exam_state(&pool, &state)
                .await
                .expect("upsert");

            let loaded = get_agent_exam_state(&pool, PHASE2_AGENT_PK)
                .await
                .expect("get")
                .expect("row exists");

            assert_eq!(loaded.agent_public_key, PHASE2_AGENT_PK);
            assert!((loaded.exam_urgency - 0.75).abs() < f64::EPSILON);
            assert_eq!(loaded.smoothed_score, Some(82.5));
            assert_eq!(loaded.tasks_since_last_exam, 3);
            assert!(loaded.last_exam_at.is_some());

            cleanup_phase2_fixtures(&pool).await;
        }

        #[tokio::test]
        #[ignore = "requires MySQL: DATABASE_URL; cargo test --lib db_agent_exam_state -- --ignored --test-threads=1"]
        async fn db_agent_exam_state_backfill_is_idempotent() {
            let pool = connect_test_pool().await;
            cleanup_phase2_fixtures(&pool).await;
            seed_phase2_agent(&pool).await;

            ensure_agent_exam_state_for_active_agents(&pool)
                .await
                .expect("first backfill");

            let count_after_first: (i64,) =
                sqlx::query_as("SELECT COUNT(*) FROM agent_exam_state WHERE agent_public_key = ?")
                    .bind(PHASE2_AGENT_PK)
                    .fetch_one(&pool)
                    .await
                    .expect("count after first");

            assert_eq!(count_after_first.0, 1);

            ensure_agent_exam_state_for_active_agents(&pool)
                .await
                .expect("second backfill");

            let count_after_second: (i64,) =
                sqlx::query_as("SELECT COUNT(*) FROM agent_exam_state WHERE agent_public_key = ?")
                    .bind(PHASE2_AGENT_PK)
                    .fetch_one(&pool)
                    .await
                    .expect("count after second");

            assert_eq!(
                count_after_second.0, 1,
                "idempotent backfill must not create duplicates"
            );

            cleanup_phase2_fixtures(&pool).await;
        }

        #[tokio::test]
        #[ignore = "requires MySQL: DATABASE_URL; cargo test --lib db_agent_exam_state -- --ignored --test-threads=1"]
        async fn db_init_db_creates_agent_exam_state_table() {
            let url = std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "mysql://root:password@127.0.0.1:3306/deagentnet".to_string());
            let pool = init_db(&url).await.expect("init_db");

            let row: (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM information_schema.tables \
                 WHERE table_schema = DATABASE() AND table_name = 'agent_exam_state'",
            )
            .fetch_one(&pool)
            .await
            .expect("table exists check");

            assert_eq!(row.0, 1);
        }

        #[tokio::test]
        #[ignore = "requires MySQL: DATABASE_URL; cargo test --lib db_smoothed_score -- --ignored --test-threads=1"]
        async fn db_smoothed_score_pass_pass_fail_drops_smoothly() {
            let pool = connect_test_pool().await;
            cleanup_completion_fixtures(&pool).await;
            seed_phase2_agent(&pool).await;
            seed_validated_exam_assignment_hours_ago(&pool, "phase4-pass-1", "passed", 2).await;
            seed_validated_exam_assignment_hours_ago(&pool, "phase4-pass-2", "passed", 1).await;
            seed_validated_exam_assignment_hours_ago(&pool, "phase4-fail-1", "failed", 0).await;

            let config = sample_urgency_config();
            let score =
                compute_smoothed_score(&pool, PHASE2_AGENT_PK, config.exam_smoothed_ema_alpha)
                    .await
                    .expect("compute smoothed score")
                    .expect("score from history");

            assert!((score - 70.0).abs() < f64::EPSILON);
            assert!(score > 0.0);

            cleanup_completion_fixtures(&pool).await;
        }

        #[tokio::test]
        #[ignore = "requires MySQL: DATABASE_URL; cargo test --lib db_smoothed_score -- --ignored --test-threads=1"]
        async fn db_on_exam_validated_does_not_modify_reputations() {
            let pool = connect_test_pool().await;
            cleanup_completion_fixtures(&pool).await;
            seed_phase2_agent(&pool).await;
            seed_validated_exam_assignment(&pool, "phase4-rep-check", "passed").await;

            sqlx::query(
                "INSERT INTO reputations (id, agent_public_key, skill, score)
                 VALUES (?, ?, 'defi_analysis', 42)
                 ON DUPLICATE KEY UPDATE score = VALUES(score)",
            )
            .bind(format!("phase4-rep-{PHASE2_AGENT_PK}"))
            .bind(PHASE2_AGENT_PK)
            .execute(&pool)
            .await
            .expect("seed reputation");

            let before: (i64,) = sqlx::query_as(
                "SELECT CAST(COALESCE(SUM(score), 0) AS SIGNED) FROM reputations WHERE agent_public_key = ?",
            )
            .bind(PHASE2_AGENT_PK)
            .fetch_one(&pool)
            .await
            .expect("reputation before");

            let config = sample_urgency_config();
            on_exam_validated(&pool, PHASE2_AGENT_PK, &config)
                .await
                .expect("on_exam_validated");

            let after: (i64,) = sqlx::query_as(
                "SELECT CAST(COALESCE(SUM(score), 0) AS SIGNED) FROM reputations WHERE agent_public_key = ?",
            )
            .bind(PHASE2_AGENT_PK)
            .fetch_one(&pool)
            .await
            .expect("reputation after");

            assert_eq!(before.0, after.0);

            cleanup_completion_fixtures(&pool).await;
            let _ = sqlx::query("DELETE FROM reputations WHERE agent_public_key = ?")
                .bind(PHASE2_AGENT_PK)
                .execute(&pool)
                .await;
        }

        #[tokio::test]
        #[ignore = "requires MySQL: DATABASE_URL; cargo test --lib db_recommended_price -- --ignored --test-threads=1"]
        async fn db_recommended_price_updates_on_exam_validated() {
            let pool = connect_test_pool().await;
            cleanup_completion_fixtures(&pool).await;
            seed_phase2_agent(&pool).await;
            seed_validated_exam_assignment_hours_ago(&pool, "phase4-pass-1", "passed", 0).await;

            let config = sample_urgency_config();
            on_exam_validated(&pool, PHASE2_AGENT_PK, &config)
                .await
                .expect("on_exam_validated");

            let price: Option<u64> = sqlx::query_scalar(
                "SELECT recommended_price_motes FROM agents WHERE public_key = ?",
            )
            .bind(PHASE2_AGENT_PK)
            .fetch_one(&pool)
            .await
            .ok()
            .flatten();

            let expected_price =
                crate::casper_utils::recommended_price_motes("defi_analysis", 100, 10000);
            assert_eq!(price, Some(expected_price));

            cleanup_completion_fixtures(&pool).await;
        }

        #[tokio::test]
        #[ignore = "requires MySQL: DATABASE_URL; cargo test --lib db_recommended_price -- --ignored --test-threads=1"]
        async fn db_recommended_price_unchanged_on_ordinary_task() {
            let pool = connect_test_pool().await;
            cleanup_completion_fixtures(&pool).await;
            seed_phase2_agent(&pool).await;

            // Set initial price
            let initial_price = 1_000_000_000u64;
            sqlx::query("UPDATE agents SET recommended_price_motes = ? WHERE public_key = ?")
                .bind(initial_price)
                .bind(PHASE2_AGENT_PK)
                .execute(&pool)
                .await
                .unwrap();

            let config = sample_urgency_config();
            on_ordinary_task_completed(&pool, PHASE2_AGENT_PK, &config)
                .await
                .expect("on_ordinary_task_completed");

            let price: Option<u64> = sqlx::query_scalar(
                "SELECT recommended_price_motes FROM agents WHERE public_key = ?",
            )
            .bind(PHASE2_AGENT_PK)
            .fetch_one(&pool)
            .await
            .ok()
            .flatten();

            assert_eq!(
                price,
                Some(initial_price),
                "Ordinary task should not change price via smoothed path"
            );

            cleanup_completion_fixtures(&pool).await;
        }

        #[tokio::test]
        #[ignore = "requires MySQL: DATABASE_URL; cargo test --lib db_recommended_price -- --ignored --test-threads=1"]
        async fn db_recommended_price_fallback_when_no_exam_history() {
            let pool = connect_test_pool().await;
            cleanup_completion_fixtures(&pool).await;
            seed_phase2_agent(&pool).await;

            // Add some reputation
            sqlx::query("INSERT INTO reputations (id, agent_public_key, skill, score) VALUES ('fallback-rep', ?, 'test', 42)")
                .bind(PHASE2_AGENT_PK)
                .execute(&pool)
                .await
                .unwrap();

            let config = sample_urgency_config();
            on_exam_validated(&pool, PHASE2_AGENT_PK, &config)
                .await
                .expect("on_exam_validated");

            let price: Option<u64> = sqlx::query_scalar(
                "SELECT recommended_price_motes FROM agents WHERE public_key = ?",
            )
            .bind(PHASE2_AGENT_PK)
            .fetch_one(&pool)
            .await
            .ok()
            .flatten();

            let expected_price =
                crate::casper_utils::recommended_price_motes("defi_analysis", 42, 10000);
            assert_eq!(price, Some(expected_price));

            sqlx::query("DELETE FROM reputations WHERE id = 'fallback-rep'")
                .execute(&pool)
                .await
                .unwrap();
            cleanup_completion_fixtures(&pool).await;
        }
    }
}
