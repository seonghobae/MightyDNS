use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tracing::{debug, error, info};

use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct SetupTotpRequest {
    pub tenant_id: String,
}

#[derive(Debug, Serialize)]
pub struct SetupTotpResponse {
    pub success: bool,
    pub secret_key: String,
    pub qr_code_url: String,
    pub issuer: String,
    pub account_name: String,
}

#[derive(Debug, Deserialize)]
pub struct VerifyTotpRequest {
    pub email_address: String,
    pub totp_code: String,
}

#[derive(Debug, Serialize)]
pub struct VerifyTotpResponse {
    pub success: bool,
    pub token: Option<String>,
    pub tenant_id: Option<String>,
    pub message: String,
}

/// Setup TOTP for a tenant (requires prior authentication)
/// POST /api/v1/auth/totp/setup
pub async fn setup_totp(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SetupTotpRequest>,
) -> Result<(StatusCode, Json<SetupTotpResponse>), (StatusCode, Json<Value>)> {
    debug!("TOTP setup request for tenant: {}", payload.tenant_id);

    // Setup TOTP through auth service
    match state
        .auth_service
        .setup_totp(&payload.tenant_id)
        .await
    {
        Ok((secret, qr_url, account_name)) => {
            info!("TOTP setup successful for tenant: {}", payload.tenant_id);
            Ok((
                StatusCode::OK,
                Json(SetupTotpResponse {
                    success: true,
                    secret_key: secret,
                    qr_code_url: qr_url,
                    issuer: "MightyDNS".to_string(),
                    account_name,
                }),
            ))
        }
        Err(e) => {
            error!("Failed to setup TOTP: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Failed to setup TOTP",
                    "details": e.to_string()
                })),
            ))
        }
    }
}

/// Verify TOTP code and create session
/// POST /api/v1/auth/totp/verify
pub async fn verify_totp(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<VerifyTotpRequest>,
) -> Result<(StatusCode, Json<VerifyTotpResponse>), (StatusCode, Json<Value>)> {
    debug!(
        "TOTP verification attempt for email: {}",
        payload.email_address
    );

    // Verify TOTP through auth service
    match state
        .auth_service
        .verify_totp(&payload.email_address, &payload.totp_code)
        .await
    {
        Ok((tenant, token)) => {
            info!(
                "TOTP verified successfully for tenant: {}",
                tenant.tenant_id
            );
            Ok((
                StatusCode::OK,
                Json(VerifyTotpResponse {
                    success: true,
                    token: Some(token),
                    tenant_id: Some(tenant.tenant_identifier.clone()),
                    message: "Authentication successful".to_string(),
                }),
            ))
        }
        Err(e) => {
            error!("TOTP verification failed: {}", e);
            Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "error": "Invalid TOTP code",
                    "details": e.to_string()
                })),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_setup_totp_request_structure() {
        let request = SetupTotpRequest {
            tenant_id: "test_tenant_123".to_string(),
        };
        
        assert_eq!(request.tenant_id, "test_tenant_123");
        assert!(!request.tenant_id.is_empty());
    }

    #[test]
    fn test_setup_totp_response_structure() {
        let response = SetupTotpResponse {
            success: true,
            secret_key: "ABCDEFGHIJKLMNOP".to_string(),
            qr_code_url: "otpauth://totp/MightyDNS:user@example.com?secret=ABCDEFGHIJKLMNOP&issuer=MightyDNS".to_string(),
            issuer: "MightyDNS".to_string(),
            account_name: "MightyDNS:user@example.com".to_string(),
        };
        
        assert!(response.success);
        assert_eq!(response.secret_key.len(), 16);
        assert!(response.qr_code_url.starts_with("otpauth://totp/"));
        assert!(response.qr_code_url.contains("secret="));
        assert!(response.qr_code_url.contains("issuer="));
        assert_eq!(response.issuer, "MightyDNS");
    }

    #[test]
    fn test_verify_totp_request_structure() {
        let request = VerifyTotpRequest {
            email_address: "user@example.com".to_string(),
            totp_code: "123456".to_string(),
        };
        
        assert_eq!(request.email_address, "user@example.com");
        assert_eq!(request.totp_code, "123456");
        assert_eq!(request.totp_code.len(), 6);
    }

    #[test]
    fn test_verify_totp_response_success() {
        let response = VerifyTotpResponse {
            success: true,
            token: Some("jwt.token.here".to_string()),
            tenant_id: Some("tenant_abc123".to_string()),
            message: "Authentication successful".to_string(),
        };
        
        assert!(response.success);
        assert!(response.token.is_some());
        assert!(response.tenant_id.is_some());
        assert!(!response.message.is_empty());
    }

    #[test]
    fn test_verify_totp_response_failure() {
        let response = VerifyTotpResponse {
            success: false,
            token: None,
            tenant_id: None,
            message: "Invalid TOTP code".to_string(),
        };
        
        assert!(!response.success);
        assert!(response.token.is_none());
        assert!(response.tenant_id.is_none());
        assert_eq!(response.message, "Invalid TOTP code");
    }

    #[test]
    fn test_totp_code_format() {
        let valid_codes = vec!["123456", "000000", "999999", "012345"];
        
        for code in valid_codes {
            assert_eq!(code.len(), 6);
            assert!(code.chars().all(|c| c.is_ascii_digit()));
        }
    }

    #[test]
    fn test_qr_code_url_format() {
        let url = "otpauth://totp/MightyDNS:user@example.com?secret=JBSWY3DPEHPK3PXP&issuer=MightyDNS";
        
        assert!(url.starts_with("otpauth://totp/"));
        assert!(url.contains("secret="));
        assert!(url.contains("issuer="));
        assert!(url.contains("@"));
    }

    #[test]
    fn test_setup_response_serialization() {
        let response = SetupTotpResponse {
            success: true,
            secret_key: "TESTSECRET123456".to_string(),
            qr_code_url: "otpauth://totp/test".to_string(),
            issuer: "MightyDNS".to_string(),
            account_name: "test:user@test.com".to_string(),
        };
        
        let json = serde_json::to_string(&response).expect("Serialization failed");
        assert!(json.contains("success"));
        assert!(json.contains("secret_key"));
        assert!(json.contains("qr_code_url"));
    }

    #[test]
    fn test_verify_request_deserialization() {
        let json = r#"{"email_address":"test@example.com","totp_code":"123456"}"#;
        let request: VerifyTotpRequest = serde_json::from_str(json).expect("Deserialization failed");
        
        assert_eq!(request.email_address, "test@example.com");
        assert_eq!(request.totp_code, "123456");
    }
}