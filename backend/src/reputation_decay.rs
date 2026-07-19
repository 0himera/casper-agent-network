//! Time-Weighted Reputation Decay Module (§2.3)

pub const HALF_LIFE_MS: u64 = 30 * 86_400 * 1000; // 30 days in milliseconds

/// Calculates decayed reputation values based on exponential half-life decay.
/// `decay_ratio = 0.5 ^ ((now_ms - last_update_ms) / HALF_LIFE_MS)`
pub fn calculate_decay(
    weighted_sum: u64,
    total_weight: u64,
    last_update_ms: u64,
    now_ms: u64,
) -> (u64, u64) {
    if now_ms <= last_update_ms || total_weight == 0 {
        return (weighted_sum, total_weight);
    }

    let elapsed_ms = now_ms - last_update_ms;
    let elapsed_periods = elapsed_ms as f64 / HALF_LIFE_MS as f64;
    let decay_ratio = 0.5_f64.powf(elapsed_periods);

    let decayed_weighted_sum = (weighted_sum as f64 * decay_ratio).round() as u64;
    let decayed_total_weight = (total_weight as f64 * decay_ratio).round() as u64;

    (decayed_weighted_sum, decayed_total_weight)
}

use crate::config::Config;
use crate::db::DbPool;
use std::process::Command;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::{self, MissedTickBehavior};

type StopSender = mpsc::Sender<()>;

pub async fn run_decay_iteration(pool: &DbPool, _config: &Config) -> Result<(), String> {
    tracing::debug!("reputation decay loop tick");

    // 1. Get all reputations
    let reps: Vec<(String, String)> =
        sqlx::query_as::<_, (String, String)>("SELECT agent_public_key, skill FROM reputations")
            .fetch_all(pool)
            .await
            .map_err(|e| format!("DB query failed: {}", e))?;

    for (agent_pk, skill) in reps {
        tracing::info!(agent = %agent_pk, skill = %skill, "Running decay check");

        let bin_path =
            if std::path::Path::new("/usr/local/bin/agent_network_decay_reputation").exists() {
                "/usr/local/bin/agent_network_decay_reputation"
            } else {
                "cargo"
            };

        let mut cmd = Command::new(bin_path);
        if bin_path == "cargo" {
            cmd.args([
                "run",
                "--bin",
                "agent_network_decay_reputation",
                "--features",
                "livenet",
                "--",
                &agent_pk,
                &skill,
            ])
            .current_dir("../smart-contract");
        } else {
            cmd.args([&agent_pk, &skill]);
        }

        // Pass CONTRACT_HASH
        let contract_hash = std::env::var("CONTRACT_PACKAGE_HASH")
            .or_else(|_| std::env::var("CONTRACT_HASH"))
            .unwrap_or_default();
        cmd.env("CONTRACT_HASH", &contract_hash);

        // Pass admin private key path (the admin is the one running this sync)
        if let Ok(key_path) = std::env::var("ADMIN_SECRET_KEY_PATH")
            .or_else(|_| std::env::var("ODRA_CASPER_LIVENET_SECRET_KEY_PATH"))
        {
            cmd.env("ODRA_CASPER_LIVENET_SECRET_KEY_PATH", key_path);
        }

        let output = cmd.output();
        match output {
            Ok(out) if out.status.success() => {
                tracing::info!(agent = %agent_pk, skill = %skill, "Reputation decay processed successfully");
            }
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                tracing::warn!(
                    agent = %agent_pk,
                    skill = %skill,
                    stderr = %stderr,
                    "Reputation decay CLI completed with non-zero status"
                );
            }
            Err(e) => {
                tracing::error!(agent = %agent_pk, skill = %skill, error = %e, "Failed to run decay CLI");
            }
        }
    }

    Ok(())
}

pub fn spawn_decay_loop_if_enabled(
    pool: DbPool,
    config: Config,
) -> Option<(StopSender, JoinHandle<()>)> {
    let enabled = std::env::var("REPUTATION_DECAY_LOOP_ENABLED")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    if !enabled {
        return None;
    }

    let interval_secs = std::env::var("REPUTATION_DECAY_POLL_INTERVAL_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(86400); // Once a day by default

    tracing::info!(
        interval_secs = interval_secs,
        "reputation decay background loop enabled"
    );

    let (stop_tx, mut stop_rx) = mpsc::channel::<()>(1);
    let handle = tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(interval_secs));
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if let Err(e) = run_decay_iteration(&pool, &config).await {
                        tracing::error!(error = %e, "reputation decay loop tick error");
                    }
                }
                maybe = stop_rx.recv() => {
                    if maybe.is_some() {
                        tracing::info!("reputation decay loop stopped");
                        break;
                    }
                }
            }
        }
    });

    Some((stop_tx, handle))
}

pub async fn shutdown(stop_tx: StopSender, handle: JoinHandle<()>, timeout: Duration) {
    let _ = stop_tx.send(()).await;
    if tokio::time::timeout(timeout, handle).await.is_err() {
        tracing::warn!("reputation decay loop shutdown timed out");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero_elapsed_returns_same() {
        let (ws, tw) = calculate_decay(1000, 10, 100, 100);
        assert_eq!(ws, 1000);
        assert_eq!(tw, 10);
    }

    #[test]
    fn test_half_life_decay_halves_values() {
        let now = 100 + HALF_LIFE_MS;
        let (ws, tw) = calculate_decay(1000, 10, 100, now);
        assert_eq!(ws, 500);
        assert_eq!(tw, 5);
    }

    #[test]
    fn test_two_half_lives_quarters_values() {
        let now = 100 + HALF_LIFE_MS * 2;
        let (ws, tw) = calculate_decay(1000, 20, 100, now);
        assert_eq!(ws, 250);
        assert_eq!(tw, 5);
    }

    #[tokio::test]
    async fn test_spawn_decay_loop_returns_none_when_disabled() {
        let pool = sqlx::mysql::MySqlPoolOptions::new()
            .connect_lazy("mysql://ignored:ignored@127.0.0.1:1/ignored")
            .expect("lazy pool");
        let config = crate::config::Config::from_env();

        temp_env::async_with_vars([("REPUTATION_DECAY_LOOP_ENABLED", Some("false"))], async {
            assert!(spawn_decay_loop_if_enabled(pool, config).is_none());
        })
        .await;
    }
}
