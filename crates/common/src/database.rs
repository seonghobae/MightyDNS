use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::Row;
use std::time::Duration;
use uuid::Uuid;

use crate::config::DatabaseConfig;
use crate::error::{Error, Result};
use crate::models::*;

/// Database connection pool wrapper
#[derive(Clone)]
pub struct Database {
    pool: PgPool,
}

impl Database {
    /// Create a new database connection pool
    pub async fn new(config: &DatabaseConfig) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .acquire_timeout(Duration::from_secs(config.connect_timeout_secs))
            .idle_timeout(Duration::from_secs(config.idle_timeout_secs))
            .test_before_acquire(true)
            .connect(&config.url)
            .await?;

        tracing::info!("Database connection pool established");

        Ok(Self { pool })
    }

    /// Get a reference to the underlying pool
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Health check - verify database connectivity
    pub async fn health_check(&self) -> Result<()> {
        sqlx::query("SELECT 1")
            .fetch_one(&self.pool)
            .await?;
        Ok(())
    }

    // ============================================================================
    // Tenant Account Queries
    // ============================================================================

    /// Get tenant account by ID
    pub async fn get_tenant_by_id(&self, tenant_id: &Uuid) -> Result<Option<TenantAccount>> {
        let tenant = sqlx::query_as!(
            TenantAccount,
            r#"
            SELECT
                tenant_id,
                email_address,
                tenant_identifier,
                subscription_tier as "subscription_tier: SubscriptionTier",
                subscription_status as "subscription_status: SubscriptionStatus",
                account_created_at,
                account_updated_at,
                is_account_active
            FROM tenant_account
            WHERE tenant_id = $1 AND is_account_active = TRUE
            "#,
            tenant_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(tenant)
    }

    /// Get tenant account by email
    pub async fn get_tenant_by_email(&self, email: &str) -> Result<Option<TenantAccount>> {
        let tenant = sqlx::query_as!(
            TenantAccount,
            r#"
            SELECT
                tenant_id,
                email_address,
                tenant_identifier,
                subscription_tier as "subscription_tier: SubscriptionTier",
                subscription_status as "subscription_status: SubscriptionStatus",
                account_created_at,
                account_updated_at,
                is_account_active
            FROM tenant_account
            WHERE email_address = $1 AND is_account_active = TRUE
            "#,
            email
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(tenant)
    }

    /// Get tenant account by identifier (for DNS lookups)
    pub async fn get_tenant_by_identifier(&self, identifier: &str) -> Result<Option<TenantAccount>> {
        let tenant = sqlx::query_as!(
            TenantAccount,
            r#"
            SELECT
                tenant_id,
                email_address,
                tenant_identifier,
                subscription_tier as "subscription_tier: SubscriptionTier",
                subscription_status as "subscription_status: SubscriptionStatus",
                account_created_at,
                account_updated_at,
                is_account_active
            FROM tenant_account
            WHERE tenant_identifier = $1 AND is_account_active = TRUE
            "#,
            identifier
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(tenant)
    }

    /// Create a new tenant account
    pub async fn create_tenant(&self, email: &str, identifier: &str) -> Result<TenantAccount> {
        let tenant = sqlx::query_as!(
            TenantAccount,
            r#"
            INSERT INTO tenant_account (email_address, tenant_identifier)
            VALUES ($1, $2)
            RETURNING
                tenant_id,
                email_address,
                tenant_identifier,
                subscription_tier as "subscription_tier: SubscriptionTier",
                subscription_status as "subscription_status: SubscriptionStatus",
                account_created_at,
                account_updated_at,
                is_account_active
            "#,
            email,
            identifier
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(tenant)
    }

    // ============================================================================
    // Tenant Config Queries
    // ============================================================================

    /// Get tenant config by ID with tenant isolation
    pub async fn get_config_by_id(&self, tenant_id: &Uuid, config_id: &Uuid) -> Result<Option<TenantConfig>> {
        let config = sqlx::query_as!(
            TenantConfig,
            r#"
            SELECT
                config_id,
                tenant_id,
                config_name,
                config_description,
                is_logging_enabled,
                is_dnssec_enabled,
                blocked_response_ip::TEXT as "blocked_response_ip?",
                config_created_at,
                config_updated_at,
                is_config_active
            FROM tenant_config
            WHERE config_id = $1 AND tenant_id = $2 AND is_config_active = TRUE
            "#,
            config_id,
            tenant_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(config)
    }

    /// Get all active configs for a tenant
    pub async fn get_tenant_configs(&self, tenant_id: &Uuid) -> Result<Vec<TenantConfig>> {
        let configs = sqlx::query_as!(
            TenantConfig,
            r#"
            SELECT
                config_id,
                tenant_id,
                config_name,
                config_description,
                is_logging_enabled,
                is_dnssec_enabled,
                blocked_response_ip::TEXT as "blocked_response_ip?",
                config_created_at,
                config_updated_at,
                is_config_active
            FROM tenant_config
            WHERE tenant_id = $1 AND is_config_active = TRUE
            ORDER BY config_created_at ASC
            "#,
            tenant_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(configs)
    }

    // ============================================================================
    // IP Binding Queries
    // ============================================================================

    /// Get tenant by IP address (for UDP/53 lookups)
    pub async fn get_tenant_by_ip(&self, ip_address: &str) -> Result<Option<(TenantAccount, TenantConfig)>> {
        let result = sqlx::query!(
            r#"
            SELECT
                ta.tenant_id,
                ta.email_address,
                ta.tenant_identifier,
                ta.subscription_tier as "subscription_tier: SubscriptionTier",
                ta.subscription_status as "subscription_status: SubscriptionStatus",
                ta.account_created_at,
                ta.account_updated_at,
                ta.is_account_active,
                tc.config_id,
                tc.config_name,
                tc.config_description,
                tc.is_logging_enabled,
                tc.is_dnssec_enabled,
                tc.blocked_response_ip::TEXT as "blocked_response_ip?",
                tc.config_created_at,
                tc.config_updated_at,
                tc.is_config_active
            FROM tenant_ip_binding tib
            JOIN tenant_account ta ON tib.tenant_id = ta.tenant_id
            JOIN tenant_config tc ON tib.config_id = tc.config_id
            WHERE tib.ip_address = $1::inet
              AND tib.is_binding_active = TRUE
              AND ta.is_account_active = TRUE
              AND tc.is_config_active = TRUE
              AND (tib.binding_expires_at IS NULL OR tib.binding_expires_at > NOW())
            LIMIT 1
            "#,
            ip_address
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = result {
            let tenant = TenantAccount {
                tenant_id: row.tenant_id,
                email_address: row.email_address,
                tenant_identifier: row.tenant_identifier,
                subscription_tier: row.subscription_tier,
                subscription_status: row.subscription_status,
                account_created_at: row.account_created_at,
                account_updated_at: row.account_updated_at,
                is_account_active: row.is_account_active,
            };

            let config = TenantConfig {
                config_id: row.config_id,
                tenant_id: row.tenant_id,
                config_name: row.config_name,
                config_description: row.config_description,
                is_logging_enabled: row.is_logging_enabled,
                is_dnssec_enabled: row.is_dnssec_enabled,
                blocked_response_ip: row.blocked_response_ip,
                config_created_at: row.config_created_at,
                config_updated_at: row.config_updated_at,
                is_config_active: row.is_config_active,
            };

            Ok(Some((tenant, config)))
        } else {
            Ok(None)
        }
    }

    // ============================================================================
    // Blocklist/Whitelist Queries
    // ============================================================================

    /// Check if domain is in blocklist
    pub async fn is_domain_blocked(&self, domain: &str) -> Result<Option<BlockListEntry>> {
        let entry = sqlx::query_as!(
            BlockListEntry,
            r#"
            SELECT
                block_entry_id,
                block_domain_name,
                source_id,
                block_category as "block_category: BlockCategory",
                block_reason,
                block_added_at,
                block_updated_at,
                is_block_active
            FROM block_list_entry
            WHERE block_domain_name = $1 AND is_block_active = TRUE
            LIMIT 1
            "#,
            domain
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(entry)
    }

    /// Check if domain is in tenant's whitelist
    pub async fn is_domain_whitelisted(&self, tenant_id: &Uuid, domain: &str) -> Result<bool> {
        let exists = sqlx::query!(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM white_list_entry
                WHERE tenant_id = $1
                  AND white_domain_name = $2
                  AND is_white_active = TRUE
            ) as "exists!"
            "#,
            tenant_id,
            domain
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(exists.exists)
    }

    /// Check domain against blocklist with tenant config
    pub async fn check_domain_filter(
        &self,
        tenant_id: &Uuid,
        config_id: &Uuid,
        domain: &str,
    ) -> Result<DnsResultType> {
        // Use the PostgreSQL function we created
        let result = sqlx::query!(
            r#"
            SELECT
                is_blocked,
                is_whitelisted
            FROM check_domain_blocklist($1, $2, $3)
            "#,
            domain,
            tenant_id,
            config_id
        )
        .fetch_one(&self.pool)
        .await?;

        if result.is_whitelisted.unwrap_or(false) {
            Ok(DnsResultType::Whitelisted)
        } else if result.is_blocked.unwrap_or(false) {
            Ok(DnsResultType::Blocked)
        } else {
            Ok(DnsResultType::Allowed)
        }
    }

    // ============================================================================
    // DNS Query Logging
    // ============================================================================

    /// Log a DNS query (should be called asynchronously via NATS)
    pub async fn log_dns_query(
        &self,
        tenant_id: &Uuid,
        config_id: Option<&Uuid>,
        domain: &str,
        query_type: DnsQueryType,
        result_type: DnsResultType,
        response_ip: Option<&str>,
        latency_ms: Option<i16>,
        source_protocol: Option<&str>,
        source_ip: Option<&str>,
    ) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO dns_query_log (
                tenant_id,
                config_id,
                query_domain_name,
                query_type,
                query_result_type,
                response_ip_address,
                query_latency_ms,
                query_source_protocol,
                query_source_ip
            )
            VALUES ($1, $2, $3, $4, $5, $6::inet, $7, $8, $9::inet)
            "#,
            tenant_id,
            config_id,
            domain,
            query_type as DnsQueryType,
            result_type as DnsResultType,
            response_ip,
            latency_ms,
            source_protocol,
            source_ip
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // ============================================================================
    // Authentication Queries
    // ============================================================================

    /// Get active credentials for a tenant
    pub async fn get_tenant_credentials(&self, tenant_id: &Uuid) -> Result<Vec<AuthCredential>> {
        let credentials = sqlx::query_as!(
            AuthCredential,
            r#"
            SELECT
                credential_id,
                tenant_id,
                credential_type as "credential_type: CredentialType",
                credential_public_key,
                credential_counter,
                totp_secret_key,
                credential_name,
                credential_created_at,
                credential_last_used_at,
                is_credential_active
            FROM auth_credential
            WHERE tenant_id = $1 AND is_credential_active = TRUE
            ORDER BY credential_created_at DESC
            "#,
            tenant_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(credentials)
    }

    /// Create a new auth session (full version with IP and user agent)
    pub async fn create_session_full(
        &self,
        tenant_id: &Uuid,
        token_hash: &str,
        ip_address: &str,
        user_agent: Option<&str>,
        expires_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<AuthSession> {
        let session = sqlx::query_as!(
            AuthSession,
            r#"
            INSERT INTO auth_session (
                tenant_id,
                session_token_hash,
                session_ip_address,
                session_user_agent,
                session_expires_at
            )
            VALUES ($1, $2, $3::inet, $4, $5)
            RETURNING
                session_id,
                tenant_id,
                session_token_hash,
                session_ip_address::TEXT as "session_ip_address!",
                session_user_agent,
                session_created_at,
                session_expires_at,
                session_last_activity_at,
                is_session_active
            "#,
            tenant_id,
            token_hash,
            ip_address,
            user_agent,
            expires_at
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(session)
    }

    /// Create a new auth session (simplified for API auth)
    pub async fn create_session(
        &self,
        tenant_id: &Uuid,
        token: &str,
        expires_at: chrono::DateTime<chrono::Utc>,
        secret: &str,
    ) -> Result<()> {
        // Hash token for storage using HMAC-SHA256 with server secret
        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        type HmacSha256 = Hmac<Sha256>;
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(token.as_bytes());
        let token_hash = format!("{:x}", mac.finalize().into_bytes());

        sqlx::query!(
            r#"
            INSERT INTO auth_session (
                tenant_id,
                session_token_hash,
                session_ip_address,
                session_expires_at
            )
            VALUES ($1, $2, '0.0.0.0'::inet, $3)
            "#,
            tenant_id,
            token_hash,
            expires_at
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get session by token hash
    pub async fn get_session_by_token(&self, token_hash: &str) -> Result<Option<AuthSession>> {
        let session = sqlx::query_as!(
            AuthSession,
            r#"
            SELECT
                session_id,
                tenant_id,
                session_token_hash,
                session_ip_address::TEXT as "session_ip_address!",
                session_user_agent,
                session_created_at,
                session_expires_at,
                session_last_activity_at,
                is_session_active
            FROM auth_session
            WHERE session_token_hash = $1
              AND is_session_active = TRUE
              AND session_expires_at > NOW()
            "#,
            token_hash
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(session)
    }

    // ============================================================================
    // Email OTP Authentication
    // ============================================================================

    /// Store email OTP for tenant
    pub async fn store_email_otp(
        &self,
        tenant_id: &Uuid,
        otp_code: &str,
        expires_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<()> {
        // Delete any existing OTP for this tenant first
        sqlx::query!(
            "DELETE FROM email_otp WHERE tenant_id = $1",
            tenant_id
        )
        .execute(&self.pool)
        .await?;

        // Insert new OTP
        sqlx::query!(
            r#"
            INSERT INTO email_otp (tenant_id, otp_code, expires_at)
            VALUES ($1, $2, $3)
            "#,
            tenant_id,
            otp_code,
            expires_at
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Verify email OTP for tenant with rate limiting
    pub async fn verify_email_otp(&self, tenant_id: &Uuid, otp_code: &str) -> Result<bool> {
        // Check if account is locked
        let lock_check = sqlx::query!(
            r#"
            SELECT locked_until, verification_attempts
            FROM email_otp
            WHERE tenant_id = $1
            "#,
            tenant_id
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(lock_row) = lock_check {
            // Check if account is currently locked
            if let Some(locked_until) = lock_row.locked_until {
                if locked_until > chrono::Utc::now() {
                    return Err(anyhow::anyhow!("Account locked due to too many failed attempts. Try again later."));
                }
            }

            // Check if too many attempts (even if not locked yet)
            if lock_row.verification_attempts >= 5 {
                // Lock the account for 15 minutes
                let locked_until = chrono::Utc::now() + chrono::Duration::minutes(15);
                sqlx::query!(
                    "UPDATE email_otp SET locked_until = $1 WHERE tenant_id = $2",
                    locked_until,
                    tenant_id
                )
                .execute(&self.pool)
                .await?;

                return Err(anyhow::anyhow!("Too many failed attempts. Account locked for 15 minutes."));
            }
        }

        // Fetch OTP record
        let result = sqlx::query!(
            r#"
            SELECT otp_code, expires_at
            FROM email_otp
            WHERE tenant_id = $1
              AND expires_at > NOW()
            "#,
            tenant_id
        )
        .fetch_optional(&self.pool)
        .await?;

        match result {
            Some(row) => {
                let is_valid = row.otp_code == otp_code;

                if is_valid {
                    // Reset attempts on success and delete OTP
                    sqlx::query!(
                        "UPDATE email_otp SET verification_attempts = 0 WHERE tenant_id = $1",
                        tenant_id
                    )
                    .execute(&self.pool)
                    .await?;
                    Ok(true)
                } else {
                    // Increment failed attempts
                    sqlx::query!(
                        r#"
                        UPDATE email_otp
                        SET verification_attempts = verification_attempts + 1,
                            last_attempt_at = NOW()
                        WHERE tenant_id = $1
                        "#,
                        tenant_id
                    )
                    .execute(&self.pool)
                    .await?;
                    Ok(false)
                }
            }
            None => Ok(false),
        }
    }

    /// Delete email OTP for tenant (after successful verification)
    pub async fn delete_email_otp(&self, tenant_id: &Uuid) -> Result<()> {
        sqlx::query!(
            "DELETE FROM email_otp WHERE tenant_id = $1",
            tenant_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // ============================================================================
    // TOTP Authentication
    // ============================================================================

    /// Store TOTP secret for tenant
    pub async fn store_totp_secret(&self, tenant_id: &Uuid, secret: &str) -> Result<()> {
        // Insert or update TOTP credential
        // Note: Requires UNIQUE constraint on (tenant_id, credential_type)
        sqlx::query!(
            r#"
            INSERT INTO auth_credential (
                tenant_id,
                credential_type,
                totp_secret_key,
                credential_name
            )
            VALUES ($1, 'totp', $2, 'TOTP Authenticator')
            ON CONFLICT (tenant_id, credential_type)
            DO UPDATE SET
                totp_secret_key = $2,
                is_credential_active = TRUE
            "#,
            tenant_id,
            secret
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Verify TOTP code for tenant
    pub async fn verify_totp(&self, tenant_id: &Uuid, totp_code: &str) -> Result<bool> {
        // Get TOTP secret
        let result = sqlx::query!(
            r#"
            SELECT totp_secret_key
            FROM auth_credential
            WHERE tenant_id = $1
              AND credential_type = 'totp'
              AND is_credential_active = TRUE
            "#,
            tenant_id
        )
        .fetch_optional(&self.pool)
        .await?;

        match result {
            Some(row) => {
                if let Some(secret) = row.totp_secret_key {
                    // TODO: Implement actual TOTP verification using totp-lite
                    // For now, simple comparison (should use time-based verification)
                    Ok(verify_totp_code(&secret, totp_code))
                } else {
                    Ok(false)
                }
            }
            None => Ok(false),
        }
    }

    // ============================================================================
    // Whitelist Management
    // ============================================================================

    /// Get whitelists for a tenant
    pub async fn get_whitelists(&self, tenant_id: &Uuid) -> Result<Vec<WhiteListEntry>> {
        let entries = sqlx::query_as!(
            WhiteListEntry,
            r#"
            SELECT
                entry_id,
                tenant_id,
                config_id,
                entry_domain_name,
                entry_added_reason,
                entry_created_at,
                is_entry_active
            FROM white_list_entry
            WHERE tenant_id = $1 AND is_entry_active = TRUE
            ORDER BY entry_created_at DESC
            "#,
            tenant_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(entries)
    }

    /// Add domain to whitelist
    pub async fn add_whitelist(
        &self,
        tenant_id: &Uuid,
        config_id: &Uuid,
        domain: &str,
        reason: Option<&str>,
    ) -> Result<WhiteListEntry> {
        let entry = sqlx::query_as!(
            WhiteListEntry,
            r#"
            INSERT INTO white_list_entry (
                tenant_id,
                config_id,
                entry_domain_name,
                entry_added_reason
            )
            VALUES ($1, $2, $3, $4)
            RETURNING
                entry_id,
                tenant_id,
                config_id,
                entry_domain_name,
                entry_added_reason,
                entry_created_at,
                is_entry_active
            "#,
            tenant_id,
            config_id,
            domain,
            reason
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(entry)
    }

    /// Delete whitelist entry
    pub async fn delete_whitelist(&self, tenant_id: &Uuid, entry_id: &Uuid) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE white_list_entry
            SET is_entry_active = FALSE
            WHERE entry_id = $1 AND tenant_id = $2
            "#,
            entry_id,
            tenant_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // ============================================================================
    // Configuration Management
    // ============================================================================

    /// Update tenant configuration
    pub async fn update_tenant_config(
        &self,
        config_id: &Uuid,
        tenant_id: &Uuid,
        config_name: Option<&str>,
        config_description: Option<&str>,
        is_dnssec_enabled: Option<bool>,
        is_logging_enabled: Option<bool>,
        blocked_response_ip: Option<&str>,
    ) -> Result<TenantConfig> {
        let config = sqlx::query_as!(
            TenantConfig,
            r#"
            UPDATE tenant_config
            SET
                config_name = COALESCE($3, config_name),
                config_description = COALESCE($4, config_description),
                is_dnssec_enabled = COALESCE($5, is_dnssec_enabled),
                is_logging_enabled = COALESCE($6, is_logging_enabled),
                blocked_response_ip = COALESCE($7, blocked_response_ip),
                config_updated_at = NOW()
            WHERE config_id = $1 AND tenant_id = $2
            RETURNING
                config_id,
                tenant_id,
                config_name,
                config_description,
                is_logging_enabled,
                is_dnssec_enabled,
                blocked_response_ip::TEXT as "blocked_response_ip?",
                config_created_at,
                config_updated_at,
                is_config_active
            "#,
            config_id,
            tenant_id,
            config_name,
            config_description,
            is_dnssec_enabled,
            is_logging_enabled,
            blocked_response_ip
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(config)
    }
}

/// Verify TOTP code against secret
fn verify_totp_code(secret: &str, code: &str) -> bool {
    use totp_lite::{totp_custom, Sha1};

    // Validate input
    if secret.is_empty() || code.len() != 6 {
        return false;
    }

    // Validate code is numeric
    if !code.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }

    // Decode base32 secret (TOTP secrets are typically base32 encoded)
    // Try both padded and unpadded base32 formats
    let secret_upper = secret.to_uppercase();
    let secret_bytes = match data_encoding::BASE32.decode(secret_upper.as_bytes()) {
        Ok(bytes) => bytes,
        Err(_) => {
            // Try unpadded base32
            match data_encoding::BASE32_NOPAD.decode(secret_upper.as_bytes()) {
                Ok(bytes) => bytes,
                Err(_) => return false, // Invalid base32 secret
            }
        }
    };

    // Get current Unix timestamp
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Check current time window and ±1 window for clock skew tolerance
    for time_offset in [-1i64, 0, 1] {
        let time = (now as i64 + (time_offset * 30)) as u64;
        let totp = totp_custom::<Sha1>(30, 6, &secret_bytes, time);

        // Format TOTP as 6-digit zero-padded string and use constant-time comparison
        let totp_str = format!("{:06}", totp);

        // Use constant-time comparison to prevent timing attacks
        use subtle::ConstantTimeEq;
        if totp_str.as_bytes().ct_eq(code.as_bytes()).into() {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests will be added in a separate commit
    // For now, we'll skip tests and focus on implementation
}
