//! E7 background exam dispatch loop (middle scenario Phase 1).

use std::time::Duration;

use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::{self, MissedTickBehavior};

use crate::config::Config;
use crate::db::DbPool;
use crate::exam_dispatch::{self, DispatchOutcome};

type StopSender = mpsc::Sender<()>;

/// Logs one loop iteration outcome (structured; no secrets).
pub fn log_dispatch_outcome(outcome: &DispatchOutcome) {
    if outcome.created {
        tracing::info!(
            outcome = "created",
            task_id = outcome.task_id.as_deref().unwrap_or(""),
            agent_public_key = outcome.agent_public_key.as_deref().unwrap_or(""),
            template_id = outcome.template_id.as_deref().unwrap_or(""),
            bucket = outcome.bucket.as_deref().unwrap_or(""),
            "exam dispatch loop iteration"
        );
    } else {
        tracing::info!(
            outcome = "skipped",
            skip_reason = outcome.skip_reason.as_deref().unwrap_or("unknown"),
            "exam dispatch loop iteration"
        );
    }
}

/// Logs a dispatch error; loop continues on the next tick.
pub fn log_dispatch_error(error: &str) {
    tracing::error!(outcome = "error", error = %error, "exam dispatch loop iteration");
}

/// Runs one background loop iteration: `dispatch_once` plus structured logging.
pub async fn run_iteration(pool: &DbPool, config: &Config) {
    match exam_dispatch::dispatch_once(pool, config).await {
        Ok(outcome) => log_dispatch_outcome(&outcome),
        Err(e) => log_dispatch_error(&e),
    }
}

/// Spawns the opt-in background loop when enabled in config.
/// Returns stop channel and join handle, or `None` when loop is disabled.
pub fn spawn_if_enabled(pool: DbPool, config: Config) -> Option<(StopSender, JoinHandle<()>)> {
    if !config.exam_dispatch_loop_enabled {
        return None;
    }

    let interval_secs = config.exam_dispatch_loop_interval_secs;
    tracing::info!(
        interval_secs = interval_secs,
        "exam dispatch background loop enabled"
    );

    let (stop_tx, mut stop_rx) = mpsc::channel::<()>(1);
    let handle = tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(interval_secs));
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    run_iteration(&pool, &config).await;
                }
                maybe = stop_rx.recv() => {
                    if maybe.is_some() {
                        tracing::info!("exam dispatch loop stopped");
                        break;
                    }
                }
            }
        }
    });

    Some((stop_tx, handle))
}

/// Signals the loop to stop and waits up to `timeout` for clean exit.
pub async fn shutdown(stop_tx: StopSender, handle: JoinHandle<()>, timeout: Duration) {
    let _ = stop_tx.send(()).await;
    tokio::select! {
        _ = handle => {
            tracing::debug!("exam dispatch loop joined cleanly");
        }
        _ = time::sleep(timeout) => {
            tracing::warn!("exam dispatch loop shutdown timed out");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, ExamSelectionMode, ValidatorPipeline};

    fn minimal_disabled_config() -> Config {
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
            admin_account: String::new(),
            internal_service_key: None,
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
            exam_selection_mode: ExamSelectionMode::Bucket,
            exam_urgency_base_prob: 0.1,
            exam_urgency_task_weight: 0.05,
            exam_urgency_variance_weight: 0.2,
            exam_urgency_recent_verdicts: 5,
            exam_smoothed_ema_alpha: 0.3,
            exam_leaderboard_use_smoothed: false,
        }
    }

    #[tokio::test]
    async fn spawn_if_enabled_returns_none_when_loop_disabled() {
        let pool = sqlx::mysql::MySqlPoolOptions::new()
            .connect_lazy("mysql://ignored:ignored@127.0.0.1:1/ignored")
            .expect("lazy pool");
        let config = minimal_disabled_config();
        assert!(spawn_if_enabled(pool, config).is_none());
    }

    #[test]
    fn log_dispatch_outcome_created_does_not_panic() {
        let outcome = DispatchOutcome {
            created: true,
            task_id: Some("task-1".to_string()),
            agent_public_key: Some("agent-1".to_string()),
            template_id: Some("tpl-1".to_string()),
            bucket: Some("audit".to_string()),
            skip_reason: None,
        };
        log_dispatch_outcome(&outcome);
    }

    #[test]
    fn log_dispatch_outcome_skipped_does_not_panic() {
        let outcome = DispatchOutcome {
            created: false,
            task_id: None,
            agent_public_key: None,
            template_id: None,
            bucket: None,
            skip_reason: Some("no_active_templates".to_string()),
        };
        log_dispatch_outcome(&outcome);
    }
}
