use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tracing::{debug, error, info};

use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct RegisterStartRequest {
    pub email_address: String,
}

#[derive(Debug, Serialize)]
pub struct RegisterStartResponse {
    pub success: bool,
    pub challenge: Value,
}

#[derive(Debug, Deserialize)]
pub struct RegisterFinishRequest {
    pub email_address: String,
    pub credential: Value,
}

#[derive(Debug, Serialize)]
pub struct RegisterFinishResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginStartRequest {
    pub email_address: String,
}

#[derive(Debug, Serialize)]
pub struct LoginStartResponse {
    pub success: bool,
    pub challenge: Value,
}

#[derive(Debug, Deserialize)]
pub struct LoginFinishRequest {
    pub email_address: String,
    pub credential: Value,
}

#[derive(Debug, Serialize)]
pub struct LoginFinishResponse {
    pub success: bool,
    pub token: Option<String>,
    pub tenant_id: Option<String>,
    pub message: String,
}

/// Start WebAuthn registration
/// POST /api/v1/auth/webauthn/register/start
pub async fn register_start(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RegisterStartRequest>,
) -> Result<(StatusCode, Json<RegisterStartResponse>), (StatusCode, Json<Value>)> {
    debug!("WebAuthn registration start for: {}", payload.email_address);

    match state
        .auth_service
        .webauthn_register_start(&payload.email_address)
        .await
    {
        Ok(challenge) => {
            info!("WebAuthn registration challenge created for {}", payload.email_address);
            Ok((
                StatusCode::OK,
                Json(RegisterStartResponse {
                    success: true,
                    challenge,
                }),
            ))
        }
        Err(e) => {
            error!("Failed to start WebAuthn registration: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Failed to start registration",
                    "error": "Operation failed"
                })),
            ))
        }
    }
}

/// Finish WebAuthn registration
/// POST /api/v1/auth/webauthn/register/finish
pub async fn register_finish(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RegisterFinishRequest>,
) -> Result<(StatusCode, Json<RegisterFinishResponse>), (StatusCode, Json<Value>)> {
    debug!("WebAuthn registration finish for: {}", payload.email_address);

    match state
        .auth_service
        .webauthn_register_finish(&payload.email_address, payload.credential)
        .await
    {
        Ok(_) => {
            info!("WebAuthn registration successful for {}", payload.email_address);
            Ok((
                StatusCode::OK,
                Json(RegisterFinishResponse {
                    success: true,
                    message: "WebAuthn credential registered successfully".to_string(),
                }),
            ))
        }
        Err(e) => {
            error!("Failed to finish WebAuthn registration: {}", e);
            Err((
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "Failed to complete registration",
                    "error": "Operation failed"
                })),
            ))
        }
    }
}

/// Start WebAuthn login
/// POST /api/v1/auth/webauthn/login/start
pub async fn login_start(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LoginStartRequest>,
) -> Result<(StatusCode, Json<LoginStartResponse>), (StatusCode, Json<Value>)> {
    debug!("WebAuthn login start for: {}", payload.email_address);

    match state
        .auth_service
        .webauthn_login_start(&payload.email_address)
        .await
    {
        Ok(challenge) => {
            info!("WebAuthn login challenge created for {}", payload.email_address);
            Ok((
                StatusCode::OK,
                Json(LoginStartResponse {
                    success: true,
                    challenge,
                }),
            ))
        }
        Err(e) => {
            error!("Failed to start WebAuthn login: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Failed to start login",
                    "error": "Operation failed"
                })),
            ))
        }
    }
}

/// Finish WebAuthn login
/// POST /api/v1/auth/webauthn/login/finish
pub async fn login_finish(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LoginFinishRequest>,
) -> Result<(StatusCode, Json<LoginFinishResponse>), (StatusCode, Json<Value>)> {
    debug!("WebAuthn login finish for: {}", payload.email_address);

    match state
        .auth_service
        .webauthn_login_finish(&payload.email_address, payload.credential)
        .await
    {
        Ok((tenant, token)) => {
            info!("WebAuthn login successful for tenant: {}", tenant.tenant_id);
            Ok((
                StatusCode::OK,
                Json(LoginFinishResponse {
                    success: true,
                    token: Some(token),
                    tenant_id: Some(tenant.tenant_identifier.clone()),
                    message: "Authentication successful".to_string(),
                }),
            ))
        }
        Err(e) => {
            error!("Failed to finish WebAuthn login: {}", e);
            Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "error": "Authentication failed",
                    "error": "Operation failed"
                })),
            ))
        }
    }
}
