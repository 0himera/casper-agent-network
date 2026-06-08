mod config;
mod db;
mod api;
mod validator;
mod orchestrator;

use std::net::SocketAddr;
use axum::http::Method;
use tower_http::cors::{Any, CorsLayer};
use crate::config::Config;
use crate::db::init_db;
use crate::api::create_router;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting Casper Agent Network Backend...");

    // 1. Load configuration
    let config = Config::from_env();

    // 2. Initialize database
    let pool = init_db(&config.database_url).await?;

    // 3. Configure CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(vec![Method::GET, Method::POST, Method::PATCH, Method::PUT, Method::DELETE])
        .allow_headers(Any);

    // 4. Build router
    let app = create_router(pool, config.clone()).layer(cors);

    // 5. Start listener
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("Server running on http://{}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}
