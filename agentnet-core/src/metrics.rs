//! Metrics recording utilities for agentnet-core.

pub fn record_validator_decision(verdict: &str) {
    metrics::counter!("validator_decisions_total", "verdict" => verdict.to_string()).increment(1);
    tracing::debug!(verdict = verdict, "validator_decision recorded");
}

pub fn record_onchain_tx(action: &str) {
    metrics::counter!("onchain_tx_total", "action" => action.to_string()).increment(1);
    tracing::debug!(action = action, "onchain_tx recorded");
}

pub fn record_task_lifecycle(elapsed_seconds: f64) {
    metrics::histogram!("task_lifecycle_seconds").record(elapsed_seconds);
    tracing::debug!(elapsed_seconds = elapsed_seconds, "task_lifecycle recorded");
}
