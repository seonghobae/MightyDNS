pub mod email;
pub mod totp;
pub mod webauthn;

use axum::{extract::State, http::StatusCode, Json};
use serde_json::{json, Value};
use std::sync::Arc;

use crate::AppState;

/// Logout endpoint - invalidate session
pub async fn logout(
    State(_state): State<Arc<AppState>>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    // TODO: Implement session invalidation
    // For now, client-side will delete JWT token
    Ok((
        StatusCode::OK,
        Json(json!({
            "success": true,
            "message": "Logged out successfully"
        })),
    ))
}
