use std::process::Command;
use agentnet_core::casper_utils::public_key_to_account_hash;
use agentnet_core::db::models::Task;
use agentnet_core::db::DbPool;
use agentnet_core::metrics;
use tokio_util::sync::CancellationToken;

use crate::config::ValidatorNodeConfig;

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
        let bin_path = if std::path::Path::new("/usr/local/bin/agent_network_submit_validation").exists() {
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

        let output = cmd.output();
        match output {
            Ok(out) if out.status.success() => {
                tracing::info!(task_id = %task.id, score = score, "On-chain validation submitted successfully");
                metrics::record_onchain_tx("submit_validation");

                // Record validation record in DB
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

            let fin_output = fin_cmd.output();
            match fin_output {
                Ok(out) if out.status.success() => {
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
