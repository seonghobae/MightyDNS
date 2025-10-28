use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tracing::{debug, error, info};
use uuid::Uuid;

use crate::AppState;

#[derive(Debug, Serialize)]
pub struct WhitelistEntry {
    pub entry_id: String,
    pub domain_name: String,
    pub added_reason: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct WhitelistResponse {
    pub whitelists: Vec<WhitelistEntry>,
    pub total: usize,
}

#[derive(Debug, Deserialize)]
pub struct AddWhitelistRequest {
    pub domain_name: String,
    pub reason: Option<String>,
}

/// Get whitelists for tenant
/// GET /api/v1/whitelists
pub async fn get_whitelists(
    State(state): State<Arc<AppState>>,
    // TODO: Extract tenant from JWT
) -> Result<(StatusCode, Json<WhitelistResponse>), (StatusCode, Json<Value>)> {
    debug!("Get whitelists request");

    // TODO: Get tenant from middleware
    // For now, return empty list
    Ok((
        StatusCode::OK,
        Json(WhitelistResponse {
            whitelists: vec![],
            total: 0,
        }),
    ))
}

/// Add domain to whitelist
/// POST /api/v1/whitelists
pub async fn add_whitelist(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<AddWhitelistRequest>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    debug!("Add whitelist request: {}", payload.domain_name);

    // Validate domain name
    if payload.domain_name.is_empty() || payload.domain_name.len() > 253 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "Invalid domain name"
            })),
        ));
    }

    // TODO: Get tenant from middleware
    // TODO: Add to whitelist in database

    info!("Domain {} added to whitelist", payload.domain_name);

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "success": true,
            "message": "Domain added to whitelist",
            "domain": payload.domain_name
        })),
    ))
}

/// Remove domain from whitelist
/// DELETE /api/v1/whitelists/:id
pub async fn delete_whitelist(
    State(state): State<Arc<AppState>>,
    Path(entry_id): Path<Uuid>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    debug!("Delete whitelist entry: {}", entry_id);

    // TODO: Get tenant from middleware
    // TODO: Verify entry belongs to tenant
    // TODO: Delete from database

    Ok((
        StatusCode::OK,
        Json(json!({
            "success": true,
            "message": "Whitelist entry deleted"
        })),
    ))
}
