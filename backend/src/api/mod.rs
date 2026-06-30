pub mod agents;
pub mod exams;
pub mod leaderboard;
pub mod reputations;
pub mod tasks;
pub mod x402;

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use crate::casper::contract::CasperClient;
use crate::config::Config;
use crate::db::DbPool;
use axum::{
    Router,
    routing::{get, patch, post},
};

/// Tracks in-flight `/validate` jobs per task id to deduplicate concurrent retries.
#[derive(Clone, Default)]
pub struct ValidateInflight {
    tasks: Arc<Mutex<HashSet<String>>>,
}

impl ValidateInflight {
    /// Returns `true` if this caller acquired the in-flight slot for `task_id`.
    pub fn try_start(&self, task_id: &str) -> bool {
        let mut guard = self.tasks.lock().expect("validate inflight lock poisoned");
        guard.insert(task_id.to_string())
    }

    pub fn finish(&self, task_id: &str) {
        let mut guard = self.tasks.lock().expect("validate inflight lock poisoned");
        guard.remove(task_id);
    }
}

#[derive(Clone)]
pub struct AppState {
    pub pool: DbPool,
    pub config: Config,
    pub casper_client: CasperClient,
    pub validate_inflight: ValidateInflight,
}

pub fn create_router(pool: DbPool, config: Config, casper_client: CasperClient) -> Router {
    let state = AppState {
        pool,
        config,
        casper_client,
        validate_inflight: ValidateInflight::default(),
    };

    Router::new()
        .route(
            "/health",
            get(|| async { axum::Json(serde_json::json!({ "status": "ok" })) }),
        )
        .route("/api/agents", get(agents::get_agents))
        .route("/api/agents/{public_key}", get(agents::get_agent))
        .route("/api/agents/register", post(agents::register_agent))
        .route(
            "/api/agents/{public_key}/price",
            patch(agents::update_agent_price),
        )
        .route(
            "/api/agents/{public_key}/benchmarks",
            get(agents::get_agent_benchmarks),
        )
        .route(
            "/api/agents/{public_key}/capabilities",
            post(agents::update_agent_capabilities),
        )
        .route(
            "/api/tasks",
            get(tasks::get_tasks).post(tasks::create_or_update_task),
        )
        .route("/api/tasks/{id}", get(tasks::get_task))
        .route("/api/tasks/{id}/execute", post(tasks::execute_task_handler))
        .route("/api/tasks/{id}/raw_result", post(tasks::raw_result_handler))
        .route("/api/tasks/{id}/validate", post(tasks::validate_task_handler))
        .route("/api/reputations", get(reputations::get_reputations))
        .route(
            "/api/reputations/{agent_pubkey}",
            get(reputations::get_agent_reputations),
        )
        .route("/api/leaderboard", get(leaderboard::get_global_leaderboard))
        .route(
            "/api/leaderboard/{domain}",
            get(leaderboard::get_domain_leaderboard),
        )
        .route(
            "/api/admin/exams/dispatch",
            post(exams::dispatch_exam_handler),
        )
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::ValidateInflight;

    #[test]
    fn validate_inflight_guard_deduplicates_same_task() {
        let guard = ValidateInflight::default();
        assert!(guard.try_start("task-1"));
        assert!(!guard.try_start("task-1"));
        guard.finish("task-1");
        assert!(guard.try_start("task-1"));
    }
}
