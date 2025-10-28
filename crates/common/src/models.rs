use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

// ============================================================================
// Tenant Account Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TenantAccount {
    pub tenant_id: Uuid,
    pub email_address: String,
    pub tenant_identifier: String,
    pub subscription_tier: SubscriptionTier,
    pub subscription_status: SubscriptionStatus,
    pub account_created_at: DateTime<Utc>,
    pub account_updated_at: DateTime<Utc>,
    pub is_account_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "subscription_tier_enum", rename_all = "lowercase")]
pub enum SubscriptionTier {
    Free,
    Pro,
    Family,
    Business,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "subscription_status_enum", rename_all = "lowercase")]
pub enum SubscriptionStatus {
    Active,
    Expired,
    Cancelled,
    Suspended,
}

// ============================================================================
// Tenant Config Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TenantConfig {
    pub config_id: Uuid,
    pub tenant_id: Uuid,
    pub config_name: String,
    pub config_description: Option<String>,
    pub is_logging_enabled: bool,
    pub is_dnssec_enabled: bool,
    pub blocked_response_ip: Option<String>,  // Nullable in DB schema
    pub config_created_at: DateTime<Utc>,
    pub config_updated_at: DateTime<Utc>,
    pub is_config_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantConfigWithCategories {
    #[serde(flatten)]
    pub config: TenantConfig,
    pub enabled_categories: Vec<BlockCategory>,
}

// ============================================================================
// Block List Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BlockListEntry {
    pub block_entry_id: i64,
    pub block_domain_name: String,
    pub source_id: Option<Uuid>,
    pub block_category: BlockCategory,
    pub block_reason: Option<String>,
    pub block_added_at: DateTime<Utc>,
    pub block_updated_at: DateTime<Utc>,
    pub is_block_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "block_category_enum", rename_all = "snake_case")]
pub enum BlockCategory {
    Advertising,
    MalwarePhishing,
    AdultContent,
    GamblingBetting,
    SocialMedia,
    StreamingMedia,
    TrackingTelemetry,
    Cryptomining,
    PiracyTorrents,
    CustomUser,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BlockListSource {
    pub source_id: Uuid,
    pub source_name: String,
    pub source_url: String,
    pub source_format: String,
    pub source_category: BlockCategory,
    pub source_description: Option<String>,
    pub source_last_updated_at: Option<DateTime<Utc>>,
    pub source_next_update_at: Option<DateTime<Utc>>,
    pub source_update_frequency_hours: i32,
    pub is_source_active: bool,
}

// ============================================================================
// White List Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct WhiteListEntry {
    pub entry_id: Uuid,
    pub tenant_id: Uuid,
    pub config_id: Uuid,
    pub entry_domain_name: String,
    pub entry_added_reason: Option<String>,
    pub entry_created_at: DateTime<Utc>,
    pub is_entry_active: bool,
}

// ============================================================================
// DNS Query Log Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DnsQueryLog {
    pub query_id: i64,
    pub tenant_id: Uuid,
    pub config_id: Option<Uuid>,
    pub query_timestamp: DateTime<Utc>,
    pub query_domain_name: String,
    pub query_type: DnsQueryType,
    pub query_result_type: DnsResultType,
    pub response_ip_address: Option<String>,
    pub query_latency_ms: Option<i32>,  // i32 to avoid overflow (max 32767ms too low)
    pub query_source_protocol: Option<String>,
    pub query_source_ip: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "dns_query_type_enum")]
pub enum DnsQueryType {
    A,
    AAAA,
    CNAME,
    MX,
    TXT,
    NS,
    SOA,
    PTR,
    SRV,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "dns_result_type_enum", rename_all = "lowercase")]
pub enum DnsResultType {
    Allowed,
    Blocked,
    Whitelisted,
    Error,
}

// ============================================================================
// Authentication Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AuthCredential {
    pub credential_id: Uuid,
    pub tenant_id: Uuid,
    pub credential_type: CredentialType,
    pub credential_public_key: Option<Vec<u8>>,
    pub credential_counter: i64,
    pub totp_secret_key: Option<String>,
    pub credential_name: Option<String>,
    pub credential_created_at: DateTime<Utc>,
    pub credential_last_used_at: Option<DateTime<Utc>>,
    pub is_credential_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "credential_type_enum", rename_all = "snake_case")]
pub enum CredentialType {
    Fido2,
    Totp,
    EmailOtp,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AuthSession {
    pub session_id: Uuid,
    pub tenant_id: Uuid,
    pub session_token_hash: String,
    pub session_ip_address: String,
    pub session_user_agent: Option<String>,
    pub session_created_at: DateTime<Utc>,
    pub session_expires_at: DateTime<Utc>,
    pub session_last_activity_at: DateTime<Utc>,
    pub is_session_active: bool,
}

// ============================================================================
// IP Binding Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TenantIpBinding {
    pub binding_id: Uuid,
    pub tenant_id: Uuid,
    pub config_id: Uuid,
    pub ip_address: String,
    pub ip_description: Option<String>,
    pub binding_created_at: DateTime<Utc>,
    pub binding_expires_at: Option<DateTime<Utc>>,
    pub is_binding_active: bool,
}

// ============================================================================
// Analytics Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AnalyticsHourlySummary {
    pub summary_id: Uuid,
    pub tenant_id: Uuid,
    pub hour_timestamp: DateTime<Utc>,
    pub total_query_count: i64,
    pub blocked_query_count: i64,
    pub whitelisted_query_count: i64,
    pub avg_query_latency_ms: Option<i16>,
    pub top_blocked_domains: Option<serde_json::Value>,
    pub top_allowed_domains: Option<serde_json::Value>,
}

// ============================================================================
// Subscription Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TenantSubscription {
    pub subscription_id: Uuid,
    pub tenant_id: Uuid,
    pub lemon_squeezy_subscription_id: Option<String>,
    pub lemon_squeezy_customer_id: Option<String>,
    pub subscription_tier: SubscriptionTier,
    pub subscription_status: SubscriptionStatus,
    pub subscription_started_at: DateTime<Utc>,
    pub subscription_expires_at: Option<DateTime<Utc>>,
    pub subscription_cancelled_at: Option<DateTime<Utc>>,
    pub monthly_query_limit: Option<i64>,
    pub config_limit: i32,
    pub subscription_metadata: Option<serde_json::Value>,
    pub is_subscription_active: bool,
}

// ============================================================================
// Helper Functions
// ============================================================================

impl SubscriptionTier {
    pub fn monthly_query_limit(&self) -> Option<i64> {
        match self {
            SubscriptionTier::Free => Some(300_000),
            _ => None, // Unlimited
        }
    }

    pub fn config_limit(&self) -> i32 {
        match self {
            SubscriptionTier::Free => 1,
            SubscriptionTier::Pro => 10,
            SubscriptionTier::Family => 10,
            SubscriptionTier::Business => 50,
        }
    }
}

impl std::fmt::Display for BlockCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BlockCategory::Advertising => write!(f, "advertising"),
            BlockCategory::MalwarePhishing => write!(f, "malware_phishing"),
            BlockCategory::AdultContent => write!(f, "adult_content"),
            BlockCategory::GamblingBetting => write!(f, "gambling_betting"),
            BlockCategory::SocialMedia => write!(f, "social_media"),
            BlockCategory::StreamingMedia => write!(f, "streaming_media"),
            BlockCategory::TrackingTelemetry => write!(f, "tracking_telemetry"),
            BlockCategory::Cryptomining => write!(f, "cryptomining"),
            BlockCategory::PiracyTorrents => write!(f, "piracy_torrents"),
            BlockCategory::CustomUser => write!(f, "custom_user"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subscription_tier_query_limits() {
        assert_eq!(SubscriptionTier::Free.monthly_query_limit(), Some(300_000));
        assert_eq!(SubscriptionTier::Pro.monthly_query_limit(), None);
        assert_eq!(SubscriptionTier::Family.monthly_query_limit(), None);
        assert_eq!(SubscriptionTier::Business.monthly_query_limit(), None);
    }

    #[test]
    fn test_subscription_tier_config_limits() {
        assert_eq!(SubscriptionTier::Free.config_limit(), 1);
        assert_eq!(SubscriptionTier::Pro.config_limit(), 10);
        assert_eq!(SubscriptionTier::Family.config_limit(), 10);
        assert_eq!(SubscriptionTier::Business.config_limit(), 50);
    }

    #[test]
    fn test_block_category_display() {
        assert_eq!(BlockCategory::Advertising.to_string(), "advertising");
        assert_eq!(BlockCategory::MalwarePhishing.to_string(), "malware_phishing");
        assert_eq!(BlockCategory::AdultContent.to_string(), "adult_content");
        assert_eq!(BlockCategory::GamblingBetting.to_string(), "gambling_betting");
        assert_eq!(BlockCategory::SocialMedia.to_string(), "social_media");
        assert_eq!(BlockCategory::StreamingMedia.to_string(), "streaming_media");
        assert_eq!(BlockCategory::TrackingTelemetry.to_string(), "tracking_telemetry");
        assert_eq!(BlockCategory::Cryptomining.to_string(), "cryptomining");
        assert_eq!(BlockCategory::PiracyTorrents.to_string(), "piracy_torrents");
        assert_eq!(BlockCategory::CustomUser.to_string(), "custom_user");
    }

    #[test]
    fn test_tenant_account_serialization() {
        let tenant = TenantAccount {
            tenant_id: Uuid::new_v4(),
            email_address: "test@example.com".to_string(),
            tenant_identifier: "test123".to_string(),
            subscription_tier: SubscriptionTier::Free,
            subscription_status: SubscriptionStatus::Active,
            account_created_at: Utc::now(),
            account_updated_at: Utc::now(),
            is_account_active: true,
        };

        let json = serde_json::to_string(&tenant).expect("Failed to serialize");
        let deserialized: TenantAccount = serde_json::from_str(&json).expect("Failed to deserialize");
        
        assert_eq!(tenant.tenant_id, deserialized.tenant_id);
        assert_eq!(tenant.email_address, deserialized.email_address);
        assert_eq!(tenant.tenant_identifier, deserialized.tenant_identifier);
    }

    #[test]
    fn test_dns_result_type_variants() {
        let allowed = DnsResultType::Allowed;
        let blocked = DnsResultType::Blocked;
        let whitelisted = DnsResultType::Whitelisted;
        let error = DnsResultType::Error;

        // Test serialization
        assert_eq!(serde_json::to_string(&allowed).unwrap(), "\"allowed\"");
        assert_eq!(serde_json::to_string(&blocked).unwrap(), "\"blocked\"");
        assert_eq!(serde_json::to_string(&whitelisted).unwrap(), "\"whitelisted\"");
        assert_eq!(serde_json::to_string(&error).unwrap(), "\"error\"");
    }

    #[test]
    fn test_dns_query_type_variants() {
        let types = vec![
            DnsQueryType::A,
            DnsQueryType::AAAA,
            DnsQueryType::CNAME,
            DnsQueryType::MX,
            DnsQueryType::TXT,
            DnsQueryType::NS,
            DnsQueryType::SOA,
            DnsQueryType::PTR,
            DnsQueryType::SRV,
        ];

        // All should serialize without error
        for query_type in types {
            let json = serde_json::to_string(&query_type).expect("Serialization failed");
            assert!(!json.is_empty());
        }
    }

    #[test]
    fn test_credential_type_variants() {
        let fido2 = CredentialType::Fido2;
        let totp = CredentialType::Totp;
        let email_otp = CredentialType::EmailOtp;

        // Test serialization with snake_case
        assert_eq!(serde_json::to_string(&fido2).unwrap(), "\"fido2\"");
        assert_eq!(serde_json::to_string(&totp).unwrap(), "\"totp\"");
        assert_eq!(serde_json::to_string(&email_otp).unwrap(), "\"email_otp\"");
    }

    #[test]
    fn test_subscription_status_variants() {
        let statuses = vec![
            SubscriptionStatus::Active,
            SubscriptionStatus::Expired,
            SubscriptionStatus::Cancelled,
            SubscriptionStatus::Suspended,
        ];

        for status in statuses {
            let json = serde_json::to_string(&status).unwrap();
            let deserialized: SubscriptionStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(format!("{:?}", status), format!("{:?}", deserialized));
        }
    }

    #[test]
    fn test_tenant_config_structure() {
        let config = TenantConfig {
            config_id: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            config_name: "Default".to_string(),
            config_description: Some("Default configuration".to_string()),
            is_logging_enabled: true,
            is_dnssec_enabled: false,
            blocked_response_ip: Some("0.0.0.0".to_string()),
            config_created_at: Utc::now(),
            config_updated_at: Utc::now(),
            is_config_active: true,
        };

        assert!(config.is_logging_enabled);
        assert!(!config.is_dnssec_enabled);
        assert_eq!(config.blocked_response_ip, Some("0.0.0.0".to_string()));
    }

    #[test]
    fn test_blocklist_entry_creation() {
        let entry = BlockListEntry {
            block_entry_id: 1,
            block_domain_name: "malicious.com".to_string(),
            source_id: Some(Uuid::new_v4()),
            block_category: BlockCategory::MalwarePhishing,
            block_reason: Some("Known malware distributor".to_string()),
            block_added_at: Utc::now(),
            block_updated_at: Utc::now(),
            is_block_active: true,
        };

        assert_eq!(entry.block_domain_name, "malicious.com");
        assert!(entry.is_block_active);
        assert!(entry.block_reason.is_some());
    }

    #[test]
    fn test_whitelist_entry_creation() {
        let entry = WhiteListEntry {
            entry_id: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            config_id: Uuid::new_v4(),
            entry_domain_name: "trusted.com".to_string(),
            entry_added_reason: Some("Business partner".to_string()),
            entry_created_at: Utc::now(),
            is_entry_active: true,
        };

        assert_eq!(entry.entry_domain_name, "trusted.com");
        assert!(entry.is_entry_active);
    }

    #[test]
    fn test_dns_query_log_serialization() {
        let log = DnsQueryLog {
            query_id: 12345,
            tenant_id: Uuid::new_v4(),
            config_id: Some(Uuid::new_v4()),
            query_timestamp: Utc::now(),
            query_domain_name: "example.com".to_string(),
            query_type: DnsQueryType::A,
            query_result_type: DnsResultType::Allowed,
            response_ip_address: Some("93.184.216.34".to_string()),
            query_latency_ms: Some(42),
            query_source_protocol: Some("DoH".to_string()),
            query_source_ip: Some("192.168.1.100".to_string()),
        };

        let json = serde_json::to_string(&log).expect("Serialization failed");
        let deserialized: DnsQueryLog = serde_json::from_str(&json).expect("Deserialization failed");
        
        assert_eq!(log.query_domain_name, deserialized.query_domain_name);
        assert_eq!(log.query_latency_ms, deserialized.query_latency_ms);
    }

    #[test]
    fn test_auth_session_fields() {
        let session = AuthSession {
            session_id: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            session_token_hash: "abc123hash".to_string(),
            session_ip_address: "192.168.1.1".to_string(),
            session_user_agent: Some("Mozilla/5.0".to_string()),
            session_created_at: Utc::now(),
            session_expires_at: Utc::now() + chrono::Duration::hours(1),
            session_last_activity_at: Utc::now(),
            is_session_active: true,
        };

        assert!(session.is_session_active);
        assert!(session.session_expires_at > session.session_created_at);
        assert!(session.session_user_agent.is_some());
    }

    #[test]
    fn test_ip_binding_structure() {
        let binding = TenantIpBinding {
            binding_id: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            config_id: Uuid::new_v4(),
            ip_address: "10.0.0.5".to_string(),
            ip_description: Some("Home network".to_string()),
            binding_created_at: Utc::now(),
            binding_expires_at: Some(Utc::now() + chrono::Duration::days(30)),
            is_binding_active: true,
        };

        assert_eq!(binding.ip_address, "10.0.0.5");
        assert!(binding.binding_expires_at.is_some());
    }

    #[test]
    fn test_subscription_tier_ordering() {
        // Test that free tier has most restrictive limits
        assert!(SubscriptionTier::Free.config_limit() < SubscriptionTier::Pro.config_limit());
        assert!(SubscriptionTier::Free.config_limit() < SubscriptionTier::Business.config_limit());
        
        // Test that free tier has query limit while others don't
        assert!(SubscriptionTier::Free.monthly_query_limit().is_some());
        assert!(SubscriptionTier::Pro.monthly_query_limit().is_none());
    }

    #[test]
    fn test_block_category_serialization_roundtrip() {
        let categories = vec![
            BlockCategory::Advertising,
            BlockCategory::MalwarePhishing,
            BlockCategory::AdultContent,
            BlockCategory::GamblingBetting,
            BlockCategory::SocialMedia,
            BlockCategory::StreamingMedia,
            BlockCategory::TrackingTelemetry,
            BlockCategory::Cryptomining,
            BlockCategory::PiracyTorrents,
            BlockCategory::CustomUser,
        ];

        for category in categories {
            let json = serde_json::to_string(&category).unwrap();
            let deserialized: BlockCategory = serde_json::from_str(&json).unwrap();
            assert_eq!(category.to_string(), deserialized.to_string());
        }
    }
}