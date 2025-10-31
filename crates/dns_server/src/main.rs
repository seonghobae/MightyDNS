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
    let mut doh_handle = tokio::spawn(handlers::doh::serve(state.clone()));
    let mut dot_handle = tokio::spawn(handlers::dot::serve(state.clone()));
    let mut udp_handle = tokio::spawn(handlers::udp::serve(state.clone()));

    // Spawn metrics server
    let metrics_port = config.server.metrics_port;
    let mut metrics_handle = tokio::spawn(serve_metrics(metrics_port));

    info!("All DNS servers started successfully");
    info!(
        "  DoH: 0.0.0.0:{}/dns-query/{{tenant_id}} (scheme depends on TLS termination)",
        config.dns.doh_port
    );
    info!("  DoT: tls://0.0.0.0:{}",  config.dns.dot_port);
    info!("  UDP: udp://0.0.0.0:{}", config.dns.udp_port);
    info!("  Metrics: http://0.0.0.0:{}/metrics", metrics_port);

    // Wait for shutdown signal (Ctrl+C or SIGTERM for containerized deployments)
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C signal, gracefully shutting down...");
        },
        _ = terminate => {
            info!("Received SIGTERM signal, gracefully shutting down...");
        },
        res = &mut doh_handle => {
            error!("DoH task exited unexpectedly: {:?}", res);
            error!("Shutting down all services due to DoH failure");
        },
        res = &mut dot_handle => {
            error!("DoT task exited unexpectedly: {:?}", res);
            error!("Shutting down all services due to DoT failure");
        },
        res = &mut udp_handle => {
            error!("UDP task exited unexpectedly: {:?}", res);
            error!("Shutting down all services due to UDP failure");
        },
        res = &mut metrics_handle => {
            error!("Metrics task exited unexpectedly: {:?}", res);
            error!("Shutting down all services due to Metrics failure");
        },
    }

    // Graceful shutdown (abort all tasks)
    doh_handle.abort();
    dot_handle.abort();
    udp_handle.abort();
    metrics_handle.abort();

    info!("MightyDNS Server shutdown complete");

    Ok(())
}

/// Serve Prometheus metrics on configurable port (default: 9090)
async fn serve_metrics(port: u16) -> Result<()> {
    use axum::{routing::get, Router};

    let app = Router::new().route("/metrics", get(metrics_handler));

    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!("Metrics server listening on http://{}/metrics", addr);

    axum::serve(listener, app).await?;

    Ok(())
}

async fn metrics_handler() -> impl axum::response::IntoResponse {
    use axum::{http::header, response::IntoResponse};
    let body = common::metrics::metrics_handler();
    ([(header::CONTENT_TYPE, "text/plain; version=0.0.4; charset=utf-8")], body)
}
