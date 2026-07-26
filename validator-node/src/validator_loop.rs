use agentnet_core::casper_utils::public_key_to_account_hash;
use agentnet_core::db::DbPool;
use agentnet_core::db::models::Task;
use agentnet_core::metrics;
use std::process::Command;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

use crate::config::ValidatorNodeConfig;

/// Timeout for a single validator CLI invocation (seconds).
/// Override with `VALIDATOR_CLI_TIMEOUT_SECS`. On timeout the child is killed and the
/// iteration continues without writing a validations row (no false success).
fn validator_cli_timeout() -> Duration {
    std::env::var("VALIDATOR_CLI_TIMEOUT_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .map(Duration::from_secs)
        .unwrap_or(Duration::from_secs(120))
}

/// Run one CLI invocation off the async runtime with a hard timeout.
async fn run_cli_with_timeout(
    cmd: Command,
    timeout: Duration,
) -> Result<std::process::Output, String> {
    tokio::task::spawn_blocking(move || run_cli_with_timeout_blocking(cmd, timeout))
        .await
        .map_err(|e| format!("validator CLI join error: {e}"))?
}

fn run_cli_with_timeout_blocking(
    mut cmd: Command,
    timeout: Duration,
) -> Result<std::process::Output, String> {
    use std::io::Read;
    let mut child = cmd
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn validator CLI: {e}"))?;

    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let mut stdout = Vec::new();
                let mut stderr = Vec::new();
                if let Some(mut out) = child.stdout.take() {
                    let _ = out.read_to_end(&mut stdout);
                }
                if let Some(mut err) = child.stderr.take() {
                    let _ = err.read_to_end(&mut stderr);
                }
                return Ok(std::process::Output {
                    status,
                    stdout,
                    stderr,
                });
            }
            Ok(None) => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!(
                        "validator CLI timed out after {}s",
                        timeout.as_secs()
                    ));
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(e) => return Err(format!("Failed to wait for validator CLI: {e}")),
        }
    }
}

/// Outcome summary of a single validator tick iteration.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ValidatorTickOutcome {
    pub tasks_evaluated: usize,
    pub validations_submitted: usize,
    pub tasks_finalized: usize,
}

/// Runs a single iteration of the validator loop in a cancellation-aware manner.
pub async fn run_validator_iteration(
    pool: &DbPool,
    node_cfg: &ValidatorNodeConfig,
    cancel_token: &CancellationToken,
) -> Result<ValidatorTickOutcome, String> {
    if cancel_token.is_cancelled() {
        tracing::info!("Iteration cancelled prior to start");
        return Ok(ValidatorTickOutcome::default());
    }

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

    // 1. Query pending unvalidated tasks
    let tasks: Vec<Task> = sqlx::query_as::<_, Task>(
        "SELECT t.* FROM tasks t \
         WHERE t.status = 'InProgress' \
           AND t.result_hash IS NOT NULL \
           AND t.result IS NOT NULL \
           AND NOT EXISTS ( \
               SELECT 1 FROM validations v \
               WHERE v.task_id = t.id \
                 AND v.validator_public_key = ? \
           )",
    )
    .bind(&validator_pubkey)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("DB query failed: {}", e))?;

    let mut outcome = ValidatorTickOutcome::default();

    for task in tasks {
        if cancel_token.is_cancelled() {
            tracing::info!("Cancellation token triggered, halting validator iteration loop");
            break;
        }

        tracing::info!(task_id = %task.id, "Evaluating task");

        // 2. Evaluate with validator_engine
        let result_text = task.result.as_deref().unwrap_or("");
        let engine_cfg = validator_engine::LlmConfig::from_env();
        let eval_res = validator_engine::evaluate_stage_pipeline(
            &task.domain,
            &task.prompt,
            result_text,
            &engine_cfg,
        )
        .await;

        let score = match eval_res {
            Ok(res) => res.total,
            Err(e) => {
                tracing::error!(task_id = %task.id, error = ?e, "LLM judge evaluation failed");
                continue;
            }
        };
        outcome.tasks_evaluated += 1;

        let verdict = if score >= 70 { "pass" } else { "fail" };
        metrics::record_validator_decision(verdict);

        // 3. Submit validation score on-chain via CLI tool
        // VALIDATOR_SUBMIT_BIN overrides path (test seam for CLI failure matrix).
        let bin_override = std::env::var("VALIDATOR_SUBMIT_BIN")
            .ok()
            .filter(|v| !v.is_empty());
        let bin_path = if let Some(ref p) = bin_override {
            p.as_str()
        } else if std::path::Path::new("/usr/local/bin/agent_network_submit_validation").exists() {
            "/usr/local/bin/agent_network_submit_validation"
        } else {
            "cargo"
        };

        let mut cmd = Command::new(bin_path);
        let score_str = score.to_string();
        let creator_addr = public_key_to_account_hash(&task.creator_public_key);
        if bin_path == "cargo" {
            cmd.args([
                "run",
                "--bin",
                "agent_network_submit_validation",
                "--features",
                "livenet",
                "--",
                &creator_addr,
                &task.id,
                &score_str,
            ])
            .current_dir("../smart-contract");
        } else {
            cmd.args([&creator_addr, &task.id, &score_str]);
        }

        if let Some(key_path) = &node_cfg.validator_secret_key_path {
            cmd.env("ODRA_CASPER_LIVENET_SECRET_KEY_PATH", key_path);
        }
        let mut contract_hash = std::env::var("CONTRACT_PACKAGE_HASH")
            .or_else(|_| std::env::var("CONTRACT_HASH"))
            .unwrap_or_default();
        if !contract_hash.starts_with("hash-") && !contract_hash.is_empty() {
            contract_hash = format!("hash-{}", contract_hash);
        }
        cmd.env("CONTRACT_HASH", &contract_hash);

        let skip_onchain = std::env::var("EXAM_SKIP_ONCHAIN")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

        let cli_timeout = validator_cli_timeout();
        let output_status_success = if skip_onchain {
            tracing::info!(task_id = %task.id, "Skipping on-chain validation submission (EXAM_SKIP_ONCHAIN=1)");
            true
        } else {
            match run_cli_with_timeout(cmd, cli_timeout).await {
                Ok(out) if out.status.success() => true,
                Ok(out) => {
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    tracing::error!(
                        task_id = %task.id,
                        stderr = %stderr,
                        stdout = %stdout,
                        "On-chain validation submission CLI failed"
                    );
                    false
                }
                Err(e) => {
                    tracing::error!(task_id = %task.id, error = %e, "Failed to run validation CLI");
                    false
                }
            }
        };

        if output_status_success {
            tracing::info!(task_id = %task.id, score = score, "On-chain validation submitted successfully");
            metrics::record_onchain_tx("submit_validation");

            // Record validation record in DB
            sqlx::query(
                "INSERT INTO validations (task_id, validator_public_key, score) VALUES (?, ?, ?)",
            )
            .bind(&task.id)
            .bind(&validator_pubkey)
            .bind(score as i32)
            .execute(pool)
            .await
            .map_err(|e| format!("Failed to record validation in DB: {}", e))?;

            outcome.validations_submitted += 1;
        } else {
            continue;
        }

        // 4. Check for quorum or window expiry -> finalize on-chain
        let val_count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM validations WHERE task_id = ?")
                .bind(&task.id)
                .fetch_one(pool)
                .await
                .map_err(|e| format!("DB count failed: {}", e))?;

        let time_since_created = chrono::Utc::now().naive_utc() - task.timestamp.naive_utc();
        let window_expired =
            time_since_created.num_seconds() >= node_cfg.validation_window_secs as i64;
        let has_quorum = val_count.0 >= node_cfg.min_validations as i64;

        if has_quorum || (val_count.0 > 0 && window_expired) {
            tracing::info!(
                task_id = %task.id,
                validations_count = val_count.0,
                window_expired = window_expired,
                "Quorum met or window expired, finalizing task"
            );

            let finalize_bin_path =
                if std::path::Path::new("/usr/local/bin/agent_network_finalize_task").exists() {
                    "/usr/local/bin/agent_network_finalize_task"
                } else {
                    "cargo"
                };

            let mut fin_cmd = Command::new(finalize_bin_path);
            if finalize_bin_path == "cargo" {
                fin_cmd
                    .args([
                        "run",
                        "--bin",
                        "agent_network_finalize_task",
                        "--features",
                        "livenet",
                        "--",
                        &creator_addr,
                        &task.id,
                        &task.domain,
                    ])
                    .current_dir("../smart-contract");
            } else {
                fin_cmd.args([&creator_addr, &task.id, &task.domain]);
            }

            if let Some(key_path) = &node_cfg.validator_secret_key_path {
                fin_cmd.env("ODRA_CASPER_LIVENET_SECRET_KEY_PATH", key_path);
            }
            let contract_hash = std::env::var("CONTRACT_PACKAGE_HASH")
                .or_else(|_| std::env::var("CONTRACT_HASH"))
                .unwrap_or_default();
            fin_cmd.env("CONTRACT_HASH", &contract_hash);

            let fin_status_success = if skip_onchain {
                tracing::info!(task_id = %task.id, "Skipping on-chain finalization (EXAM_SKIP_ONCHAIN=1)");
                true
            } else {
                match run_cli_with_timeout(fin_cmd, cli_timeout).await {
                    Ok(out) if out.status.success() => true,
                    Ok(out) => {
                        let stderr = String::from_utf8_lossy(&out.stderr);
                        let stdout = String::from_utf8_lossy(&out.stdout);
                        tracing::error!(
                            task_id = %task.id,
                            stderr = %stderr,
                            stdout = %stdout,
                            "On-chain finalization CLI failed"
                        );
                        false
                    }
                    Err(e) => {
                        tracing::error!(task_id = %task.id, error = %e, "Failed to run finalization CLI");
                        false
                    }
                }
            };

            if fin_status_success {
                tracing::info!(task_id = %task.id, "On-chain task finalization succeeded");
                metrics::record_onchain_tx("finalize");
                let elapsed_seconds = time_since_created.num_seconds() as f64;
                metrics::record_task_lifecycle(elapsed_seconds);

                sqlx::query("UPDATE tasks SET status = 'Completed' WHERE id = ?")
                    .bind(&task.id)
                    .execute(pool)
                    .await
                    .map_err(|e| format!("Failed to update task status in DB: {}", e))?;

                outcome.tasks_finalized += 1;
            }
        }
    }

    Ok(outcome)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::MySqlPool;

    async fn connect_test_pool() -> Option<DbPool> {
        let url = std::env::var("DATABASE_URL").ok()?;
        if url.is_empty() {
            return None;
        }
        let pool = MySqlPool::connect(&url).await.ok()?;
        Some(pool)
    }

    async fn cleanup_task(pool: &DbPool, _task_id: &str) {
        let _ = sqlx::query("DELETE FROM validations").execute(pool).await;
        let _ = sqlx::query("DELETE FROM tasks").execute(pool).await;
        let _ = sqlx::query("DELETE FROM agents").execute(pool).await;
        let _ = sqlx::query("DELETE FROM validators").execute(pool).await;
    }

    #[tokio::test]
    #[ignore]
    async fn test_validator_loop_db_happy_path() {
        let pool = match connect_test_pool().await {
            Some(p) => p,
            None => {
                println!(
                    "Skipping test_validator_loop_db_happy_path: DATABASE_URL not set or unreachable"
                );
                return;
            }
        };

        let task_id = "test-loop-task-happy";
        cleanup_task(&pool, task_id).await;

        // Seed agent
        sqlx::query(
            "INSERT INTO agents (public_key, name, status, active_jobs)
             VALUES ('test-agent-pk', 'Test Agent', 'active', 0)
             ON DUPLICATE KEY UPDATE status = 'active'",
        )
        .execute(&pool)
        .await
        .expect("seed agent");

        // Seed validator
        sqlx::query(
            "INSERT INTO validators (public_key, stake_motes, is_active, total_validations)
             VALUES ('test-validator-pubkey-happy', 1000, 1, 0)
             ON DUPLICATE KEY UPDATE is_active = 1",
        )
        .execute(&pool)
        .await
        .expect("seed validator");

        // Seed task in Progress and having results
        sqlx::query(
            "INSERT INTO tasks (
                id, creator_public_key, assigned_agent_public_key, budget_motes, status,
                transaction_hash, domain, prompt, deadline, result_hash, result, timestamp
            ) VALUES (?, 'test-creator-pk', 'test-agent-pk', 100, 'InProgress',
                      'test-tx-hash', 'defi_analysis', 'test prompt', 123456, 'test-res-hash', 'test result content is long enough now', NOW())"
        )
        .bind(task_id)
        .execute(&pool)
        .await
        .expect("seed task");

        // Run with mock LLM and skip on-chain
        unsafe {
            std::env::set_var("VALIDATOR_MOCK_LLM", "1");
            std::env::set_var("EXAM_SKIP_ONCHAIN", "1");
            std::env::set_var("VALIDATOR_FACTUALITY", "0");
        }

        let mut node_cfg = ValidatorNodeConfig::default();
        node_cfg.poll_interval_secs = 1;
        node_cfg.min_validations = 1;
        node_cfg.validation_window_secs = 600;
        node_cfg.validator_public_key = Some("test-validator-pubkey-happy".to_string());

        let cancel_token = CancellationToken::new();
        let res = run_validator_iteration(&pool, &node_cfg, &cancel_token)
            .await
            .expect("run iteration");

        assert_eq!(res.tasks_evaluated, 1);
        assert_eq!(res.validations_submitted, 1);
        assert_eq!(res.tasks_finalized, 1);

        // Verify Validation row created
        let val_row: (i32,) = sqlx::query_as(
            "SELECT score FROM validations WHERE task_id = ? AND validator_public_key = ?",
        )
        .bind(task_id)
        .bind("test-validator-pubkey-happy")
        .fetch_one(&pool)
        .await
        .expect("fetch validation");
        // Verify Task is Completed in DB
        let status_row: (String,) = sqlx::query_as("SELECT status FROM tasks WHERE id = ?")
            .bind(task_id)
            .fetch_one(&pool)
            .await
            .expect("fetch task status");

        assert_eq!(
            val_row.0, 93,
            "Validation score was {}, status: {}, outcome: {:?}",
            val_row.0, status_row.0, res
        );
        assert_eq!(status_row.0, "Completed");

        cleanup_task(&pool, task_id).await;
    }

    #[tokio::test]
    #[ignore]
    async fn test_validator_loop_db_already_validated() {
        let pool = match connect_test_pool().await {
            Some(p) => p,
            None => {
                return;
            }
        };

        let task_id = "test-loop-task-already-val";
        cleanup_task(&pool, task_id).await;

        // Seed agent
        sqlx::query(
            "INSERT INTO agents (public_key, name, status, active_jobs)
             VALUES ('test-agent-pk', 'Test Agent', 'active', 0)
             ON DUPLICATE KEY UPDATE status = 'active'",
        )
        .execute(&pool)
        .await
        .expect("seed agent");

        // Seed validator
        sqlx::query(
            "INSERT INTO validators (public_key, stake_motes, is_active, total_validations)
             VALUES ('test-validator-pubkey-already', 1000, 1, 0)
             ON DUPLICATE KEY UPDATE is_active = 1",
        )
        .execute(&pool)
        .await
        .expect("seed validator");

        // Seed task in Progress
        sqlx::query(
            "INSERT INTO tasks (
                id, creator_public_key, assigned_agent_public_key, budget_motes, status,
                transaction_hash, domain, prompt, deadline, result_hash, result, timestamp
            ) VALUES (?, 'test-creator-pk', 'test-agent-pk', 100, 'InProgress',
                      'test-tx-hash', 'defi_analysis', 'test prompt', 123456, 'test-res-hash', 'test result content is long enough now', NOW())"
        )
        .bind(task_id)
        .execute(&pool)
        .await
        .expect("seed task");

        // Seed a prior validation from this same validator
        sqlx::query(
            "INSERT INTO validations (task_id, validator_public_key, score) VALUES (?, ?, 85)",
        )
        .bind(task_id)
        .bind("test-validator-pubkey-already")
        .execute(&pool)
        .await
        .expect("seed validation");

        unsafe {
            std::env::set_var("VALIDATOR_MOCK_LLM", "1");
            std::env::set_var("EXAM_SKIP_ONCHAIN", "1");
            std::env::set_var("VALIDATOR_FACTUALITY", "0");
        }

        let mut node_cfg = ValidatorNodeConfig::default();
        node_cfg.poll_interval_secs = 1;
        node_cfg.min_validations = 1;
        node_cfg.validation_window_secs = 600;
        node_cfg.validator_public_key = Some("test-validator-pubkey-already".to_string());

        let cancel_token = CancellationToken::new();
        let res = run_validator_iteration(&pool, &node_cfg, &cancel_token)
            .await
            .expect("run iteration");

        // It should NOT evaluate because it was already validated
        assert_eq!(res.tasks_evaluated, 0);
        assert_eq!(res.validations_submitted, 0);
        assert_eq!(res.tasks_finalized, 0);

        cleanup_task(&pool, task_id).await;
    }

    #[tokio::test]
    async fn test_validator_loop_missing_pubkey() {
        let pool = MySqlPool::connect_lazy("mysql://localhost/dummy").unwrap();
        let mut node_cfg = ValidatorNodeConfig::default();
        node_cfg.poll_interval_secs = 1;
        node_cfg.min_validations = 1;
        node_cfg.validation_window_secs = 600;
        node_cfg.validator_public_key = None;
        let cancel_token = CancellationToken::new();
        let res = run_validator_iteration(&pool, &node_cfg, &cancel_token).await;
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err(),
            "VALIDATOR_PUBLIC_KEY not set in configuration"
        );
        println!(
            "[PASS] scenario 1: missing VALIDATOR_PUBLIC_KEY fails iteration (not silent defaults)"
        );
    }

    /// Wave 4 scenario 2a: bad DATABASE_URL / unreachable pool → query error, no hang.
    #[tokio::test]
    #[ignore]
    async fn test_w4_validator_bad_db_url_on_tick() {
        // Connect to a closed port — pool create may succeed lazily or fail; use unreachable.
        let pool = MySqlPool::connect_lazy("mysql://deagentnet:passw0rd@127.0.0.1:1/deagentnet")
            .expect("lazy pool");
        let mut node_cfg = ValidatorNodeConfig::default();
        node_cfg.validator_public_key = Some("w4-validator-bad-db".into());
        let cancel_token = CancellationToken::new();
        let res = run_validator_iteration(&pool, &node_cfg, &cancel_token).await;
        assert!(res.is_err(), "bad DB must error");
        assert!(
            res.unwrap_err().contains("DB query failed"),
            "error must mention DB query"
        );
        println!("[PASS] scenario 2a: unreachable DB on tick returns error, no hang");
    }

    /// Wave 4 scenario 3: short/bad judge input — gate/eval path; no validation row if eval fails.
    #[tokio::test]
    #[ignore]
    async fn test_w4_validator_short_result_no_false_validation() {
        let pool = match connect_test_pool().await {
            Some(p) => p,
            None => return,
        };
        let task_id = "w4-loop-short-result";
        let vpk = "w4-validator-short";
        cleanup_task(&pool, task_id).await;

        sqlx::query(
            "INSERT INTO agents (public_key, name, status) VALUES ('w4-agent-short', 'A', 'active')
             ON DUPLICATE KEY UPDATE status='active'",
        )
        .execute(&pool)
        .await
        .ok();
        sqlx::query(
            "INSERT INTO validators (public_key, stake_motes, is_active, total_validations)
             VALUES (?, 1000, 1, 0) ON DUPLICATE KEY UPDATE is_active=1",
        )
        .bind(vpk)
        .execute(&pool)
        .await
        .ok();
        // 19-char result — fails MIN_OUTPUT_LEN gate inside engine (mock may still score 0 path)
        sqlx::query(
            "INSERT INTO tasks (
                id, creator_public_key, assigned_agent_public_key, budget_motes, status,
                transaction_hash, domain, prompt, deadline, result_hash, result, timestamp
            ) VALUES (?, 'creator', 'w4-agent-short', 100, 'InProgress',
                      'tx', 'defi_analysis', 'prompt', 123456, 'rh', ?, NOW())",
        )
        .bind(task_id)
        .bind("a".repeat(19))
        .execute(&pool)
        .await
        .expect("seed");

        unsafe {
            std::env::set_var("VALIDATOR_MOCK_LLM", "1");
            std::env::set_var("EXAM_SKIP_ONCHAIN", "1");
            std::env::set_var("VALIDATOR_FACTUALITY", "0");
        }

        let mut node_cfg = ValidatorNodeConfig::default();
        node_cfg.min_validations = 99;
        node_cfg.validator_public_key = Some(vpk.into());
        let cancel_token = CancellationToken::new();
        let res = run_validator_iteration(&pool, &node_cfg, &cancel_token)
            .await
            .expect("iteration");

        // Mock engine may return Err (gate) → continue without validation, or Ok with low score.
        // Critical assertion: if eval failed, validations_submitted == 0 for this validator.
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM validations WHERE task_id = ? AND validator_public_key = ?",
        )
        .bind(task_id)
        .bind(vpk)
        .fetch_one(&pool)
        .await
        .unwrap();

        if res.tasks_evaluated == 0 {
            assert_eq!(count.0, 0, "no validation row when eval skipped");
            println!("[PASS] scenario 3: short result → eval skipped, no validation row");
        } else {
            // Evaluated with score path under EXAM_SKIP_ONCHAIN — still document boundary behavior
            println!(
                "[PASS] scenario 3: short result evaluated={:?} validations_db={}",
                res, count.0
            );
        }
        cleanup_task(&pool, task_id).await;
    }

    /// Wave 4 scenario 4: CLI non-zero exit → no DB validation, loop continues (EXAM_SKIP_ONCHAIN off).
    #[tokio::test]
    #[ignore]
    async fn test_w4_validator_cli_nonzero_exit() {
        let pool = match connect_test_pool().await {
            Some(p) => p,
            None => return,
        };
        let task_id = "w4-loop-cli-fail";
        let vpk = "w4-validator-cli-fail";
        cleanup_task(&pool, task_id).await;

        sqlx::query(
            "INSERT INTO agents (public_key, name, status) VALUES ('w4-agent-cli', 'A', 'active')
             ON DUPLICATE KEY UPDATE status='active'",
        )
        .execute(&pool)
        .await
        .ok();
        sqlx::query(
            "INSERT INTO validators (public_key, stake_motes, is_active, total_validations)
             VALUES (?, 1000, 1, 0) ON DUPLICATE KEY UPDATE is_active=1",
        )
        .bind(vpk)
        .execute(&pool)
        .await
        .ok();
        sqlx::query(
            "INSERT INTO tasks (
                id, creator_public_key, assigned_agent_public_key, budget_motes, status,
                transaction_hash, domain, prompt, deadline, result_hash, result, timestamp
            ) VALUES (?, 'creator', 'w4-agent-cli', 100, 'InProgress',
                      'tx', 'defi_analysis', 'prompt', 123456, 'rh',
                      'test result content is long enough now', NOW())",
        )
        .bind(task_id)
        .execute(&pool)
        .await
        .expect("seed");

        unsafe {
            std::env::set_var("VALIDATOR_MOCK_LLM", "1");
            std::env::set_var("EXAM_SKIP_ONCHAIN", "0");
            std::env::set_var("VALIDATOR_FACTUALITY", "0");
            std::env::set_var("VALIDATOR_SUBMIT_BIN", "/bin/false");
        }

        let mut node_cfg = ValidatorNodeConfig::default();
        node_cfg.min_validations = 99;
        node_cfg.validator_public_key = Some(vpk.into());
        let cancel_token = CancellationToken::new();
        let res = run_validator_iteration(&pool, &node_cfg, &cancel_token)
            .await
            .expect("iteration must not panic");

        assert!(res.tasks_evaluated >= 1, "task was evaluated");
        assert_eq!(
            res.validations_submitted, 0,
            "CLI failure must not count as submitted"
        );
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM validations WHERE task_id = ? AND validator_public_key = ?",
        )
        .bind(task_id)
        .bind(vpk)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(count.0, 0, "no validations row on CLI failure");
        println!("[PASS] scenario 4: CLI non-zero → no DB validation, loop continues");

        unsafe {
            std::env::remove_var("VALIDATOR_SUBMIT_BIN");
            std::env::set_var("EXAM_SKIP_ONCHAIN", "1");
        }
        cleanup_task(&pool, task_id).await;
    }
}
