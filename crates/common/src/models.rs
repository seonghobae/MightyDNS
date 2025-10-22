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
    pub blocked_response_ip: String,
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
    pub white_entry_id: Uuid,
    pub tenant_id: Uuid,
    pub white_domain_name: String,
    pub white_reason: Option<String>,
    pub white_added_at: DateTime<Utc>,
    pub is_white_active: bool,
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
    pub query_latency_ms: Option<i16>,
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
