use anyhow::{anyhow, Result};
use chrono::{Duration, Utc};
use common::{config::Config, database::Database, models::TenantAccount, nats_client::NatsClient};
use jsonwebtoken::{encode, EncodingKey, Header};
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tracing::{debug, error, info};
use uuid::Uuid;

/// JWT claims structure
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,       // tenant_id (UUID)
    pub email: String,     // email_address
    pub tenant_id: String, // tenant_identifier
    pub jti: String,       // JWT ID for revocation
    pub exp: i64,          // expiration timestamp
    pub iat: i64,          // issued at timestamp
}

/// Authentication service
pub struct AuthService {
    config: Arc<Config>,
    database: Database,
    nats: NatsClient,
}

impl AuthService {
    pub fn new(config: Arc<Config>, database: Database, nats: NatsClient) -> Self {
        Self {
            config,
            database,
            nats,
        }
    }

    // ============================================================================
    // Email OTP Authentication
    // ============================================================================

    /// Request OTP via email
    pub async fn request_email_otp(&self, email: &str) -> Result<i64> {
        debug!("Requesting OTP for email: {}", email);

        // Check if tenant exists, create if not
        let tenant = match self.database.get_tenant_by_email(email).await? {
            Some(t) => {
                debug!("Existing tenant found: {}", t.tenant_id);
                t
            }
            None => {
                info!("Creating new tenant for email: {}", email);
                self.create_tenant(email).await?
            }
        };

        // Generate 6-digit OTP
        let otp_code = generate_otp();
        debug!("Generated OTP for tenant");

        // Store OTP in database with expiration
        let expires_at = Utc::now() + Duration::minutes(self.config.auth.otp_expiry_minutes);
        self.database
            .store_email_otp(&tenant.tenant_id, &otp_code, expires_at)
            .await?;

        // Send email via NATS (async)
        let email_message = common::nats_client::messages::EmailMessage {
            to: email.to_string(),
            subject: "Your MightyDNS Login Code".to_string(),
            template: "email_otp".to_string(),
            variables: json!({
                "otp_code": otp_code,
                "expires_in_minutes": self.config.auth.otp_expiry_minutes,
            }),
            priority: common::nats_client::messages::EmailPriority::High,
        };

        if let Err(e) = self.nats.publish("email.send", &email_message).await {
            error!("Failed to publish email send message: {}", e);
            // Don't fail the request, just log the error
        }

        Ok(self.config.auth.otp_expiry_minutes * 60)
    }

    /// Verify OTP and create session
    pub async fn verify_email_otp(&self, email: &str, otp_code: &str) -> Result<(TenantAccount, String)> {
        debug!("Verifying OTP for email: {}", email);

        // Get tenant
        let tenant = self
            .database
            .get_tenant_by_email(email)
            .await?
            .ok_or_else(|| anyhow!("Tenant not found"))?;

        // Verify OTP
        let is_valid = self
            .database
            .verify_email_otp(&tenant.tenant_id, otp_code)
            .await?;

        if !is_valid {
            return Err(anyhow!("Invalid or expired OTP"));
        }

        // Delete used OTP
        self.database
            .delete_email_otp(&tenant.tenant_id)
            .await?;

        // Generate JWT token
        let token = self.generate_jwt_token(&tenant)?;

        // Create session
        self.create_session(&tenant.tenant_id, &token).await?;

        info!("OTP verified successfully for tenant: {}", tenant.tenant_id);

        Ok((tenant, token))
    }

    // ============================================================================
    // TOTP Authentication
    // ============================================================================

    /// Setup TOTP for a tenant
    pub async fn setup_totp(&self, tenant_identifier: &str) -> Result<(String, String, String)> {
        debug!("Setting up TOTP for tenant: {}", tenant_identifier);

        // Get tenant
        let tenant = self
            .database
            .get_tenant_by_identifier(tenant_identifier)
            .await?
            .ok_or_else(|| anyhow!("Tenant not found"))?;

        // Generate TOTP secret
        let secret = generate_totp_secret();

        // Store TOTP secret in database
        self.database
            .store_totp_secret(&tenant.tenant_id, &secret)
            .await?;

        // Generate QR code URL
        let account_name = format!("{}:{}", "MightyDNS", tenant.email_address);
        let qr_url = format!(
            "otpauth://totp/{}?secret={}&issuer=MightyDNS",
            urlencoding::encode(&account_name),
            secret
        );

        info!("TOTP setup successful for tenant: {}", tenant.tenant_id);

        Ok((secret, qr_url, account_name))
    }

    /// Verify TOTP code
    pub async fn verify_totp(&self, email: &str, totp_code: &str) -> Result<(TenantAccount, String)> {
        debug!("Verifying TOTP for email: {}", email);

        // Get tenant
        let tenant = self
            .database
            .get_tenant_by_email(email)
            .await?
            .ok_or_else(|| anyhow!("Tenant not found"))?;

        // Verify TOTP
        let is_valid = self
            .database
            .verify_totp(&tenant.tenant_id, totp_code)
            .await?;

        if !is_valid {
            return Err(anyhow!("Invalid TOTP code"));
        }

        // Generate JWT token
        let token = self.generate_jwt_token(&tenant)?;

        // Create session
        self.create_session(&tenant.tenant_id, &token).await?;

        info!("TOTP verified successfully for tenant: {}", tenant.tenant_id);

        Ok((tenant, token))
    }

    // ============================================================================
    // WebAuthn Authentication
    // ============================================================================

    /// Start WebAuthn registration
    pub async fn webauthn_register_start(&self, email: &str) -> Result<Value> {
        debug!("Starting WebAuthn registration for: {}", email);

        // TODO: Implement WebAuthn registration using webauthn-rs
        // For now, return a placeholder
        Ok(json!({
            "challenge": "base64_encoded_challenge",
            "rp": {
                "name": "MightyDNS",
                "id": self.config.auth.webauthn_rp_id
            },
            "user": {
                "id": "base64_user_id",
                "name": email,
                "displayName": email
            },
            "pubKeyCredParams": [
                {"type": "public-key", "alg": -7},
                {"type": "public-key", "alg": -257}
            ],
            "timeout": 60000,
            "attestation": "none",
            "authenticatorSelection": {
                "authenticatorAttachment": "platform",
                "requireResidentKey": false,
                "userVerification": "preferred"
            }
        }))
    }

    /// Finish WebAuthn registration
    pub async fn webauthn_register_finish(&self, _email: &str, _credential: Value) -> Result<()> {
        // TODO: Implement WebAuthn registration finish
        debug!("WebAuthn registration finish - not yet implemented");
        Ok(())
    }

    /// Start WebAuthn login
    pub async fn webauthn_login_start(&self, email: &str) -> Result<Value> {
        debug!("Starting WebAuthn login for: {}", email);

        // TODO: Implement WebAuthn login using webauthn-rs
        Ok(json!({
            "challenge": "base64_encoded_challenge",
            "timeout": 60000,
            "rpId": self.config.auth.webauthn_rp_id,
            "userVerification": "preferred"
        }))
    }

    /// Finish WebAuthn login
    pub async fn webauthn_login_finish(&self, email: &str, _credential: Value) -> Result<(TenantAccount, String)> {
        debug!("Finishing WebAuthn login for: {}", email);

        // TODO: Implement WebAuthn login finish with actual credential verification
        // SECURITY: Do not remove this error until full WebAuthn verification is implemented
        // Issuing tokens without credential verification is a critical security vulnerability
        Err(anyhow!("WebAuthn authentication not yet implemented. Use Email OTP or TOTP instead."))
    }

    // ============================================================================
    // Helper Methods
    // ============================================================================

    /// Create a new tenant account
    async fn create_tenant(&self, email: &str) -> Result<TenantAccount> {
        let tenant_id = Uuid::new_v4();
        let tenant_identifier = generate_tenant_identifier();

        info!(
            "Creating tenant: {} with identifier: {}",
            tenant_id, tenant_identifier
        );

        self.database
            .create_tenant(email, &tenant_identifier)
            .await
    }

    /// Generate JWT token for tenant
    fn generate_jwt_token(&self, tenant: &TenantAccount) -> Result<String> {
        let now = Utc::now();
        let exp = now + Duration::hours(self.config.auth.jwt_expiry_hours);

        // Generate unique JWT ID for revocation support
        let jti = Uuid::new_v4().to_string();

        let claims = Claims {
            sub: tenant.tenant_id.to_string(),
            email: tenant.email_address.clone(),
            tenant_id: tenant.tenant_identifier.clone(),
            jti,
            exp: exp.timestamp(),
            iat: now.timestamp(),
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.config.auth.jwt_secret.as_bytes()),
        )?;

        Ok(token)
    }

    /// Create session in database
    async fn create_session(&self, tenant_id: &Uuid, token: &str) -> Result<()> {
        let expires_at = Utc::now() + Duration::hours(self.config.auth.jwt_expiry_hours);

        self.database
            .create_session(tenant_id, token, expires_at)
            .await
    }
}

/// Generate 6-digit OTP
fn generate_otp() -> String {
    let mut rng = rand::thread_rng();
    format!("{:06}", rng.gen_range(0..1000000))
}

/// Generate TOTP secret (base32 encoded)
fn generate_totp_secret() -> String {
    use rand::distributions::Alphanumeric;
    use rand::Rng;

    let secret: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();

    // Convert to base32 (simplified - should use proper base32 encoding)
    secret.to_uppercase()
}

/// Generate random tenant identifier (8-32 chars, alphanumeric)
fn generate_tenant_identifier() -> String {
    use rand::distributions::Alphanumeric;
    use rand::Rng;

    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(16)
        .map(|c| c.to_ascii_lowercase() as char)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_otp_format() {
        for _ in 0..100 {
            let otp = generate_otp();
            assert_eq!(otp.len(), 6, "OTP should be 6 digits");
            assert!(otp.chars().all(|c| c.is_ascii_digit()), "OTP should only contain digits");
            let num: u32 = otp.parse().expect("OTP should be a valid number");
            assert!(num < 1_000_000, "OTP should be less than 1000000");
        }
    }

    #[test]
    fn test_generate_otp_uniqueness() {
        let mut otps = std::collections::HashSet::new();
        for _ in 0..50 {
            let otp = generate_otp();
            otps.insert(otp);
        }
        // With random generation, we should get mostly unique values
        assert!(otps.len() > 40, "Should generate mostly unique OTPs");
    }

    #[test]
    fn test_generate_totp_secret_length() {
        for _ in 0..10 {
            let secret = generate_totp_secret();
            assert_eq!(secret.len(), 32, "TOTP secret should be 32 characters");
            assert!(secret.chars().all(|c| c.is_ascii_alphanumeric()), 
                "TOTP secret should only contain alphanumeric characters");
            assert!(secret.chars().all(|c| c.is_ascii_uppercase()),
                "TOTP secret should be uppercase");
        }
    }

    #[test]
    fn test_generate_tenant_identifier_format() {
        for _ in 0..20 {
            let identifier = generate_tenant_identifier();
            assert_eq!(identifier.len(), 16, "Tenant identifier should be 16 characters");
            assert!(identifier.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()),
                "Tenant identifier should only contain lowercase alphanumeric");
        }
    }

    #[test]
    fn test_generate_tenant_identifier_uniqueness() {
        let mut identifiers = std::collections::HashSet::new();
        for _ in 0..50 {
            let id = generate_tenant_identifier();
            identifiers.insert(id);
        }
        assert_eq!(identifiers.len(), 50, "Should generate unique identifiers");
    }

    #[test]
    fn test_claims_structure() {
        let claims = Claims {
            sub: "tenant-123".to_string(),
            email: "test@example.com".to_string(),
            tenant_id: "abc123def456".to_string(),
            exp: 1234567890,
            iat: 1234567800,
        };

        assert_eq!(claims.sub, "tenant-123");
        assert_eq!(claims.email, "test@example.com");
        assert!(claims.exp > claims.iat);
    }

    #[test]
    fn test_claims_serialization() {
        let claims = Claims {
            sub: Uuid::new_v4().to_string(),
            email: "user@test.com".to_string(),
            tenant_id: "testid".to_string(),
            exp: Utc::now().timestamp() + 3600,
            iat: Utc::now().timestamp(),
        };

        let json = serde_json::to_string(&claims).expect("Serialization failed");
        let deserialized: Claims = serde_json::from_str(&json).expect("Deserialization failed");

        assert_eq!(claims.sub, deserialized.sub);
        assert_eq!(claims.email, deserialized.email);
        assert_eq!(claims.tenant_id, deserialized.tenant_id);
    }

    #[test]
    fn test_otp_leading_zeros() {
        // Test that OTPs can have leading zeros
        for _ in 0..1000 {
            let otp = generate_otp();
            assert_eq!(otp.len(), 6);
            // Verify that parsing works even with leading zeros
            let _: u32 = otp.parse().expect("Should parse as number");
        }
    }

    #[test]
    fn test_totp_secret_randomness() {
        let secret1 = generate_totp_secret();
        let secret2 = generate_totp_secret();
        assert_ne!(secret1, secret2, "TOTP secrets should be unique");
    }

    #[test]
    fn test_tenant_identifier_no_special_chars() {
        for _ in 0..30 {
            let id = generate_tenant_identifier();
            assert!(!id.contains('-'));
            assert!(!id.contains('_'));
            assert!(!id.contains(' '));
            assert!(!id.contains('.'));
        }
    }
}