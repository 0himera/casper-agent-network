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

/// Timeout for a single decay CLI invocation (seconds). Override with `DECAY_CLI_TIMEOUT_SECS`.
fn decay_cli_timeout() -> Duration {
    std::env::var("DECAY_CLI_TIMEOUT_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .map(Duration::from_secs)
        .unwrap_or(Duration::from_secs(120))
}

fn resolve_decay_bin() -> String {
    if let Ok(p) = std::env::var("DECAY_CLI_BIN").map(|v| v.trim().to_string()) {
        if !p.is_empty() {
            return p;
        }
    }
    if std::path::Path::new("/usr/local/bin/agent_network_decay_reputation").exists() {
        "/usr/local/bin/agent_network_decay_reputation".to_string()
    } else {
        "cargo".to_string()
    }
}

/// Run one CLI invocation with a hard timeout so a hanging binary cannot block the runtime forever.
fn run_decay_cli_with_timeout(
    mut cmd: Command,
    timeout: Duration,
) -> Result<std::process::Output, String> {
    use std::io::Read;
    let mut child = cmd
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn decay CLI: {}", e))?;

    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_status)) => {
                let mut stdout = Vec::new();
                let mut stderr = Vec::new();
                if let Some(mut out) = child.stdout.take() {
                    let _ = out.read_to_end(&mut stdout);
                }
                if let Some(mut err) = child.stderr.take() {
                    let _ = err.read_to_end(&mut stderr);
                }
                return Ok(std::process::Output {
                    status: _status,
                    stdout,
                    stderr,
                });
            }
            Ok(None) => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!(
                        "decay CLI timed out after {}s",
                        timeout.as_secs()
                    ));
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(e) => return Err(format!("Failed to wait for decay CLI: {}", e)),
        }
    }
}

pub async fn run_decay_iteration(pool: &DbPool, _config: &Config) -> Result<(), String> {
    tracing::debug!("reputation decay loop tick");

    // 1. Get all reputations
    let reps: Vec<(String, String)> =
        sqlx::query_as::<_, (String, String)>("SELECT agent_public_key, skill FROM reputations")
            .fetch_all(pool)
            .await
            .map_err(|e| format!("DB query failed: {}", e))?;

    let timeout = decay_cli_timeout();

    for (agent_pk, skill) in reps {
        tracing::info!(agent = %agent_pk, skill = %skill, "Running decay check");

        let bin_path = resolve_decay_bin();

        let mut cmd = Command::new(&bin_path);
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

        // Blocking CLI runs on a worker thread so the async runtime stays responsive.
        let agent_pk_log = agent_pk.clone();
        let skill_log = skill.clone();
        let cli_result = tokio::task::spawn_blocking(move || {
            run_decay_cli_with_timeout(cmd, timeout)
        })
        .await
        .map_err(|e| format!("decay CLI join error: {}", e))?;

        match cli_result {
            Ok(out) if out.status.success() => {
                tracing::info!(agent = %agent_pk_log, skill = %skill_log, "Reputation decay processed successfully");
            }
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                tracing::warn!(
                    agent = %agent_pk_log,
                    skill = %skill_log,
                    stderr = %stderr,
                    "Reputation decay CLI completed with non-zero status"
                );
            }
            Err(e) => {
                tracing::error!(agent = %agent_pk_log, skill = %skill_log, error = %e, "Failed to run decay CLI");
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
    fn test_decay_past_time_returns_same() {
        let (ws, tw) = calculate_decay(1000, 10, 100, 50); // now_ms < last_update_ms
        assert_eq!(ws, 1000);
        assert_eq!(tw, 10);
    }

    #[test]
    fn test_decay_zero_weight_returns_same() {
        let (ws, tw) = calculate_decay(1000, 0, 100, 200); // total_weight == 0
        assert_eq!(ws, 1000);
        assert_eq!(tw, 0);
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

    async fn connect_test_pool() -> Option<DbPool> {
        let url = std::env::var("DATABASE_URL").ok()?;
        if url.is_empty() {
            return None;
        }
        sqlx::MySqlPool::connect(&url).await.ok()
    }

    /// Wave 4 scenario 14: empty reputations → Ok, no CLI needed.
    #[tokio::test]
    #[ignore]
    async fn test_w4_decay_empty_db_is_ok() {
        let pool = match connect_test_pool().await {
            Some(p) => p,
            None => {
                println!("skip: DATABASE_URL unset");
                return;
            }
        };
        // Isolate: only check that empty query path returns Ok.
        // Delete only our fixture keys if any; empty table is ideal but may share DB.
        let _ = sqlx::query("DELETE FROM reputations WHERE agent_public_key LIKE 'w4-decay-%'")
            .execute(&pool)
            .await;
        let config = crate::config::Config::from_env();
        // Point CLI at a failing stub so any unexpected invocation would error loudly in logs,
        // but empty set means CLI is never called.
        temp_env::async_with_vars(
            [
                ("DECAY_CLI_BIN", Some("/bin/false")),
                ("DECAY_CLI_TIMEOUT_SECS", Some("2")),
            ],
            async {
                let res = run_decay_iteration(&pool, &config).await;
                assert!(res.is_ok(), "empty decay tick must be Ok: {:?}", res);
                println!("[PASS] scenario 14: empty reputations tick is Ok");
            },
        )
        .await;
    }

    /// Wave 4 scenario 15: missing binary / missing key → warn path, iteration still Ok.
    #[tokio::test]
    #[ignore]
    async fn test_w4_decay_missing_binary_survives() {
        let pool = match connect_test_pool().await {
            Some(p) => p,
            None => return,
        };
        let agent = "w4-decay-agent-missing-bin";
        let _ = sqlx::query("DELETE FROM reputations WHERE agent_public_key = ?")
            .bind(agent)
            .execute(&pool)
            .await;
        let _ = sqlx::query(
            "INSERT INTO agents (public_key, name, status) VALUES (?, 'W4 Decay', 'active')
             ON DUPLICATE KEY UPDATE status='active'",
        )
        .bind(agent)
        .execute(&pool)
        .await;
        sqlx::query(
            "INSERT INTO reputations (id, agent_public_key, skill, score)
             VALUES ('w4-decay-rep-1', ?, 'defi_analysis', 50)
             ON DUPLICATE KEY UPDATE score=50",
        )
        .bind(agent)
        .execute(&pool)
        .await
        .expect("seed rep");

        let config = crate::config::Config::from_env();
        temp_env::async_with_vars(
            [
                ("DECAY_CLI_BIN", Some("/nonexistent/agent_network_decay_reputation")),
                ("DECAY_CLI_TIMEOUT_SECS", Some("2")),
                ("ADMIN_SECRET_KEY_PATH", None::<&str>),
            ],
            async {
                let res = run_decay_iteration(&pool, &config).await;
                assert!(
                    res.is_ok(),
                    "missing binary must not kill the iteration: {:?}",
                    res
                );
                println!("[PASS] scenario 15: missing decay binary → Ok with error log");
            },
        )
        .await;

        let _ = sqlx::query("DELETE FROM reputations WHERE agent_public_key = ?")
            .bind(agent)
            .execute(&pool)
            .await;
        let _ = sqlx::query("DELETE FROM agents WHERE public_key = ?")
            .bind(agent)
            .execute(&pool)
            .await;
    }

    /// Wave 4 scenario 16: hanging CLI is killed by timeout; iteration returns Ok.
    #[tokio::test]
    #[ignore]
    async fn test_w4_decay_hanging_cli_times_out() {
        let pool = match connect_test_pool().await {
            Some(p) => p,
            None => return,
        };
        let agent = "w4-decay-agent-hang";
        let _ = sqlx::query("DELETE FROM reputations WHERE agent_public_key = ?")
            .bind(agent)
            .execute(&pool)
            .await;
        let _ = sqlx::query(
            "INSERT INTO agents (public_key, name, status) VALUES (?, 'W4 Hang', 'active')
             ON DUPLICATE KEY UPDATE status='active'",
        )
        .bind(agent)
        .execute(&pool)
        .await;
        sqlx::query(
            "INSERT INTO reputations (id, agent_public_key, skill, score)
             VALUES ('w4-decay-rep-hang', ?, 'defi_analysis', 40)
             ON DUPLICATE KEY UPDATE score=40",
        )
        .bind(agent)
        .execute(&pool)
        .await
        .expect("seed");

        let hang_script = std::env::temp_dir().join("w4_decay_hang.sh");
        std::fs::write(&hang_script, "#!/bin/sh\nexec sleep 60\n").expect("write hang script");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&hang_script).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&hang_script, perms).unwrap();
        }
        let hang_path = hang_script.to_string_lossy().to_string();

        let config = crate::config::Config::from_env();
        let start = std::time::Instant::now();
        temp_env::async_with_vars(
            [
                ("DECAY_CLI_BIN", Some(hang_path.as_str())),
                ("DECAY_CLI_TIMEOUT_SECS", Some("1")),
            ],
            async {
                let res = run_decay_iteration(&pool, &config).await;
                assert!(res.is_ok(), "timeout path must return Ok: {:?}", res);
                assert!(
                    start.elapsed() < std::time::Duration::from_secs(10),
                    "must not block forever"
                );
                println!("[PASS] scenario 16: hanging CLI timed out; runtime not blocked forever");
            },
        )
        .await;

        let _ = sqlx::query("DELETE FROM reputations WHERE agent_public_key = ?")
            .bind(agent)
            .execute(&pool)
            .await;
        let _ = sqlx::query("DELETE FROM agents WHERE public_key = ?")
            .bind(agent)
            .execute(&pool)
            .await;
    }
}
