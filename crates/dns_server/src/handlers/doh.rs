use axum::{
    body::Bytes,
    extract::{Path, Query, State},
    http::{header::CONTENT_TYPE, HeaderValue, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use hickory_proto::op::Message;
use serde::Deserialize;
use std::sync::Arc;
use tracing::{debug, error, info};

use crate::{resolver, AppState};

/// DNS over HTTPS query parameters
#[derive(Debug, Deserialize)]
pub struct DohQueryParams {
    dns: Option<String>, // Base64url-encoded DNS query
}

/// Serve DNS over HTTPS (DoH) on specified port
pub async fn serve(state: Arc<AppState>) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/dns-query/:tenant_id", get(handle_doh_get))
        .route("/dns-query/:tenant_id", post(handle_doh_post))
        .route("/health", get(health_check))
        .with_state(state.clone());

    let port = state.config.dns.doh_port;
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;

    info!("DoH server listening on port {}", port);

    axum::serve(listener, app).await?;

    Ok(())
}

/// Handle GET requests (RFC 8484 - query in URL parameter)
async fn handle_doh_get(
    Path(tenant_id): Path<String>,
    Query(params): Query<DohQueryParams>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    debug!("DoH GET request for tenant: {}", tenant_id);

    // Extract DNS query from URL parameter
    let dns_query_bytes = match params.dns {
        Some(base64_query) => match URL_SAFE_NO_PAD.decode(&base64_query) {
            Ok(bytes) => bytes,
            Err(e) => {
                error!("Failed to decode base64 query: {}", e);
                return (
                    StatusCode::BAD_REQUEST,
                    [(CONTENT_TYPE, HeaderValue::from_static("application/dns-message"))],
                    Vec::<u8>::new(),
                )
                    .into_response();
            }
        },
        None => {
            error!("Missing 'dns' parameter in GET request");
            return (
                StatusCode::BAD_REQUEST,
                [(CONTENT_TYPE, HeaderValue::from_static("application/dns-message"))],
                Vec::<u8>::new(),
            )
                .into_response();
        }
    };

    // Parse DNS message
    let dns_message = match Message::from_vec(&dns_query_bytes) {
        Ok(msg) => msg,
        Err(e) => {
            error!("Failed to parse DNS message: {}", e);
            return (
                StatusCode::BAD_REQUEST,
                "application/dns-message",
                vec![],
            )
                .into_response();
        }
    };

    // Resolve DNS query
    match resolver::resolve_dns_query(&state, &tenant_id, dns_message, "doh").await {
        Ok(response) => {
            let response_bytes = match response.to_vec() {
                Ok(bytes) => bytes,
                Err(e) => {
                    error!("Failed to serialize DNS response: {}", e);
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "application/dns-message",
                        vec![],
                    )
                        .into_response();
                }
            };

            (StatusCode::OK, [(CONTENT_TYPE, HeaderValue::from_static("application/dns-message"))], response_bytes).into_response()
        }
        Err(e) => {
            error!("DNS query resolution failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "application/dns-message",
                vec![],
            )
                .into_response()
        }
    }
}

/// Handle POST requests (RFC 8484 - query in body)
async fn handle_doh_post(
    Path(tenant_id): Path<String>,
    State(state): State<Arc<AppState>>,
    body: Bytes,
) -> impl IntoResponse {
    debug!("DoH POST request for tenant: {}, body size: {} bytes", tenant_id, body.len());

    // Parse DNS message from body
    let dns_message = match Message::from_vec(&body) {
        Ok(msg) => msg,
        Err(e) => {
            error!("Failed to parse DNS message: {}", e);
            return (
                StatusCode::BAD_REQUEST,
                "application/dns-message",
                vec![],
            )
                .into_response();
        }
    };

    // Resolve DNS query
    match resolver::resolve_dns_query(&state, &tenant_id, dns_message, "doh").await {
        Ok(response) => {
            let response_bytes = match response.to_vec() {
                Ok(bytes) => bytes,
                Err(e) => {
                    error!("Failed to serialize DNS response: {}", e);
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "application/dns-message",
                        vec![],
                    )
                        .into_response();
                }
            };

            (StatusCode::OK, [(CONTENT_TYPE, HeaderValue::from_static("application/dns-message"))], response_bytes).into_response()
        }
        Err(e) => {
            error!("DNS query resolution failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "application/dns-message",
                vec![],
            )
                .into_response()
        }
    }
}

/// Health check endpoint
async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}
