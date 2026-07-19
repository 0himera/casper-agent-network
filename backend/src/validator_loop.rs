//! Independent Validator Node Loop (Part B §2.2 Multi-Validator Consensus)

use std::time::Duration;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::{self, MissedTickBehavior};

use crate::config::Config;
use crate::db::DbPool;

type StopSender = mpsc::Sender<()>;

/// Validator node loop configuration
#[derive(Debug, Clone)]
pub struct ValidatorNodeConfig {
    pub enabled: bool,
    pub poll_interval_secs: u64,
    pub validator_secret_key_path: Option<String>,
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
            llm_provider: None,
            llm_model: None,
            min_validations: 3,
            validation_window_secs: 300,
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
    tracing::debug!(
        poll_interval = node_cfg.poll_interval_secs,
        provider = node_cfg.llm_provider.as_deref().unwrap_or("default"),
        "validator loop tick"
    );

    // 1. Fetch submitted tasks from DB that need validation
    // 2. Evaluate with judge LLM
    // 3. Submit validation score on-chain
    // 4. Check for quorum / window expiry -> finalize
    
    Ok(ValidatorTickOutcome {
        tasks_evaluated: 0,
        validations_submitted: 0,
        tasks_finalized: 0,
    })
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
        provider = node_cfg.llm_provider.as_deref().unwrap_or("default"),
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
}
