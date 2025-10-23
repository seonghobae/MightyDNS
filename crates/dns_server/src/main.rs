mod handlers;
mod resolver;

use anyhow::Result;
use common::{cache::Cache, config::Config, database::Database, nats_client::NatsClient};
use std::sync::Arc;
use tokio::signal;
use tracing::{info, warn};

/// Shared application state
pub struct AppState {
    pub config: Config,
    pub database: Database,
    pub cache: Cache,
    pub nats: NatsClient,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,dns_server=debug".into()),
        )
        .json()
        .init();

    info!("Starting MightyDNS Server...");

    // Load configuration
    let config = Config::from_env().expect("Failed to load configuration");
    info!("Configuration loaded");

    // Initialize database
    let database = Database::new(&config.database)
        .await
        .expect("Failed to connect to database");
    database.health_check().await.expect("Database health check failed");
    info!("Database connected");

    // Initialize cache
    let cache = Cache::new(&config.valkey, database.clone())
        .await
        .expect("Failed to initialize cache");
    info!("Cache initialized");

    // Initialize NATS client
    let nats = NatsClient::new(&config.nats)
        .await
        .expect("Failed to connect to NATS");
    nats.health_check().await.expect("NATS health check failed");
    info!("NATS connected");

    // Create shared application state
    let state = Arc::new(AppState {
        config: config.clone(),
        database,
        cache,
        nats,
    });

    // Spawn DNS servers concurrently
    let doh_handle = tokio::spawn(handlers::doh::serve(state.clone()));
    let dot_handle = tokio::spawn(handlers::dot::serve(state.clone()));
    let udp_handle = tokio::spawn(handlers::udp::serve(state.clone()));

    // Spawn metrics server
    let metrics_handle = tokio::spawn(serve_metrics());

    info!("All DNS servers started successfully");
    info!("  DoH: https://0.0.0.0:{}/dns-query/{{tenant_id}}", config.dns.doh_port);
    info!("  DoT: tls://0.0.0.0:{}",  config.dns.dot_port);
    info!("  UDP: udp://0.0.0.0:{}", config.dns.udp_port);
    info!("  Metrics: http://0.0.0.0:9090/metrics");

    // Wait for shutdown signal
    match signal::ctrl_c().await {
        Ok(()) => {
            info!("Shutdown signal received, gracefully shutting down...");
        }
        Err(err) => {
            warn!("Unable to listen for shutdown signal: {}", err);
        }
    }

    // Graceful shutdown (abort all tasks)
    doh_handle.abort();
    dot_handle.abort();
    udp_handle.abort();
    metrics_handle.abort();

    info!("MightyDNS Server shutdown complete");

    Ok(())
}

/// Serve Prometheus metrics on port 9090
async fn serve_metrics() -> Result<()> {
    use axum::{routing::get, Router};

    let app = Router::new().route("/metrics", get(metrics_handler));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:9090").await?;
    info!("Metrics server listening on http://0.0.0.0:9090/metrics");

    axum::serve(listener, app).await?;

    Ok(())
}

async fn metrics_handler() -> String {
    common::metrics::metrics_handler()
}
