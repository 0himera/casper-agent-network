//! Prometheus metrics recording wrappers for Casper Agent Network (CAN).

pub fn record_validator_decision(verdict: &str) {
    metrics::counter!("validator_decisions_total", "verdict" => verdict.to_string()).increment(1);
}

pub fn record_onchain_tx(action: &str) {
    metrics::counter!("onchain_tx_total", "action" => action.to_string()).increment(1);
}

pub fn record_task_lifecycle(elapsed_seconds: f64) {
    metrics::histogram!("task_lifecycle_seconds").record(elapsed_seconds);
}
