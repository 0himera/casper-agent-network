pub mod agents;
pub mod tasks;
pub mod reputations;
pub mod leaderboard;

use axum::{
    routing::{get, post, patch},
    Router,
};
use crate::db::DbPool;
use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub pool: DbPool,
    pub config: Config,
}

pub fn create_router(pool: DbPool, config: Config) -> Router {
    let state = AppState { pool, config };

    Router::new()
        .route("/api/agents", get(agents::get_agents))
        .route("/api/agents/:public_key", get(agents::get_agent))
        .route("/api/agents/register", post(agents::register_agent))
        .route("/api/agents/:public_key/price", patch(agents::update_agent_price))
        .route("/api/tasks", get(tasks::get_tasks))
        .route("/api/tasks/:id", get(tasks::get_task))
        .route("/api/reputations", get(reputations::get_reputations))
        .route("/api/reputations/:agent_pubkey", get(reputations::get_agent_reputations))
        .route("/api/leaderboard", get(leaderboard::get_global_leaderboard))
        .route("/api/leaderboard/:domain", get(leaderboard::get_domain_leaderboard))
        .with_state(state)
}
