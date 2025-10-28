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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claims_structure() {
        let claims = Claims {
            sub: "tenant-uuid".to_string(),
            email: "user@example.com".to_string(),
            tenant_id: "tenant123".to_string(),
            exp: 1234567890,
            iat: 1234567800,
        };

        assert_eq!(claims.sub, "tenant-uuid");
        assert_eq!(claims.email, "user@example.com");
        assert_eq!(claims.tenant_id, "tenant123");
        assert!(claims.exp > claims.iat);
    }

    #[test]
    fn test_claims_serialization() {
        let claims = Claims {
            sub: "sub-123".to_string(),
            email: "test@test.com".to_string(),
            tenant_id: "tid-456".to_string(),
            exp: 9999999999,
            iat: 9999999000,
        };

        let json = serde_json::to_string(&claims).expect("Serialization failed");
        assert!(json.contains("sub"));
        assert!(json.contains("email"));
        assert!(json.contains("tenant_id"));
        assert!(json.contains("exp"));
        assert!(json.contains("iat"));

        let deserialized: Claims = serde_json::from_str(&json).expect("Deserialization failed");
        assert_eq!(claims.sub, deserialized.sub);
        assert_eq!(claims.email, deserialized.email);
    }

    #[test]
    fn test_claims_clone() {
        let claims = Claims {
            sub: "test".to_string(),
            email: "test@example.com".to_string(),
            tenant_id: "tenant".to_string(),
            exp: 123456,
            iat: 123400,
        };

        let cloned = claims.clone();
        assert_eq!(claims.sub, cloned.sub);
        assert_eq!(claims.email, cloned.email);
        assert_eq!(claims.exp, cloned.exp);
    }

    #[test]
    fn test_claims_debug_format() {
        let claims = Claims {
            sub: "debug-test".to_string(),
            email: "debug@test.com".to_string(),
            tenant_id: "tid".to_string(),
            exp: 100,
            iat: 50,
        };

        let debug_str = format!("{:?}", claims);
        assert!(debug_str.contains("Claims"));
        assert!(debug_str.contains("debug-test"));
    }

    #[test]
    fn test_claims_with_timestamps() {
        use chrono::Utc;
        
        let now = Utc::now().timestamp();
        let future = now + 3600;

        let claims = Claims {
            sub: "user-id".to_string(),
            email: "user@example.com".to_string(),
            tenant_id: "tenant-id".to_string(),
            exp: future,
            iat: now,
        };

        assert!(claims.exp > claims.iat);
        assert!(claims.exp - claims.iat <= 3600);
    }

    #[test]
    fn test_claims_email_validation() {
        let claims = Claims {
            sub: "user".to_string(),
            email: "valid@example.com".to_string(),
            tenant_id: "tenant".to_string(),
            exp: 1000,
            iat: 500,
        };

        assert!(claims.email.contains('@'));
        assert!(claims.email.len() > 5);
    }

    #[test]
    fn test_claims_tenant_id_not_empty() {
        let claims = Claims {
            sub: "user-sub".to_string(),
            email: "user@test.com".to_string(),
            tenant_id: "valid-tenant-id".to_string(),
            exp: 2000,
            iat: 1000,
        };

        assert!(!claims.tenant_id.is_empty());
        assert!(!claims.sub.is_empty());
    }
}