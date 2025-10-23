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
    fn test_email_validation() {
        assert!(is_valid_email("user@example.com"));
        assert!(is_valid_email("test+tag@domain.co.uk"));
        assert!(!is_valid_email("invalid"));
        assert!(!is_valid_email("@example.com"));
        assert!(!is_valid_email("user@"));
    }
}
