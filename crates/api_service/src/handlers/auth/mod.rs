pub mod email;
pub mod totp;
pub mod webauthn;

use axum::{extract::{Extension, State}, http::StatusCode, Json};
use serde_json::{json, Value};
use std::sync::Arc;
use tracing::{error, info};

use crate::{AppState, middleware::auth::Claims};

/// Logout endpoint - invalidate session
/// Requires: Authorization header with valid JWT
pub async fn logout(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    // Revoke JWT by storing jti in Valkey with TTL
    let ttl_seconds = (claims.exp - chrono::Utc::now().timestamp()).max(0) as u64;

    // Store revoked JTI in cache with TTL matching token expiry
    if let Err(e) = state.cache.set_with_ttl(
        &format!("revoked_jti:{}", claims.jti),
        "1",
        ttl_seconds,
    ).await {
        error!("Failed to revoke JWT: {}", e);
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": "Failed to logout"
            })),
        ));
    }

    info!("User logged out: {}", claims.email);

    Ok((
        StatusCode::OK,
        Json(json!({
            "success": true,
            "message": "Logged out successfully"
        })),
    ))
}
