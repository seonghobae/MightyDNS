use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tracing::{debug, error, info};

use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct RequestOtpRequest {
    pub email_address: String,
}

#[derive(Debug, Serialize)]
pub struct RequestOtpResponse {
    pub success: bool,
    pub message: String,
    pub expires_in_seconds: i64,
}

#[derive(Debug, Deserialize)]
pub struct VerifyOtpRequest {
    pub email_address: String,
    pub otp_code: String,
}

#[derive(Debug, Serialize)]
pub struct VerifyOtpResponse {
    pub success: bool,
    pub token: Option<String>,
    pub tenant_id: Option<String>,
    pub message: String,
}

/// Request OTP via email
/// POST /api/v1/auth/email/request
pub async fn request_otp(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RequestOtpRequest>,
) -> Result<(StatusCode, Json<RequestOtpResponse>), (StatusCode, Json<Value>)> {
    debug!("OTP request for email: {}", payload.email_address);

    // Validate email format
    if !is_valid_email(&payload.email_address) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "Invalid email address format"
            })),
        ));
    }

    // Request OTP through auth service
    match state
        .auth_service
        .request_email_otp(&payload.email_address)
        .await
    {
        Ok(expires_in) => {
            info!("OTP sent to {}", payload.email_address);
            Ok((
                StatusCode::OK,
                Json(RequestOtpResponse {
                    success: true,
                    message: "OTP sent to your email address".to_string(),
                    expires_in_seconds: expires_in,
                }),
            ))
        }
        Err(e) => {
            error!("Failed to send OTP: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Failed to send OTP",
                    "details": e.to_string()
                })),
            ))
        }
    }
}

/// Verify OTP and create session
/// POST /api/v1/auth/email/verify
pub async fn verify_otp(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<VerifyOtpRequest>,
) -> Result<(StatusCode, Json<VerifyOtpResponse>), (StatusCode, Json<Value>)> {
    debug!(
        "OTP verification attempt for email: {}",
        payload.email_address
    );

    // Verify OTP through auth service
    match state
        .auth_service
        .verify_email_otp(&payload.email_address, &payload.otp_code)
        .await
    {
        Ok((tenant, token)) => {
            info!(
                "OTP verified successfully for tenant: {}",
                tenant.tenant_id
            );
            Ok((
                StatusCode::OK,
                Json(VerifyOtpResponse {
                    success: true,
                    token: Some(token),
                    tenant_id: Some(tenant.tenant_identifier.clone()),
                    message: "Authentication successful".to_string(),
                }),
            ))
        }
        Err(e) => {
            error!("OTP verification failed: {}", e);
            Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "error": "Invalid or expired OTP",
                    "details": e.to_string()
                })),
            ))
        }
    }
}

/// Basic email validation
fn is_valid_email(email: &str) -> bool {
    email.contains('@') && email.len() >= 5 && email.len() <= 255
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_validation_valid_emails() {
        assert!(is_valid_email("user@example.com"));
        assert!(is_valid_email("test@test.co.uk"));
        assert!(is_valid_email("first.last@company.com"));
        assert!(is_valid_email("user+tag@domain.org"));
        assert!(is_valid_email("a@b.c"));
        assert!(is_valid_email("test_user@example.com"));
        assert!(is_valid_email("123@456.789"));
    }

    #[test]
    fn test_email_validation_invalid_emails() {
        assert!(!is_valid_email("invalid"));
        assert!(!is_valid_email("@example.com"));
        assert!(!is_valid_email("user@"));
        assert!(!is_valid_email("user"));
        assert!(!is_valid_email("@"));
        assert!(!is_valid_email(""));
        assert!(!is_valid_email("a@b"));  // Too short
    }

    #[test]
    fn test_email_validation_edge_cases() {
        // Too short
        assert!(!is_valid_email("a@b"));
        assert!(!is_valid_email("a@bc"));
        assert!(is_valid_email("a@b.c"));  // Exactly 5 chars

        // Too long (over 255 chars)
        let long_email = format!("{}@example.com", "a".repeat(250));
        assert!(!is_valid_email(&long_email));

        // Multiple @ signs
        assert!(!is_valid_email("user@@example.com"));

        // Whitespace
        assert!(!is_valid_email("user @example.com"));
        assert!(!is_valid_email(" user@example.com"));
        assert!(!is_valid_email("user@example.com "));
    }

    #[test]
    fn test_email_validation_length_boundaries() {
        // 5 characters - minimum valid
        assert!(is_valid_email("a@b.c"));
        
        // 255 characters - maximum valid
        let max_email = format!("{}@b.c", "a".repeat(250));
        assert_eq!(max_email.len(), 255);
        assert!(is_valid_email(&max_email));
        
        // 256 characters - invalid
        let too_long = format!("{}@b.c", "a".repeat(251));
        assert_eq!(too_long.len(), 256);
        assert!(!is_valid_email(&too_long));
    }

    #[test]
    fn test_email_validation_special_characters() {
        assert!(is_valid_email("user+tag@example.com"));
        assert!(is_valid_email("user.name@example.com"));
        assert!(is_valid_email("user_name@example.com"));
        assert!(is_valid_email("user-name@example.com"));
        assert!(!is_valid_email("user name@example.com"));  // No spaces
    }

    #[test]
    fn test_request_otp_request_structure() {
        let request = RequestOtpRequest {
            email_address: "test@example.com".to_string(),
        };
        assert_eq!(request.email_address, "test@example.com");
    }

    #[test]
    fn test_request_otp_response_structure() {
        let response = RequestOtpResponse {
            success: true,
            message: "OTP sent".to_string(),
            expires_in_seconds: 600,
        };
        
        assert!(response.success);
        assert_eq!(response.expires_in_seconds, 600);
        assert!(!response.message.is_empty());
    }

    #[test]
    fn test_verify_otp_request_structure() {
        let request = VerifyOtpRequest {
            email_address: "test@example.com".to_string(),
            otp_code: "123456".to_string(),
        };
        
        assert_eq!(request.email_address, "test@example.com");
        assert_eq!(request.otp_code, "123456");
    }

    #[test]
    fn test_verify_otp_response_success() {
        let response = VerifyOtpResponse {
            success: true,
            token: Some("jwt.token.here".to_string()),
            tenant_id: Some("tenant123".to_string()),
            message: "Success".to_string(),
        };
        
        assert!(response.success);
        assert!(response.token.is_some());
        assert!(response.tenant_id.is_some());
    }

    #[test]
    fn test_verify_otp_response_failure() {
        let response = VerifyOtpResponse {
            success: false,
            token: None,
            tenant_id: None,
            message: "Invalid OTP".to_string(),
        };
        
        assert!(!response.success);
        assert!(response.token.is_none());
        assert!(response.tenant_id.is_none());
    }

    #[test]
    fn test_email_validation_case_sensitivity() {
        // Email addresses should work with any case
        assert!(is_valid_email("User@Example.COM"));
        assert!(is_valid_email("USER@EXAMPLE.COM"));
        assert!(is_valid_email("user@example.com"));
    }

    #[test]
    fn test_email_validation_international_domains() {
        // Basic ASCII domains
        assert!(is_valid_email("user@example.com"));
        assert!(is_valid_email("user@sub.domain.example.com"));
        
        // Domains with hyphens
        assert!(is_valid_email("user@my-domain.com"));
        assert!(is_valid_email("user@my-company.co.uk"));
    }
}