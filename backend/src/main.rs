use std::net::SocketAddr;

use axum::http::Method;
use backend::api::create_router;
use backend::config::Config;
use backend::db::init_db;
use tower_http::cors::{Any, CorsLayer};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    tracing::info!("Starting Casper Agent Network Backend...");

    let config = Config::from_env();
    let pool = init_db(&config.database_url).await?;

    let casper_client = backend::casper::contract::CasperClient::from_env()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(vec![
            Method::GET,
            Method::POST,
            Method::PATCH,
            Method::PUT,
            Method::DELETE,
        ])
        .allow_headers(Any);

    let app = create_router(pool, config.clone(), casper_client).layer(cors);

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("Server running on http://{}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}
