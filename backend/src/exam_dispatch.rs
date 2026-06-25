//! E4 admin exam dispatch: bucket policy, frequency cap, task + assignment creation.

use chrono::{Duration, Utc};
use rand::Rng;
use serde::Serialize;

use crate::config::Config;
use crate::db::DbPool;
use crate::db::exam::{
    DispatchCandidate, DispatchedExamTaskParams, count_recent_exam_assignments,
    insert_dispatched_exam_task, insert_exam_assignment, list_dispatch_candidates,
    pick_random_active_exam_template,
};

/// Dispatch bucket for agent selection policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bucket {
    Audit,
    Rehab,
}

impl Bucket {
    pub fn as_str(self) -> &'static str {
        match self {
            Bucket::Audit => "audit",
            Bucket::Rehab => "rehab",
        }
    }
}

/// Result of a single dispatch attempt (returned to admin endpoint).
#[derive(Clone, Debug, Serialize)]
pub struct DispatchOutcome {
    pub created: bool,
    pub task_id: Option<String>,
    pub agent_public_key: Option<String>,
    pub template_id: Option<String>,
    pub bucket: Option<String>,
    pub skip_reason: Option<String>,
}

impl DispatchOutcome {
    fn skipped(reason: &str) -> Self {
        Self {
            created: false,
            task_id: None,
            agent_public_key: None,
            template_id: None,
            bucket: None,
            skip_reason: Some(reason.to_string()),
        }
    }

    fn created(
        task_id: String,
        agent_public_key: String,
        template_id: String,
        bucket: Bucket,
    ) -> Self {
        Self {
            created: true,
            task_id: Some(task_id),
            agent_public_key: Some(agent_public_key),
            template_id: Some(template_id),
            bucket: Some(bucket.as_str().to_string()),
            skip_reason: None,
        }
    }
}

/// Classify an agent into audit or rehab bucket (or none).
pub fn classify_bucket(candidate: &DispatchCandidate, config: &Config) -> Option<Bucket> {
    if candidate.active_jobs >= config.exam_audit_active_jobs_threshold {
        return Some(Bucket::Audit);
    }
    if candidate.reputation_score <= i64::from(config.exam_rehab_score_threshold) {
        return Some(Bucket::Rehab);
    }
    Some(Bucket::Audit)
}

/// Probabilistic gate — probabilities must stay below 1.0 to avoid deterministic gaming.
pub fn passes_probability(bucket: Bucket, roll: f32, config: &Config) -> bool {
    let prob = match bucket {
        Bucket::Audit => config.exam_dispatch_prob_audit,
        Bucket::Rehab => config.exam_dispatch_prob_rehab,
    };
    if prob <= 0.0 {
        return false;
    }
    roll < prob.min(0.99)
}

/// Frequency cap: agent must have fewer than max assignments in the rolling window.
pub fn passes_frequency_cap(recent_count: i64, config: &Config) -> bool {
    recent_count < i64::from(config.exam_max_per_agent_per_period)
}

fn generate_task_id() -> String {
    format!(
        "exam-dispatch-{}-{}",
        Utc::now().timestamp_millis(),
        rand::thread_rng().gen_range(0..u32::MAX)
    )
}

struct EligibleCandidate {
    candidate: DispatchCandidate,
    bucket: Bucket,
}

/// Run one dispatch cycle: pick agent + template, create task and exam_assignment.
pub async fn dispatch_once(pool: &DbPool, config: &Config) -> Result<DispatchOutcome, String> {
    let template = match pick_random_active_exam_template(pool)
        .await
        .map_err(|e| e.to_string())?
    {
        Some(t) => t,
        None => return Ok(DispatchOutcome::skipped("no_active_templates")),
    };

    let candidates = list_dispatch_candidates(pool)
        .await
        .map_err(|e| e.to_string())?;

    if candidates.is_empty() {
        return Ok(DispatchOutcome::skipped("no_active_agents"));
    }

    let since =
        Utc::now() - Duration::hours(config.exam_dispatch_period_hours.try_into().unwrap_or(24));

    let mut eligible: Vec<EligibleCandidate> = Vec::new();
    let mut saw_cap_block = false;
    let mut saw_prob_block = false;

    for candidate in candidates {
        let Some(bucket) = classify_bucket(&candidate, config) else {
            continue;
        };
        let roll = rand::thread_rng().gen_range(0.0f32..1.0f32);
        if !passes_probability(bucket, roll, config) {
            saw_prob_block = true;
            continue;
        }
        let recent = count_recent_exam_assignments(pool, &candidate.public_key, since)
            .await
            .map_err(|e| e.to_string())?;
        if !passes_frequency_cap(recent, config) {
            saw_cap_block = true;
            continue;
        }
        eligible.push(EligibleCandidate { candidate, bucket });
    }

    if eligible.is_empty() {
        if saw_cap_block && !saw_prob_block {
            return Ok(DispatchOutcome::skipped("frequency_cap"));
        }
        return Ok(DispatchOutcome::skipped("no_eligible_agents"));
    }

    let chosen = &eligible[rand::thread_rng().gen_range(0..eligible.len())];
    let task_id = generate_task_id();
    let tx_hash = format!("exam-dispatch-{task_id}");

    insert_dispatched_exam_task(
        pool,
        DispatchedExamTaskParams {
            task_id: &task_id,
            creator_public_key: &config.exam_dispatch_creator_public_key,
            assigned_agent_public_key: &chosen.candidate.public_key,
            budget_motes: config.exam_dispatch_budget_motes,
            transaction_hash: &tx_hash,
            domain: &template.domain,
            prompt: &template.prompt,
        },
    )
    .await
    .map_err(|e| e.to_string())?;

    insert_exam_assignment(
        pool,
        &task_id,
        &template.id,
        &chosen.candidate.public_key,
        chosen.bucket.as_str(),
        "assigned",
    )
    .await
    .map_err(|e| e.to_string())?;

    tracing::info!(
        "exam_dispatch task_id={} agent={} template={} bucket={}",
        task_id,
        chosen.candidate.public_key,
        template.id,
        chosen.bucket.as_str()
    );

    Ok(DispatchOutcome::created(
        task_id,
        chosen.candidate.public_key.clone(),
        template.id,
        chosen.bucket,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ValidatorPipeline;

    fn sample_config() -> Config {
        Config {
            database_url: String::new(),
            port: 3000,
            openai_api_key: None,
            claude_api_key: None,
            ollama_url: None,
            ollama_model: None,
            cloudflare_account_id: None,
            cloudflare_api_token: None,
            fireworks_api_key: None,
            fireworks_model: None,
            validator_url: None,
            validator_api_key: None,
            validator_model: None,
            validator_provider: None,
            validator_pipeline: ValidatorPipeline::Legacy,
            admin_account: "admin-pk".into(),
            internal_service_key: None,
            exam_weight: 300,
            exam_dispatch_prob_audit: 0.2,
            exam_dispatch_prob_rehab: 0.5,
            exam_max_per_agent_per_period: 1,
            exam_dispatch_period_hours: 24,
            exam_rehab_score_threshold: 0,
            exam_audit_active_jobs_threshold: 2,
            exam_dispatch_budget_motes: 5_000_000_000,
            exam_dispatch_creator_public_key: "admin-pk".into(),
        }
    }

    fn candidate(reputation_score: i64, active_jobs: i32) -> DispatchCandidate {
        DispatchCandidate {
            public_key: "agent-1".into(),
            active_jobs,
            reputation_score,
        }
    }

    #[test]
    fn classify_bucket_rehab_for_low_reputation() {
        let config = sample_config();
        assert_eq!(
            classify_bucket(&candidate(-5, 0), &config),
            Some(Bucket::Rehab)
        );
    }

    #[test]
    fn classify_bucket_audit_for_high_reputation() {
        let config = sample_config();
        assert_eq!(
            classify_bucket(&candidate(10, 0), &config),
            Some(Bucket::Audit)
        );
    }

    #[test]
    fn classify_bucket_audit_for_high_active_jobs() {
        let config = sample_config();
        assert_eq!(
            classify_bucket(&candidate(0, 3), &config),
            Some(Bucket::Audit)
        );
        assert_eq!(
            classify_bucket(&candidate(-5, 3), &config),
            Some(Bucket::Audit)
        );
        assert_eq!(
            classify_bucket(&candidate(5, 3), &config),
            Some(Bucket::Audit)
        );
    }

    #[test]
    fn passes_probability_zero_never_passes() {
        let mut config = sample_config();
        config.exam_dispatch_prob_audit = 0.0;
        assert!(!passes_probability(Bucket::Audit, 0.0, &config));
        assert!(!passes_probability(Bucket::Audit, 0.5, &config));
    }

    #[test]
    fn passes_probability_rejects_one_point_zero() {
        let mut config = sample_config();
        config.exam_dispatch_prob_rehab = 1.0;
        assert!(!passes_probability(Bucket::Rehab, 0.99, &config));
    }

    #[test]
    fn passes_probability_accepts_high_but_sub_one() {
        let mut config = sample_config();
        config.exam_dispatch_prob_rehab = 0.99;
        assert!(passes_probability(Bucket::Rehab, 0.98, &config));
    }

    #[test]
    fn passes_frequency_cap_blocks_at_max() {
        let config = sample_config();
        assert!(passes_frequency_cap(0, &config));
        assert!(!passes_frequency_cap(1, &config));
    }

    #[cfg(test)]
    mod db_tests {
        use super::*;
        use crate::db::exam::get_exam_assignment_by_task_id;
        use crate::db::models::Task;
        use chrono::Utc;
        use sqlx::{MySqlPool, Row};

        const E4_AGENT_PK: &str = "e4-dispatch-agent";
        const E4_TEMPLATE_ID: &str = "e4-dispatch-template";

        async fn connect_test_pool() -> MySqlPool {
            let url = std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "mysql://root:password@127.0.0.1:3306/deagentnet".to_string());
            MySqlPool::connect(&url)
                .await
                .expect("connect test database")
        }

        async fn cleanup_e4_fixtures(pool: &DbPool, task_prefix: &str) {
            let _ = sqlx::query("DELETE FROM exam_assignments WHERE task_id LIKE ?")
                .bind(format!("{task_prefix}%"))
                .execute(pool)
                .await;
            let _ = sqlx::query("DELETE FROM tasks WHERE id LIKE ?")
                .bind(format!("{task_prefix}%"))
                .execute(pool)
                .await;
            let _ = sqlx::query("DELETE FROM exam_templates WHERE id = ?")
                .bind(E4_TEMPLATE_ID)
                .execute(pool)
                .await;
            let _ = sqlx::query("DELETE FROM agents WHERE public_key = ?")
                .bind(E4_AGENT_PK)
                .execute(pool)
                .await;
        }

        async fn seed_e4_fixtures(pool: &DbPool) {
            sqlx::query(
                "INSERT INTO agents (public_key, name, status, active_jobs)
                 VALUES (?, 'E4 Agent', 'active', 0)
                 ON DUPLICATE KEY UPDATE status = 'active', active_jobs = 0",
            )
            .bind(E4_AGENT_PK)
            .execute(pool)
            .await
            .expect("seed agent");

            sqlx::query(
                "INSERT INTO exam_templates (id, prompt, expected_answer_canonical, domain, status)
                 VALUES (?, 'Return strictly: ANSWER: 42 usd', '42 usd', 'defi_analysis', 'active')
                 ON DUPLICATE KEY UPDATE status = 'active'",
            )
            .bind(E4_TEMPLATE_ID)
            .execute(pool)
            .await
            .expect("seed template");
        }

        fn dispatch_config(prob: f32) -> Config {
            let mut config = sample_config();
            config.exam_dispatch_prob_audit = prob;
            config.exam_dispatch_prob_rehab = prob;
            config.exam_dispatch_creator_public_key = "e4-creator".into();
            config
        }

        async fn seed_agent_with_reputation(
            pool: &DbPool,
            public_key: &str,
            active_jobs: i32,
            reputation_score: i64,
        ) {
            sqlx::query(
                "INSERT INTO agents (public_key, name, status, active_jobs)
                 VALUES (?, ?, 'active', ?)
                 ON DUPLICATE KEY UPDATE status = 'active', active_jobs = VALUES(active_jobs)",
            )
            .bind(public_key)
            .bind(format!("Agent {public_key}"))
            .bind(active_jobs)
            .execute(pool)
            .await
            .expect("seed agent");

            let _ = sqlx::query("DELETE FROM reputations WHERE agent_public_key = ?")
                .bind(public_key)
                .execute(pool)
                .await;

            if reputation_score != 0 {
                sqlx::query(
                    "INSERT INTO reputations (id, agent_public_key, skill, score)
                     VALUES (?, ?, 'defi_analysis', ?)",
                )
                .bind(format!("rep-{public_key}"))
                .bind(public_key)
                .bind(reputation_score as i32)
                .execute(pool)
                .await
                .expect("seed reputation");
            }
        }

        async fn cleanup_agent(pool: &DbPool, public_key: &str, task_prefix: &str) {
            let _ = sqlx::query("DELETE FROM exam_assignments WHERE task_id LIKE ?")
                .bind(format!("{task_prefix}%"))
                .execute(pool)
                .await;
            let _ = sqlx::query("DELETE FROM tasks WHERE id LIKE ?")
                .bind(format!("{task_prefix}%"))
                .execute(pool)
                .await;
            let _ = sqlx::query("DELETE FROM reputations WHERE agent_public_key = ?")
                .bind(public_key)
                .execute(pool)
                .await;
            let _ = sqlx::query("DELETE FROM agents WHERE public_key = ?")
                .bind(public_key)
                .execute(pool)
                .await;
        }

        async fn save_and_deactivate_all_templates(pool: &DbPool) -> Vec<String> {
            let rows = sqlx::query("SELECT id FROM exam_templates WHERE status='active'")
                .fetch_all(pool)
                .await
                .unwrap_or_default();
            let ids: Vec<String> = rows.iter().map(|r| r.try_get("id").unwrap()).collect();
            let _ =
                sqlx::query("UPDATE exam_templates SET status='inactive' WHERE status='active'")
                    .execute(pool)
                    .await;
            ids
        }

        async fn restore_templates(pool: &DbPool, ids: &[String]) {
            for id in ids {
                let _ = sqlx::query("UPDATE exam_templates SET status='active' WHERE id=?")
                    .bind(id)
                    .execute(pool)
                    .await;
            }
        }

        async fn save_and_deactivate_other_agents(pool: &DbPool, keep: &str) -> Vec<String> {
            let rows = sqlx::query(
                "SELECT public_key FROM agents WHERE status='active' AND public_key != ?",
            )
            .bind(keep)
            .fetch_all(pool)
            .await
            .unwrap_or_default();
            let pks: Vec<String> = rows
                .iter()
                .map(|r| r.try_get("public_key").unwrap())
                .collect();
            let _ = sqlx::query(
                "UPDATE agents SET status='inactive' WHERE status='active' AND public_key != ?",
            )
            .bind(keep)
            .execute(pool)
            .await;
            pks
        }

        async fn restore_agents(pool: &DbPool, pks: &[String]) {
            for pk in pks {
                let _ = sqlx::query("UPDATE agents SET status='active' WHERE public_key=?")
                    .bind(pk)
                    .execute(pool)
                    .await;
            }
        }

        #[tokio::test]
        #[ignore = "requires MySQL: DATABASE_URL; cargo test --lib db_exam_dispatch -- --ignored --test-threads=1"]
        async fn db_exam_dispatch_creates_task_and_assignment() {
            let pool = connect_test_pool().await;
            cleanup_e4_fixtures(&pool, "exam-dispatch-e4-test").await;
            let saved_templates = save_and_deactivate_all_templates(&pool).await;
            let saved_agents = save_and_deactivate_other_agents(&pool, E4_AGENT_PK).await;
            seed_e4_fixtures(&pool).await;

            let config = dispatch_config(0.99);
            let outcome = dispatch_once(&pool, &config).await.expect("dispatch");

            assert!(
                outcome.created,
                "expected created, got {:?}",
                outcome.skip_reason
            );
            let task_id = outcome.task_id.expect("task_id");
            assert_eq!(outcome.agent_public_key.as_deref(), Some(E4_AGENT_PK));
            assert_eq!(outcome.template_id.as_deref(), Some(E4_TEMPLATE_ID));
            assert!(outcome.bucket.is_some());

            let task: Task = sqlx::query_as("SELECT * FROM tasks WHERE id = ?")
                .bind(&task_id)
                .fetch_one(&pool)
                .await
                .expect("task row");
            assert_eq!(task.status, "InProgress");
            assert_eq!(task.assigned_agent_public_key.as_deref(), Some(E4_AGENT_PK));
            assert!(!task.prompt.contains("42 usd") || task.prompt.contains("ANSWER:"));
            assert!(!task.prompt.contains("expected_answer"));

            let assignment = get_exam_assignment_by_task_id(&pool, &task_id)
                .await
                .expect("assignment query")
                .expect("assignment row");
            assert_eq!(assignment.agent_public_key, E4_AGENT_PK);
            assert_eq!(assignment.template_id, E4_TEMPLATE_ID);
            assert_eq!(assignment.status, "assigned");
            assert!(assignment.bucket == "audit" || assignment.bucket == "rehab");

            cleanup_e4_fixtures(&pool, "exam-dispatch-e4-test").await;
            restore_templates(&pool, &saved_templates).await;
            restore_agents(&pool, &saved_agents).await;
        }

        #[tokio::test]
        #[ignore = "requires MySQL: DATABASE_URL; cargo test --lib db_exam_dispatch -- --ignored --test-threads=1"]
        async fn db_exam_dispatch_skips_when_frequency_cap_reached() {
            let pool = connect_test_pool().await;
            cleanup_e4_fixtures(&pool, "exam-dispatch-e4-cap").await;
            let saved_templates = save_and_deactivate_all_templates(&pool).await;
            let saved_agents = save_and_deactivate_other_agents(&pool, E4_AGENT_PK).await;
            seed_e4_fixtures(&pool).await;

            let prior_task_id = "exam-dispatch-e4-cap-prior";
            sqlx::query(
                "INSERT INTO tasks (
                    id, creator_public_key, assigned_agent_public_key, budget_motes, status,
                    transaction_hash, domain, prompt, deadline
                 ) VALUES (?, 'e4-creator', ?, 5000000000, 'InProgress', 'tx-cap', 'defi_analysis', 'prompt', 0)",
            )
            .bind(prior_task_id)
            .bind(E4_AGENT_PK)
            .execute(&pool)
            .await
            .expect("prior task");

            sqlx::query(
                "INSERT INTO exam_assignments (task_id, template_id, agent_public_key, bucket, status, created_at)
                 VALUES (?, ?, ?, 'audit', 'assigned', ?)",
            )
            .bind(prior_task_id)
            .bind(E4_TEMPLATE_ID)
            .bind(E4_AGENT_PK)
            .bind(Utc::now())
            .execute(&pool)
            .await
            .expect("prior assignment");

            let config = dispatch_config(0.99);
            let outcome = dispatch_once(&pool, &config).await.expect("dispatch");

            assert!(!outcome.created);
            assert_eq!(outcome.skip_reason.as_deref(), Some("frequency_cap"));

            cleanup_e4_fixtures(&pool, "exam-dispatch-e4-cap").await;
            restore_templates(&pool, &saved_templates).await;
            restore_agents(&pool, &saved_agents).await;
        }

        #[tokio::test]
        #[ignore = "requires MySQL: DATABASE_URL; cargo test --lib db_exam_dispatch -- --ignored --test-threads=1"]
        async fn db_exam_dispatch_skips_when_no_templates() {
            const EMPTY_AGENT: &str = "e4-empty-agent";
            let pool = connect_test_pool().await;
            cleanup_e4_fixtures(&pool, "exam-dispatch-e4-empty").await;
            let saved_templates = save_and_deactivate_all_templates(&pool).await;

            sqlx::query(
                "INSERT INTO agents (public_key, name, status)
                 VALUES (?, 'Empty Agent', 'active')
                 ON DUPLICATE KEY UPDATE status = 'active'",
            )
            .bind(EMPTY_AGENT)
            .execute(&pool)
            .await
            .expect("seed agent");

            let config = dispatch_config(0.99);
            let outcome = dispatch_once(&pool, &config).await.expect("dispatch");

            assert!(!outcome.created);
            assert_eq!(outcome.skip_reason.as_deref(), Some("no_active_templates"));

            let _ = sqlx::query("DELETE FROM agents WHERE public_key = ?")
                .bind(EMPTY_AGENT)
                .execute(&pool)
                .await;
            restore_templates(&pool, &saved_templates).await;
        }

        #[tokio::test]
        #[ignore = "requires MySQL: DATABASE_URL; cargo test --lib db_exam_dispatch -- --ignored --test-threads=1"]
        async fn db_exam_dispatch_skips_when_prob_zero() {
            let pool = connect_test_pool().await;
            cleanup_e4_fixtures(&pool, "exam-dispatch-e4-prob0").await;
            seed_e4_fixtures(&pool).await;

            let config = dispatch_config(0.0);
            let outcome = dispatch_once(&pool, &config).await.expect("dispatch");

            assert!(!outcome.created);
            assert_eq!(outcome.skip_reason.as_deref(), Some("no_eligible_agents"));

            cleanup_e4_fixtures(&pool, "exam-dispatch-e4-prob0").await;
        }

        #[tokio::test]
        #[ignore = "requires MySQL: DATABASE_URL; cargo test --lib db_exam_dispatch -- --ignored --test-threads=1"]
        async fn db_exam_dispatch_low_rep_assigns_rehab_bucket() {
            const LOW_REP_AGENT: &str = "e4-dispatch-low-rep";
            let pool = connect_test_pool().await;
            cleanup_agent(&pool, LOW_REP_AGENT, "exam-dispatch-e4-rehab").await;
            cleanup_e4_fixtures(&pool, "exam-dispatch-e4-rehab").await;
            let saved_templates = save_and_deactivate_all_templates(&pool).await;
            let saved_agents = save_and_deactivate_other_agents(&pool, LOW_REP_AGENT).await;

            seed_agent_with_reputation(&pool, LOW_REP_AGENT, 0, -5).await;
            sqlx::query(
                "INSERT INTO exam_templates (id, prompt, expected_answer_canonical, domain, status)
                 VALUES (?, 'Return strictly: ANSWER: 42 usd', '42 usd', 'defi_analysis', 'active')
                 ON DUPLICATE KEY UPDATE status = 'active'",
            )
            .bind(E4_TEMPLATE_ID)
            .execute(&pool)
            .await
            .expect("seed template");

            let config = dispatch_config(0.99);
            let outcome = dispatch_once(&pool, &config).await.expect("dispatch");

            assert!(
                outcome.created,
                "expected created: {:?}",
                outcome.skip_reason
            );
            assert_eq!(outcome.bucket.as_deref(), Some("rehab"));

            cleanup_e4_fixtures(&pool, "exam-dispatch-e4-rehab").await;
            cleanup_agent(&pool, LOW_REP_AGENT, "exam-dispatch-e4-rehab").await;
            restore_templates(&pool, &saved_templates).await;
            restore_agents(&pool, &saved_agents).await;
        }

        #[tokio::test]
        #[ignore = "requires MySQL: DATABASE_URL; cargo test --lib db_exam_dispatch -- --ignored --test-threads=1"]
        async fn db_exam_dispatch_high_rep_assigns_audit_bucket() {
            const HIGH_REP_AGENT: &str = "e4-dispatch-high-rep";
            let pool = connect_test_pool().await;
            cleanup_agent(&pool, HIGH_REP_AGENT, "exam-dispatch-e4-audit").await;
            cleanup_e4_fixtures(&pool, "exam-dispatch-e4-audit").await;
            let saved_templates = save_and_deactivate_all_templates(&pool).await;
            let saved_agents = save_and_deactivate_other_agents(&pool, HIGH_REP_AGENT).await;

            seed_agent_with_reputation(&pool, HIGH_REP_AGENT, 0, 50).await;
            sqlx::query(
                "INSERT INTO exam_templates (id, prompt, expected_answer_canonical, domain, status)
                 VALUES (?, 'Return strictly: ANSWER: 42 usd', '42 usd', 'defi_analysis', 'active')
                 ON DUPLICATE KEY UPDATE status = 'active'",
            )
            .bind(E4_TEMPLATE_ID)
            .execute(&pool)
            .await
            .expect("seed template");

            let config = dispatch_config(0.99);
            let outcome = dispatch_once(&pool, &config).await.expect("dispatch");

            assert!(
                outcome.created,
                "expected created: {:?}",
                outcome.skip_reason
            );
            assert_eq!(outcome.bucket.as_deref(), Some("audit"));

            cleanup_e4_fixtures(&pool, "exam-dispatch-e4-audit").await;
            cleanup_agent(&pool, HIGH_REP_AGENT, "exam-dispatch-e4-audit").await;
            restore_templates(&pool, &saved_templates).await;
            restore_agents(&pool, &saved_agents).await;
        }
    }
}
