pub mod agents;
pub mod tasks;
pub mod reputations;
pub mod leaderboard;
pub mod x402;

use axum::{
    routing::{get, post, patch},
    Router,
};
use crate::db::DbPool;
use crate::config::Config;
use crate::casper::contract::CasperClient;

#[derive(Clone)]
pub struct AppState {
    pub pool: DbPool,
    pub config: Config,
    pub casper_client: CasperClient,
}

pub fn create_router(pool: DbPool, config: Config, casper_client: CasperClient) -> Router {
    let state = AppState { pool, config, casper_client };

    Router::new()
        .route("/api/agents", get(agents::get_agents))
        .route("/api/agents/:public_key", get(agents::get_agent))
        .route("/api/agents/register", post(agents::register_agent))
        .route("/api/agents/:public_key/price", patch(agents::update_agent_price))
        .route("/api/agents/:public_key/benchmarks", get(agents::get_agent_benchmarks))
        .route("/api/tasks", get(tasks::get_tasks).post(tasks::create_or_update_task))
        .route("/api/tasks/:id", get(tasks::get_task))
        .route("/api/tasks/:id/execute", post(tasks::execute_task_handler))
        .route("/api/reputations", get(reputations::get_reputations))
        .route("/api/reputations/:agent_pubkey", get(reputations::get_agent_reputations))
        .route("/api/leaderboard", get(leaderboard::get_global_leaderboard))
        .route("/api/leaderboard/:domain", get(leaderboard::get_domain_leaderboard))
        .with_state(state)
}
