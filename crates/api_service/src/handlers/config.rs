use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tracing::{debug, error, info};

use crate::AppState;

#[derive(Debug, Serialize)]
pub struct ConfigResponse {
    pub config_id: String,
    pub config_name: String,
    pub is_blocking_enabled: bool,
    pub is_logging_enabled: bool,
    pub blocked_response_ip: String,
    pub dns_over_https_enabled: bool,
    pub dns_over_tls_enabled: bool,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateConfigRequest {
    pub config_name: Option<String>,
    pub is_blocking_enabled: Option<bool>,
    pub is_logging_enabled: Option<bool>,
    pub blocked_response_ip: Option<String>,
}

/// Get tenant configuration
/// GET /api/v1/config
pub async fn get_config(
    State(state): State<Arc<AppState>>,
    // TODO: Extract tenant from JWT
) -> Result<(StatusCode, Json<ConfigResponse>), (StatusCode, Json<Value>)> {
    debug!("Get config request");

    // TODO: Get tenant from middleware
    // TODO: Get config from database

    Err((
        StatusCode::NOT_IMPLEMENTED,
        Json(json!({
            "error": "Not yet implemented"
        })),
    ))
}

/// Update tenant configuration
/// PUT /api/v1/config
pub async fn update_config(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<UpdateConfigRequest>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    debug!("Update config request");

    // Validate blocked_response_ip if provided
    if let Some(ref ip) = payload.blocked_response_ip {
        if !is_valid_ipv4(ip) {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "Invalid IPv4 address"
                })),
            ));
        }
    }

    // TODO: Get tenant from middleware
    // TODO: Update config in database
    // TODO: Invalidate cache

    info!("Configuration updated");

    Ok((
        StatusCode::OK,
        Json(json!({
            "success": true,
            "message": "Configuration updated"
        })),
    ))
}

/// Basic IPv4 validation
fn is_valid_ipv4(ip: &str) -> bool {
    ip.split('.')
        .filter_map(|s| s.parse::<u8>().ok())
        .count()
        == 4
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ipv4_validation() {
        assert!(is_valid_ipv4("0.0.0.0"));
        assert!(is_valid_ipv4("192.168.1.1"));
        assert!(is_valid_ipv4("8.8.8.8"));
        assert!(!is_valid_ipv4("256.1.1.1"));
        assert!(!is_valid_ipv4("invalid"));
        assert!(!is_valid_ipv4("1.2.3"));
    }
}
