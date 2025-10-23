use axum::{
    extract::Request,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};

/// JWT claims structure (must match services::auth::Claims)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,       // tenant_id
    pub email: String,     // email_address
    pub tenant_id: String, // tenant_identifier
    pub exp: i64,          // expiration timestamp
    pub iat: i64,          // issued at timestamp
}

/// Authentication middleware - validates JWT tokens
pub async fn auth_middleware(
    headers: HeaderMap,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Extract Authorization header
    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Check for Bearer token
    if !auth_header.starts_with("Bearer ") {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let token = &auth_header[7..];

    // TODO: Get JWT secret from app state
    let jwt_secret = std::env::var("MIGHTYDNS__AUTH__JWT_SECRET")
        .unwrap_or_else(|_| "change-me-in-production".to_string());

    // Decode and validate JWT
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Add claims to request extensions for handlers to access
    req.extensions_mut().insert(token_data.claims);

    Ok(next.run(req).await)
}
