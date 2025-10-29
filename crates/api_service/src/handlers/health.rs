use axum::{extract::State, http::StatusCode, Json};
use serde_json::{json, Value};
use std::sync::Arc;
use tracing::error;

use crate::AppState;

/// Health check endpoint with dependency verification
pub async fn health_check(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<Value>) {
    let mut healthy = true;
    let mut checks = serde_json::Map::new();

    // Check database connectivity
    match state.database.health_check().await {
        Ok(_) => {
            checks.insert("database".to_string(), json!("healthy"));
        }
        Err(e) => {
            error!("Database health check failed: {}", e);
            checks.insert("database".to_string(), json!("unhealthy"));
            healthy = false;
        }
    }

    // Check Valkey/cache connectivity
    match state.cache.health_check().await {
        Ok(_) => {
            checks.insert("cache".to_string(), json!("healthy"));
        }
        Err(e) => {
            error!("Cache health check failed: {}", e);
            checks.insert("cache".to_string(), json!("unhealthy"));
            healthy = false;
        }
    }

    // Check NATS connectivity
    match state.nats.health_check().await {
        Ok(_) => {
            checks.insert("nats".to_string(), json!("healthy"));
        }
        Err(e) => {
            error!("NATS health check failed: {}", e);
            checks.insert("nats".to_string(), json!("unhealthy"));
            healthy = false;
        }
    }

    let status_code = if healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (
        status_code,
        Json(json!({
            "status": if healthy { "healthy" } else { "unhealthy" },
            "service": "mightydns-api",
            "version": env!("CARGO_PKG_VERSION"),
            "checks": checks,
        })),
    )
}

/// Prometheus metrics endpoint
pub async fn metrics() -> String {
    common::metrics::metrics_handler()
}
