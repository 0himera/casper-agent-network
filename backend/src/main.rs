use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::http::Method;
use backend::api::create_router;
use backend::config::Config;
use backend::db::init_db;
use tower_http::cors::{Any, CorsLayer};
use axum_prometheus::PrometheusMetricLayerBuilder;
use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Install tracing subscriber
    tracing_subscriber::fmt::init();
    tracing::info!("Starting Casper Agent Network Backend...");

    let config = Config::from_env();
    let pool = init_db(&config.database_url).await?;

    let casper_client = backend::casper::contract::CasperClient::from_env()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    // 1. Prometheus Metrics configuration
    let (prometheus_layer, metric_handle) = PrometheusMetricLayerBuilder::new()
        .with_ignore_patterns(&["/metrics", "/health"])
        .with_default_metrics()
        .build_pair();

    // 2. Tower Governor Rate Limiting (1 request/sec per IP, burst of 10)
    let governor_config = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(1)
            .burst_size(10)
            .use_headers()
            .finish()
            .unwrap(),
    );

    // Background task to clean up old entries in rate limiter storage
    let governor_limiter = governor_config.limiter().clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            tracing::debug!("rate limiting storage size: {}", governor_limiter.len());
            governor_limiter.retain_recent();
        }
    });

    let governor_layer = GovernorLayer::new(governor_config)
        .error_handler(|error| {
            use tower_governor::errors::GovernorError;
            match error {
                GovernorError::TooManyRequests { wait_time, headers } => {
                    let body = serde_json::json!({
                        "error": "too_many_requests",
                        "retry_after_seconds": wait_time
                    }).to_string();
                    let mut response = axum::response::Response::new(axum::body::Body::from(body));
                    *response.status_mut() = axum::http::StatusCode::TOO_MANY_REQUESTS;
                    response.headers_mut().insert(
                        axum::http::header::CONTENT_TYPE,
                        axum::http::HeaderValue::from_static("application/json"),
                    );
                    if let Some(headers_map) = headers {
                        response.headers_mut().extend(headers_map);
                    }
                    response
                }
                GovernorError::UnableToExtractKey => {
                    let body = serde_json::json!({ "error": "unable_to_extract_key" }).to_string();
                    let mut response = axum::response::Response::new(axum::body::Body::from(body));
                    *response.status_mut() = axum::http::StatusCode::BAD_REQUEST;
                    response.headers_mut().insert(
                        axum::http::header::CONTENT_TYPE,
                        axum::http::HeaderValue::from_static("application/json"),
                    );
                    response
                }
                GovernorError::Other { code, msg, headers } => {
                    let body = serde_json::json!({
                        "error": "rate_limiting_error",
                        "message": msg.unwrap_or_else(|| "Error".to_string())
                    }).to_string();
                    let mut response = axum::response::Response::new(axum::body::Body::from(body));
                    *response.status_mut() = code;
                    response.headers_mut().insert(
                        axum::http::header::CONTENT_TYPE,
                        axum::http::HeaderValue::from_static("application/json"),
                    );
                    if let Some(headers_map) = headers {
                        response.headers_mut().extend(headers_map);
                    }
                    response
                }
            }
        });

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

    // Assemble the router with prometheus metrics, rate limiting, and CORS
    let app = create_router(pool.clone(), config.clone(), casper_client)
        .route("/metrics", axum::routing::get(move || async move { metric_handle.render() }))
        .layer(prometheus_layer)
        .layer(governor_layer)
        .layer(cors);

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("Server running on http://{}", addr);

    // 3. Graceful Shutdown Implementation with 10-second timeout
    let (close_tx, close_rx) = tokio::sync::oneshot::channel::<()>();
    
    let server_task = tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(async move {
            let _ = close_rx.await;
        })
        .await
    });

    // Wait for shutdown signals
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("SIGINT received.");
        },
        _ = terminate => {
            tracing::info!("SIGTERM received.");
        },
    }

    tracing::info!("Shutdown signal received. Starting 10-second graceful timeout...");
    let _ = close_tx.send(());

    tokio::select! {
        res = server_task => {
            match res {
                Ok(Ok(())) => tracing::info!("Server stopped gracefully."),
                Ok(Err(e)) => tracing::error!("Server error: {}", e),
                Err(e) => tracing::error!("Server task joined with error: {}", e),
            }
        }
        _ = tokio::time::sleep(Duration::from_secs(10)) => {
            tracing::warn!("Graceful shutdown timed out after 10s. Force terminating...");
        }
    }

    pool.close().await;
    tracing::info!("Database connections closed. Goodbye.");

    Ok(())
}

