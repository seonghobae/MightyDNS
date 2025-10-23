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
