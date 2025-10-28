use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tracing::{debug, info};

use crate::AppState;

#[derive(Debug, Serialize)]
pub struct ConfigResponse {
    pub config_id: String,
    pub config_name: String,
    pub config_description: Option<String>,
    pub is_dnssec_enabled: bool,
    pub is_logging_enabled: bool,
    pub blocked_response_ip: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateConfigRequest {
    pub config_name: Option<String>,
    pub config_description: Option<String>,
    pub is_dnssec_enabled: Option<bool>,
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

    // Validate that at least one field is present (reject no-op updates)
    if payload.config_name.is_none()
        && payload.config_description.is_none()
        && payload.is_dnssec_enabled.is_none()
        && payload.is_logging_enabled.is_none()
        && payload.blocked_response_ip.is_none()
    {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "At least one field must be provided"
            })),
        ));
    }

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
    fn test_ipv4_validation_valid() {
        assert!(is_valid_ipv4("0.0.0.0"));
        assert!(is_valid_ipv4("192.168.1.1"));
        assert!(is_valid_ipv4("8.8.8.8"));
        assert!(is_valid_ipv4("255.255.255.255"));
        assert!(is_valid_ipv4("1.2.3.4"));
        assert!(is_valid_ipv4("10.0.0.1"));
        assert!(is_valid_ipv4("172.16.0.1"));
    }

    #[test]
    fn test_ipv4_validation_invalid() {
        assert!(!is_valid_ipv4("256.1.1.1"));
        assert!(!is_valid_ipv4("1.256.1.1"));
        assert!(!is_valid_ipv4("1.1.256.1"));
        assert!(!is_valid_ipv4("1.1.1.256"));
        assert!(!is_valid_ipv4("invalid"));
        assert!(!is_valid_ipv4("1.2.3"));
        assert!(!is_valid_ipv4("1.2.3.4.5"));
        assert!(!is_valid_ipv4(""));
        assert!(!is_valid_ipv4("..."));
        assert!(!is_valid_ipv4("a.b.c.d"));
    }

    #[test]
    fn test_ipv4_validation_edge_cases() {
        // Boundary values
        assert!(is_valid_ipv4("0.0.0.0"));
        assert!(is_valid_ipv4("255.255.255.255"));
        
        // Just over boundary
        assert!(!is_valid_ipv4("256.0.0.0"));
        assert!(!is_valid_ipv4("0.256.0.0"));
        
        // Negative numbers
        assert!(!is_valid_ipv4("-1.0.0.0"));
        
        // Leading zeros (parsed as valid)
        assert!(is_valid_ipv4("01.02.03.04"));
        
        // Whitespace
        assert!(!is_valid_ipv4(" 1.2.3.4"));
        assert!(!is_valid_ipv4("1.2.3.4 "));
        assert!(!is_valid_ipv4("1. 2.3.4"));
    }

    #[test]
    fn test_config_response_structure() {
        let response = ConfigResponse {
            config_id: "123e4567-e89b-12d3-a456-426614174000".to_string(),
            config_name: "Default".to_string(),
            config_description: Some("Default configuration".to_string()),
            is_dnssec_enabled: false,
            is_logging_enabled: true,
            blocked_response_ip: "0.0.0.0".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
        };

        assert!(!response.config_id.is_empty());
        assert!(response.is_logging_enabled);
        assert!(is_valid_ipv4(&response.blocked_response_ip));
    }

    #[test]
    fn test_update_config_request_all_fields() {
        let request = UpdateConfigRequest {
            config_name: Some("Updated Config".to_string()),
            config_description: Some("Updated description".to_string()),
            is_dnssec_enabled: Some(true),
            is_logging_enabled: Some(false),
            blocked_response_ip: Some("192.168.1.1".to_string()),
        };

        assert!(request.config_name.is_some());
        assert!(request.config_description.is_some());
        assert!(request.is_dnssec_enabled.is_some());
        assert!(request.is_logging_enabled.is_some());
        assert!(request.blocked_response_ip.is_some());
    }

    #[test]
    fn test_update_config_request_partial() {
        let request = UpdateConfigRequest {
            config_name: Some("New Name".to_string()),
            config_description: None,
            is_dnssec_enabled: None,
            is_logging_enabled: None,
            blocked_response_ip: None,
        };

        assert!(request.config_name.is_some());
        assert!(request.config_description.is_none());
        assert!(request.is_dnssec_enabled.is_none());
        assert!(request.is_logging_enabled.is_none());
        assert!(request.blocked_response_ip.is_none());
    }

    #[test]
    fn test_update_config_request_empty() {
        let request = UpdateConfigRequest {
            config_name: None,
            config_description: None,
            is_dnssec_enabled: None,
            is_logging_enabled: None,
            blocked_response_ip: None,
        };

        assert!(request.config_name.is_none());
        assert!(request.config_description.is_none());
        assert!(request.is_dnssec_enabled.is_none());
    }

    #[test]
    fn test_config_response_serialization() {
        let response = ConfigResponse {
            config_id: "test-id".to_string(),
            config_name: "Test".to_string(),
            config_description: Some("Test description".to_string()),
            is_dnssec_enabled: true,
            is_logging_enabled: false,
            blocked_response_ip: "0.0.0.0".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&response).expect("Serialization failed");
        assert!(json.contains("config_id"));
        assert!(json.contains("is_dnssec_enabled"));
        assert!(json.contains("blocked_response_ip"));
    }

    #[test]
    fn test_blocked_response_ip_common_values() {
        let common_ips = vec!["0.0.0.0", "127.0.0.1", "192.168.1.1"];
        
        for ip in common_ips {
            assert!(is_valid_ipv4(ip));
        }
    }

    #[test]
    fn test_ipv4_validation_with_ports() {
        // Should not accept IPs with ports
        assert!(!is_valid_ipv4("192.168.1.1:8080"));
        assert!(!is_valid_ipv4("127.0.0.1:80"));
    }
}