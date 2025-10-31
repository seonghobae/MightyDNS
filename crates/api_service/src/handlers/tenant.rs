use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tracing::{debug, error};

use crate::AppState;

#[derive(Debug, Serialize)]
pub struct TenantProfile {
    pub tenant_id: String,
    pub email_address: String,
    pub tenant_identifier: String,
    pub subscription_tier: String,
    pub subscription_status: String,
    pub account_created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProfileRequest {
    pub tenant_identifier: Option<String>,
}

/// Get tenant profile (requires authentication)
/// GET /api/v1/tenant/profile
pub async fn get_profile(
    State(_state): State<Arc<AppState>>,
    // TODO: Extract tenant from JWT via middleware
) -> Result<(StatusCode, Json<TenantProfile>), (StatusCode, Json<Value>)> {
    debug!("Get tenant profile request");

    // TODO: Get tenant from auth middleware claims
    // For now, return mock data
    Err((
        StatusCode::NOT_IMPLEMENTED,
        Json(json!({
            "error": "Authentication middleware not yet implemented"
        })),
    ))
}

/// Update tenant profile (requires authentication)
/// POST /api/v1/tenant/profile
pub async fn update_profile(
    State(_state): State<Arc<AppState>>,
    Json(_payload): Json<UpdateProfileRequest>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    debug!("Update tenant profile request");

    // TODO: Implement profile update
    Err((
        StatusCode::NOT_IMPLEMENTED,
        Json(json!({
            "error": "Not yet implemented"
        })),
    ))
}
