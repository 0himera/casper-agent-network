use std::env;
use std::net::SocketAddr;
use std::process::ExitCode;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::signal;
use tokio::time::{MissedTickBehavior, interval};
use tokio_util::sync::CancellationToken;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod validator_loop;

use config::ValidatorNodeConfig;

#[tokio::main]
async fn main() -> Result<ExitCode, Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let health_port: u16 = env::var("HEALTH_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(9090);

    // 1. Healthcheck subcommand check
    if args.len() > 1 && args[1] == "--healthcheck" {
        let addr = format!("127.0.0.1:{}", health_port);
        match TcpStream::connect(&addr).await {
            Ok(_) => {
                println!("Healthcheck OK");
                return Ok(ExitCode::SUCCESS);
            }
            Err(e) => {
                eprintln!("Healthcheck failed connecting to {}: {}", addr, e);
                return Ok(ExitCode::FAILURE);
            }
        }
    }

    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "validator_node=info,agentnet_core=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let node_config = ValidatorNodeConfig::from_env();
    tracing::info!(
        enabled = node_config.enabled,
        poll_interval = node_config.poll_interval_secs,
        health_port = health_port,
        "Starting validator-node service"
    );

    if !node_config.enabled {
        tracing::warn!("Validator node is disabled (VALIDATOR_ENABLED=false). Exiting.");
        return Ok(ExitCode::SUCCESS);
    }

    if let Err(err) = node_config.validate_startup() {
        tracing::error!(error = %err, "Validator-node fail-fast config check failed");
        eprintln!("validator-node config error: {err}");
        return Ok(ExitCode::FAILURE);
    }

    // 2. Initialize DB pool via agentnet_core::db::init_db
    let pool = agentnet_core::db::init_db(&node_config.database_url).await?;

    // Auto-register this validator in the DB so FK constraints are satisfied
    if let Some(ref pk) = node_config.validator_public_key {
        sqlx::query(
            "INSERT INTO validators (public_key, stake_motes, is_active, total_validations, timestamp) \
             VALUES (?, 0, 1, 0, NOW()) \
             ON DUPLICATE KEY UPDATE is_active = 1"
        )
        .bind(pk)
        .execute(&pool)
        .await?;
        tracing::info!(public_key = %pk, "Validator auto-registered in DB");
    }

    let cancel_token = CancellationToken::new();

    // 3. Setup TCP healthcheck listener on HEALTH_PORT
    let listener_addr: SocketAddr = format!("0.0.0.0:{}", health_port).parse()?;
    let listener = match TcpListener::bind(listener_addr).await {
        Ok(l) => {
            tracing::info!(addr = %listener_addr, "Health check TCP server bound");
            Some(l)
        }
        Err(e) => {
            tracing::error!(error = %e, addr = %listener_addr, "Failed to bind health check port");
            None
        }
    };

    // Spawn health check server background loop
    let health_token = cancel_token.clone();
    tokio::spawn(async move {
        if let Some(listener) = listener {
            loop {
                tokio::select! {
                    _ = health_token.cancelled() => {
                        tracing::info!("Health check server shutting down...");
                        break;
                    }
                    res = listener.accept() => {
                        match res {
                            Ok((stream, _)) => {
                                drop(stream);
                            }
                            Err(e) => {
                                tracing::warn!(error = %e, "Health check accept error");
                            }
                        }
                    }
                }
            }
        }
    });

    // 4. Main validator loop task
    let loop_token = cancel_token.clone();
    let poll_interval_secs = node_config.poll_interval_secs;
    let node_config_clone = node_config.clone();

    let mut loop_handle = tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(poll_interval_secs));
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                _ = loop_token.cancelled() => {
                    tracing::info!("Validator loop cancelled");
                    break;
                }
                _ = ticker.tick() => {
                    if loop_token.is_cancelled() {
                        break;
                    }
                    if let Err(err) = validator_loop::run_validator_iteration(&pool, &node_config_clone, &loop_token).await {
                        tracing::error!(error = %err, "Error in validator iteration");
                    }
                }
            }
        }
    });

    // 5. Graceful shutdown handler (SIGINT / SIGTERM)
    let shutdown_token = cancel_token.clone();
    tokio::spawn(async move {
        let ctrl_c = async {
            signal::ctrl_c().await.expect("failed to listen for ctrl+c");
        };

        #[cfg(unix)]
        let terminate = async {
            if let Ok(mut sig) = signal::unix::signal(signal::unix::SignalKind::terminate()) {
                sig.recv().await;
            }
        };

        #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

        tokio::select! {
            _ = ctrl_c => tracing::info!("Received SIGINT signal"),
            _ = terminate => tracing::info!("Received SIGTERM signal"),
        }

        tracing::info!("Initiating graceful shutdown...");
        shutdown_token.cancel();
    });

    // Wait for cancellation or loop completion
    tokio::select! {
        _ = cancel_token.cancelled() => {
            tracing::info!("Shutdown signal received, waiting for main loop to exit...");
        }
        _ = &mut loop_handle => {
            tracing::info!("Validator loop finished unexpectedly.");
        }
    }

    // Wait for main loop task to finish with a 5s timeout join
    match tokio::time::timeout(Duration::from_secs(5), loop_handle).await {
        Ok(res) => {
            if let Err(e) = res {
                tracing::error!(error = %e, "Validator loop task panicked");
            } else {
                tracing::info!("Validator loop task completed cleanly");
            }
        }
        Err(_) => {
            tracing::warn!("Validator loop shutdown timed out after 5s");
        }
    }

    tracing::info!("validator-node shut down complete.");
    Ok(ExitCode::SUCCESS)
}
