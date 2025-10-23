use axum::{http::StatusCode, Json};
use serde_json::{json, Value};

/// Health check endpoint
pub async fn health_check() -> (StatusCode, Json<Value>) {
    (
        StatusCode::OK,
        Json(json!({
            "status": "healthy",
            "service": "mightydns-api",
            "version": env!("CARGO_PKG_VERSION"),
        })),
    )
}

/// Prometheus metrics endpoint
pub async fn metrics() -> String {
    common::metrics::metrics_handler()
}
