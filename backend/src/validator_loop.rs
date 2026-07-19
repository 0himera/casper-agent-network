//! Independent Validator Node Loop (Part B §2.2 Multi-Validator Consensus)

use std::time::Duration;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::{self, MissedTickBehavior};
use std::process::Command;

use crate::config::Config;
use crate::db::DbPool;
use crate::db::models::Task;

type StopSender = mpsc::Sender<()>;

/// Validator node loop configuration
#[derive(Debug, Clone)]
pub struct ValidatorNodeConfig {
    pub enabled: bool,
    pub poll_interval_secs: u64,
    pub validator_secret_key_path: Option<String>,
    pub validator_public_key: Option<String>,
    pub llm_provider: Option<String>,
    pub llm_model: Option<String>,
    pub min_validations: u32,
    pub validation_window_secs: u64,
}

impl Default for ValidatorNodeConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            poll_interval_secs: 15,
            validator_secret_key_path: None,
            validator_public_key: None,
            llm_provider: None,
            llm_model: None,
            min_validations: 3,
            validation_window_secs: 300,
        }
    }
}

impl ValidatorNodeConfig {
    pub fn from_env() -> Self {
        let enabled = std::env::var("VALIDATOR_ENABLED")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        
        let poll_interval_secs = std::env::var("VALIDATOR_POLL_INTERVAL_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(15);
            
        let validator_secret_key_path = std::env::var("VALIDATOR_SECRET_KEY_PATH")
            .ok()
            .filter(|v| !v.is_empty());

        let validator_public_key = std::env::var("VALIDATOR_PUBLIC_KEY")
            .ok()
            .filter(|v| !v.is_empty());
            
        let llm_provider = std::env::var("VALIDATOR_LLM_PROVIDER")
            .ok()
            .filter(|v| !v.is_empty());
            
        let llm_model = std::env::var("VALIDATOR_LLM_MODEL")
            .ok()
            .filter(|v| !v.is_empty());
            
        let min_validations = std::env::var("VALIDATOR_MIN_VALIDATIONS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(3);
            
        let validation_window_secs = std::env::var("VALIDATOR_WINDOW_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(300);

        Self {
            enabled,
            poll_interval_secs,
            validator_secret_key_path,
            validator_public_key,
            llm_provider,
            llm_model,
            min_validations,
            validation_window_secs,
        }
    }
}

/// Outcome of a single validator node evaluation tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatorTickOutcome {
    pub tasks_evaluated: usize,
    pub validations_submitted: usize,
    pub tasks_finalized: usize,
}

/// Runs a single iteration of the validator node loop.
pub async fn run_validator_iteration(
    pool: &DbPool,
    config: &Config,
    node_cfg: &ValidatorNodeConfig,
) -> Result<ValidatorTickOutcome, String> {
    let validator_pubkey = match &node_cfg.validator_public_key {
        Some(pk) => pk.clone(),
        None => {
            return Err("VALIDATOR_PUBLIC_KEY not set in configuration".to_string());
        }
    };

    tracing::debug!(
        poll_interval = node_cfg.poll_interval_secs,
        validator = %validator_pubkey,
        "validator loop tick"
    );

    // 1. Fetch InProgress tasks with result submitted that this validator hasn't evaluated yet
    let tasks: Vec<Task> = sqlx::query_as::<_, Task>(
        "SELECT t.* FROM tasks t \
         WHERE t.status = 'InProgress' \
           AND t.result_hash IS NOT NULL \
           AND t.result IS NOT NULL \
           AND NOT EXISTS ( \
               SELECT 1 FROM validations v \
               WHERE v.task_id = t.id \
                 AND v.validator_public_key = ? \
           )"
    )
    .bind(&validator_pubkey)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("DB query failed: {}", e))?;

    let mut outcome = ValidatorTickOutcome {
        tasks_evaluated: 0,
        validations_submitted: 0,
        tasks_finalized: 0,
    };

    for task in tasks {
        tracing::info!(task_id = %task.id, "Evaluating task");

        // 2. Evaluate with LLM judge
        let result_text = task.result.as_deref().unwrap_or("");
        let eval_res = crate::validator::evaluate_task(
            &task.domain,
            &task.prompt,
            result_text,
            1000, // Dummy processing time
            config
        ).await;

        let score = match eval_res {
            Ok(res) => res.total,
            Err(e) => {
                tracing::error!(task_id = %task.id, error = %e, "LLM judge evaluation failed");
                continue;
            }
        };

        outcome.tasks_evaluated += 1;
        let verdict = if score >= 70 { "pass" } else { "fail" };
        crate::metrics::record_validator_decision(verdict);

        // 3. Submit validation score on-chain via CLI
        let bin_path = if std::path::Path::new("/usr/local/bin/agent_network_submit_validation").exists() {
            "/usr/local/bin/agent_network_submit_validation"
        } else {
            "cargo"
        };

        let mut cmd = Command::new(bin_path);
        let score_str = score.to_string();
        if bin_path == "cargo" {
            cmd.args([
                "run",
                "--bin",
                "agent_network_submit_validation",
                "--features",
                "livenet",
                "--",
                &task.creator_public_key,
                &task.id,
                &score_str,
            ])
            .current_dir("../smart-contract");
        } else {
            cmd.args([
                &task.creator_public_key,
                &task.id,
                &score_str,
            ]);
        }

        if let Some(key_path) = &node_cfg.validator_secret_key_path {
            cmd.env("ODRA_CASPER_LIVENET_SECRET_KEY_PATH", key_path);
        }
        let contract_hash = std::env::var("CONTRACT_PACKAGE_HASH")
            .or_else(|_| std::env::var("CONTRACT_HASH"))
            .unwrap_or_default();
        cmd.env("CONTRACT_HASH", &contract_hash);

        let output = cmd.output();
        match output {
            Ok(out) if out.status.success() => {
                tracing::info!(task_id = %task.id, score = score, "On-chain validation submitted successfully");
                crate::metrics::record_onchain_tx("submit_validation");
                
                // Write validation record to DB
                sqlx::query(
                    "INSERT INTO validations (task_id, validator_public_key, score) VALUES (?, ?, ?)"
                )
                .bind(&task.id)
                .bind(&validator_pubkey)
                .bind(score as i32)
                .execute(pool)
                .await
                .map_err(|e| format!("Failed to record validation in DB: {}", e))?;

                outcome.validations_submitted += 1;
            }
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                let stdout = String::from_utf8_lossy(&out.stdout);
                tracing::error!(
                    task_id = %task.id,
                    stderr = %stderr,
                    stdout = %stdout,
                    "On-chain validation submission CLI failed"
                );
                continue;
            }
            Err(e) => {
                tracing::error!(task_id = %task.id, error = %e, "Failed to run validation CLI");
                continue;
            }
        }

        // 4. Check for quorum / window expiry -> finalize
        // Count all validations in DB for this task
        let val_count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM validations WHERE task_id = ?"
        )
        .bind(&task.id)
        .fetch_one(pool)
        .await
        .map_err(|e| format!("DB count failed: {}", e))?;

        let time_since_created = chrono::Utc::now().naive_utc() - task.timestamp.naive_utc();
        let window_expired = time_since_created.num_seconds() >= node_cfg.validation_window_secs as i64;
        let has_quorum = val_count.0 >= node_cfg.min_validations as i64;

        if has_quorum || (val_count.0 > 0 && window_expired) {
            tracing::info!(
                task_id = %task.id,
                validations_count = val_count.0,
                window_expired = window_expired,
                "Quorum met or window expired, finalizing task"
            );

            let finalize_bin_path = if std::path::Path::new("/usr/local/bin/agent_network_finalize_task").exists() {
                "/usr/local/bin/agent_network_finalize_task"
            } else {
                "cargo"
            };

            let mut fin_cmd = Command::new(finalize_bin_path);
            if finalize_bin_path == "cargo" {
                fin_cmd.args([
                    "run",
                    "--bin",
                    "agent_network_finalize_task",
                    "--features",
                    "livenet",
                    "--",
                    &task.creator_public_key,
                    &task.id,
                    &task.domain,
                ])
                .current_dir("../smart-contract");
            } else {
                fin_cmd.args([
                    &task.creator_public_key,
                    &task.id,
                    &task.domain,
                ]);
            }

            if let Some(key_path) = &node_cfg.validator_secret_key_path {
                fin_cmd.env("ODRA_CASPER_LIVENET_SECRET_KEY_PATH", key_path);
            }
            let contract_hash = std::env::var("CONTRACT_PACKAGE_HASH")
                .or_else(|_| std::env::var("CONTRACT_HASH"))
                .unwrap_or_default();
            fin_cmd.env("CONTRACT_HASH", &contract_hash);

            let fin_output = fin_cmd.output();
            match fin_output {
                Ok(out) if out.status.success() => {
                    tracing::info!(task_id = %task.id, "On-chain task finalization succeeded");
                    crate::metrics::record_onchain_tx("finalize");
                    let elapsed_seconds = time_since_created.num_seconds() as f64;
                    crate::metrics::record_task_lifecycle(elapsed_seconds);
                    
                    // Mark task as completed in DB
                    sqlx::query("UPDATE tasks SET status = 'Completed' WHERE id = ?")
                        .bind(&task.id)
                        .execute(pool)
                        .await
                        .map_err(|e| format!("Failed to update task status in DB: {}", e))?;

                    outcome.tasks_finalized += 1;
                }
                Ok(out) => {
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    tracing::error!(
                        task_id = %task.id,
                        stderr = %stderr,
                        stdout = %stdout,
                        "On-chain finalization CLI failed"
                    );
                }
                Err(e) => {
                    tracing::error!(task_id = %task.id, error = %e, "Failed to run finalization CLI");
                }
            }
        }
    }

    Ok(outcome)
}

/// Spawns the background validator node loop if enabled.
pub fn spawn_if_enabled(
    pool: DbPool,
    config: Config,
    node_cfg: ValidatorNodeConfig,
) -> Option<(StopSender, JoinHandle<()>)> {
    if !node_cfg.enabled {
        return None;
    }

    let interval_secs = node_cfg.poll_interval_secs;
    tracing::info!(
        interval_secs = interval_secs,
        validator = ?node_cfg.validator_public_key,
        "multi-validator background loop enabled"
    );

    let (stop_tx, mut stop_rx) = mpsc::channel::<()>(1);
    let handle = tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(interval_secs));
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if let Err(e) = run_validator_iteration(&pool, &config, &node_cfg).await {
                        tracing::error!(error = %e, "validator loop tick error");
                    }
                }
                maybe = stop_rx.recv() => {
                    if maybe.is_some() {
                        tracing::info!("validator loop stopped");
                        break;
                    }
                }
            }
        }
    });

    Some((stop_tx, handle))
}

/// Gracefully shuts down the validator node loop.
pub async fn shutdown(stop_tx: StopSender, handle: JoinHandle<()>, timeout: Duration) {
    let _ = stop_tx.send(()).await;
    if tokio::time::timeout(timeout, handle).await.is_err() {
        tracing::warn!("validator loop shutdown timed out");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_sane_defaults() {
        let cfg = ValidatorNodeConfig::default();
        assert!(cfg.enabled);
        assert_eq!(cfg.poll_interval_secs, 15);
        assert_eq!(cfg.min_validations, 3);
        assert_eq!(cfg.validation_window_secs, 300);
    }

    #[test]
    fn test_from_env_defaults() {
        temp_env::with_vars(
            [
                ("VALIDATOR_ENABLED", None::<&str>),
                ("VALIDATOR_POLL_INTERVAL_SECS", None::<&str>),
                ("VALIDATOR_SECRET_KEY_PATH", None::<&str>),
                ("VALIDATOR_PUBLIC_KEY", None::<&str>),
                ("VALIDATOR_LLM_PROVIDER", None::<&str>),
                ("VALIDATOR_LLM_MODEL", None::<&str>),
                ("VALIDATOR_MIN_VALIDATIONS", None::<&str>),
                ("VALIDATOR_WINDOW_SECS", None::<&str>),
            ],
            || {
                let cfg = ValidatorNodeConfig::from_env();
                assert!(!cfg.enabled); // default false in from_env
                assert_eq!(cfg.poll_interval_secs, 15);
                assert_eq!(cfg.min_validations, 3);
                assert_eq!(cfg.validation_window_secs, 300);
            }
        );
    }

    #[test]
    fn test_from_env_custom() {
        temp_env::with_vars(
            [
                ("VALIDATOR_ENABLED", Some("true")),
                ("VALIDATOR_POLL_INTERVAL_SECS", Some("30")),
                ("VALIDATOR_SECRET_KEY_PATH", Some("/keys/validator.pem")),
                ("VALIDATOR_PUBLIC_KEY", Some("010203")),
                ("VALIDATOR_LLM_PROVIDER", Some("openai")),
                ("VALIDATOR_LLM_MODEL", Some("gpt-4o")),
                ("VALIDATOR_MIN_VALIDATIONS", Some("5")),
                ("VALIDATOR_WINDOW_SECS", Some("600")),
            ],
            || {
                let cfg = ValidatorNodeConfig::from_env();
                assert!(cfg.enabled);
                assert_eq!(cfg.poll_interval_secs, 30);
                assert_eq!(cfg.validator_secret_key_path.as_deref(), Some("/keys/validator.pem"));
                assert_eq!(cfg.validator_public_key.as_deref(), Some("010203"));
                assert_eq!(cfg.llm_provider.as_deref(), Some("openai"));
                assert_eq!(cfg.llm_model.as_deref(), Some("gpt-4o"));
                assert_eq!(cfg.min_validations, 5);
                assert_eq!(cfg.validation_window_secs, 600);
            }
        );
    }

    #[tokio::test]
    async fn test_spawn_if_enabled_returns_none_when_disabled() {
        let pool = sqlx::mysql::MySqlPoolOptions::new()
            .connect_lazy("mysql://ignored:ignored@127.0.0.1:1/ignored")
            .expect("lazy pool");
        
        let config = crate::config::Config::from_env();
        let mut node_cfg = ValidatorNodeConfig::default();
        node_cfg.enabled = false;
        
        assert!(spawn_if_enabled(pool, config, node_cfg).is_none());
    }
}
