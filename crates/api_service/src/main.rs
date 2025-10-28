use anyhow::Result;
use axum::{
    middleware as axum_middleware,
    routing::{get, post},
    Router,
};
use common::{cache::CacheManager, config::Config, database::Database, nats_client::NatsClient};
use std::sync::Arc;
use tokio::signal;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::{info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod handlers;
mod middleware;
mod models;
mod services;

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub database: Database,
    pub cache: CacheManager,
    pub nats: NatsClient,
    pub auth_service: Arc<services::auth::AuthService>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "api_service=debug,common=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting MightyDNS API Service");

    // Load configuration
    dotenvy::dotenv().ok();
    let config = Arc::new(Config::from_env()?);
    info!("Configuration loaded");

    // Initialize database
    let database = Database::new(&config.database).await?;
    info!("Database connection pool initialized");

    // Initialize cache manager
    let cache = CacheManager::new(config.clone(), database.clone()).await?;
    info!("Cache manager initialized");

    // Initialize NATS client
    let nats = NatsClient::new(&config.nats.url).await?;
    info!("NATS client connected");

    // Initialize authentication service
    let auth_service = Arc::new(services::auth::AuthService::new(
        config.clone(),
        database.clone(),
        nats.clone(),
    ));
    info!("Authentication service initialized");

    // Create shared application state
    let state = Arc::new(AppState {
        config: config.clone(),
        database,
        cache,
        nats,
        auth_service,
    });

    // Build application router
    let app = create_router(state);

    // Start server
    let addr = format!("{}:{}", config.server.host, config.server.port);
    info!("API service listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;

    // Run server with graceful shutdown
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("API service shut down gracefully");
    Ok(())
}

/// Create the application router with all routes
fn create_router(state: Arc<AppState>) -> Router {
    // Public routes (no authentication required)
    let public_routes = Router::new()
        .route("/health", get(handlers::health::health_check))
        .route("/metrics", get(handlers::health::metrics))
        .route("/api/v1/auth/email/request", post(handlers::auth::email::request_otp))
        .route("/api/v1/auth/email/verify", post(handlers::auth::email::verify_otp))
        .route("/api/v1/auth/webauthn/register/start", post(handlers::auth::webauthn::register_start))
        .route("/api/v1/auth/webauthn/register/finish", post(handlers::auth::webauthn::register_finish))
        .route("/api/v1/auth/webauthn/login/start", post(handlers::auth::webauthn::login_start))
        .route("/api/v1/auth/webauthn/login/finish", post(handlers::auth::webauthn::login_finish))
        .route("/api/v1/auth/totp/verify", post(handlers::auth::totp::verify_totp));

    // Protected routes (authentication required)
    let protected_routes = Router::new()
        .route("/api/v1/auth/totp/setup", post(handlers::auth::totp::setup_totp))
        .route("/api/v1/auth/logout", post(handlers::auth::logout))
        .route("/api/v1/tenant/profile", get(handlers::tenant::get_profile))
        .route("/api/v1/tenant/profile", post(handlers::tenant::update_profile))
        .route("/api/v1/blocklists", get(handlers::blocklist::get_blocklists))
        .route("/api/v1/whitelists", get(handlers::whitelist::get_whitelists))
        .route("/api/v1/whitelists", post(handlers::whitelist::add_whitelist))
        .route("/api/v1/whitelists/:id", axum::routing::delete(handlers::whitelist::delete_whitelist))
        .route("/api/v1/config", get(handlers::config::get_config))
        .route("/api/v1/config", axum::routing::put(handlers::config::update_config))
        .layer(axum_middleware::from_fn(middleware::auth::auth_middleware));

    // Merge all routes
    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
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

    info!("Starting graceful shutdown");
}
