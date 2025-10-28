use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tracing::{debug, error};

use crate::AppState;

#[derive(Debug, Serialize)]
pub struct BlocklistResponse {
    pub blocklists: Vec<BlocklistInfo>,
    pub total_domains: i64,
}

#[derive(Debug, Serialize)]
pub struct BlocklistInfo {
    pub source_id: String,
    pub source_name: String,
    pub domain_count: i64,
    pub is_enabled: bool,
}

/// Get active blocklists for tenant
/// GET /api/v1/blocklists
pub async fn get_blocklists(
    State(_state): State<Arc<AppState>>,
    // TODO: Extract tenant from JWT via middleware
) -> Result<(StatusCode, Json<BlocklistResponse>), (StatusCode, Json<Value>)> {
    debug!("Get blocklists request");

    // TODO: Get tenant from auth middleware
    // TODO: Query database for active blocklists

    Err((
        StatusCode::NOT_IMPLEMENTED,
        Json(json!({
            "error": "Not yet implemented"
        })),
    ))
}
