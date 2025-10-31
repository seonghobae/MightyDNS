use anyhow::Result;
use common::{config::Config, database::Database, nats_client::NatsClient};
use std::sync::Arc;
use tokio::signal;
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod dns_logger;
mod email_sender;

/// Worker application state
#[derive(Clone)]
pub struct WorkerState {
    pub config: Arc<Config>,
    pub database: Database,
    pub nats: NatsClient,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "workers=debug,common=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting MightyDNS Background Workers");

    // Load configuration
    dotenvy::dotenv().ok();
    let config = Arc::new(Config::from_env()?);
    info!("Configuration loaded");

    // Initialize database
    let database = Database::new(&config.database).await?;
    info!("Database connection pool initialized");

    // Initialize NATS client
    let nats = NatsClient::new(&config.nats).await?;
    info!("NATS client connected");

    // Ensure streams exist
    setup_nats_streams(&nats).await?;

    // Create shared worker state
    let state = Arc::new(WorkerState {
        config,
        database,
        nats,
    });

    // Spawn worker tasks
    info!("Starting background worker tasks...");

    let dns_logger_handle = tokio::spawn({
        let state = state.clone();
        async move {
            if let Err(e) = dns_logger::run(state).await {
                error!("DNS logger worker failed: {}", e);
            }
        }
    });

    let email_sender_handle = tokio::spawn({
        let state = state.clone();
        async move {
            if let Err(e) = email_sender::run(state).await {
                error!("Email sender worker failed: {}", e);
            }
        }
    });

    info!("All workers started successfully");
    info!("Workers running, press Ctrl+C to shutdown");

    // Wait for shutdown signal
    shutdown_signal().await;

    // Graceful shutdown
    info!("Shutting down workers...");
    dns_logger_handle.abort();
    email_sender_handle.abort();

    info!("Workers shut down gracefully");
    Ok(())
}

/// Setup NATS JetStream streams
async fn setup_nats_streams(nats: &NatsClient) -> Result<()> {
    use common::nats_client::streams;

    info!("Setting up NATS JetStream streams...");

    // Create DNS query log stream
    nats.create_stream(streams::dns_query_log_stream()).await?;
    info!("✓ DNS_QUERIES stream ready");

    // Create email stream
    nats.create_stream(streams::email_stream()).await?;
    info!("✓ EMAILS stream ready");

    // Create blocklist update stream
    nats.create_stream(streams::blocklist_update_stream())
        .await?;
    info!("✓ BLOCKLIST_UPDATES stream ready");

    // Create analytics stream
    nats.create_stream(streams::analytics_stream()).await?;
    info!("✓ ANALYTICS stream ready");

    Ok(())
}

/// Graceful shutdown signal handler
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            warn!("Received Ctrl+C signal");
        },
        _ = terminate => {
            warn!("Received terminate signal");
        },
    }
}
